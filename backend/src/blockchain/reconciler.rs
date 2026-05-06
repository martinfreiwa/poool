/// Stuck-trade reconciler — recovers trades stuck in `submitted` state.
///
/// A trade enters `on_chain_status='submitted'` after the settlement worker
/// sends a `settleBatch()` TX but before the receipt comes back. If the
/// worker crashes, the RPC times out, the TX gets dropped from the mempool,
/// or a node-resync swallows the receipt, the trade can sit in `submitted`
/// indefinitely — blocking it from ever flipping to `confirmed` or `failed`.
///
/// This worker periodically scans for trades that have been `submitted` for
/// more than `STUCK_THRESHOLD_SECS` and re-checks their TX hash directly on
/// the chain:
///   - Receipt found + status=0x1 → mark `confirmed`
///   - Receipt found + status=0x0 → mark `failed`, reset to `pending` for retry
///   - No receipt after grace window → mark `failed`, reset to `pending`
///
/// Idempotent: safe to run alongside the main settlement worker.
use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use super::service::ChainConfig;

/// How long a trade must be `submitted` before we treat it as stuck.
const STUCK_THRESHOLD_SECS: i64 = 600; // 10 minutes

/// How long a trade can be `submitted` with NO tx_hash before we treat it
/// as an orphan (settlement worker crashed between status flip and broadcast).
/// Shorter than STUCK_THRESHOLD because no TX was actually sent — safe to reset.
const ORPHAN_THRESHOLD_SECS: i64 = 120; // 2 minutes

/// How often the reconciler runs.
const RECONCILER_INTERVAL_SECS: u64 = 120; // 2 minutes

/// Hard deadline. After this many seconds in `submitted`, give up waiting
/// and reset the trade to `pending` so the main worker re-batches it.
const HARD_TIMEOUT_SECS: i64 = 3600; // 1 hour

pub async fn run_reconciler(pool: &PgPool) {
    let config = match ChainConfig::from_env().await {
        Some(c) if c.enabled => c,
        _ => {
            tracing::info!("⛓️ Reconciler: blockchain not enabled — reconciler will not start");
            return;
        }
    };

    tracing::info!(
        "⛓️ Stuck-trade reconciler starting (interval={}s, stuck>={}s, hard_timeout={}s)",
        RECONCILER_INTERVAL_SECS,
        STUCK_THRESHOLD_SECS,
        HARD_TIMEOUT_SECS
    );

    let client = Client::new();

    // Initial delay so we don't fight cold-start congestion.
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    loop {
        if let Err(e) = reconcile_once(pool, &config, &client).await {
            tracing::error!("⛓️ Reconciler cycle failed: {}", e);
            sentry::capture_message(
                &format!("Reconciler cycle failed: {}", e),
                sentry::Level::Warning,
            );
        }
        tokio::time::sleep(std::time::Duration::from_secs(RECONCILER_INTERVAL_SECS)).await;
    }
}

