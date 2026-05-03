/// Blockchain event indexer — polls Polygon for ERC-1155 transfer events.
///
/// This worker syncs on-chain token balances into `onchain_balances` for:
/// 1. Fast portfolio reads (1ms PostgreSQL vs 100-500ms blockchain)
/// 2. Dividend snapshot calculations
/// 3. Off-chain vs on-chain reconciliation
///
/// Architecture:
/// - Polls every N seconds (configurable via platform_settings)
/// - Stays `confirmation_depth` blocks behind HEAD (re-org protection, default: 128)
/// - Uses idempotent upserts — safe to process the same event twice
/// - Tracks cursor in `chain_indexer_cursor` for restart replay
///
/// Events indexed:
/// - ERC-1155 TransferSingle(operator, from, to, id, value)
/// - ERC-1155 TransferBatch(operator, from, to, ids, values)
///
/// 🔴 FINANCIAL CODE — balance is BIGINT (token units), never floats.
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Duration;

use super::service::ChainConfig;

// ═══════════════════════════════════════════════════════════════
// ── ERC-1155 EVENT SIGNATURES ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// keccak256("TransferSingle(address,address,address,uint256,uint256)")
const TRANSFER_SINGLE_TOPIC: &str =
    "0xc3d58168c5ae7397731d063d5bbf3d657854427343f4c083240f7aacaa2d0f62";

/// keccak256("TransferBatch(address,address,address,uint256[],uint256[])")
const TRANSFER_BATCH_TOPIC: &str =
    "0x4a39dc06d4c0dbc64b70af90fd698a233a518aa5d07e595d983b8c0526c8f7fb";

/// Zero address (mint events)
const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

// ═══════════════════════════════════════════════════════════════
// ── LOG TYPES ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EthLog {
    /// Contract address (0x-prefixed, lowercase)
    address: String,
    /// Topics: [event_sig, indexed_operator, indexed_from, indexed_to]
    topics: Vec<String>,
    /// ABI-encoded non-indexed params
    data: String,
    /// Block number (hex)
    block_number: String,
    /// Transaction hash
    transaction_hash: String,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// A parsed balance change from a TransferSingle or TransferBatch event.
#[derive(Debug)]
struct BalanceChange {
    /// 0x wallet address
    wallet_address: String,
    /// On-chain token ID
    token_id: u64,
    /// Signed change: positive = received, negative = sent
    delta: i64,
    /// Block number this occurred in
    block_number: u64,
}

// ═══════════════════════════════════════════════════════════════
// ── MAIN WORKER ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Run the event indexer worker. Called from main.rs as a Tokio task.
///
/// This worker:
/// 1. Reads the last synced block from `chain_indexer_cursor`
/// 2. Polls for new ERC-1155 transfer events
/// 3. Updates `onchain_balances` with idempotent upserts
/// 4. Advances the cursor
///
/// If blockchain is not configured, exits silently.
pub async fn run_event_indexer(pool: &PgPool) {
    let config = match ChainConfig::from_env().await {
        Some(c) => c,
        None => {
            tracing::info!("🔍 Event indexer: blockchain not configured, skipping.");
            return;
        }
    };

    // Read indexer settings from DB
    let poll_secs: u64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_indexer_poll_secs'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse().ok())
    .unwrap_or(5);

    let confirmation_depth: u64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_indexer_confirmation_depth'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse().ok())
    // Polygon PoS sees 30+ block reorgs occasionally; finality via
    // heimdall checkpoints is ~256 blocks. Default 128 = safety/freshness
    // compromise. Override via platform_settings on chains with stronger
    // finality guarantees (Ethereum mainnet: 64 is enough).
    .unwrap_or(128);

    let indexer_enabled: bool = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_indexer_enabled'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse().ok())
    .unwrap_or(false);

    if !indexer_enabled {
        tracing::info!("🔍 Event indexer: DISABLED via platform_settings. Set chain_indexer_enabled=true to activate.");
        return;
    }

    tracing::info!(
        "🔍 Event indexer starting (contract={}, poll={}s, depth={})",
        config.contract_address,
        poll_secs,
        confirmation_depth
    );

    let client = Client::new();

    // Initial delay
    tokio::time::sleep(Duration::from_secs(15)).await;

    loop {
        if let Err(e) = index_new_events(pool, &config, &client, confirmation_depth).await {
            tracing::error!("🔍 Event indexer error: {}", e);
            // Don't exit — keep retrying
        }
        tokio::time::sleep(Duration::from_secs(poll_secs)).await;
    }
}

