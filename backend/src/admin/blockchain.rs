//! Admin blockchain API — tokenization, treasury, emergency controls.
use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

const BLOCKCHAIN_CONTROL_PERMISSION: &str = "blockchain.manage";
const BLOCKCHAIN_READ_PERMISSION: &str = "treasury.read";
const BLOCKCHAIN_TOKENIZE_PERMISSION: &str = "blockchain.tokenize";

/// Resolve settlement wallet address.
///
/// **Key-first** — derive from the actual private key whenever possible.
/// Env `CHAIN_SETTLEMENT_ADDRESS` is consulted only as a *display* fallback
/// when no key is configured. This is intentional: when the env var was
/// allowed to override key-derivation, ops accidentally set it to a
/// throwaway address and used it as `mintTo` at deploy — minting 2,000
/// Demo Villa tokens to a wallet whose private key was never saved.
/// Defensive default: address always matches the active signer, so the
/// signer can always operate on funds it just minted.
fn resolve_settlement_address() -> String {
    if let Ok(pk) = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY") {
        if let Ok(addr) = crate::blockchain::signing::address_from_private_key(&pk) {
            return addr;
        }
    }
    // KMS-backed signer — env var holds the pre-computed display address
    // because we can't synchronously derive without an RPC roundtrip.
    if let Ok(addr) = std::env::var("CHAIN_SETTLEMENT_ADDRESS") {
        if !addr.is_empty() {
            return addr;
        }
    }
    "Not configured".to_string()
}

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
    /// Contract address for POOOLAssetToken
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
    pub pause_state: String,
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
    pub chain_network: String,
    pub explorer_url: String,
    pub metadata_uri: String,
    pub already_tokenized: bool,
    pub published: bool,
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
    pub legal_documents_present: bool,
    pub funding_ready: bool,
    pub metadata_uri_ready: bool,
    pub chain_configured: bool,
    pub operator_can_tokenize: bool,
    pub not_already_tokenized: bool,
    pub all_passed: bool,
}

/// Candidate asset for the generic tokenization page picker.
#[derive(Serialize)]
pub struct TokenizeCandidate {
    pub asset_id: String,
    pub title: String,
    pub funding_status: String,
    pub tokens_total: i32,
    pub token_price_cents: i64,
    pub total_value_cents: i64,
    pub already_tokenized: bool,
    pub created_at: String,
    pub updated_at: String,
    pub document_count: i64,
    pub risk_flags: Vec<RiskFlag>,
}

/// A risk signal for an asset, computed server-side from authoritative state.
#[derive(Serialize)]
pub struct RiskFlag {
    pub code: String,
    pub severity: String, // "warn" | "danger"
    pub message: String,
}

/// Response for tokenizable asset candidates.
#[derive(Serialize)]
pub struct TokenizeCandidatesResponse {
    pub assets: Vec<TokenizeCandidate>,
    pub wallet_address: String,
    pub contract_address: String,
    pub network: String,
    pub explorer_url: String,
    /// Settlement-wallet gas balance in wei (string, may exceed i64). None if never sampled.
    pub deployer_balance_wei: Option<String>,
    /// When the balance was last read by the gas-monitor worker.
    pub deployer_balance_checked_at: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/treasury ────────────────────────
// ═══════════════════════════════════════════════════════════════

fn validate_contract_address(address: &str) -> Result<String, ApiError> {
    let trimmed = address.trim();
    let is_valid = trimmed.len() == 42
        && trimmed.starts_with("0x")
        && trimmed[2..].chars().all(|c| c.is_ascii_hexdigit());

    if is_valid {
        Ok(trimmed.to_ascii_lowercase())
    } else {
        Err(ApiError::BadRequest(
            "Invalid contract address format".to_string(),
        ))
    }
}

async fn has_exact_permission(
    pool: &PgPool,
    user_id: uuid::Uuid,
    permission: &str,
) -> Result<bool, ApiError> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM user_roles ur
            JOIN admin_permissions ap ON ap.role_id = ur.role_id
            WHERE ur.user_id = $1
              AND ur.is_active = TRUE
              AND ap.permission = $2
        )
        "#,
    )
    .bind(user_id)
    .bind(permission)
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)
}

pub async fn has_blockchain_tokenize_permission(pool: &PgPool, user_id: uuid::Uuid) -> bool {
    has_exact_permission(pool, user_id, BLOCKCHAIN_TOKENIZE_PERMISSION)
        .await
        .unwrap_or(false)
}

async fn require_blockchain_tokenize_permission(
    admin: &AdminUser,
    pool: &PgPool,
) -> Result<(), ApiError> {
    if has_exact_permission(pool, admin.user.id, BLOCKCHAIN_TOKENIZE_PERMISSION).await? {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "Missing permission: {}",
            BLOCKCHAIN_TOKENIZE_PERMISSION
        )))
    }
}

async fn require_blockchain_control_permission(
    admin: &AdminUser,
    pool: &PgPool,
) -> Result<(), ApiError> {
    if admin.is_super_admin(pool).await
        || has_exact_permission(pool, admin.user.id, BLOCKCHAIN_CONTROL_PERMISSION).await?
    {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "Missing permission: {}",
            BLOCKCHAIN_CONTROL_PERMISSION
        )))
    }
}

