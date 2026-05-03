/// Settlement-wallet gas-balance monitor.
///
/// The settlement wallet is a HOT WALLET — it holds the private key in
/// memory and signs every batch settlement TX. It MUST hold MATIC to pay
/// gas, but MUST NOT hold significant user funds (key compromise = total
/// drain). This worker:
///
/// 1. Polls the wallet's MATIC balance every 5 minutes
/// 2. Alerts via Sentry if balance drops below `LOW_GAS_THRESHOLD_WEI`
///    (≈ enough gas for ~50 batch settlements)
/// 3. Alerts at higher severity if it drops below `CRITICAL_GAS_THRESHOLD_WEI`
///    (≈ enough for ~10 settlements — refill NOW)
/// 4. Stores the latest balance in `chain_wallet_balance` for the admin UI
///
/// This is preventative — it does NOT auto-refill (refills require manual
/// approval from the cold wallet). The goal is to catch low balance before
/// settlements start failing.
use reqwest::Client;
use sqlx::PgPool;

use super::service::ChainConfig;

/// Below this many wei of MATIC, log a warning and emit a Sentry message.
/// 0.05 MATIC ≈ 50 batch settlements at ~0.001 MATIC each.
const LOW_GAS_THRESHOLD_WEI: u128 = 50_000_000_000_000_000; // 0.05 MATIC

/// Below this, escalate to error severity. Refill within hours, not days.
/// 0.01 MATIC ≈ 10 batch settlements.
const CRITICAL_GAS_THRESHOLD_WEI: u128 = 10_000_000_000_000_000; // 0.01 MATIC

/// How often to poll. Cheap call (one eth_getBalance), so 5 min is fine.
const POLL_INTERVAL_SECS: u64 = 300;

pub async fn run_gas_monitor(pool: &PgPool) {
    let config = match ChainConfig::from_env().await {
        Some(c) if c.enabled => c,
        _ => {
            tracing::info!("⛓️ Gas monitor: blockchain not enabled — monitor will not start");
            return;
        }
    };

    let address = super::signing::format_address(&config.signer.address());

    tracing::info!(
        "⛓️ Gas monitor starting (wallet={}, low={} wei, critical={} wei, interval={}s)",
        address,
        LOW_GAS_THRESHOLD_WEI,
        CRITICAL_GAS_THRESHOLD_WEI,
        POLL_INTERVAL_SECS
    );

    let client = Client::new();

    // Initial delay
    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

    loop {
        match check_balance(&client, &config.rpc_url, &address).await {
            Ok(wei) => {
                let _ = persist_balance(pool, &address, wei).await;

                if wei < CRITICAL_GAS_THRESHOLD_WEI {
                    tracing::error!(
                        "🔴 Settlement wallet CRITICAL gas: {} wei (< {} threshold) — REFILL IMMEDIATELY",
                        wei,
                        CRITICAL_GAS_THRESHOLD_WEI
                    );
                    sentry::capture_message(
                        &format!(
                            "CRITICAL: Settlement wallet gas balance = {} wei (< {} critical threshold). Wallet: {}. Refill immediately or settlements will fail.",
                            wei, CRITICAL_GAS_THRESHOLD_WEI, address
                        ),
                        sentry::Level::Error,
                    );
                } else if wei < LOW_GAS_THRESHOLD_WEI {
                    tracing::warn!(
                        "⚠️ Settlement wallet low gas: {} wei (< {} threshold)",
                        wei,
                        LOW_GAS_THRESHOLD_WEI
                    );
                    sentry::capture_message(
                        &format!(
                            "Settlement wallet gas balance low: {} wei (< {} low threshold). Wallet: {}. Plan a refill.",
                            wei, LOW_GAS_THRESHOLD_WEI, address
                        ),
                        sentry::Level::Warning,
                    );
                } else {
                    tracing::debug!("⛓️ Gas monitor: balance OK ({} wei)", wei);
                }
            }
            Err(e) => {
                tracing::warn!("⛓️ Gas monitor: balance check failed: {}", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

async fn check_balance(client: &Client, rpc_url: &str, address: &str) -> Result<u128, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
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

    let hex = json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing result".to_string())?;

    u128::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("bad hex balance '{}': {}", hex, e))
}

async fn persist_balance(pool: &PgPool, address: &str, wei: u128) -> Result<(), String> {
    // Persist as TEXT — wei values can exceed i64. Admin UI parses as needed.
    sqlx::query(
        "INSERT INTO chain_wallet_balance (address, balance_wei, checked_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (address) DO UPDATE SET
            balance_wei = EXCLUDED.balance_wei,
            checked_at = EXCLUDED.checked_at",
    )
    .bind(address)
    .bind(wei.to_string())
    .execute(pool)
    .await
    .map_err(|e| format!("DB persist failed: {}", e))?;
    Ok(())
}