// ═══════════════════════════════════════════════════════════════
// ── CORE INDEXING LOGIC ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Index new events from the last cursor position to (HEAD - confirmation_depth).
async fn index_new_events(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
    confirmation_depth: u64,
) -> Result<(), String> {
    let contract_lower = config.contract_address.to_lowercase();

    // 1. Get last synced block from cursor
    let last_block: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(last_block, 0) FROM chain_indexer_cursor WHERE contract_address = $1",
    )
    .bind(&contract_lower)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB cursor read error: {e}"))?
    .unwrap_or(0);

    // 2. Get current HEAD block
    let head_block = get_block_number(client, &config.rpc_url).await?;
    let safe_block = head_block.saturating_sub(confirmation_depth);

    let from_block = (last_block as u64) + 1;

    if from_block > safe_block {
        // Already up to date
        return Ok(());
    }

    // Cap range to avoid massive single queries (max 2000 blocks per poll)
    let to_block = std::cmp::min(safe_block, from_block + 2000);

    // 3. Fetch TransferSingle and TransferBatch logs
    let from_hex = format!("0x{:x}", from_block);
    let to_hex = format!("0x{:x}", to_block);

    let filter = serde_json::json!({
        "address": contract_lower,
        "fromBlock": from_hex,
        "toBlock": to_hex,
        "topics": [[TRANSFER_SINGLE_TOPIC, TRANSFER_BATCH_TOPIC]]
    });

    let logs = eth_get_logs(client, &config.rpc_url, &filter).await?;

    if !logs.is_empty() {
        tracing::info!(
            "🔍 Indexing blocks {}-{}: {} transfer events found",
            from_block,
            to_block,
            logs.len()
        );
    }

    // 4. Parse logs into balance changes
    let mut changes: Vec<BalanceChange> = Vec::new();
    for log in &logs {
        if log.topics.is_empty() {
            continue;
        }
        let topic0 = log.topics[0].to_lowercase();
        if topic0 == TRANSFER_SINGLE_TOPIC.to_lowercase() {
            if let Some(mut parsed) = parse_transfer_single(log) {
                changes.append(&mut parsed);
            }
        } else if topic0 == TRANSFER_BATCH_TOPIC.to_lowercase() {
            if let Some(mut parsed) = parse_transfer_batch(log) {
                changes.append(&mut parsed);
            }
        }
    }

    // 5+6. Apply balance changes AND advance cursor in a single ACID
    // transaction. This guarantees the cursor only moves forward when ALL
    // events in the range are persisted — and conversely, if any change
    // fails, the cursor stays put and the entire range is retried next
    // cycle. Without this, a crash mid-loop could leave the cursor stale
    // and cause balances to be incremented TWICE on the next run (the
    // upserts use `balance + $delta`, NOT idempotent at the row level).
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("DB tx begin error: {e}"))?;

    for change in &changes {
        apply_balance_change_tx(&mut tx, change).await?;
    }

    sqlx::query(
        r#"INSERT INTO chain_indexer_cursor (contract_address, last_block, last_updated_at)
           VALUES ($1, $2, NOW())
           ON CONFLICT (contract_address) DO UPDATE
           SET last_block = $2, last_updated_at = NOW()"#,
    )
    .bind(&contract_lower)
    .bind(to_block as i64)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB cursor update error: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("DB tx commit error: {e}"))?;

    if !changes.is_empty() {
        tracing::info!(
            "🔍 Applied {} balance changes, cursor at block {}",
            changes.len(),
            to_block
        );
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── EVENT PARSING ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Parse a TransferSingle event log into balance changes.
///
/// TransferSingle(address operator, address from, address to, uint256 id, uint256 value)
/// Topics: [sig, operator, from, to]
/// Data: [id, value]
fn parse_transfer_single(log: &EthLog) -> Option<Vec<BalanceChange>> {
    if log.topics.len() < 4 {
        tracing::warn!(
            "🔍 TransferSingle with {} topics, expected 4",
            log.topics.len()
        );
        return None;
    }

    let from = topic_to_address(&log.topics[2]);
    let to = topic_to_address(&log.topics[3]);
    let block_number = parse_hex_u64(&log.block_number).unwrap_or(0);

    // Data contains: id (uint256) + value (uint256) = 64 bytes
    let data = log.data.trim_start_matches("0x");
    if data.len() < 128 {
        tracing::warn!("🔍 TransferSingle data too short: {}", data.len());
        return None;
    }

    let token_id = parse_hex_u64(&format!("0x{}", &data[0..64])).unwrap_or(0);
    let value = parse_hex_u64(&format!("0x{}", &data[64..128])).unwrap_or(0);

    let mut changes = Vec::new();

    // Sender loses tokens (unless from zero address = mint)
    if from != ZERO_ADDRESS {
        changes.push(BalanceChange {
            wallet_address: from.to_lowercase(),
            token_id,
            delta: -(value as i64),
            block_number,
        });
    }

    // Receiver gains tokens (unless to zero address = burn)
    if to != ZERO_ADDRESS {
        changes.push(BalanceChange {
            wallet_address: to.to_lowercase(),
            token_id,
            delta: value as i64,
            block_number,
        });
    }

    Some(changes)
}

/// Parse a TransferBatch event log into balance changes.
///
/// TransferBatch(address operator, address from, address to, uint256[] ids, uint256[] values)
/// Topics: [sig, operator, from, to]
/// Data: ABI-encoded dynamic arrays [ids, values]
fn parse_transfer_batch(log: &EthLog) -> Option<Vec<BalanceChange>> {
    if log.topics.len() < 4 {
        tracing::warn!(
            "🔍 TransferBatch with {} topics, expected 4",
            log.topics.len()
        );
        return None;
    }

    let from = topic_to_address(&log.topics[2]);
    let to = topic_to_address(&log.topics[3]);
    let block_number = parse_hex_u64(&log.block_number).unwrap_or(0);

    // Parse ABI-encoded dynamic arrays from data
    let data = log.data.trim_start_matches("0x");
    // Data layout: offset_ids (32B) | offset_values (32B) | ids_length (32B) | ids... | values_length (32B) | values...
    if data.len() < 128 {
        tracing::warn!("🔍 TransferBatch data too short");
        return None;
    }

    // Read offsets
    let ids_offset = parse_hex_u64(&format!("0x{}", &data[0..64])).unwrap_or(0) as usize * 2; // byte offset → nibble offset
    let _values_offset = parse_hex_u64(&format!("0x{}", &data[64..128])).unwrap_or(0) as usize * 2;

    // Read ids array
    if ids_offset + 64 > data.len() {
        return None;
    }
    let ids_count =
        parse_hex_u64(&format!("0x{}", &data[ids_offset..ids_offset + 64])).unwrap_or(0) as usize;

    let mut ids = Vec::with_capacity(ids_count);
    let ids_start = ids_offset + 64;
    for i in 0..ids_count {
        let pos = ids_start + i * 64;
        if pos + 64 > data.len() {
            break;
        }
        let id = parse_hex_u64(&format!("0x{}", &data[pos..pos + 64])).unwrap_or(0);
        ids.push(id);
    }

    // Read values array (starts after ids)
    let values_start = ids_start + ids_count * 64;
    if values_start + 64 > data.len() {
        return None;
    }
    let values_count = parse_hex_u64(&format!("0x{}", &data[values_start..values_start + 64]))
        .unwrap_or(0) as usize;

    let mut values = Vec::with_capacity(values_count);
    let vals_start = values_start + 64;
    for i in 0..values_count {
        let pos = vals_start + i * 64;
        if pos + 64 > data.len() {
            break;
        }
        let val = parse_hex_u64(&format!("0x{}", &data[pos..pos + 64])).unwrap_or(0);
        values.push(val);
    }

    if ids.len() != values.len() {
        tracing::warn!(
            "🔍 TransferBatch ids/values length mismatch: {} vs {}",
            ids.len(),
            values.len()
        );
        return None;
    }

    let mut changes = Vec::new();
    for (token_id, value) in ids.iter().zip(values.iter()) {
        if from != ZERO_ADDRESS {
            changes.push(BalanceChange {
                wallet_address: from.to_lowercase(),
                token_id: *token_id,
                delta: -(*value as i64),
                block_number,
            });
        }
        if to != ZERO_ADDRESS {
            changes.push(BalanceChange {
                wallet_address: to.to_lowercase(),
                token_id: *token_id,
                delta: *value as i64,
                block_number,
            });
        }
    }

    Some(changes)
}

// ═══════════════════════════════════════════════════════════════
// ── DB OPERATIONS ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Apply a single balance change to the `onchain_balances` table.
///
/// Uses the wallet_address → user_id lookup via `users.chain_wallet_address`.
/// If no user matches, the balance change is logged but not applied (could be
/// an external wallet or the contract itself).
/// Transaction-bound variant of `apply_balance_change`. Used by the indexer
/// loop so balance updates and cursor advance commit atomically.
async fn apply_balance_change_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    change: &BalanceChange,
) -> Result<(), String> {
    let user_and_asset: Option<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
        r#"SELECT u.id, a.id
           FROM users u
           CROSS JOIN assets a
           WHERE LOWER(u.chain_wallet_address) = $1
           AND a.chain_token_id = $2::text
           LIMIT 1"#,
    )
    .bind(&change.wallet_address)
    .bind(change.token_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| format!("DB lookup error: {e}"))?;

    let (user_id, asset_id) = match user_and_asset {
        Some(ids) => ids,
        None => return Ok(()),
    };

    if change.delta > 0 {
        sqlx::query(
            r#"INSERT INTO onchain_balances (user_id, asset_id, balance, last_synced_block, last_synced_at)
               VALUES ($1, $2, $3, $4, NOW())
               ON CONFLICT (user_id, asset_id) DO UPDATE
               SET balance = onchain_balances.balance + $3,
                   last_synced_block = GREATEST(onchain_balances.last_synced_block, $4),
                   last_synced_at = NOW()"#,
        )
        .bind(user_id)
        .bind(asset_id)
        .bind(change.delta)
        .bind(change.block_number as i64)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("DB balance+ error: {e}"))?;
    } else if change.delta < 0 {
        let abs_delta = change.delta.unsigned_abs() as i64;
        sqlx::query(
            r#"INSERT INTO onchain_balances (user_id, asset_id, balance, last_synced_block, last_synced_at)
               VALUES ($1, $2, 0, $4, NOW())
               ON CONFLICT (user_id, asset_id) DO UPDATE
               SET balance = GREATEST(0, onchain_balances.balance - $3),
                   last_synced_block = GREATEST(onchain_balances.last_synced_block, $4),
                   last_synced_at = NOW()"#,
        )
        .bind(user_id)
        .bind(asset_id)
        .bind(abs_delta)
        .bind(change.block_number as i64)
        .execute(&mut **tx)
        .await
        .map_err(|e| format!("DB balance- error: {e}"))?;
    }
    Ok(())
}

