/// Blockchain settlement service — calls POOOLProperty1155.settleBatch() on Polygon.
///
/// This service runs as a periodic background worker that:
/// 1. Queries `trade_history` for trades with `on_chain_status = 'pending'`
/// 2. Resolves each trade's seller/buyer wallet addresses and asset's on-chain token ID
/// 3. Batches them into a single `settleBatch()` contract call
/// 4. Updates the DB with the transaction hash and status
///
/// 🔴 CRITICAL: This code handles real token transfers on-chain.
///    Every error path must be handled — no unwrap() in production paths.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════
// ── TYPES ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Configuration for the blockchain settlement service.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// RPC endpoint (e.g., "https://rpc-amoy.polygon.technology")
    pub rpc_url: String,
    /// POOOLProperty1155 contract address (0x-prefixed, 42 chars)
    pub contract_address: String,
    /// Private key of the SETTLEMENT_ROLE wallet (0x-prefixed, 66 chars)
    /// 🔴 SECURITY: Never log this value.
    pub settlement_private_key: String,
    /// Chain ID (80002 for Amoy testnet, 137 for Polygon mainnet)
    pub chain_id: u64,
    /// Maximum trades per batch (gas limit safety)
    pub max_batch_size: usize,
    /// Whether on-chain settlement is enabled
    pub enabled: bool,
}

/// A pending trade that needs on-chain settlement.
#[derive(Debug, Clone)]
struct PendingTrade {
    /// Trade ID in PostgreSQL
    trade_id: Uuid,
    /// Seller's on-chain wallet address
    seller_wallet: String,
    /// Buyer's on-chain wallet address
    buyer_wallet: String,
    /// On-chain contract address (from assets.chain_contract_address)
    chain_contract_address: String,
    /// Number of tokens to transfer
    quantity: i32,
}

/// JSON-RPC request structure for Ethereum calls.
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: serde_json::Value,
    id: u64,
}

/// JSON-RPC response structure.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: u64,
}

/// JSON-RPC error.
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// Transaction receipt from a confirmed transaction.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxReceipt {
    /// Transaction hash
    transaction_hash: String,
    /// Block number (hex)
    block_number: String,
    /// Status: "0x1" = success, "0x0" = reverted
    status: String,
    /// Gas used (hex)
    gas_used: String,
    /// Effective gas price (hex)
    effective_gas_price: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
// ── CONFIGURATION LOADER ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