async fn fetch_clone_asset(
    pool: &PgPool,
    normalized_address: &str,
) -> Result<(uuid::Uuid, String, i32, i32, Option<bool>), ApiError> {
    sqlx::query_as::<_, (uuid::Uuid, String, i32, i32, Option<bool>)>(
        r#"
        SELECT a.id, a.title, a.tokens_total, a.tokens_available, c.is_paused
        FROM assets a
        LEFT JOIN chain_contract_controls c
          ON LOWER(c.contract_address) = LOWER(a.chain_contract_address)
        WHERE LOWER(a.chain_contract_address) = LOWER($1)
        "#,
    )
    .bind(normalized_address)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::from)?
    .ok_or_else(|| ApiError::NotFound("EIP-1167 Clone not found mapped to this address".into()))
}

fn parse_cast_tx_hash(stdout: &str) -> String {
    serde_json::from_str::<serde_json::Value>(stdout)
        .ok()
        .and_then(|v| {
            v.get("transactionHash")
                .and_then(|h| h.as_str().map(String::from))
        })
        .unwrap_or_else(|| stdout.trim().to_string())
}

fn explorer_url_for_network(network: &str) -> &'static str {
    match network {
        "polygon" | "polygon_mainnet" => "https://polygonscan.com",
        _ => "https://amoy.polygonscan.com",
    }
}

fn chain_tokenize_mock_enabled() -> bool {
    match std::env::var("CHAIN_TOKENIZE_MOCK") {
        Ok(value) => value.eq_ignore_ascii_case("true") || value == "1",
        Err(_) => cfg!(debug_assertions),
    }
}

fn chain_configured_for_tokenize() -> bool {
    chain_tokenize_mock_enabled()
        || (std::env::var("CHAIN_CONTRACT_ADDRESS").is_ok()
            && (std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").is_ok()
                || std::env::var("CHAIN_KMS_KEY").is_ok()))
}

fn parse_clone_address_from_receipt(receipt_val: &serde_json::Value) -> Result<String, ApiError> {
    let clone_address = receipt_val
        .get("logs")
        .and_then(|logs| logs.as_array())
        .and_then(|logs| {
            logs.iter().find_map(|log| {
                let topics = log.get("topics")?.as_array()?;
                if topics.len() < 2 {
                    return None;
                }
                let topic0 = topics[0].as_str()?;
                if topic0 != "0xf54b4b9b8f2bef47422e0fed45f313d33ca3c25388cb5034358aecb5dcd85714" {
                    return None;
                }
                let addr_topic = topics[1].as_str()?;
                if addr_topic.len() == 66 && addr_topic.starts_with("0x") {
                    return Some(format!("0x{}", &addr_topic[26..]));
                }
                None
            })
        })
        .ok_or_else(|| {
            ApiError::Internal(
                "Tokenization transaction did not emit a parseable clone address".to_string(),
            )
        })?;

    validate_contract_address(&clone_address)
}

fn execute_asset_tokenization(
    factory_address: &str,
    settlement_address: &str,
    metadata_uri: &str,
    initial_supply: i32,
) -> Result<(String, String, bool), ApiError> {
    if chain_tokenize_mock_enabled() {
        let tx_suffix = uuid::Uuid::new_v4().simple().to_string();
        let clone_suffix = uuid::Uuid::new_v4().simple().to_string();
        return Ok((
            format!("0x{tx_suffix:0<64}"),
            format!("0x{clone_suffix:0<40}"),
            true,
        ));
    }

    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

    let output = std::process::Command::new("cast")
        .args([
            "send",
            factory_address,
            "deployAsset(address,string,uint256,address)",
            settlement_address,
            metadata_uri,
            &initial_supply.to_string(),
            settlement_address,
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
            "Tokenization failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt_val: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        ApiError::Internal(format!(
            "Tokenization returned unparseable receipt JSON: {}",
            e
        ))
    })?;
    let tx_hash = receipt_val
        .get("transactionHash")
        .and_then(|h| h.as_str().map(String::from))
        .ok_or_else(|| {
            ApiError::Internal("Tokenization receipt missing transaction hash".to_string())
        })?;
    let clone_address = parse_clone_address_from_receipt(&receipt_val)?;

    Ok((tx_hash, clone_address, false))
}

