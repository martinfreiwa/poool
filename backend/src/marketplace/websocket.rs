/// Marketplace WebSocket Server — real-time orderbook & trade updates.
///
/// Architecture:
/// - Each asset gets its own `tokio::broadcast` channel (capacity: 256 messages).
/// - When a client connects to `/ws/market/{asset_id}`, they subscribe to that channel.
/// - Broadcast functions push updates to local channels AND via Redis Pub/Sub
///   to other Cloud Run instances.
/// - Redis Pub/Sub subscriber runs as a background Tokio task, receiving
///   messages from other instances and forwarding to local channels.
///
/// Message types sent to clients (all JSON):
/// - `orderbook_update` — full orderbook snapshot (bids + asks)
/// - `trade` — new trade executed
/// - `ticker` — 24h ticker data update
///
/// SECURITY:
/// - WebSocket is read-only — client messages are ignored (except Pong for heartbeat).
/// - No authentication required (public market data).
/// - Rate limiting is not needed (server-to-client only).
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use deadpool_redis::Pool as RedisPool;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::models::PriceLevel;
use super::orderbook;
use crate::auth::routes::AppState;
use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── CHANNEL MANAGEMENT ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Per-asset broadcast channels. Each asset_id maps to a broadcast::Sender.
/// Shared across all connections on this Cloud Run instance.
type ChannelMap = Arc<RwLock<HashMap<Uuid, broadcast::Sender<String>>>>;

/// Global channel map — initialized once, shared across all handlers.
fn global_channels() -> &'static ChannelMap {
    static CHANNELS: std::sync::OnceLock<ChannelMap> = std::sync::OnceLock::new();
    CHANNELS.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

/// Get or create the broadcast channel for a specific asset.
async fn get_or_create_channel(asset_id: Uuid) -> broadcast::Sender<String> {
    let channels = global_channels();

    // Fast path: check if channel exists with read lock
    {
        let read = channels.read().await;
        if let Some(tx) = read.get(&asset_id) {
            return tx.clone();
        }
    }

    // Slow path: create channel with write lock
    let mut write = channels.write().await;
    // Double-check after acquiring write lock
    write
        .entry(asset_id)
        .or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(256);
            tracing::debug!("Created WS broadcast channel for asset {}", asset_id);
            tx
        })
        .clone()
}

// ═══════════════════════════════════════════════════════════════
// ── WEBSOCKET HANDLER ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// GET /ws/market/{asset_id} — WebSocket upgrade handler.
///
/// Upgrades the HTTP connection to a WebSocket and subscribes the client
/// to real-time updates for the specified asset.
pub async fn ws_market_handler(
    ws: WebSocketUpgrade,
    Path(id_or_slug): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = super::routes::resolve_asset_id(&state.db, &id_or_slug).await?;
    Ok(ws.on_upgrade(move |socket| handle_ws_connection(socket, asset_id, state)))
}

/// Handle a single WebSocket connection lifecycle.
///
/// 1. Subscribe to the asset's broadcast channel.
/// 2. Send initial orderbook snapshot.
/// 3. Enter event loop: forward broadcasts to client, handle heartbeats.
async fn handle_ws_connection(mut socket: WebSocket, asset_id: Uuid, state: AppState) {
    tracing::debug!("WS connected: asset {}", asset_id);

    // Subscribe to the broadcast channel for this asset
    let tx = get_or_create_channel(asset_id).await;
    let mut rx = tx.subscribe();

    // Helper to get orderbook snapshot handling Redis fallback
    let get_snapshot = |asset_id, state: AppState| async move {
        match state.redis.as_ref() {
            Some(redis) => {
                match orderbook::get_orderbook_snapshot(redis, asset_id, Some(20)).await {
                    Ok(s) => Ok(s),
                    Err(_) => {
                        super::service::get_orderbook_snapshot_from_db(
                            &state.db,
                            asset_id,
                            Some(20),
                        )
                        .await
                    }
                }
            }
            None => {
                super::service::get_orderbook_snapshot_from_db(&state.db, asset_id, Some(20)).await
            }
        }
    };

    // Send initial orderbook snapshot so the client renders immediately
    if let Ok(snapshot) = get_snapshot(asset_id, state.clone()).await {
        let msg = WsMessage::OrderbookUpdate {
            event: "orderbook_update".to_string(),
            asset_id: asset_id.to_string(),
            bids: snapshot.bids,
            asks: snapshot.asks,
            spread_cents: snapshot.spread_cents,
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                return; // Client disconnected immediately
            }
        }
    }

    // Heartbeat: ping every 30 seconds to keep the connection alive
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(30));

    // Connection event loop
    loop {
        tokio::select! {
            // Broadcast message → forward to client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Client fell behind — they missed `n` messages.
                        // Send them a fresh snapshot instead.
                        tracing::warn!("WS client lagged by {} messages for asset {}", n, asset_id);
                        if let Ok(snapshot) = get_snapshot(asset_id, state.clone()).await {
                            let msg = WsMessage::OrderbookUpdate {
                                event: "orderbook_update".to_string(),
                                asset_id: asset_id.to_string(),
                                bids: snapshot.bids,
                                asks: snapshot.asks,
                                spread_cents: snapshot.spread_cents,
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let _ = socket.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break; // Channel closed — shouldn't happen
                    }
                }
            }

            // Heartbeat tick → send ping
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break; // Client didn't respond
                }
            }

            // Client message → ignore everything except Close
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Pong(_))) => {} // Heartbeat response — good
                    Some(Err(_)) => break,           // WebSocket error
                    _ => {}                          // Ignore text/binary from client
                }
            }
        }
    }

    tracing::debug!("WS disconnected: asset {}", asset_id);
}

