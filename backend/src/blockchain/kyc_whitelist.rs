/// KYC → Whitelist Sync Worker
///
/// Monitors for newly KYC-approved users and automatically adds them to the
/// on-chain whitelist on the POOOLProperty1155 contract.
///
/// Flow:
/// 1. Polls DB every 60 seconds for users with kyc_status='approved' AND no chain_wallet_address
/// 2. Generates a deterministic wallet address for the user (from platform settlement wallet)
/// 3. Calls `addToWhitelist(address)` on the smart contract
/// 4. Updates `users.chain_wallet_address` with the whitelisted address
///
/// This ensures that every KYC-approved user can receive token transfers on settlement.
///
/// 🔴 SECURITY: Only the ADMIN_ROLE wallet can call setWhitelisted().
///    The settlement private key must have ADMIN_ROLE on the Identity Registry.
use reqwest::Client;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use super::service::ChainConfig;

// ═══════════════════════════════════════════════════════════════
// ── TYPES ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// A user that needs to be whitelisted on-chain.
#[derive(Debug)]
struct PendingWhitelist {
    user_id: Uuid,
    email: String,
}

// ═══════════════════════════════════════════════════════════════
// ── MAIN WORKER ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Run the KYC → Whitelist sync worker. Called from main.rs as a Tokio task.
///
/// Polls every 60 seconds for KYC-approved users without a chain_wallet_address,
/// generates addresses, and calls setWhitelisted() on-chain.
pub async fn run_kyc_whitelist_worker(pool: &PgPool) {
    let config = match ChainConfig::from_env().await {
        Some(c) if c.enabled => c,
        Some(_) => {
            tracing::info!("🔑 KYC→Whitelist: blockchain configured but DISABLED.");
            return;
        }
        None => {
            tracing::info!("🔑 KYC→Whitelist: blockchain not configured, skipping.");
            return;
        }
    };

    // Retrieve Registry Address
    let registry_address = std::env::var("CHAIN_IDENTITY_REGISTRY_ADDRESS")
        .unwrap_or_else(|_| "Not configured".to_string());

    tracing::info!(
        "🔑 KYC→Whitelist sync worker starting (registry={})",
        registry_address
    );

    let client = Client::new();

    // Initial delay
    tokio::time::sleep(Duration::from_secs(45)).await;

    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Err(e) = process_pending_whitelists(pool, &config, &client).await {
            tracing::error!("🔑 KYC→Whitelist error: {}", e);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// ── CORE LOGIC ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Find KYC-approved users with a sovereign wallet bound but not yet
/// whitelisted on-chain, and whitelist them in a single batch TX.
///
/// Sovereign-wallet model: `chain_wallet_address` is set via the SIWE
/// wallet-binding endpoint (user proves ownership by signing a nonce).
/// This worker just lifts that already-verified address onto the
/// on-chain whitelist. No fake addresses, no derivations.
async fn process_pending_whitelists(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
) -> Result<(), String> {
    let pending: Vec<(Uuid, String, String)> = sqlx::query_as::<_, (Uuid, String, String)>(
        r#"SELECT u.id, u.email, u.chain_wallet_address
           FROM users u
           JOIN kyc_records k ON k.user_id = u.id
           WHERE k.status = 'approved'
             AND u.chain_wallet_address IS NOT NULL
             AND u.chain_wallet_address <> ''
             AND u.chain_whitelisted_at IS NULL
             AND u.status = 'active'
           ORDER BY k.verified_at ASC
           LIMIT 50"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query error: {e}"))?;

    if pending.is_empty() {
        return Ok(());
    }

    tracing::info!(
        "🔑 Found {} KYC-approved users with bound wallets pending whitelist",
        pending.len()
    );

    // Use the user-supplied (and SIWE-verified) addresses directly.
    let entries: Vec<(String, Uuid, String)> = pending
        .iter()
        .map(|(uid, email, addr)| (addr.clone(), *uid, email.clone()))
        .collect();
    let addresses: Vec<String> = entries.iter().map(|(a, _, _)| a.clone()).collect();

    let registry_address = std::env::var("CHAIN_IDENTITY_REGISTRY_ADDRESS")
        .unwrap_or_else(|_| config.contract_address.clone());

    match send_batch_whitelist_tx(config, client, &registry_address, &addresses).await {
        Ok(tx_hash) => {
            tracing::info!(
                "🔑 ✅ batchSetWhitelisted ({} users) tx={}",
                addresses.len(),
                tx_hash
            );
            // Stamp `chain_whitelisted_at` so the next cycle skips these
            // rows. If any DB write fails, the user gets re-whitelisted
            // next cycle — the contract is idempotent on bool=true.
            for (address, user_id, _email) in &entries {
                let _ = sqlx::query("UPDATE users SET chain_whitelisted_at = NOW() WHERE id = $1")
                    .bind(user_id)
                    .execute(pool)
                    .await;
                let _ = sqlx::query(
                    r#"INSERT INTO audit_logs (user_id, action, details, ip_address, created_at)
                       VALUES ($1, 'kyc_whitelist_sync', $2, '0.0.0.0', NOW())"#,
                )
                .bind(user_id)
                .bind(
                    serde_json::json!({
                        "wallet_address": address,
                        "tx_hash": &tx_hash,
                        "contract": &registry_address,
                        "batch": true,
                        "batch_size": addresses.len(),
                    })
                    .to_string(),
                )
                .execute(pool)
                .await;
            }
        }
        Err(e) => {
            tracing::error!(
                "🔑 ❌ batchSetWhitelisted failed for {} users: {}",
                addresses.len(),
                e
            );
            sentry::capture_message(
                &format!(
                    "KYC batch whitelist failed for {} users: {}",
                    addresses.len(),
                    e
                ),
                sentry::Level::Error,
            );
        }
    }

    Ok(())
}

/// Build and broadcast a single `batchSetWhitelisted(addresses, statuses)`
/// transaction in-process. All addresses get `true`. Returns the tx hash.
///
/// In-process (k256) — no `cast` subprocess, no key leak via process args.
async fn send_batch_whitelist_tx(
    config: &ChainConfig,
    client: &Client,
    registry_address: &str,
    addresses: &[String],
) -> Result<String, String> {
    if addresses.is_empty() {
        return Err("empty address batch".to_string());
    }

    // ABI-encode batchSetWhitelisted(address[], bool[]).
    // selector = keccak256("batchSetWhitelisted(address[],bool[])")[..4]
    // = 0xd38c6523 (verified: `cast sig 'batchSetWhitelisted(address[],bool[])'`).
    // The previous value (0x9beb20f8) was wrong — every call hit a
    // non-existent function selector and reverted silently for weeks.
    let calldata = encode_batch_set_whitelisted_calldata(addresses)?;

    let sender = super::signing::format_address(&config.signer.address());

    // RPC params — same shape as the settlement worker.
    let nonce_resp = rpc_call(
        client,
        &config.rpc_url,
        "eth_getTransactionCount",
        serde_json::json!([sender, "pending"]),
    )
    .await?;
    let nonce_hex = nonce_resp.as_str().ok_or("nonce: bad RPC reply")?;
    let nonce = u64::from_str_radix(nonce_hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("nonce parse: {}", e))?;

    let gas_price_resp = rpc_call(
        client,
        &config.rpc_url,
        "eth_gasPrice",
        serde_json::json!([]),
    )
    .await?;
    let gas_price_hex = gas_price_resp.as_str().ok_or("gas_price: bad RPC reply")?;
    let gas_price = u64::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("gas_price parse: {}", e))?;

    let gas_estimate_resp = rpc_call(
        client,
        &config.rpc_url,
        "eth_estimateGas",
        serde_json::json!([{
            "from": sender,
            "to": registry_address,
            "data": calldata,
        }]),
    )
    .await?;
    let gas_est_hex = gas_estimate_resp
        .as_str()
        .ok_or("gas_estimate: bad RPC reply")?;
    let gas_estimate = u64::from_str_radix(gas_est_hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("gas_estimate parse: {}", e))?;
    let gas_limit = gas_estimate + (gas_estimate / 5);

    let signed = super::signing::sign_legacy_transaction_with(
        &*config.signer,
        config.chain_id,
        nonce,
        gas_price,
        gas_limit,
        registry_address,
        0,
        &calldata,
    )
    .await?;

    let tx_hash_resp = rpc_call(
        client,
        &config.rpc_url,
        "eth_sendRawTransaction",
        serde_json::json!([signed]),
    )
    .await?;
    tx_hash_resp
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "send_raw: bad RPC reply".to_string())
}