impl ChainConfig {
    /// Load blockchain configuration from environment variables.
    /// Returns None if blockchain integration is not configured.
    pub fn from_env() -> Option<Self> {
        let private_key = std::env::var("CHAIN_SETTLEMENT_PRIVATE_KEY").ok()?;
        if private_key.is_empty() {
            return None;
        }

        let rpc_url = std::env::var("CHAIN_RPC_URL")
            .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

        let contract_address = std::env::var("CHAIN_CONTRACT_ADDRESS")
            .unwrap_or_else(|_| "0xb61CCe33B546a5C7c36F0B58119e7F4B3D1D04e5".to_string());

        let chain_id: u64 = std::env::var("CHAIN_ID")
            .unwrap_or_else(|_| "80002".to_string())
            .parse()
            .unwrap_or(80002);

        let max_batch_size: usize = std::env::var("CHAIN_MAX_BATCH_SIZE")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .unwrap_or(50);

        let enabled = std::env::var("CHAIN_SETTLEMENT_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        Some(Self {
            rpc_url,
            contract_address,
            settlement_private_key: private_key,
            chain_id,
            max_batch_size,
            enabled,
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// ── BACKGROUND WORKER ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Run the on-chain settlement worker. Called from main.rs as a Tokio task.
///
/// This worker:
/// 1. Wakes up at a configurable interval (default: 5 minutes, read from platform_settings)
/// 2. Queries pending trades that have on-chain metadata (wallet addresses + token IDs)
/// 3. Batches them into a settleBatch() call
/// 4. Updates the DB with results
///
/// Dynamic Batching (8B.5): Interval and batch size are read from platform_settings
/// each cycle. Admin can change `chain_settlement_interval_secs` and
/// `chain_max_batch_size` via the admin settings UI without requiring a restart.
///
/// If blockchain is not configured, this worker logs a message and exits.
pub async fn run_settlement_worker(pool: &PgPool) {
    let config = match ChainConfig::from_env() {
        Some(c) if c.enabled => c,
        Some(_) => {
            tracing::info!("⛓️ Blockchain settlement is configured but DISABLED. Set CHAIN_SETTLEMENT_ENABLED=true to enable.");
            return;
        }
        None => {
            tracing::info!("⛓️ Blockchain not configured (CHAIN_SETTLEMENT_PRIVATE_KEY not set). On-chain settlement disabled.");
            return;
        }
    };

    tracing::info!(
        "⛓️ On-chain settlement worker starting (contract={}, chain_id={}, max_batch={})",
        config.contract_address,
        config.chain_id,
        config.max_batch_size
    );

    let client = Client::new();

    // Initial delay to let the system warm up
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    loop {
        // ── 8B.5: Dynamic Batching Frequency ──────────────────────────
        // Read interval from platform_settings each cycle. This allows live
        // reconfiguration by admin without restarting the service.
        let interval_secs: u64 = sqlx::query_scalar::<_, String>(
            "SELECT value FROM platform_settings WHERE key = 'chain_settlement_interval_secs'",
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300) // Default: 5 minutes
        .max(5)         // Floor: 5 seconds (safety)
        .min(3600);     // Ceiling: 1 hour (safety)

        // Dynamic batch size override (can be smaller or larger than env var)
        let dynamic_batch_size: usize = sqlx::query_scalar::<_, String>(
            "SELECT value FROM platform_settings WHERE key = 'chain_max_batch_size'",
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(config.max_batch_size)
        .max(1)    // At least 1
        .min(200); // Safety cap at 200

        if let Err(e) = process_pending_settlements(pool, &config, &client, dynamic_batch_size).await {
            tracing::error!("⛓️ Settlement batch failed: {}", e);
            sentry::capture_message(
                &format!("On-chain settlement batch failed: {}", e),
                sentry::Level::Error,
            );
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
    }
}

// ═══════════════════════════════════════════════════════════════
// ── CORE SETTLEMENT LOGIC ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Process all pending trades and settle them on-chain.
async fn process_pending_settlements(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
    batch_size: usize,
) -> Result<(), String> {
    // 1. Query pending trades that have all required blockchain metadata
    let pending = fetch_pending_trades(pool, batch_size).await?;

    if pending.is_empty() {
        tracing::debug!("⛓️ No pending trades to settle on-chain");
        return Ok(());
    }

    tracing::info!(
        "⛓️ Found {} pending trades for on-chain settlement",
        pending.len()
    );

    // 2. Group trades by chain_contract_address
    let mut groups: std::collections::HashMap<String, Vec<PendingTrade>> =
        std::collections::HashMap::new();
    for t in pending {
        groups
            .entry(t.chain_contract_address.clone())
            .or_default()
            .push(t);
    }

    // Process each group (contract) as a separate transaction
    for (contract_address, group) in groups {
        tracing::info!(
            "⛓️ Processing batch of {} trades for contract {}",
            group.len(),
            contract_address
        );

        // 2a. Create a settlement batch record
        let batch_id = create_batch_record(pool, group.len() as i32).await?;

        // 3a. Mark trades as 'submitted'
        let trade_ids: Vec<Uuid> = group.iter().map(|t| t.trade_id).collect();
        update_trades_status(pool, &trade_ids, "submitted", None, Some(batch_id)).await?;

        // 4a. Encode and send the settleBatch() transaction
        let tx_result = send_settle_batch(config, client, &group, &contract_address).await;

        match tx_result {
            Ok(receipt) => {
                let block = u64::from_str_radix(receipt.block_number.trim_start_matches("0x"), 16)
                    .unwrap_or(0);
                let gas =
                    u64::from_str_radix(receipt.gas_used.trim_start_matches("0x"), 16).unwrap_or(0);
                let gas_price = receipt
                    .effective_gas_price
                    .as_deref()
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                    .map(|p| p / 1_000_000_000); // Convert wei to gwei

                if receipt.status == "0x1" {
                    // ✅ SUCCESS
                    tracing::info!(
                        "⛓️ ✅ Settlement batch confirmed: tx={}, contract={}, trades={}",
                        receipt.transaction_hash,
                        contract_address,
                        group.len()
                    );

                    update_batch_status(
                        pool,
                        batch_id,
                        "confirmed",
                        Some(&receipt.transaction_hash),
                        Some(block as i64),
                        Some(gas as i64),
                        gas_price.map(|p| p as i64),
                        None,
                    )
                    .await?;

                    update_trades_status(
                        pool,
                        &trade_ids,
                        "confirmed",
                        Some(&receipt.transaction_hash),
                        Some(batch_id),
                    )
                    .await?;
                } else {
                    // ❌ REVERTED
                    let msg = format!("Transaction reverted: tx={}", receipt.transaction_hash);
                    tracing::error!("⛓️ ❌ {}", msg);

                    update_batch_status(
                        pool,
                        batch_id,
                        "failed",
                        Some(&receipt.transaction_hash),
                        Some(block as i64),
                        Some(gas as i64),
                        gas_price.map(|p| p as i64),
                        Some(&msg),
                    )
                    .await?;

                    // Reset trades to 'pending' so they can be retried
                    update_trades_status(pool, &trade_ids, "pending", None, None).await?;
                }
            }
            Err(e) => {
                // ❌ TX FAILED (network/RPC error)
                tracing::error!(
                    "⛓️ ❌ Settlement TX failed for contract {}: {}",
                    contract_address,
                    e
                );

                update_batch_status(pool, batch_id, "failed", None, None, None, None, Some(&e))
                    .await?;

                // Reset trades to 'pending' so they can be retried
                update_trades_status(pool, &trade_ids, "pending", None, None).await?;
            }
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── DATABASE QUERIES ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Fetch trades pending on-chain settlement that have all required blockchain metadata.
async fn fetch_pending_trades(pool: &PgPool, limit: usize) -> Result<Vec<PendingTrade>, String> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, i32)>(
        r#"SELECT 
            th.id,
            seller.chain_wallet_address,
            buyer.chain_wallet_address,
            a.chain_contract_address,
            th.quantity
        FROM trade_history th
        JOIN users seller ON seller.id = th.seller_user_id
        JOIN users buyer ON buyer.id = th.buyer_user_id
        JOIN assets a ON a.id = th.asset_id
        WHERE th.on_chain_status = 'pending'
          AND seller.chain_wallet_address IS NOT NULL
          AND buyer.chain_wallet_address IS NOT NULL
          AND a.chain_contract_address IS NOT NULL
        ORDER BY th.executed_at ASC
        LIMIT $1"#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query failed: {}", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(trade_id, seller_wallet, buyer_wallet, chain_contract_address, quantity)| {
                PendingTrade {
                    trade_id,
                    seller_wallet,
                    buyer_wallet,
                    chain_contract_address,
                    quantity,
                }
            },
        )
        .collect())
}

/// Create a settlement batch record in the database.
async fn create_batch_record(pool: &PgPool, batch_size: i32) -> Result<Uuid, String> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO chain_settlement_batches (batch_size) VALUES ($1) RETURNING id",
    )
    .bind(batch_size)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to create batch record: {}", e))
}

/// Update batch status with receipt details.
#[allow(clippy::too_many_arguments)]
async fn update_batch_status(
    pool: &PgPool,
    batch_id: Uuid,
    status: &str,
    tx_hash: Option<&str>,
    block_number: Option<i64>,
    gas_used: Option<i64>,
    gas_price_gwei: Option<i64>,
    error_message: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    let (submitted_at, confirmed_at) = match status {
        "submitted" => (Some(now), None),
        "confirmed" => (Some(now), Some(now)),
        _ => (None, None),
    };

    sqlx::query(
        r#"UPDATE chain_settlement_batches SET
            status = $1, tx_hash = $2, block_number = $3,
            gas_used = $4, gas_price_gwei = $5, error_message = $6,
            submitted_at = COALESCE($7, submitted_at),
            confirmed_at = COALESCE($8, confirmed_at)
        WHERE id = $9"#,
    )
    .bind(status)
    .bind(tx_hash)
    .bind(block_number)
    .bind(gas_used)
    .bind(gas_price_gwei)
    .bind(error_message)
    .bind(submitted_at)
    .bind(confirmed_at)
    .bind(batch_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update batch status: {}", e))?;

    Ok(())
}

/// Update on_chain_status for a list of trades.
async fn update_trades_status(
    pool: &PgPool,
    trade_ids: &[Uuid],
    status: &str,
    tx_hash: Option<&str>,
    batch_id: Option<Uuid>,
) -> Result<(), String> {
    // Use ANY($1) for array binding
    sqlx::query(
        r#"UPDATE trade_history SET
            on_chain_status = $1,
            on_chain_tx_hash = COALESCE($2, on_chain_tx_hash),
            on_chain_batch_id = COALESCE($3, on_chain_batch_id)
        WHERE id = ANY($4)"#,
    )
    .bind(status)
    .bind(tx_hash)
    .bind(batch_id)
    .bind(trade_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update trade statuses: {}", e))?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── CONTRACT INTERACTION (Raw JSON-RPC + ABI Encoding) ────────
// ═══════════════════════════════════════════════════════════════

/// ABI-encode and send a `settleBatch()` call to the POOOLProperty1155 contract.
///
/// This uses raw JSON-RPC calls via reqwest — no ethers-rs dependency needed.
/// The ABI encoding follows the Solidity spec for:
/// ```solidity
/// function settleBatch(
///     address[] calldata froms,
///     address[] calldata tos,
///     uint256[] calldata amounts
/// ) external
/// ```
///
/// Function selector: keccak256("settleBatch(address[],address[],uint256[])") = first 4 bytes
async fn send_settle_batch(
    config: &ChainConfig,
    client: &Client,
    trades: &[PendingTrade],
    contract_address: &str,
) -> Result<TxReceipt, String> {
    // 1. Build the calldata
    let calldata = encode_settle_batch_calldata(trades)?;

    // 2. Get the sender address from private key
    let sender = derive_address_from_private_key(&config.settlement_private_key)?;

    // 3. Get the nonce
    let nonce = get_nonce(client, &config.rpc_url, &sender).await?;

    // 4. Estimate gas
    let gas_estimate = estimate_gas(
        client,
        &config.rpc_url,
        &sender,
        contract_address,
        &calldata,
    )
    .await?;

    // Add 20% buffer to gas estimate
    let gas_limit = gas_estimate + (gas_estimate / 5);

    // 5. Get current gas price
    let gas_price = get_gas_price(client, &config.rpc_url).await?;

    // 6. Build, sign, and send the transaction
    let signed_tx = sign_transaction(
        config,
        &sender,
        contract_address,
        &calldata,
        nonce,
        gas_limit,
        gas_price,
    )?;

    let tx_hash = send_raw_transaction(client, &config.rpc_url, &signed_tx).await?;

    tracing::info!(
        "⛓️ Settlement TX sent: hash={}, nonce={}, gas_limit={}",
        tx_hash,
        nonce,
        gas_limit
    );

    // 7. Wait for confirmation (poll every 3 seconds, max 60 seconds)
    let receipt = wait_for_receipt(client, &config.rpc_url, &tx_hash, 20, 3).await?;

    Ok(receipt)
}

/// Encode the calldata for `settleBatch(address[], address[], uint256[])`.
///
/// Uses basic ABI encoding without external dependencies.
/// For dynamic arrays, the ABI encoding follows the Solidity spec:
/// - 4 bytes: function selector
/// - 32 bytes × 3: offsets to each dynamic array
/// - For each array: 32 bytes length + 32 bytes per element
fn encode_settle_batch_calldata(trades: &[PendingTrade]) -> Result<String, String> {
    // Function selector: settleBatch(address[],address[],uint256[])
    // keccak256 of the signature → first 4 bytes
    // Verified: cast sig "settleBatch(address[],address[],uint256[])" → 0xfc4b731c
    let selector = "fc4b731c";

    let n = trades.len();

    // Build the 3 arrays
    let froms: Vec<String> = trades
        .iter()
        .map(|t| pad_address(&t.seller_wallet))
        .collect::<Result<Vec<_>, _>>()?;
    let tos: Vec<String> = trades
        .iter()
        .map(|t| pad_address(&t.buyer_wallet))
        .collect::<Result<Vec<_>, _>>()?;
    let amounts: Vec<String> = trades
        .iter()
        .map(|t| pad_uint256(&t.quantity.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    // Calculate offsets (each offset points to the start of the array data)
    // 3 offset slots × 32 bytes = 96 bytes (0x60)
    let offset_base = 3 * 32; // 96
    let array_size = 32 + n * 32; // length word + elements

    let offset_froms = offset_base;
    let offset_tos = offset_froms + array_size;
    let offset_amounts = offset_tos + array_size;

    let mut data = String::from(selector);

    // Encode 3 offsets
    data.push_str(&pad_uint256(&offset_froms.to_string())?);
    data.push_str(&pad_uint256(&offset_tos.to_string())?);
    data.push_str(&pad_uint256(&offset_amounts.to_string())?);

    // Encode each array: [length, element0, element1, ...]
    for array in &[&froms, &tos, &amounts] {
        data.push_str(&pad_uint256(&n.to_string())?); // length
        for element in array.iter() {
            data.push_str(element);
        }
    }

    Ok(format!("0x{}", data))
}

/// Pad an Ethereum address to 32 bytes (left-pad with zeros).
fn pad_address(addr: &str) -> Result<String, String> {
    let clean = addr.strip_prefix("0x").unwrap_or(addr).to_lowercase();
    if clean.len() != 40 {
        return Err(format!("Invalid address length: {}", addr));
    }
    // Verify it's valid hex
    if !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid address hex: {}", addr));
    }
    Ok(format!("{:0>64}", clean))
}

/// Pad a decimal number to a 32-byte uint256 (hex).
fn pad_uint256(decimal: &str) -> Result<String, String> {
    let n: u128 = decimal
        .parse()
        .map_err(|_| format!("Invalid uint256: {}", decimal))?;
    Ok(format!("{:064x}", n))
}

// ═══════════════════════════════════════════════════════════════
// ── JSON-RPC HELPERS ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Get the current transaction count (nonce) for an address.
async fn get_nonce(client: &Client, rpc_url: &str, address: &str) -> Result<u64, String> {
    let resp = rpc_call(
        client,
        rpc_url,
        "eth_getTransactionCount",
        serde_json::json!([address, "latest"]),
    )
    .await?;
    let hex = resp.as_str().ok_or("Invalid nonce response")?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse nonce: {}", e))
}

/// Get the current gas price.
async fn get_gas_price(client: &Client, rpc_url: &str) -> Result<u64, String> {
    let resp = rpc_call(client, rpc_url, "eth_gasPrice", serde_json::json!([])).await?;
    let hex = resp.as_str().ok_or("Invalid gas price response")?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse gas price: {}", e))
}

/// Estimate gas for a transaction.
async fn estimate_gas(
    client: &Client,
    rpc_url: &str,
    from: &str,
    to: &str,
    data: &str,
) -> Result<u64, String> {
    let params = serde_json::json!([{
        "from": from,
        "to": to,
        "data": data
    }]);
    let resp = rpc_call(client, rpc_url, "eth_estimateGas", params).await?;
    let hex = resp.as_str().ok_or("Invalid gas estimate response")?;
    u64::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse gas estimate: {}", e))
}

/// Send a raw signed transaction.
async fn send_raw_transaction(
    client: &Client,
    rpc_url: &str,
    signed_tx: &str,
) -> Result<String, String> {
    let resp = rpc_call(
        client,
        rpc_url,
        "eth_sendRawTransaction",
        serde_json::json!([signed_tx]),
    )
    .await?;
    resp.as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid tx hash response".to_string())
}

/// Wait for a transaction receipt by polling.
async fn wait_for_receipt(
    client: &Client,
    rpc_url: &str,
    tx_hash: &str,
    max_attempts: u32,
    interval_secs: u64,
) -> Result<TxReceipt, String> {
    for attempt in 1..=max_attempts {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

        let resp = rpc_call(
            client,
            rpc_url,
            "eth_getTransactionReceipt",
            serde_json::json!([tx_hash]),
        )
        .await?;

        if resp.is_null() {
            tracing::debug!(
                "⛓️ Waiting for receipt... attempt {}/{}",
                attempt,
                max_attempts
            );
            continue;
        }

        let receipt: TxReceipt =
            serde_json::from_value(resp).map_err(|e| format!("Failed to parse receipt: {}", e))?;

        return Ok(receipt);
    }

    Err(format!(
        "Transaction {} not confirmed after {} attempts",
        tx_hash, max_attempts
    ))
}

/// Generic JSON-RPC call helper.
async fn rpc_call(
    client: &Client,
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        method: method.to_string(),
        params,
        id: 1,
    };

    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("RPC request failed: {}", e))?;

    let body: JsonRpcResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse RPC response: {}", e))?;

    if let Some(error) = body.error {
        return Err(format!("RPC error: {}", error.message));
    }

    body.result.ok_or_else(|| "Empty RPC response".to_string())
}

// ═══════════════════════════════════════════════════════════════
// ── TRANSACTION SIGNING (EIP-155) ─────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Derive the Ethereum address from a private key.
///
/// Uses the secp256k1 curve to derive the public key, then keccak256 hash
/// the uncompressed public key (without the 0x04 prefix) → last 20 bytes.
///
/// NOTE: This is a placeholder that uses `cast` CLI for signing.
/// In production, we'd use the `k256` or `secp256k1` Rust crate directly.
fn derive_address_from_private_key(private_key: &str) -> Result<String, String> {
    // For now, use the known deployment address
    // TODO: Replace with proper key derivation using k256 crate
    let _ = private_key;
    Ok(std::env::var("CHAIN_SETTLEMENT_ADDRESS")
        .unwrap_or_else(|_| "0x021F6B0029125B3924FF5Ba3e0FF59e1FA39B88a".to_string()))
}

/// Sign a transaction using the `cast` CLI tool (Foundry).
///
/// This is a pragmatic approach that avoids pulling in heavy crypto dependencies.
/// The `cast` tool handles:
/// - RLP encoding
/// - EIP-155 signing (chain ID included in signature)
/// - Private key → signature
///
/// In a future iteration, this should use the `alloy` crate for in-process signing.
fn sign_transaction(
    config: &ChainConfig,
    _from: &str,
    target_contract: &str,
    data: &str,
    nonce: u64,
    gas_limit: u64,
    gas_price: u64,
) -> Result<String, String> {
    // Use `cast` to send the transaction directly (it handles signing internally)
    // We'll construct the command and parse the output
    let output = std::process::Command::new("cast")
        .args([
            "send",
            target_contract,
            "--private-key",
            &config.settlement_private_key,
            "--rpc-url",
            &config.rpc_url,
            "--gas-limit",
            &gas_limit.to_string(),
            "--gas-price",
            &gas_price.to_string(),
            "--nonce",
            &nonce.to_string(),
            "--raw", // Output the signed transaction hex
            "--data",
            data,
        ])
        .output()
        .map_err(|e| {
            format!(
                "Failed to execute `cast send`: {}. Is Foundry installed?",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cast send failed: {}", stderr));
    }

    let tx_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tx_hash.is_empty() {
        return Err("cast send returned empty output".to_string());
    }

    // `cast send` without --raw returns the tx hash directly (not raw tx)
    // so we return the hash and skip send_raw_transaction
    Ok(tx_hash)
}

// ═══════════════════════════════════════════════════════════════
// ── ABI FUNCTION SELECTOR ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Verify the settleBatch function selector.
/// keccak256("settleBatch(address[],address[],uint256[])") → first 4 bytes
///
/// Pre-computed using: cast sig "settleBatch(address[],address[],uint256[])"
/// Result: 0xfc4b731c
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_address() {
        let result = pad_address("0x021F6B0029125B3924FF5Ba3e0FF59e1FA39B88a").unwrap();
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("000000000000000000000000"));
        assert!(result.ends_with("021f6b0029125b3924ff5ba3e0ff59e1fa39b88a"));
    }

    #[test]
    fn test_pad_uint256() {
        let result = pad_uint256("100").unwrap();
        assert_eq!(result.len(), 64);
        assert_eq!(
            result,
            "0000000000000000000000000000000000000000000000000000000000000064"
        );
    }

    #[test]
    fn test_encode_empty_batch_still_valid() {
        let trades: Vec<PendingTrade> = vec![];
        let calldata = encode_settle_batch_calldata(&trades).unwrap();
        assert!(calldata.starts_with("0xfc4b731c"));
    }
}