async fn mark_tokenization_job_failed(pool: &PgPool, job_id: uuid::Uuid, message: &str) {
    if let Err(err) = sqlx::query(
        r#"
        UPDATE asset_tokenization_jobs
        SET status = 'failed',
            error_message = $2,
            updated_at = NOW(),
            completed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(message.chars().take(2000).collect::<String>())
    .execute(pool)
    .await
    {
        tracing::error!("Failed to mark tokenization job failed: {}", err);
    }
}

fn execute_clone_control(address: &str, method: &str) -> Result<(String, bool), ApiError> {
    if std::env::var("CHAIN_CONTROL_MOCK")
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false)
    {
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        return Ok((format!("0x{suffix:0<64}"), true));
    }

    let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").map_err(|_| {
        ApiError::Internal("CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string())
    })?;
    let rpc_url = std::env::var("CHAIN_RPC_URL")
        .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

    let selector = match method {
        "pause" => "pause()",
        "unpause" => "unpause()",
        _ => {
            return Err(ApiError::BadRequest(
                "Unsupported blockchain control action".to_string(),
            ))
        }
    };

    let output = std::process::Command::new("cast")
        .args([
            "send",
            address,
            selector,
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
            "Clone {} failed: {}",
            method, stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok((parse_cast_tx_hash(&stdout), false))
}

async fn persist_clone_control_result(
    pool: &PgPool,
    admin: &AdminUser,
    asset_id: uuid::Uuid,
    address: &str,
    action: &str,
    is_paused: bool,
    tx_hash: &str,
    mocked: bool,
) -> Result<(), ApiError> {
    let mut tx = pool.begin().await.map_err(ApiError::from)?;

    sqlx::query(
        r#"
        INSERT INTO chain_contract_controls (
            contract_address, asset_id, is_paused, last_action,
            last_tx_hash, updated_by, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        ON CONFLICT (contract_address) DO UPDATE SET
            asset_id = EXCLUDED.asset_id,
            is_paused = EXCLUDED.is_paused,
            last_action = EXCLUDED.last_action,
            last_tx_hash = EXCLUDED.last_tx_hash,
            updated_by = EXCLUDED.updated_by,
            updated_at = NOW()
        "#,
    )
    .bind(address)
    .bind(asset_id)
    .bind(is_paused)
    .bind(action)
    .bind(tx_hash)
    .bind(admin.user.id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, $2, 'contract', $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(format!("blockchain.clone_{}", action))
    .bind(asset_id)
    .bind(serde_json::json!({
        "contract_address": address,
        "tx_hash": tx_hash,
        "action": action,
        "is_paused": is_paused,
        "mocked": mocked,
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)
}

/// Blockchain treasury overview — wallet, contracts, batches, chain stats.
pub async fn api_admin_blockchain_treasury(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<BlockchainTreasuryResponse>, ApiError> {
    let pool = &state.db;
    admin.require_permission(pool, "treasury.read").await?;

    // Load chain config from env
    let wallet_address = resolve_settlement_address();
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
            .map_err(ApiError::from)?;

    let total_assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
        .fetch_one(pool)
        .await
        .map_err(ApiError::from)?;

    // Query batch stats
    let total_batches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chain_settlement_batches")
        .fetch_one(pool)
        .await
        .map_err(ApiError::from)?;

    let confirmed_batches: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chain_settlement_batches WHERE status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)?;

    let failed_batches: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chain_settlement_batches WHERE status = 'failed'")
            .fetch_one(pool)
            .await
            .map_err(ApiError::from)?;

    // Query trade settlement stats
    let confirmed_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)?;

    let pending_trades: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'pending'")
            .fetch_one(pool)
            .await
            .map_err(ApiError::from)?;

    let submitted_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'submitted'",
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)?;

    // Query whitelisted users
    let whitelisted_users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE chain_wallet_address IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(ApiError::from)?;

    // Fetch tokenized assets
    let tokenized_assets = fetch_tokenized_assets(pool).await?;

    // Fetch recent batches
    let recent_batches = fetch_recent_batches(pool).await?;

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
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<CloneDetailResponse>, ApiError> {
    let pool = &state.db;
    admin.require_permission(pool, "treasury.read").await?;
    let normalized_address = validate_contract_address(&address)?;

    // 1. Find asset by contract address
    let asset = fetch_clone_asset(pool, &normalized_address).await?;

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
           WHERE ob.asset_id = $1
             AND ob.balance > 0
             AND u.chain_wallet_address IS NOT NULL
             AND u.chain_wallet_address <> ''
           ORDER BY ob.balance DESC
           LIMIT 100"#,
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::from)?;

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

    let pause_state = match asset.4 {
        Some(true) => "paused",
        Some(false) => "live",
        None => "unknown",
    }
    .to_string();

    Ok(Json(CloneDetailResponse {
        asset_id: asset_id.to_string(),
        title: asset.1,
        contract_address: normalized_address,
        total_supply: asset.2,
        tokens_sold,
        is_paused: asset.4.unwrap_or(false),
        pause_state,
        holders,
    }))
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/tokenize-candidates ─────────────
// ═══════════════════════════════════════════════════════════════

/// List assets that can be inspected from the generic tokenization page.
pub async fn api_admin_blockchain_tokenize_candidates(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<TokenizeCandidatesResponse>, ApiError> {
    require_blockchain_tokenize_permission(&admin, &state.db).await?;
    let pool = &state.db;

    let rows = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            String,
            i32,
            i64,
            i64,
            Option<String>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            i64,
        ),
    >(
        r#"
        SELECT a.id, a.title, a.funding_status, a.tokens_total, a.token_price_cents,
               a.total_value_cents, a.chain_token_id, a.chain_contract_address,
               a.created_at, a.updated_at,
               (SELECT COUNT(*) FROM asset_documents ad WHERE ad.asset_id = a.id) AS document_count
        FROM assets a
        WHERE a.published = TRUE
        ORDER BY a.chain_token_id IS NOT NULL, a.updated_at DESC, a.created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(ApiError::from)?;

    let wallet_address = resolve_settlement_address();
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
    let explorer_url = match network.as_str() {
        "polygon" | "polygon_mainnet" => "https://polygonscan.com".to_string(),
        _ => "https://amoy.polygonscan.com".to_string(),
    };

    // Look up cached deployer-wallet gas balance (gas_monitor worker writes this every 5 min).
    let (deployer_balance_wei, deployer_balance_checked_at) = if isaddr(&wallet_address) {
        sqlx::query_as::<_, (String, chrono::DateTime<chrono::Utc>)>(
            "SELECT balance_wei, checked_at FROM chain_wallet_balance WHERE address = $1",
        )
        .bind(wallet_address.to_lowercase())
        .fetch_optional(pool)
        .await
        .map_err(ApiError::from)?
        .map(|(bal, ts)| (Some(bal), Some(ts.to_rfc3339())))
        .unwrap_or((None, None))
    } else {
        (None, None)
    };

    Ok(Json(TokenizeCandidatesResponse {
        assets: rows
            .into_iter()
            .map(|row| {
                let already_tokenized = row.6.is_some();
                let has_chain_contract = row.7.is_some();
                let risk_flags = compute_risk_flags(
                    &row.2,
                    row.3,
                    row.4,
                    already_tokenized,
                    has_chain_contract,
                    row.9,
                    row.10,
                );
                TokenizeCandidate {
                    asset_id: row.0.to_string(),
                    title: row.1,
                    funding_status: row.2,
                    tokens_total: row.3,
                    token_price_cents: row.4,
                    total_value_cents: row.5,
                    already_tokenized,
                    created_at: row.8.to_rfc3339(),
                    updated_at: row.9.to_rfc3339(),
                    document_count: row.10,
                    risk_flags,
                }
            })
            .collect(),
        wallet_address,
        contract_address,
        network,
        explorer_url,
        deployer_balance_wei,
        deployer_balance_checked_at,
    }))
}

fn compute_risk_flags(
    funding_status: &str,
    tokens_total: i32,
    token_price_cents: i64,
    already_tokenized: bool,
    has_chain_contract: bool,
    updated_at: chrono::DateTime<chrono::Utc>,
    document_count: i64,
) -> Vec<RiskFlag> {
    let mut flags = Vec::new();

    let age_days = (chrono::Utc::now() - updated_at).num_days();
    if age_days > 30 && !already_tokenized {
        flags.push(RiskFlag {
            code: "stale".into(),
            severity: "danger".into(),
            message: format!(
                "Stale: no admin update for {} days. Re-verify before deploying.",
                age_days
            ),
        });
    }

    if funding_status == "exited" && !already_tokenized {
        flags.push(RiskFlag {
            code: "exited_without_token".into(),
            severity: "danger".into(),
            message: "Asset marked exited but never tokenized — likely shouldn't deploy.".into(),
        });
    }

    if funding_status == "funded" && !already_tokenized {
        flags.push(RiskFlag {
            code: "funded_without_token".into(),
            severity: "warn".into(),
            message: "Funded off-chain without tokenization — confirm operator intent.".into(),
        });
    }

    if tokens_total <= 0 {
        flags.push(RiskFlag {
            code: "zero_supply".into(),
            severity: "danger".into(),
            message: "Token supply is zero or missing.".into(),
        });
    }

    if token_price_cents <= 0 {
        flags.push(RiskFlag {
            code: "zero_price".into(),
            severity: "danger".into(),
            message: "Token price is zero or missing.".into(),
        });
    }

    if document_count == 0 && !already_tokenized {
        flags.push(RiskFlag {
            code: "no_documents".into(),
            severity: "warn".into(),
            message: "No legal documents attached. Pre-flight check will fail.".into(),
        });
    }

    if already_tokenized && !has_chain_contract {
        flags.push(RiskFlag {
            code: "broken_chain_state".into(),
            severity: "danger".into(),
            message: "Token ID stored but contract address missing — inconsistent on-chain state."
                .into(),
        });
    }

    flags
}

fn isaddr(s: &str) -> bool {
    let t = s.trim();
    t.len() == 42 && t.starts_with("0x") && t[2..].chars().all(|c| c.is_ascii_hexdigit())
}

// ═══════════════════════════════════════════════════════════════
// ── GET /api/admin/blockchain/tokenize/:asset_id ──────────────
// ═══════════════════════════════════════════════════════════════

/// Check tokenization eligibility for an asset.
pub async fn api_admin_blockchain_tokenize_check(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<TokenizeCheckResponse>, ApiError> {
    let pool = &state.db;
    require_blockchain_tokenize_permission(&admin, pool).await?;

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
            i64,
        ),
    >(
        r#"SELECT 
            id, title, tokens_total, token_price_cents, total_value_cents,
            funding_status, chain_token_id, chain_contract_address, chain_tx_hash,
            published,
            (SELECT COUNT(*) FROM asset_documents ad WHERE ad.asset_id = assets.id) AS document_count
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
    let legal_documents_present = row.10 > 0;
    let funding_ready = matches!(
        row.5.as_str(),
        "funding_open" | "funding_in_progress" | "funded" | "rented"
    );
    let network = std::env::var("CHAIN_NETWORK").unwrap_or_else(|_| "polygon_amoy".to_string());
    let metadata_uri = format!(
        "https://platform.poool.app/api/assets/{}/metadata.json",
        asset_id
    );
    let metadata_uri_ready = true;
    let chain_configured = chain_configured_for_tokenize();
    let operator_can_tokenize = true;
    let not_already_tokenized = !already_tokenized;
    let all_passed = asset_approved
        && has_token_supply
        && has_price
        && legal_documents_present
        && funding_ready
        && metadata_uri_ready
        && chain_configured
        && operator_can_tokenize
        && not_already_tokenized;

    Ok(Json(TokenizeCheckResponse {
        asset_id: row.0.to_string(),
        title: row.1,
        tokens_total: row.2,
        token_price_cents: row.3,
        total_value_cents: row.4,
        funding_status: row.5,
        chain_network: network.clone(),
        explorer_url: explorer_url_for_network(&network).to_string(),
        metadata_uri,
        already_tokenized,
        published: row.9,
        chain_token_id: row.6,
        chain_contract_address: row.7,
        checks: TokenizeChecks {
            asset_approved,
            has_token_supply,
            has_price,
            legal_documents_present,
            funding_ready,
            metadata_uri_ready,
            chain_configured,
            operator_can_tokenize,
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
    require_blockchain_tokenize_permission(&admin, pool).await?;

    // 1. Claim an idempotency guard before any irreversible on-chain call.
    let mut claim_tx = pool.begin().await.map_err(ApiError::from)?;
    let asset = sqlx::query_as::<_, (uuid::Uuid, String, i32, i64, bool, Option<String>, String, i64)>(
        r#"
        SELECT id, title, tokens_total, token_price_cents, published, chain_token_id,
               funding_status,
               (SELECT COUNT(*) FROM asset_documents ad WHERE ad.asset_id = assets.id) AS document_count
        FROM assets
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&mut *claim_tx)
    .await
    .map_err(ApiError::from)?
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

    if !matches!(
        asset.6.as_str(),
        "funding_open" | "funding_in_progress" | "funded" | "rented"
    ) {
        return Err(ApiError::BadRequest(
            "Asset funding status is not ready for tokenization".to_string(),
        ));
    }

    if asset.7 <= 0 {
        return Err(ApiError::BadRequest(
            "At least one asset document is required before tokenization".to_string(),
        ));
    }

    let inserted_job = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO asset_tokenization_jobs (asset_id, requested_by, status)
        VALUES ($1, $2, 'in_progress')
        ON CONFLICT (asset_id) WHERE status = 'in_progress' DO NOTHING
        RETURNING id
        "#,
    )
    .bind(asset_id)
    .bind(admin.user.id)
    .fetch_optional(&mut *claim_tx)
    .await
    .map_err(ApiError::from)?;

    let Some(job_id) = inserted_job else {
        return Err(ApiError::Conflict(
            "Tokenization is already in progress for this asset".to_string(),
        ));
    };

    claim_tx.commit().await.map_err(ApiError::from)?;

    // 2. Get chain config
    let contract_address = match validate_contract_address(
        &std::env::var("CHAIN_CONTRACT_ADDRESS")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000001".to_string()),
    ) {
        Ok(address) => address,
        Err(err) => {
            mark_tokenization_job_failed(pool, job_id, "Invalid CHAIN_CONTRACT_ADDRESS").await;
            return Err(err);
        }
    };
    let network = std::env::var("CHAIN_NETWORK").unwrap_or_else(|_| "polygon_amoy".to_string());
    let settlement_address = match validate_contract_address(&resolve_settlement_address()) {
        Ok(address) => address,
        Err(err) => {
            mark_tokenization_job_failed(pool, job_id, "Invalid CHAIN_SETTLEMENT_ADDRESS").await;
            return Err(err);
        }
    };

    if !chain_tokenize_mock_enabled() && std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").is_err() {
        mark_tokenization_job_failed(pool, job_id, "CHAIN_SETTLEMENT_PRIVATE_KEY not configured")
            .await;
        return Err(ApiError::Internal(
            "CHAIN_SETTLEMENT_PRIVATE_KEY not configured".to_string(),
        ));
    }

    // 3. (Token ID is always 1 for EIP-1167 clones, so we don't generate one from UUID anymore)

    // 4. Call deployAsset() via cast on the AssetFactory
    let metadata_uri = format!(
        "https://platform.poool.app/api/assets/{}/metadata.json",
        asset_id
    );

    let (tx_hash, clone_address, mocked) = match execute_asset_tokenization(
        &contract_address,
        &settlement_address,
        &metadata_uri,
        asset.2,
    ) {
        Ok(result) => result,
        Err(err) => {
            mark_tokenization_job_failed(pool, job_id, &format!("{err:?}")).await;
            return Err(err);
        }
    };

    // 6. Persist chain metadata and mandatory audit record atomically.
    let mut tx = pool.begin().await.map_err(ApiError::from)?;
    let updated = sqlx::query(
        r#"UPDATE assets SET
            chain_token_id = $1,
            chain_contract_address = $2,
            chain_network = $3,
            chain_tx_hash = $4,
            chain_metadata_uri = $5,
            updated_at = NOW()
        WHERE id = $6 AND chain_token_id IS NULL"#,
    )
    .bind("1") // token_id is now ALWAYS 1 for AssetFactory EIP-1167 clones
    .bind(&clone_address)
    .bind(&network)
    .bind(&tx_hash)
    .bind(&metadata_uri)
    .bind(asset_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    if updated.rows_affected() != 1 {
        mark_tokenization_job_failed(pool, job_id, "Asset was tokenized before final persistence")
            .await;
        return Err(ApiError::Conflict(
            "Asset was tokenized before this request could finish".to_string(),
        ));
    }

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
        "mocked": mocked,
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    sqlx::query(
        r#"
        UPDATE asset_tokenization_jobs
        SET status = 'succeeded',
            chain_network = $2,
            factory_address = $3,
            clone_address = $4,
            chain_tx_hash = $5,
            metadata_uri = $6,
            updated_at = NOW(),
            completed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(&network)
    .bind(&contract_address)
    .bind(&clone_address)
    .bind(&tx_hash)
    .bind(&metadata_uri)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;

    // Lift any of this asset's already-completed primary order_items
    // from NULL → 'pending' so the settlement worker picks them up next
    // cycle. Common path: re-tokenization after a broken first deploy.
    match crate::blockchain::primary_settlement::mark_asset_eligible_after_tokenization(
        pool, asset_id,
    )
    .await
    {
        Ok(0) => {}
        Ok(n) => tracing::info!(
            "⛓️ Tokenization: lifted {} order_items to settlement-eligible for asset {}",
            n,
            asset_id
        ),
        Err(e) => tracing::error!(
            "⛓️ Tokenization: failed to lift order_items for asset {}: {}",
            asset_id,
            e
        ),
    }

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
        "mocked": mocked,
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── POST /api/admin/blockchain/pause ──────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Emergency pause — calls pause() on the POOOLAssetToken contract.
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

/// Emergency unpause — calls unpause() on the POOOLAssetToken contract.
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
async fn fetch_tokenized_assets(pool: &PgPool) -> Result<Vec<TokenizedAsset>, ApiError> {
    let rows = sqlx::query_as::<
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
    .map_err(ApiError::from)?;

    Ok(rows
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
        .collect())
}

/// Fetch recent settlement batches.
async fn fetch_recent_batches(pool: &PgPool) -> Result<Vec<SettlementBatch>, ApiError> {
    let rows = sqlx::query_as::<
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
    .map_err(ApiError::from)?;

    Ok(rows
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
        .collect())
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<BlockchainSyncResponse>, ApiError> {
    let pool = &state.db;
    admin
        .require_permission(pool, BLOCKCHAIN_READ_PERMISSION)
        .await?;

    // ── Indexer Status ──
    let indexer_enabled = parse_optional_setting(
        "chain_indexer_enabled",
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM platform_settings WHERE key = 'chain_indexer_enabled'",
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read indexer setting: {}", e)))?,
        false,
    )?;

    let poll_secs = parse_optional_setting(
        "chain_indexer_poll_secs",
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM platform_settings WHERE key = 'chain_indexer_poll_secs'",
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read poll setting: {}", e)))?,
        5_i64,
    )?;

    let confirmation_depth = parse_optional_setting(
        "chain_indexer_confirmation_depth",
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM platform_settings WHERE key = 'chain_indexer_confirmation_depth'",
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read confirmation setting: {}", e)))?,
        3_i64,
    )?;

    let contract_address =
        std::env::var("CHAIN_CONTRACT_ADDRESS").unwrap_or_else(|_| "Not configured".to_string());

    // Fetch cursor (may not exist yet)
    let cursor = sqlx::query_as::<_, (i64, chrono::DateTime<chrono::Utc>)>(
        "SELECT last_block, last_updated_at FROM chain_indexer_cursor WHERE contract_address = LOWER($1) LIMIT 1",
    )
    .bind(&contract_address.to_lowercase())
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to read indexer cursor: {}", e)))?;

    let total_balance_entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM onchain_balances")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to count on-chain balances: {}", e)))?;

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
            .map_err(|e| ApiError::Internal(format!("Failed to count pending trades: {}", e)))?;

    let submitted_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'submitted'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to count submitted trades: {}", e)))?;

    let confirmed_trades: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trade_history WHERE on_chain_status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to count confirmed trades: {}", e)))?;

    let failed_batches_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chain_settlement_batches WHERE status = 'failed' AND created_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to count failed batches: {}", e)))?;

    let last_batch_at: Option<String> = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT created_at FROM chain_settlement_batches ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to read last settlement batch: {}", e)))?
    .map(|dt| dt.format("%b %d, %Y %H:%M UTC").to_string());

    let avg_batch_size: f64 = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(AVG(batch_size::float), 0) FROM chain_settlement_batches WHERE status = 'confirmed'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to calculate average batch size: {}", e)))?;

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
    .map_err(|e| ApiError::Internal(format!("Failed to read whitelist queue: {}", e)))?;

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
    admin
        .require_permission(pool, BLOCKCHAIN_CONTROL_PERMISSION)
        .await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to start transaction: {}", e)))?;

    // 1. Lock and verify the same eligibility predicate used by the queue.
    let user = sqlx::query_as::<_, (uuid::Uuid, String, String, Option<String>, bool)>(
        r#"
        SELECT u.id, u.email, u.status, u.chain_wallet_address,
               EXISTS(
                   SELECT 1
                   FROM kyc_records k
                   WHERE k.user_id = u.id
                   AND k.status = 'approved'
               ) AS has_approved_kyc
        FROM users u
        WHERE u.id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    if user.2 != "active" {
        return Err(ApiError::BadRequest(
            "User is not active and cannot be force-synced".to_string(),
        ));
    }

    if !user.4 {
        return Err(ApiError::BadRequest(
            "User does not have approved KYC and cannot be force-synced".to_string(),
        ));
    }

    if user.3.is_some() && !user.3.as_deref().unwrap_or("").is_empty() {
        return Err(ApiError::BadRequest(format!(
            "User {} already has a wallet address: {}",
            user.1,
            user.3.unwrap_or_default()
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
        .execute(&mut *tx)
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
        "wallet_address": &wallet_address,
        "triggered_by": "admin_force_sync",
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("Audit log failed: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Transaction commit failed: {}", e)))?;

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

fn parse_optional_setting<T>(key: &str, value: Option<String>, default: T) -> Result<T, ApiError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match value {
        Some(raw) => raw
            .parse::<T>()
            .map_err(|e| ApiError::Internal(format!("Invalid platform setting {}: {}", key, e))),
        None => Ok(default),
    }
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
    require_blockchain_control_permission(&admin, pool).await?;
    let normalized_address = validate_contract_address(&address)?;
    let asset = fetch_clone_asset(pool, &normalized_address).await?;
    let (tx_hash, mocked) = execute_clone_control(&normalized_address, "pause")?;

    persist_clone_control_result(
        pool,
        &admin,
        asset.0,
        &normalized_address,
        "pause",
        true,
        &tx_hash,
        mocked,
    )
    .await?;

    tracing::warn!(
        "🚨 CLONE PAUSE on {} executed by admin {} — tx: {}",
        normalized_address,
        admin.user.id,
        tx_hash
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "tx_hash": tx_hash,
        "action": "paused",
        "contract_address": normalized_address,
        "mocked": mocked,
    })))
}

/// Unpause a specific EIP-1167 clone contract.
pub async fn api_admin_blockchain_unpause_clone(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;
    require_blockchain_control_permission(&admin, pool).await?;
    let normalized_address = validate_contract_address(&address)?;
    let asset = fetch_clone_asset(pool, &normalized_address).await?;
    let (tx_hash, mocked) = execute_clone_control(&normalized_address, "unpause")?;

    persist_clone_control_result(
        pool,
        &admin,
        asset.0,
        &normalized_address,
        "unpause",
        false,
        &tx_hash,
        mocked,
    )
    .await?;

    tracing::warn!(
        "🔓 CLONE UNPAUSE on {} by admin {} — tx: {}",
        normalized_address,
        admin.user.id,
        tx_hash
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "tx_hash": tx_hash,
        "action": "unpaused",
        "contract_address": normalized_address,
        "mocked": mocked,
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

// ── POST /api/admin/blockchain/primary-settle/run ─────────────
//
// Manually trigger one cycle of the primary-issuance settlement worker
// without waiting for the next scheduled run. Useful for:
//   1. Confirming a freshly-bound buyer wallet immediately after KYC.
//   2. Clearing the backlog after fixing a config issue.
//   3. Demos where T+1 delay would defeat the visualization.
//
// The worker still skips items whose `settle_eligible_at` has not yet
// passed — this endpoint short-circuits the polling sleep, not the
// reversal-window safety gate. Pass `?ignore_delay=true` to relax even
// that, for ops use only.
//
// Response: { "settled": <usize>, "message": <human readable> }
//   `settled` is the number of items moved to 'submitted' or 'confirmed'
//   in this run. Zero is normal when the queue is empty.
pub async fn api_admin_blockchain_primary_settle_run(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<PrimarySettleRunParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;
    admin
        .require_permission(pool, BLOCKCHAIN_CONTROL_PERMISSION)
        .await?;

    // Optional: relax the T+1 delay gate. Scoped to orders that ALSO
    // have an item the worker would otherwise pick up — avoids touching
    // unrelated orders. Marked in audit log so the override is traceable.
    if params.ignore_delay.unwrap_or(false) {
        sqlx::query(
            r#"UPDATE orders SET settle_eligible_at = NOW()
               WHERE status = 'completed'
                 AND settle_eligible_at IS NOT NULL
                 AND settle_eligible_at > NOW()
                 AND id IN (
                     SELECT order_id FROM order_items
                     WHERE on_chain_status = 'pending' AND on_chain_batch_id IS NULL
                 )"#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to relax delay gate: {}", e)))?;
    }

    let settled = crate::blockchain::primary_settlement::run_primary_settlement_once(pool)
        .await
        .map_err(ApiError::Internal)?;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
           VALUES ($1, 'primary_settlement.manual_run', 'system', NULL, $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "items_settled": settled,
        "ignore_delay": params.ignore_delay.unwrap_or(false),
    }))
    .execute(pool)
    .await;

    Ok(Json(serde_json::json!({
        "settled": settled,
        "message": format!("Settled {} primary-issuance order item(s) on-chain", settled),
    })))
}

#[derive(Deserialize)]
pub struct PrimarySettleRunParams {
    #[serde(default)]
    pub ignore_delay: Option<bool>,
}

// ── GET /api/admin/blockchain/primary-settle/queue ────────────
//
// Read-only inspection of the primary-settlement queue. Returns counts
// per status + the next 50 pending items with eligibility timing. Fuels
// the admin "Primary settlement queue" panel.
pub async fn api_admin_blockchain_primary_settle_queue(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db;
    admin
        .require_permission(pool, BLOCKCHAIN_READ_PERMISSION)
        .await?;

    let counts = sqlx::query_as::<_, (Option<String>, i64)>(
        r#"SELECT on_chain_status, COUNT(*)
           FROM order_items
           GROUP BY on_chain_status"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?;

    let mut counts_obj = serde_json::Map::new();
    for (status, n) in counts {
        counts_obj.insert(
            status.unwrap_or_else(|| "null".to_string()),
            serde_json::Value::from(n),
        );
    }

    let upcoming = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            i32,
            Option<chrono::DateTime<chrono::Utc>>,
            i32,
            String,
        ),
    >(
        r#"SELECT
              oi.id,
              o.order_number,
              oi.tokens_quantity,
              o.settle_eligible_at,
              oi.settle_attempt_count,
              COALESCE(a.title, '')
           FROM order_items oi
           JOIN orders o ON o.id = oi.order_id
           JOIN assets a ON a.id = oi.asset_id
           WHERE oi.on_chain_status = 'pending'
             AND oi.on_chain_batch_id IS NULL
           ORDER BY o.completed_at ASC NULLS LAST, oi.id ASC
           LIMIT 50"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?;

    let upcoming_arr: Vec<serde_json::Value> = upcoming
        .into_iter()
        .map(|(id, order_num, qty, eligible_at, attempts, asset_title)| {
            serde_json::json!({
                "order_item_id": id,
                "order_number": order_num,
                "asset_title": asset_title,
                "tokens": qty,
                "settle_eligible_at": eligible_at,
                "attempt_count": attempts,
                "ready_now": eligible_at.map(|t| t <= chrono::Utc::now()).unwrap_or(true),
            })
        })
        .collect();

    // Recent failed primary-settlement batches with full error_message —
    // surfaces simulation reverts (NotWhitelisted, MaxOwnershipExceeded,
    // treasury balance, pause) so the operator can act without DB access.
    let failures = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            i32,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        r#"SELECT id, batch_size, tx_hash, error_message, created_at
           FROM chain_settlement_batches
           WHERE batch_type = 'primary' AND status = 'failed'
           ORDER BY created_at DESC
           LIMIT 5"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?;

    let failures_arr: Vec<serde_json::Value> = failures
        .into_iter()
        .map(|(id, size, tx, err, created)| {
            serde_json::json!({
                "batch_id":      id,
                "batch_size":    size,
                "tx_hash":       tx,
                "error_message": err,
                "created_at":    created,
            })
        })
        .collect();

    // Eligibility-blocker breakdown — count completed-order items that
    // have on_chain_status NULL grouped by reason. Helps explain why
    // counts.pending is lower than expected (e.g. wallet not bound,
    // contract not deployed, KYC not yet whitelisted on-chain).
    let blockers = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r#"SELECT
              COUNT(*) FILTER (WHERE u.chain_wallet_address IS NULL OR u.chain_wallet_address = ''),
              COUNT(*) FILTER (WHERE a.chain_contract_address IS NULL OR a.chain_contract_address = ''),
              COUNT(*) FILTER (WHERE u.chain_whitelisted_at IS NULL
                                 AND u.chain_wallet_address IS NOT NULL
                                 AND u.chain_wallet_address <> ''),
              COUNT(*)
           FROM order_items oi
           JOIN orders o ON o.id = oi.order_id
           JOIN users u  ON u.id = o.user_id
           JOIN assets a ON a.id = oi.asset_id
           WHERE o.status = 'completed'
             AND oi.on_chain_status IS NULL"#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "counts": serde_json::Value::Object(counts_obj),
        "upcoming": upcoming_arr,
        "recent_failures": failures_arr,
        "blockers": {
            "no_buyer_wallet":         blockers.0,
            "no_asset_contract":       blockers.1,
            "wallet_not_whitelisted":  blockers.2,
            "total_unsettled_null":    blockers.3,
        },
    })))
}