/// ABI-encode batchSetWhitelisted(address[],bool[]) calldata.
fn encode_batch_set_whitelisted_calldata(addresses: &[String]) -> Result<String, String> {
    let selector = "d38c6523"; // keccak256("batchSetWhitelisted(address[],bool[])")[..4]
    let n = addresses.len();

    let mut padded_addrs: Vec<String> = Vec::with_capacity(n);
    for a in addresses {
        let clean = a.strip_prefix("0x").unwrap_or(a).to_lowercase();
        if clean.len() != 40 || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("invalid address: {}", a));
        }
        padded_addrs.push(format!("{:0>64}", clean));
    }

    // Two dynamic arrays: head = 2 offsets (64 bytes total).
    let array_size = 32 + n * 32;
    let off_addrs = 2 * 32;
    let off_bools = off_addrs + array_size;

    let pad = |hex_value: String| format!("{:0>64}", hex_value);
    let mut data = String::from(selector);
    data.push_str(&pad(format!("{:x}", off_addrs)));
    data.push_str(&pad(format!("{:x}", off_bools)));
    // addresses array
    data.push_str(&pad(format!("{:x}", n)));
    for a in &padded_addrs {
        data.push_str(a);
    }
    // bools array (all true = 1)
    data.push_str(&pad(format!("{:x}", n)));
    for _ in 0..n {
        data.push_str(&pad("1".to_string()));
    }
    Ok(format!("0x{}", data))
}

/// Generic JSON-RPC call helper (mirrors service.rs::rpc_call).
async fn rpc_call(
    client: &Client,
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });
    let resp = client
        .post(rpc_url)
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("RPC parse failed: {}", e))?;
    if let Some(err) = body.get("error") {
        return Err(format!("RPC error: {}", err));
    }
    body.get("result")
        .cloned()
        .ok_or_else(|| "Empty RPC response".to_string())
}

// Removed: derive_wallet_address (fake-hash placeholder), whitelist_user
// (per-user TX path), send_whitelist_tx (cast subprocess). Sovereign-wallet
// model uses user-supplied addresses verified via SIWE; whitelisting is
// always batched.
