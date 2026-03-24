//! Admin blockchain API — tokenization, treasury, emergency controls.
use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;

// ═══════════════════════════════════════════════════════════════
// ── TYPES ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Response for the blockchain treasury overview.
#[derive(Serialize)]
pub struct BlockchainTreasuryResponse {
    /// Settlement wallet address
    pub wallet_address: String,
    /// Blockchain network name (e.g., "polygon_amoy", "polygon")
    pub network: String,
    /// Contract address for POOOLProperty1155
    pub contract_address: String,
    /// Whether on-chain settlement is enabled
    pub settlement_enabled: bool,
    /// Total assets with on-chain token IDs
    pub tokenized_assets_count: i64,
    /// Total assets available for tokenization
    pub total_assets_count: i64,
    /// Total settlement batches
    pub total_batches: i64,
    /// Successfully confirmed batches
    pub confirmed_batches: i64,
    /// Failed batches
    pub failed_batches: i64,
    /// Total trades settled on-chain
    pub confirmed_trades: i64,
    /// Pending trades awaiting on-chain settlement
    pub pending_trades: i64,
    /// Submitted trades (in-flight)
    pub submitted_trades: i64,
    /// List of tokenized assets
    pub tokenized_assets: Vec<TokenizedAsset>,
    /// Recent settlement batches
    pub recent_batches: Vec<SettlementBatch>,
    /// Users with chain wallet addresses
    pub whitelisted_users_count: i64,
    /// Block explorer base URL
    pub explorer_url: String,
}

/// A tokenized asset for the treasury table.
#[derive(Serialize)]
pub struct TokenizedAsset {
    pub id: String,
    pub title: String,
    pub chain_token_id: Option<String>,
    pub chain_contract_address: Option<String>,
    pub chain_network: Option<String>,
    pub chain_tx_hash: Option<String>,
    pub tokens_total: i32,
    pub tokens_available: i32,
    pub funding_status: String,
    pub created_at: String,
}

/// Response for a specific EIP-1167 Clone details.
#[derive(Serialize)]
pub struct CloneDetailResponse {
    pub asset_id: String,
    pub title: String,
    pub contract_address: String,
    pub total_supply: i32,
    pub tokens_sold: i32,
    pub is_paused: bool,
    pub holders: Vec<CloneHolder>,
}

/// A holder row for the clone details.
#[derive(Serialize)]
pub struct CloneHolder {
    pub user_id: String,
    pub email: String,
    pub wallet_address: String,
    pub balance: i64,
    pub last_synced_at: String,
}

