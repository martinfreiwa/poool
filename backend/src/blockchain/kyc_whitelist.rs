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
    let config = match ChainConfig::from_env() {
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

/// Find KYC-approved users without a chain_wallet_address and whitelist them.
async fn process_pending_whitelists(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
) -> Result<(), String> {
    // Find users who are KYC-approved but don't have a wallet address yet
    let pending: Vec<PendingWhitelist> = sqlx::query_as::<_, (Uuid, String)>(
        r#"SELECT u.id, u.email
           FROM users u
           JOIN kyc_records k ON k.user_id = u.id
           WHERE k.status = 'approved'
           AND (u.chain_wallet_address IS NULL OR u.chain_wallet_address = '')
           AND u.status = 'active'
           ORDER BY k.verified_at ASC
           LIMIT 10"#, // Process max 10 per cycle to avoid gas spikes
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query error: {e}"))?
    .into_iter()
    .map(|(user_id, email)| PendingWhitelist { user_id, email })
    .collect();

    if pending.is_empty() {
        return Ok(());
    }

    tracing::info!(
        "🔑 Found {} KYC-approved users pending whitelist",
        pending.len()
    );

    for user in &pending {
        match whitelist_user(pool, config, client, user).await {
            Ok(address) => {
                tracing::info!("🔑 ✅ Whitelisted user {} → {}", user.email, address);
            }
            Err(e) => {
                tracing::error!("🔑 ❌ Failed to whitelist user {}: {}", user.email, e);
                // Continue with next user — don't block the batch
            }
        }
    }

    Ok(())
}

/// Generate a deterministic address for a user and call setWhitelisted on-chain.
async fn whitelist_user(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
    user: &PendingWhitelist,
) -> Result<String, String> {
    // Generate a deterministic wallet address from user_id
    // In production, this would use GCP KMS to derive a real key pair.
    // For now, we generate a placeholder address from the user UUID.
    let wallet_address = derive_wallet_address(&user.user_id);

    let registry_address = std::env::var("CHAIN_IDENTITY_REGISTRY_ADDRESS")
        .unwrap_or_else(|_| config.contract_address.clone());

    // Send the transaction using cast (same pattern as settlement worker)
    let result = send_whitelist_tx(config, client, &registry_address, &wallet_address).await;

    match result {
        Ok(tx_hash) => {
            // Update the user's chain_wallet_address in DB
            let _ = sqlx::query("UPDATE users SET chain_wallet_address = $1 WHERE id = $2")
                .bind(&wallet_address)
                .bind(user.user_id)
                .execute(pool)
                .await
                .map_err(|e| format!("DB update error: {e}"))?;

            // Log to audit
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (user_id, action, details, ip_address, created_at)
                   VALUES ($1, 'kyc_whitelist_sync', $2, '0.0.0.0', NOW())"#,
            )
            .bind(user.user_id)
            .bind(
                serde_json::json!({
                    "wallet_address": wallet_address,
                    "tx_hash": tx_hash,
                    "contract": registry_address,
                })
                .to_string(),
            )
            .execute(pool)
            .await;

            Ok(wallet_address)
        }
        Err(e) => {
            // Log the failure but don't stop processing
            tracing::error!("🔑 setWhitelisted TX failed for {}: {}", wallet_address, e);
            Err(e)
        }
    }
}

/// Send a setWhitelisted transaction via JSON-RPC.
///
/// Uses the same signing approach as the settlement worker (via `cast` CLI).
async fn send_whitelist_tx(
    config: &ChainConfig,
    _client: &Client,
    registry_address: &str,
    target_wallet: &str,
) -> Result<String, String> {
    // Use `cast send` to sign and broadcast the transaction
    // This is the same approach used in the settlement worker
    let output = tokio::process::Command::new("cast")
        .args([
            "send",
            registry_address,
            "setWhitelisted(address,bool)",
            target_wallet,
            "true",
            "--rpc-url",
            &config.rpc_url,
            "--private-key",
            &config.settlement_private_key,
            "--chain-id",
            &config.chain_id.to_string(),
            "--json",
        ])
        .output()
        .await
        .map_err(|e| format!("cast command failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cast send failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the JSON response to get the tx hash
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(hash) = json.get("transactionHash").and_then(|v| v.as_str()) {
            return Ok(hash.to_string());
        }
    }

    // Fallback: return the raw output as the hash
    Ok(stdout.trim().to_string())
}

/// Derive a deterministic wallet address from a user UUID.
///
/// This is a placeholder implementation. In production, this would:
/// 1. Call GCP KMS to generate/derive a key pair for the user
/// 2. Return the derived Ethereum address
///
/// For now, we create a deterministic address by hashing the UUID.
/// This address won't have a real private key — it's only used for
/// on-chain whitelist tracking. The actual token transfers happen via
/// the settlement wallet's `settleBatch()` which uses `forcedTransfer()`.
fn derive_wallet_address(user_id: &Uuid) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    user_id.hash(&mut hasher);
    let hash1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    hash1.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    // Combine two 64-bit hashes into a 160-bit (20-byte) address
    // This is deterministic: same UUID → same address
    let h1 = format!("{:016x}", hash1);
    let h2 = format!("{:016x}", hash2);
    format!("0x{}{}{}", &h1[..8], h1, &h2[..12])
}