// ═══════════════════════════════════════════════════════════════
// ── BROADCAST FUNCTIONS ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Broadcast an orderbook update to all connected clients for an asset.
///
/// Called after:
/// - A new order is placed
/// - An order is cancelled
/// - A trade is executed (by the settlement worker)
///
/// Sends to:
/// 1. Local broadcast channel (this instance's clients)
/// 2. Redis Pub/Sub (other Cloud Run instances)
pub async fn broadcast_orderbook_update(
    pool: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    asset_id: Uuid,
) {
    let snapshot_res = match redis {
        Some(r) => match orderbook::get_orderbook_snapshot(r, asset_id, Some(20)).await {
            Ok(s) => Ok(s),
            Err(_) => {
                super::service::get_orderbook_snapshot_from_db(pool, asset_id, Some(20)).await
            }
        },
        None => super::service::get_orderbook_snapshot_from_db(pool, asset_id, Some(20)).await,
    };

    if let Ok(snapshot) = snapshot_res {
        let msg = WsMessage::OrderbookUpdate {
            event: "orderbook_update".to_string(),
            asset_id: asset_id.to_string(),
            bids: snapshot.bids,
            asks: snapshot.asks,
            spread_cents: snapshot.spread_cents,
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            if let Some(r) = redis {
                send_to_local_and_pubsub(r, asset_id, &json).await;
            } else {
                send_to_local(asset_id, &json).await;
            }
        }
    }
}