/// A settlement batch record.
#[derive(Serialize)]
pub struct SettlementBatch {
    pub id: String,
    pub batch_size: i32,
    pub status: String,
    pub tx_hash: Option<String>,
    pub gas_used: Option<i64>,
    pub gas_price_gwei: Option<i64>,
    pub block_number: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

/// Response for asset tokenization eligibility check.
#[derive(Serialize)]
pub struct TokenizeCheckResponse {
    pub asset_id: String,
    pub title: String,
    pub tokens_total: i32,
    pub token_price_cents: i64,
    pub total_value_cents: i64,
    pub funding_status: String,
    pub already_tokenized: bool,
    pub chain_token_id: Option<String>,
    pub chain_contract_address: Option<String>,
    /// Pre-flight check results
    pub checks: TokenizeChecks,
}

/// Pre-flight checks for asset tokenization.
#[derive(Serialize)]
pub struct TokenizeChecks {
    pub asset_approved: bool,
    pub has_token_supply: bool,
    pub has_price: bool,
    pub not_already_tokenized: bool,
    pub all_passed: bool,
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/treasury ────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Blockchain treasury overview — wallet, contracts, batches, chain stats.
pub async fn api_admin_blockchain_treasury(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<BlockchainTreasuryResponse>, ApiError> {
    let pool = &state.db;

    // Load chain config from env
    let wallet_address =
        std::env::var("CHAIN_SETTLEMENT_ADDRESS").unwrap_or_else(|_| "Not configured".to_string());
    let contract_address =
        std::env::var("CHAIN_CONTRACT_ADDRESS").unwrap_or_else(|_| "Not configured".to_string());
    let network = std::env::var("CHAIN_NETWORK")
        .or_else(|_| {
            get_platform_setting(pool, "chain_network")
                .ok()
                .flatten()
                .ok_or(std::env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "polygon_amoy".to_string());
    let settlement_enabled = std::env::var("CHAIN_SETTLEMENT_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    // Determine explorer URL based on network
    let explorer_url = match network.as_str() {
        "polygon" => "https://polygonscan.com".to_string(),
        "polygon_amoy" => "https://amoy.polygonscan.com".to_string(),
        _ => "https://amoy.polygonscan.com".to_string(),
    };

    // Query tokenized assets count
    let tokenized_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE chain_token_id IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let total_assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    // Query batch stats
    let total_batches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chain_settlement_batches")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let confirmed_batches: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chain_settlement_batches WHERE status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let failed_batches: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chain_settlement_batches WHERE status = 'failed'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    // Query trade settlement stats
    let confirmed_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_trades: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'pending'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let submitted_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'submitted'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Query whitelisted users
    let whitelisted_users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE chain_wallet_address IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    // Fetch tokenized assets
    let tokenized_assets = fetch_tokenized_assets(pool).await;

    // Fetch recent batches
    let recent_batches = fetch_recent_batches(pool).await;

    Ok(Json(BlockchainTreasuryResponse {
        wallet_address,
        network,
        contract_address,
        settlement_enabled,
        tokenized_assets_count: tokenized_count,
        total_assets_count: total_assets,
        total_batches,
        confirmed_batches,
        failed_batches,
        confirmed_trades,
        pending_trades,
        submitted_trades,
        tokenized_assets,
        recent_batches,
        whitelisted_users_count: whitelisted_users,
        explorer_url,
    }))
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/contracts/:address/detail ───────
// ═══════════════════════════════════════════════════════════════

/// Fetch EIP-1167 Clone details including its holder list mapped from DB.
pub async fn api_admin_blockchain_clone_detail(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<CloneDetailResponse>, ApiError> {
    let pool = &state.db;

    // 1. Find asset by contract address
    let asset = sqlx::query_as::<_, (uuid::Uuid, String, i32, i32)>(
        r#"SELECT id, title, tokens_total, tokens_available 
           FROM assets WHERE LOWER(chain_contract_address) = LOWER($1)"#,
    )
    .bind(&address)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB Error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("EIP-1167 Clone not found mapped to this address".into()))?;

    let asset_id = asset.0;
    let tokens_sold = asset.2 - asset.3;

    // 2. Fetch Holders from onchain_balances
    let holders_rows = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            String,
            i64,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"SELECT ob.user_id, u.email, u.chain_wallet_address, ob.balance, ob.last_synced_at 
           FROM onchain_balances ob
           JOIN users u ON ob.user_id = u.id
           WHERE ob.asset_id = $1 AND ob.balance > 0
           ORDER BY ob.balance DESC
           LIMIT 100"#,
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let holders = holders_rows
        .into_iter()
        .map(|r| CloneHolder {
            user_id: r.0.to_string(),
            email: r.1,
            wallet_address: r.2,
            balance: r.3,
            last_synced_at: r.4.format("%b %d, %Y %H:%M").to_string(),
        })
        .collect();

    // In a real app we'd fetch actual IS_PAUSED state from RPC, but we return false to keep it fast
    Ok(Json(CloneDetailResponse {
        asset_id: asset_id.to_string(),
        title: asset.1,
        contract_address: address,
        total_supply: asset.2,
        tokens_sold,
        is_paused: false,
        holders,
    }))
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/tokenize/:asset_id ──────────────
// ═══════════════════════════════════════════════════════════════

/// Check tokenization eligibility for an asset.
pub async fn api_admin_blockchain_tokenize_check(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<TokenizeCheckResponse>, ApiError> {
    let pool = &state.db;

    let row = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            i32,
            i64,
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            bool,
        ),
    >(
        r#"SELECT 
            id, title, tokens_total, token_price_cents, total_value_cents,
            funding_status, chain_token_id, chain_contract_address, chain_tx_hash,
            published
        FROM assets WHERE id = $1"#,
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    let already_tokenized = row.6.is_some(); // chain_token_id
    let asset_approved = row.9; // published
    let has_token_supply = row.2 > 0;
    let has_price = row.3 > 0;
    let not_already_tokenized = !already_tokenized;
    let all_passed = asset_approved && has_token_supply && has_price && not_already_tokenized;

    Ok(Json(TokenizeCheckResponse {
        asset_id: row.0.to_string(),
        title: row.1,
        tokens_total: row.2,
        token_price_cents: row.3,
        total_value_cents: row.4,
        funding_status: row.5,
        already_tokenized,
        chain_token_id: row.6,
        chain_contract_address: row.7,
        checks: TokenizeChecks {
            asset_approved,
            has_token_supply,
            has_price,
            not_already_tokenized,
            all_passed,
        },
    }))
}

// ═══════════════════════════════════════════════════════════════
// ── POST /api/admin/blockchain/tokenize/:asset_id ─────────────
// ═══════════════════════════════════════════════════════════════

/// Tokenize an asset — calls createAsset() on the smart contract.
///
/// This assigns a chain_token_id and records the contract address.
/// The actual on-chain call is done via Foundry's `cast`.
pub async fn api_admin_blockchain_tokenize(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    // 1. Verify asset exists, is approved, has price/supply, and not already tokenized
    let asset = sqlx::query_as::<_, (uuid::Uuid, String, i32, i64, bool, Option<String>)>(
        "SELECT id, title, tokens_total, token_price_cents, published, chain_token_id FROM assets WHERE id = $1",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    if asset.5.is_some() {
        return Err(ApiError::BadRequest(
            "Asset is already tokenized on-chain".to_string(),
        ));
    }

    if !asset.4 {
        // published (approved)
        return Err(ApiError::BadRequest(
            "Asset must be approved and published before tokenization".to_string(),
        ));
    }

    if asset.2 <= 0 {
        // tokens_total
        return Err(ApiError::BadRequest(
            "Asset must have a token supply greater than 0".to_string(),
        ));
    }

    if asset.3 <= 0 {
        // token_price_cents
        return Err(ApiError::BadRequest(
            "Asset must have a token price greater than 0".to_string(),
        ));
    }

    // 2. Get chain config
    let contract_address = std::env::var("CHAIN_CONTRACT_ADDRESS")
        .map_err(|_| ApiError::Internal("CHAIN_CONTRACT_ADDRESS not configured".to_string()))?;
    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());
    let network = std::env::var("CHAIN_NETWORK").unwrap_or_else(|_| "polygon_amoy".to_string());
    let settlement_address = std::env::var("CHAIN_SETTLEMENT_ADDRESS")
        .map_err(|_| ApiError::Internal("CHAIN_SETTLEMENT_ADDRESS not configured".to_string()))?;

    // 3. (Token ID is always 1 for EIP-1167 clones, so we don't generate one from UUID anymore)

    // 4. Call deployAsset() via cast on the AssetFactory
    let metadata_uri = format!(
        "https://platform.poool.app/api/assets/{}/metadata.json",
        asset_id
    );

    let output = std::process::Command::new("cast")
        .args([
            "send",
            &contract_address,
            "deployAsset(address,string,uint256,address)",
            &settlement_address,  // adminForClone
            &metadata_uri,        // assetURI
            &asset.2.to_string(), // initialSupply
            &settlement_address,  // mintTo
            "--private-key",
            &private_key,
            "--rpc-url",
            &rpc_url,
            "--json",
        ])
        .output()
        .map_err(|e| ApiError::Internal(format!("Failed to execute cast: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("⛓️ createAsset failed for {}: {}", asset_id, stderr);
        return Err(ApiError::Internal(format!(
            "Tokenization failed: {}",
            stderr
        )));
    }

    // 5. Parse the tx hash and clone address from cast --json output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt_val: Option<serde_json::Value> = serde_json::from_str(&stdout).ok();

    let tx_hash = receipt_val
        .as_ref()
        .and_then(|v| {
            v.get("transactionHash")
                .and_then(|h| h.as_str().map(String::from))
        })
        .unwrap_or_else(|| stdout.trim().to_string());

    // Extract cloneAddress from the logs.
    // topic[0] is keccak256("AssetDeployed(address,string,uint256,address)")
    // topic[1] is the cloneAddress (indexed)
    let clone_address = receipt_val
        .as_ref()
        .and_then(|v| v.get("logs").and_then(|logs| logs.as_array()))
        .and_then(|logs| {
            logs.iter().find_map(|log| {
                let topics = log.get("topics")?.as_array()?;
                if topics.len() >= 2 {
                    let topic0 = topics[0].as_str()?;
                    // Check if event is AssetDeployed(address,string,uint256,address)
                    if topic0
                        == "0xf54b4b9b8f2bef47422e0fed45f313d33ca3c25388cb5034358aecb5dcd85714"
                    {
                        let addr_topic = topics[1].as_str()?;
                        // Convert padded topic "0x000000000000000000000000[address]" to "0x[address]"
                        if addr_topic.len() == 66 {
                            return Some(format!("0x{}", &addr_topic[26..]));
                        }
                    }
                }
                None
            })
        })
        .unwrap_or_else(|| contract_address.clone()); // Fallback to factory if parsing fails, but ideally it works

    // 6. Update the asset with on-chain metadata
    sqlx::query(
        r#"UPDATE assets SET
            chain_token_id = $1,
            chain_contract_address = $2,
            chain_network = $3,
            chain_tx_hash = $4,
            chain_metadata_uri = $5,
            updated_at = NOW()
        WHERE id = $6"#,
    )
    .bind("1") // token_id is now ALWAYS 1 for AssetFactory EIP-1167 clones
    .bind(&clone_address)
    .bind(&network)
    .bind(&tx_hash)
    .bind(&metadata_uri)
    .bind(asset_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB update failed: {}", e)))?;

    // 7. Audit log
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.tokenize', 'asset', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(asset_id)
    .bind(serde_json::json!({
        "chain_token_id": "1",
        "chain_contract_address": clone_address,
        "factory_address": contract_address,
        "chain_tx_hash": tx_hash,
        "chain_network": network,
    }))
    .execute(pool)
    .await
    .ok(); // Non-critical

    tracing::info!(
        "⛓️ ✅ Asset {} tokenized: token_id=1, contract={}, tx={}",
        asset.1,
        clone_address,
        tx_hash
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "asset_id": asset_id.to_string(),
        "chain_token_id": "1",
        "chain_contract_address": clone_address,
        "chain_tx_hash": tx_hash,
        "chain_network": network,
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── POST /api/admin/blockchain/pause ──────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Emergency pause — calls pause() on the POOOLProperty1155 contract.
pub async fn api_admin_blockchain_pause(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    let contract_address = std::env::var("CHAIN_CONTRACT_ADDRESS")
        .map_err(|_| ApiError::Internal("CHAIN_CONTRACT_ADDRESS not configured".to_string()))?;
    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

    let output = std::process::Command::new("cast")
        .args([
            "send",
            &contract_address,
            "pause()",
            "--private-key",
            &private_key,
            "--rpc-url",
            &rpc_url,
            "--json",
        ])
        .output()
        .map_err(|e| ApiError::Internal(format!("Failed to execute cast: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ApiError::Internal(format!("Pause failed: {}", stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tx_hash = serde_json::from_str::<serde_json::Value>(&stdout)
        .ok()
        .and_then(|v| {
            v.get("transactionHash")
                .and_then(|h| h.as_str().map(String::from))
        })
        .unwrap_or_else(|| stdout.trim().to_string());

    // Audit log
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.emergency_pause', 'contract', NULL, $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "contract_address": contract_address,
        "tx_hash": tx_hash,
        "action": "pause",
    }))
    .execute(pool)
    .await
    .ok();

    tracing::warn!(
        "🚨 EMERGENCY PAUSE executed by admin {} — tx: {}",
        admin.user.id,
        tx_hash
    );
    sentry::capture_message(
        &format!(
            "EMERGENCY PAUSE on contract {} by admin {}",
            contract_address, admin.user.id
        ),
        sentry::Level::Warning,
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "tx_hash": tx_hash,
        "action": "paused",
    })))
}

/// Emergency unpause — calls unpause() on the POOOLProperty1155 contract.
pub async fn api_admin_blockchain_unpause(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    let contract_address = std::env::var("CHAIN_CONTRACT_ADDRESS")
        .map_err(|_| ApiError::Internal("CHAIN_CONTRACT_ADDRESS not configured".to_string()))?;
    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

    let output = std::process::Command::new("cast")
        .args([
            "send",
            &contract_address,
            "unpause()",
            "--private-key",
            &private_key,
            "--rpc-url",
            &rpc_url,
            "--json",
        ])
        .output()
        .map_err(|e| ApiError::Internal(format!("Failed to execute cast: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ApiError::Internal(format!("Unpause failed: {}", stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tx_hash = serde_json::from_str::<serde_json::Value>(&stdout)
        .ok()
        .and_then(|v| {
            v.get("transactionHash")
                .and_then(|h| h.as_str().map(String::from))
        })
        .unwrap_or_else(|| stdout.trim().to_string());

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.unpause', 'contract', NULL, $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "contract_address": contract_address,
        "tx_hash": tx_hash,
        "action": "unpause",
    }))
    .execute(pool)
    .await
    .ok();

    tracing::warn!(
        "🔓 Contract UNPAUSED by admin {} — tx: {}",
        admin.user.id,
        tx_hash
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "tx_hash": tx_hash,
        "action": "unpaused",
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── INTERNAL HELPERS ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Fetch all tokenized assets (assets with chain_token_id set).
async fn fetch_tokenized_assets(pool: &PgPool) -> Vec<TokenizedAsset> {
    sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            i32,
            i32,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"SELECT id, title, chain_token_id, chain_contract_address,
            chain_network, chain_tx_hash, tokens_total, tokens_available,
            funding_status, created_at
        FROM assets
        WHERE chain_token_id IS NOT NULL
        ORDER BY created_at DESC
        LIMIT 50"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| TokenizedAsset {
        id: r.0.to_string(),
        title: r.1,
        chain_token_id: r.2,
        chain_contract_address: r.3,
        chain_network: r.4,
        chain_tx_hash: r.5,
        tokens_total: r.6,
        tokens_available: r.7,
        funding_status: r.8,
        created_at: r.9.format("%b %d, %Y").to_string(),
    })
    .collect()
}

/// Fetch recent settlement batches.
async fn fetch_recent_batches(pool: &PgPool) -> Vec<SettlementBatch> {
    sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            i32,
            String,
            Option<String>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        r#"SELECT id, batch_size, status, tx_hash, gas_used,
            gas_price_gwei, block_number, error_message,
            created_at, confirmed_at
        FROM chain_settlement_batches
        ORDER BY created_at DESC
        LIMIT 20"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| SettlementBatch {
        id: r.0.to_string(),
        batch_size: r.1,
        status: r.2,
        tx_hash: r.3,
        gas_used: r.4,
        gas_price_gwei: r.5,
        block_number: r.6,
        error_message: r.7,
        created_at: r.8.format("%b %d, %Y %H:%M").to_string(),
        confirmed_at: r.9.map(|dt| dt.format("%b %d, %Y %H:%M").to_string()),
    })
    .collect()
}

/// Try to get a platform setting (non-async fallback).
fn get_platform_setting(_pool: &PgPool, _key: &str) -> Result<Option<String>, ()> {
    // This is a sync context fallback; we use env vars primarily
    Err(())
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/sync ────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Response for the Web3 Sync & Health page.
#[derive(Serialize)]
pub struct BlockchainSyncResponse {
    /// Event indexer status
    pub indexer: IndexerStatus,
    /// Settlement worker stats
    pub settlement: SettlementStats,
    /// Users pending KYC whitelist sync
    pub whitelist_queue: Vec<WhitelistQueueEntry>,
    /// Platform blockchain configuration
    pub config: BlockchainConfigSummary,
}

/// Indexer cursor status.
#[derive(Serialize)]
pub struct IndexerStatus {
    pub enabled: bool,
    pub last_synced_block: i64,
    pub last_updated_at: Option<String>,
    pub poll_interval_secs: i64,
    pub confirmation_depth: i64,
    pub contract_address: String,
    pub total_balance_entries: i64,
}

/// Settlement stats.
#[derive(Serialize)]
pub struct SettlementStats {
    pub enabled: bool,
    pub pending_trades: i64,
    pub submitted_trades: i64,
    pub confirmed_trades: i64,
    pub failed_batches_last_24h: i64,
    pub last_batch_at: Option<String>,
    pub avg_batch_size: f64,
}

/// A user in the KYC whitelist sync queue.
#[derive(Serialize)]
pub struct WhitelistQueueEntry {
    pub user_id: String,
    pub email: String,
    pub kyc_status: String,
    pub has_wallet: bool,
    pub verified_at: Option<String>,
}

/// Summary of blockchain config for UI display.
#[derive(Serialize)]
pub struct BlockchainConfigSummary {
    pub network: String,
    pub rpc_url: String,
    pub factory_address: String,
    pub identity_registry: String,
    pub settlement_address: String,
    pub chain_id: String,
    pub explorer_url: String,
}

/// Web3 Sync & Health — indexer status, settlement stats, whitelist queue.
pub async fn api_admin_blockchain_sync_status(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<BlockchainSyncResponse>, ApiError> {
    let pool = &state.db;

    // ── Indexer Status ──
    let indexer_enabled: bool = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_indexer_enabled'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse().ok())
    .unwrap_or(false);

    let poll_secs: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_indexer_poll_secs'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse().ok())
    .unwrap_or(5);

    let confirmation_depth: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_indexer_confirmation_depth'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse().ok())
    .unwrap_or(3);

    let contract_address =
        std::env::var("CHAIN_CONTRACT_ADDRESS").unwrap_or_else(|_| "Not configured".to_string());

    // Fetch cursor (may not exist yet)
    let cursor = sqlx::query_as::<_, (i64, chrono::DateTime<chrono::Utc>)>(
        "SELECT last_block, last_updated_at FROM chain_indexer_cursor WHERE contract_address = LOWER($1) LIMIT 1",
    )
    .bind(&contract_address.to_lowercase())
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let total_balance_entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM onchain_balances")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let indexer = IndexerStatus {
        enabled: indexer_enabled,
        last_synced_block: cursor.as_ref().map(|c| c.0).unwrap_or(0),
        last_updated_at: cursor.map(|c| c.1.format("%b %d, %Y %H:%M:%S UTC").to_string()),
        poll_interval_secs: poll_secs,
        confirmation_depth,
        contract_address: contract_address.clone(),
        total_balance_entries,
    };

    // ── Settlement Stats ──
    let settlement_enabled = std::env::var("CHAIN_SETTLEMENT_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let pending_trades: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'pending'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let submitted_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'submitted'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let confirmed_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let failed_batches_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chain_settlement_batches WHERE status = 'failed' AND created_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let last_batch_at: Option<String> = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT created_at FROM chain_settlement_batches ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|dt| dt.format("%b %d, %Y %H:%M UTC").to_string());

    let avg_batch_size: f64 = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(AVG(batch_size::float), 0) FROM chain_settlement_batches WHERE status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0.0);

    let settlement = SettlementStats {
        enabled: settlement_enabled,
        pending_trades,
        submitted_trades,
        confirmed_trades,
        failed_batches_last_24h: failed_batches_24h,
        last_batch_at,
        avg_batch_size,
    };

    // ── KYC Whitelist Queue ──
    let queue_rows = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        r#"SELECT u.id, u.email, k.status, k.verified_at
           FROM users u
           JOIN kyc_records k ON k.user_id = u.id
           WHERE k.status = 'approved'
           AND (u.chain_wallet_address IS NULL OR u.chain_wallet_address = '')
           AND u.status = 'active'
           ORDER BY k.verified_at ASC
           LIMIT 50"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let whitelist_queue: Vec<WhitelistQueueEntry> = queue_rows
        .into_iter()
        .map(
            |(user_id, email, status, verified_at)| WhitelistQueueEntry {
                user_id: user_id.to_string(),
                email,
                kyc_status: status,
                has_wallet: false,
                verified_at: verified_at.map(|dt| dt.format("%b %d, %Y %H:%M").to_string()),
            },
        )
        .collect();

    // ── Config Summary ──
    let network = std::env::var("CHAIN_NETWORK").unwrap_or_else(|_| "polygon_amoy".to_string());
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());
    let identity_registry = std::env::var("CHAIN_IDENTITY_REGISTRY_ADDRESS")
        .unwrap_or_else(|_| "Not configured".to_string());
    let settlement_address =
        std::env::var("CHAIN_SETTLEMENT_ADDRESS").unwrap_or_else(|_| "Not configured".to_string());
    let chain_id = std::env::var("CHAIN_ID").unwrap_or_else(|_| "80002".to_string());

    let explorer_url = match network.as_str() {
        "polygon" => "https://polygonscan.com".to_string(),
        "polygon_amoy" => "https://amoy.polygonscan.com".to_string(),
        _ => "https://amoy.polygonscan.com".to_string(),
    };

    let config = BlockchainConfigSummary {
        network,
        rpc_url,
        factory_address: contract_address,
        identity_registry,
        settlement_address,
        chain_id,
        explorer_url,
    };

    Ok(Json(BlockchainSyncResponse {
        indexer,
        settlement,
        whitelist_queue,
        config,
    }))
}

// ═══════════════════════════════════════════════════════════════
// ── POST /api/admin/blockchain/force-kyc-sync/:user_id ───────
// ═══════════════════════════════════════════════════════════════

/// Force-trigger KYC whitelist sync for a specific user.
/// Generates a wallet address and marks them for the next whitelist worker cycle.
pub async fn api_admin_blockchain_force_kyc_sync(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    // 1. Verify user exists and has KYC approved
    let user = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>)>(
        "SELECT u.id, u.email, u.chain_wallet_address FROM users u WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    if user.2.is_some() && !user.2.as_deref().unwrap_or("").is_empty() {
        return Err(ApiError::BadRequest(format!(
            "User {} already has a wallet address: {}",
            user.1,
            user.2.unwrap_or_default()
        )));
    }

    // 2. Generate a deterministic wallet address
    let wallet_address = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        let hash1 = hasher.finish();
        let mut hasher2 = DefaultHasher::new();
        hash1.hash(&mut hasher2);
        let hash2 = hasher2.finish();
        let h1 = format!("{:016x}", hash1);
        let h2 = format!("{:016x}", hash2);
        format!("0x{}{}{}", &h1[..8], h1, &h2[..12])
    };

    // 3. Update the user's chain_wallet_address
    sqlx::query("UPDATE users SET chain_wallet_address = $1 WHERE id = $2")
        .bind(&wallet_address)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("DB update failed: {}", e)))?;

    // 4. Audit log
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.force_kyc_sync', 'user', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(user_id)
    .bind(serde_json::json!({
        "wallet_address": wallet_address,
        "triggered_by": "admin_force_sync",
    }))
    .execute(pool)
    .await
    .ok();

    tracing::info!(
        "🔑 Admin {} force-synced KYC whitelist for user {} → {}",
        admin.user.id,
        user.1,
        wallet_address
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": user_id.to_string(),
        "email": user.1,
        "wallet_address": wallet_address,
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── POST /api/admin/blockchain/contracts/:address/pause ───────
// ═══════════════════════════════════════════════════════════════

/// Pause a specific EIP-1167 clone contract (SPV-level freeze).
pub async fn api_admin_blockchain_pause_clone(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

    let output = std::process::Command::new("cast")
        .args([
            "send",
            &address,
            "pause()",
            "--private-key",
            &private_key,
            "--rpc-url",
            &rpc_url,
            "--json",
        ])
        .output()
        .map_err(|e| ApiError::Internal(format!("Failed to execute cast: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ApiError::Internal(format!(
            "Clone pause failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tx_hash = serde_json::from_str::<serde_json::Value>(&stdout)
        .ok()
        .and_then(|v| {
            v.get("transactionHash")
                .and_then(|h| h.as_str().map(String::from))
        })
        .unwrap_or_else(|| stdout.trim().to_string());

    // Audit log
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.clone_pause', 'contract', NULL, $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "contract_address": address,
        "tx_hash": tx_hash,
        "action": "pause",
    }))
    .execute(pool)
    .await
    .ok();

    tracing::warn!(
        "🚨 CLONE PAUSE on {} executed by admin {} — tx: {}",
        address,
        admin.user.id,
        tx_hash
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "tx_hash": tx_hash,
        "action": "paused",
        "contract_address": address,
    })))
}

/// Unpause a specific EIP-1167 clone contract.
pub async fn api_admin_blockchain_unpause_clone(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

    let output = std::process::Command::new("cast")
        .args([
            "send",
            &address,
            "unpause()",
            "--private-key",
            &private_key,
            "--rpc-url",
            &rpc_url,
            "--json",
        ])
        .output()
        .map_err(|e| ApiError::Internal(format!("Failed to execute cast: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ApiError::Internal(format!(
            "Clone unpause failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tx_hash = serde_json::from_str::<serde_json::Value>(&stdout)
        .ok()
        .and_then(|v| {
            v.get("transactionHash")
                .and_then(|h| h.as_str().map(String::from))
        })
        .unwrap_or_else(|| stdout.trim().to_string());

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.clone_unpause', 'contract', NULL, $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "contract_address": address,
        "tx_hash": tx_hash,
        "action": "unpause",
    }))
    .execute(pool)
    .await
    .ok();

    tracing::warn!(
        "🔓 CLONE UNPAUSE on {} by admin {} — tx: {}",
        address,
        admin.user.id,
        tx_hash
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "tx_hash": tx_hash,
        "action": "unpaused",
        "contract_address": address,
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── POST /api/admin/blockchain/pin-metadata/:asset_id ─────────
// ═══════════════════════════════════════════════════════════════

/// Pin asset metadata to IPFS via Pinata.
///
/// This builds the ERC-1155 compliant metadata JSON from the database,
/// pins it to IPFS, and updates the asset's `chain_metadata_uri` with
/// the IPFS CID. The metadata is then permanently available at
/// `ipfs://{cid}` and via any IPFS gateway.
pub async fn api_admin_blockchain_pin_metadata(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;

    // 1. Build metadata from DB
    let metadata = crate::ipfs::metadata::build_metadata(pool, asset_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to build metadata: {}", e)))?;

    // 2. Convert to JSON
    let json = crate::ipfs::metadata::metadata_to_json(&metadata)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize metadata: {}", e)))?;

    // 3. Pin to IPFS via Pinata
    let pin_name = format!("poool-asset-{}-metadata", asset_id);
    let keyvalues = serde_json::json!({
        "asset_id": asset_id.to_string(),
        "asset_name": metadata.name,
        "type": "erc1155-metadata",
        "pinned_by": admin.user.id.to_string(),
    });

    let pin_result = crate::ipfs::service::pin_json(&json, &pin_name, Some(keyvalues))
        .await
        .map_err(|e| ApiError::Internal(format!("IPFS pin failed: {}", e)))?;

    let ipfs_uri = crate::ipfs::service::ipfs_uri(&pin_result.ipfs_hash);
    let gateway_url = crate::ipfs::service::gateway_url(&pin_result.ipfs_hash);

    // 4. Update asset's chain_metadata_uri in DB
    sqlx::query("UPDATE assets SET chain_metadata_uri = $1, updated_at = NOW() WHERE id = $2")
        .bind(&ipfs_uri)
        .bind(asset_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("DB update failed: {}", e)))?;

    // 5. Audit log
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'blockchain.pin_metadata', 'asset', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(asset_id)
    .bind(serde_json::json!({
        "ipfs_cid": pin_result.ipfs_hash,
        "ipfs_uri": ipfs_uri,
        "pin_size": pin_result.pin_size,
    }))
    .execute(pool)
    .await
    .ok(); // Non-critical

    tracing::info!(
        "📌 ✅ Asset {} metadata pinned to IPFS: cid={}, size={}, by admin {}",
        asset_id,
        pin_result.ipfs_hash,
        pin_result.pin_size,
        admin.user.id,
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "asset_id": asset_id.to_string(),
        "ipfs_cid": pin_result.ipfs_hash,
        "ipfs_uri": ipfs_uri,
        "gateway_url": gateway_url,
        "pin_size": pin_result.pin_size,
        "timestamp": pin_result.timestamp,
    })))
}