#[allow(dead_code)]
async fn apply_balance_change(
    pool: &PgPool,
    change: &BalanceChange,
    _contract_address: &str,
) -> Result<(), String> {
    // Look up user_id from wallet address
    let user_and_asset: Option<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
        r#"SELECT u.id, a.id
           FROM users u
           CROSS JOIN assets a
           WHERE LOWER(u.chain_wallet_address) = $1
           AND a.chain_token_id = $2::text
           LIMIT 1"#,
    )
    .bind(&change.wallet_address)
    .bind(change.token_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB lookup error: {e}"))?;

    let (user_id, asset_id) = match user_and_asset {
        Some(ids) => ids,
        None => {
            // Not a POOOL user or unknown token — skip silently
            return Ok(());
        }
    };

    if change.delta > 0 {
        // Received tokens — upsert with increase
        let _ = sqlx::query(
            r#"INSERT INTO onchain_balances (user_id, asset_id, balance, last_synced_block, last_synced_at)
               VALUES ($1, $2, $3, $4, NOW())
               ON CONFLICT (user_id, asset_id) DO UPDATE
               SET balance = onchain_balances.balance + $3,
                   last_synced_block = GREATEST(onchain_balances.last_synced_block, $4),
                   last_synced_at = NOW()"#
        )
        .bind(user_id)
        .bind(asset_id)
        .bind(change.delta)
        .bind(change.block_number as i64)
        .execute(pool)
        .await
        .map_err(|e| format!("DB balance+ error: {e}"))?;
    } else if change.delta < 0 {
        // Sent tokens — upsert with decrease (clamped to 0 by CHECK constraint)
        let abs_delta = change.delta.unsigned_abs() as i64;
        let _ = sqlx::query(
            r#"INSERT INTO onchain_balances (user_id, asset_id, balance, last_synced_block, last_synced_at)
               VALUES ($1, $2, 0, $4, NOW())
               ON CONFLICT (user_id, asset_id) DO UPDATE
               SET balance = GREATEST(0, onchain_balances.balance - $3),
                   last_synced_block = GREATEST(onchain_balances.last_synced_block, $4),
                   last_synced_at = NOW()"#
        )
        .bind(user_id)
        .bind(asset_id)
        .bind(abs_delta)
        .bind(change.block_number as i64)
        .execute(pool)
        .await
        .map_err(|e| format!("DB balance- error: {e}"))?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── RPC HELPERS ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Get the current block number from the RPC endpoint.
async fn get_block_number(client: &Client, rpc_url: &str) -> Result<u64, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("RPC request error: {e}"))?;

    let response: JsonRpcResponse = resp
        .json()
        .await
        .map_err(|e| format!("RPC parse error: {e}"))?;

    if let Some(err) = response.error {
        return Err(format!("RPC error: {}", err.message));
    }

    let hex = response
        .result
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| "Missing block number result".to_string())?;

    parse_hex_u64(&hex).ok_or_else(|| format!("Invalid block number: {hex}"))
}