async fn reconcile_once(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
) -> Result<(), String> {
    // Pass A — orphans: trades that got marked `submitted` (with batch_id)
    // but never received a tx_hash. This happens when the settlement worker
    // crashes between the status flip and the broadcast call. They are
    // safe to reset to `pending` because no TX was sent — the next worker
    // run will re-batch them cleanly.
    // `trade_history` has no `updated_at` column — `executed_at` is the
    // closest proxy and is set at trade creation. If a trade has been
    // sitting in `submitted` state without a tx_hash for longer than
    // the orphan threshold past `executed_at`, the worker crashed
    // between flipping status and broadcasting; safe to reset.
    let orphans = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT id FROM trade_history
           WHERE on_chain_status = 'submitted'
             AND on_chain_tx_hash IS NULL
             AND executed_at < NOW() - ($1 || ' seconds')::INTERVAL
           LIMIT 200"#,
    )
    .bind(ORPHAN_THRESHOLD_SECS.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| format!("orphan query failed: {}", e))?;

    if !orphans.is_empty() {
        tracing::warn!(
            "⛓️ Reconciler: {} orphan trades (submitted, no tx_hash) — resetting to pending",
            orphans.len()
        );
        sentry::capture_message(
            &format!(
                "Reset {} orphan trades to pending (settlement worker crashed before broadcast)",
                orphans.len()
            ),
            sentry::Level::Warning,
        );
        reset_to_pending(pool, &orphans).await?;
    }

    // Pass B — trades stuck in `submitted` past the threshold WITH a tx_hash.
    let stuck = sqlx::query_as::<_, (Uuid, String, i64)>(
        r#"SELECT id,
                  on_chain_tx_hash,
                  EXTRACT(EPOCH FROM (NOW() - updated_at))::BIGINT AS age_secs
           FROM trade_history
           WHERE on_chain_status = 'submitted'
             AND updated_at < NOW() - ($1 || ' seconds')::INTERVAL
             AND on_chain_tx_hash IS NOT NULL
           LIMIT 100"#,
    )
    .bind(STUCK_THRESHOLD_SECS.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query failed: {}", e))?;

    if stuck.is_empty() {
        tracing::debug!("⛓️ Reconciler: no stuck trades");
        return Ok(());
    }

    tracing::warn!(
        "⛓️ Reconciler: {} trades stuck in 'submitted' state — checking on-chain",
        stuck.len()
    );

    // Group by tx_hash so we only ask the chain once per batch.
    let mut by_hash: std::collections::HashMap<String, (Vec<Uuid>, i64)> =
        std::collections::HashMap::new();
    for (id, hash, age) in stuck {
        let entry = by_hash.entry(hash).or_insert_with(|| (Vec::new(), age));
        entry.0.push(id);
        entry.1 = entry.1.max(age); // track oldest age in the group
    }

    for (tx_hash, (trade_ids, oldest_age)) in by_hash {
        match check_receipt(client, &config.rpc_url, &tx_hash).await {
            Ok(Some(true)) => {
                tracing::info!(
                    "⛓️ Reconciler: tx={} → confirmed ({} trades)",
                    tx_hash,
                    trade_ids.len()
                );
                update_status(pool, &trade_ids, "confirmed").await?;
            }
            Ok(Some(false)) => {
                // Reverted on-chain. Single atomic update — clear tx_hash +
                // batch_id and flip back to 'pending'. (Previously two
                // separate UPDATEs caused a brief 'failed' flicker.)
                tracing::info!(
                    "⛓️ Reconciler: tx={} → reverted, resetting {} trades to pending",
                    tx_hash,
                    trade_ids.len()
                );
                reset_to_pending(pool, &trade_ids).await?;
            }
            Ok(None) => {
                // Receipt not yet on-chain.
                if oldest_age > HARD_TIMEOUT_SECS {
                    tracing::error!(
                        "⛓️ Reconciler: tx={} HARD TIMEOUT after {}s — resetting {} trades to pending",
                        tx_hash,
                        oldest_age,
                        trade_ids.len()
                    );
                    sentry::capture_message(
                        &format!(
                            "On-chain TX timed out (>{}s): tx={}, trades={}",
                            HARD_TIMEOUT_SECS,
                            tx_hash,
                            trade_ids.len()
                        ),
                        sentry::Level::Error,
                    );
                    reset_to_pending(pool, &trade_ids).await?;
                } else {
                    tracing::info!(
                        "⛓️ Reconciler: tx={} still pending on chain (age={}s) — will recheck",
                        tx_hash,
                        oldest_age
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "⛓️ Reconciler: receipt check failed for tx={}: {}",
                    tx_hash,
                    e
                );
            }
        }
    }

    Ok(())
}

/// Fetch the receipt for a TX. Returns:
/// - `Ok(Some(true))` → receipt found, success
/// - `Ok(Some(false))` → receipt found, reverted
/// - `Ok(None)` → no receipt yet (pending or dropped)
/// - `Err(_)` → RPC error
async fn check_receipt(
    client: &Client,
    rpc_url: &str,
    tx_hash: &str,
) -> Result<Option<bool>, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
        "id": 1
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("RPC parse failed: {}", e))?;

    if let Some(err) = json.get("error") {
        return Err(format!("RPC error: {}", err));
    }

    let result = match json.get("result") {
        Some(serde_json::Value::Null) | None => return Ok(None),
        Some(v) => v,
    };

    let status = result
        .get("status")
        .and_then(|s| s.as_str())
        .ok_or_else(|| "receipt missing status".to_string())?;

    Ok(Some(status == "0x1"))
}

async fn update_status(pool: &PgPool, trade_ids: &[Uuid], status: &str) -> Result<(), String> {
    sqlx::query(
        "UPDATE trade_history SET on_chain_status = $1, updated_at = NOW()
         WHERE id = ANY($2)",
    )
    .bind(status)
    .bind(trade_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("DB update failed: {}", e))?;
    Ok(())
}

/// Reset failed/timed-out trades to `pending` AND clear tx_hash + batch_id
/// so the main settlement worker picks them up cleanly on the next run.
async fn reset_to_pending(pool: &PgPool, trade_ids: &[Uuid]) -> Result<(), String> {
    sqlx::query(
        "UPDATE trade_history SET
            on_chain_status = 'pending',
            on_chain_tx_hash = NULL,
            on_chain_batch_id = NULL,
            updated_at = NOW()
         WHERE id = ANY($1)",
    )
    .bind(trade_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("Reset to pending failed: {}", e))?;
    Ok(())
}