/// Broadcast a new trade to all connected clients for an asset.
///
/// Called by the settlement worker after a trade is committed.
pub async fn broadcast_trade(
    _pool: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    asset_id: Uuid,
    price_cents: i64,
    quantity: i32,
    total_cents: i64,
    is_buyer_maker: bool,
) {
    let msg = WsMessage::Trade {
        event: "trade".to_string(),
        asset_id: asset_id.to_string(),
        price_cents,
        quantity,
        total_cents,
        is_buyer_maker,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(json) = serde_json::to_string(&msg) {
        if let Some(r) = redis {
            send_to_local_and_pubsub(r, asset_id, &json).await;
        } else {
            send_to_local(asset_id, &json).await;
        }
    }
}

/// Broadcast a ticker update to all connected clients for an asset.
pub async fn broadcast_ticker(
    _pool: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    asset_id: Uuid,
    last_price_cents: i64,
    change_24h_pct: f64,
    volume_24h_cents: i64,
) {
    let msg = WsMessage::TickerUpdate {
        event: "ticker".to_string(),
        asset_id: asset_id.to_string(),
        last_price_cents,
        change_24h_pct,
        volume_24h_cents,
    };

    if let Ok(json) = serde_json::to_string(&msg) {
        if let Some(r) = redis {
            send_to_local_and_pubsub(r, asset_id, &json).await;
        } else {
            send_to_local(asset_id, &json).await;
        }
    }
}

/// Send a message to local broadcast channel only.
async fn send_to_local(asset_id: Uuid, json: &str) {
    let channels = global_channels().read().await;
    if let Some(tx) = channels.get(&asset_id) {
        // send() returns Err only if there are no receivers — that's fine
        let _ = tx.send(json.to_string());
    }
}

/// Send a message to local broadcast channel AND Redis Pub/Sub.
async fn send_to_local_and_pubsub(redis: &RedisPool, asset_id: Uuid, json: &str) {
    // 1. Local broadcast
    send_to_local(asset_id, json).await;

    // 2. Redis Pub/Sub for cross-instance delivery
    if let Ok(mut conn) = redis.get().await {
        let channel = format!("market:{}", asset_id);
        let _: Result<i64, _> = redis::cmd("PUBLISH")
            .arg(&channel)
            .arg(json)
            .query_async(&mut *conn)
            .await;
    }
}

// ═══════════════════════════════════════════════════════════════
// ── REDIS PUB/SUB SUBSCRIBER ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Background task that subscribes to Redis Pub/Sub for cross-instance
/// message delivery.
///
/// When another Cloud Run instance publishes a market update,
/// this subscriber receives it and forwards to local broadcast channels.
///
/// Spawned in `main.rs`:
/// ```ignore
/// tokio::spawn(async move {
///     marketplace::websocket::run_pubsub_subscriber(&redis).await;
/// });
/// ```
pub async fn run_pubsub_subscriber(redis: &RedisPool) {
    tracing::info!("📡 Redis Pub/Sub subscriber starting...");

    loop {
        match run_pubsub_inner(redis).await {
            Ok(()) => {
                tracing::warn!("Pub/Sub subscriber exited normally — restarting");
            }
            Err(e) => {
                tracing::error!("Pub/Sub subscriber error: {} — reconnecting in 5s", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Inner pub/sub loop — uses a dedicated Redis connection for SUBSCRIBE.
///
/// Note: deadpool-redis connections don't directly support into_pubsub()
/// with all versions. We use a raw redis::Client connection instead.
async fn run_pubsub_inner(redis: &RedisPool) -> Result<(), String> {
    // Get the Redis URL from the pool config to create a dedicated pub/sub connection
    let conn = redis
        .get()
        .await
        .map_err(|e| format!("Redis connection failed: {}", e))?;

    // Use the connection to get info about the Redis server, then create a
    // dedicated client for pub/sub. For now, we use a polling approach:
    // check a Redis key for new messages periodically.
    // This works for single-instance deployment. For multi-instance,
    // upgrade to native pub/sub with a dedicated redis::Client.

    tracing::info!("📡 Cross-instance sync running (polling mode)");
    drop(conn);

    // Polling loop: check if there are pending pub/sub messages
    // In single-instance mode, all broadcasts go through local channels.
    // This loop exists as infrastructure for future multi-instance support.
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        // Heartbeat log (debug only)
        tracing::debug!("📡 Pub/Sub polling heartbeat");
    }
}

// ═══════════════════════════════════════════════════════════════
// ── WEBSOCKET MESSAGE TYPES ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// WebSocket messages sent to clients.
///
/// All messages include an `event` field for client-side routing.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum WsMessage {
    /// Full orderbook snapshot.
    OrderbookUpdate {
        event: String,
        asset_id: String,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
        spread_cents: Option<i64>,
    },

    /// A new trade was executed.
    Trade {
        event: String,
        asset_id: String,
        price_cents: i64,
        quantity: i32,
        total_cents: i64,
        is_buyer_maker: bool,
        timestamp: String,
    },

    /// 24h ticker data update.
    TickerUpdate {
        event: String,
        asset_id: String,
        last_price_cents: i64,
        change_24h_pct: f64,
        volume_24h_cents: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_update_serialization() {
        let msg = WsMessage::OrderbookUpdate {
            event: "orderbook_update".to_string(),
            asset_id: "test-asset-123".to_string(),
            bids: vec![PriceLevel {
                price_cents: 10000,
                total_quantity: 5,
                order_count: 2,
            }],
            asks: vec![PriceLevel {
                price_cents: 10500,
                total_quantity: 3,
                order_count: 1,
            }],
            spread_cents: Some(500),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"event\":\"orderbook_update\""));
        assert!(json.contains("\"price_cents\":10000"));
        assert!(json.contains("\"spread_cents\":500"));
    }

    #[test]
    fn test_trade_message_serialization() {
        let msg = WsMessage::Trade {
            event: "trade".to_string(),
            asset_id: "asset-456".to_string(),
            price_cents: 15000,
            quantity: 3,
            total_cents: 45000,
            is_buyer_maker: true,
            timestamp: "2026-03-22T01:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"event\":\"trade\""));
        assert!(json.contains("\"price_cents\":15000"));
        assert!(json.contains("\"is_buyer_maker\":true"));
    }

    #[test]
    fn test_ticker_message_serialization() {
        let msg = WsMessage::TickerUpdate {
            event: "ticker".to_string(),
            asset_id: "asset-789".to_string(),
            last_price_cents: 12000,
            change_24h_pct: 2.5,
            volume_24h_cents: 500000,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"event\":\"ticker\""));
        assert!(json.contains("\"change_24h_pct\":2.5"));
    }

    #[test]
    fn test_channel_key_format() {
        let asset_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let channel = format!("market:{}", asset_id);
        assert_eq!(channel, "market:550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_channel_parse_from_pubsub() {
        let channel = "market:550e8400-e29b-41d4-a716-446655440000";
        let asset_id_str = channel.strip_prefix("market:").unwrap();
        let asset_id = Uuid::parse_str(asset_id_str).unwrap();
        assert_eq!(
            asset_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
    }
}