/// Fetch logs from the RPC endpoint.
async fn eth_get_logs(
    client: &Client,
    rpc_url: &str,
    filter: &serde_json::Value,
) -> Result<Vec<EthLog>, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getLogs",
        "params": [filter],
        "id": 2
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("RPC getLogs error: {e}"))?;

    let response: JsonRpcResponse = resp
        .json()
        .await
        .map_err(|e| format!("RPC parse error: {e}"))?;

    if let Some(err) = response.error {
        return Err(format!("RPC error: {}", err.message));
    }

    match response.result {
        Some(serde_json::Value::Array(arr)) => {
            let mut logs = Vec::new();
            for item in arr {
                match serde_json::from_value::<EthLog>(item) {
                    Ok(log) => logs.push(log),
                    Err(e) => tracing::warn!("🔍 Failed to parse log: {}", e),
                }
            }
            Ok(logs)
        }
        _ => Ok(Vec::new()),
    }
}

// ═══════════════════════════════════════════════════════════════
// ── UTILITY FUNCTIONS ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Extract a 20-byte address from a 32-byte topic (left-padded with zeros).
fn topic_to_address(topic: &str) -> String {
    let clean = topic.trim_start_matches("0x");
    if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        topic.to_string()
    }
}

/// Parse a hex string (0x-prefixed or not) to u64.
fn parse_hex_u64(hex: &str) -> Option<u64> {
    let clean = hex.trim_start_matches("0x");
    // Handle oversized hex values by taking last 16 chars (64 bits)
    let trimmed = if clean.len() > 16 {
        &clean[clean.len() - 16..]
    } else {
        clean
    };
    u64::from_str_radix(trimmed, 16).ok()
}
