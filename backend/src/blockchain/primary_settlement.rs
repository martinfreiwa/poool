//! Primary-issuance on-chain settlement.
//!
//! When a user completes a primary purchase (bank wire / card → admin
//! approves), the buyer is credited in the off-chain ledger immediately
//! (`investments.tokens_owned += qty`). This worker is the bridge from
//! that off-chain credit to the on-chain ERC-1155 balance: it batches
//! eligible `order_items` rows and calls
//! `POOOLAssetToken.settleBatch(treasury → buyer transfers)` once per
//! contract per cycle.
//!
//! ## Why a separate worker from `service::run_settlement_worker`
//!
//! - **Source table** differs (`order_items` vs `trade_history`).
//! - **`from` address** is always the treasury (= settlement signer's
//!   own address) for primary issuance, never another user.
//! - **T+1 delay**: bank wires can reverse for ~3 business days. We
//!   wait `chain_primary_settle_delay_secs` past `orders.completed_at`
//!   (and require `orders.settle_eligible_at <= NOW()`) before lifting
//!   to chain, so reversed wires never produce orphan on-chain
//!   transfers.
//!
//! Otherwise the lifecycle (reserve → simulate → broadcast → confirm)
//! is identical to P2P, and the helpers in `service` are reused via
//! `pub(crate)` exposure.

use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use super::service::{
    create_batch_record, encode_settle_batch_calldata_raw, estimate_gas, get_gas_price,
    pad_address, release_nonce, reserve_nonce, rpc_call, sign_and_send_in_process,
    update_batch_status, wait_for_receipt, ChainConfig,
};

/// One pending primary-issuance order item ready for chain settlement.
#[derive(Debug, Clone)]
struct PendingPrimaryItem {
    /// `order_items.id` — the unit we lock + flip status on.
    order_item_id: Uuid,
    /// Buyer's verified on-chain wallet (SIWE-bound + KYC-whitelisted).
    buyer_wallet: String,
    /// `assets.chain_contract_address` — the ERC-1155 clone.
    chain_contract_address: String,
    /// Token quantity to transfer treasury → buyer.
    quantity: i32,
}

// ═══════════════════════════════════════════════════════════════
// ── BACKGROUND WORKER ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Long-running primary-issuance settlement loop. Spawned from `main.rs`
/// under the `BlockchainPrimarySettlement` leader lock.
///
/// Each cycle:
///   1. Read `chain_primary_settlement_*` settings from `platform_settings`.
///   2. Skip the cycle if disabled.
///   3. Run one pass of `process_pending_primary`.
///   4. Sleep for `chain_primary_settlement_interval_secs`.
///
/// Honors the same "no signer configured = no-op" contract as the P2P
/// worker, so it cleanly idles in dev without `CHAIN_*` env set.
pub async fn run_primary_settlement_worker(pool: &PgPool) {
    let config = match ChainConfig::from_env().await {
        Some(c) if c.enabled => c,
        Some(_) => {
            tracing::info!(
                "⛓️ Primary settlement: chain settlement DISABLED (CHAIN_SETTLEMENT_ENABLED=false). Worker idle."
            );
            return;
        }
        None => {
            tracing::info!("⛓️ Primary settlement: no signer configured. Worker idle.");
            return;
        }
    };

    let treasury_address = resolve_treasury_address(&config);
    tracing::info!(
        "⛓️ Primary issuance settlement worker starting (treasury={}, signer={}, chain_id={})",
        treasury_address,
        super::signing::format_address(&config.signer.address()),
        config.chain_id
    );

    let client = Client::new();

    // Initial warm-up before first cycle, matches P2P worker behavior.
    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

    loop {
        let enabled = read_setting_bool(pool, "chain_primary_settlement_enabled", true).await;
        let interval_secs =
            read_setting_u64(pool, "chain_primary_settlement_interval_secs", 300, 5, 3600).await;
        let batch_size = read_setting_usize(pool, "chain_primary_max_batch_size", 50, 1, 200).await;

        if !enabled {
            tracing::debug!(
                "⛓️ Primary settlement disabled via platform_settings — skipping cycle"
            );
        } else if let Err(e) =
            process_pending_primary(pool, &config, &client, &treasury_address, batch_size).await
        {
            tracing::error!("⛓️ Primary settlement cycle failed: {}", e);
            sentry::capture_message(
                &format!("Primary on-chain settlement failed: {}", e),
                sentry::Level::Error,
            );
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
    }
}

/// One-shot run of `process_pending_primary`. Used by the admin
/// "Run primary settlement now" button to skip the polling delay
/// without restarting the worker.
///
/// Returns the number of order items successfully marked `confirmed`
/// (or `submitted`, if the receipt poll timed out — the reconciler
/// will resolve the rest). On any setup error returns Err with a
/// human-readable message suitable for the admin UI toast.
pub async fn run_primary_settlement_once(pool: &PgPool) -> Result<usize, String> {
    let config = ChainConfig::from_env()
        .await
        .ok_or_else(|| "Chain signer not configured".to_string())?;
    if !config.enabled {
        return Err("On-chain settlement is disabled (CHAIN_SETTLEMENT_ENABLED=false)".into());
    }
    let treasury_address = resolve_treasury_address(&config);
    let batch_size = read_setting_usize(pool, "chain_primary_max_batch_size", 50, 1, 200).await;
    let client = Client::new();

    let before = count_settled(pool).await.unwrap_or(0);
    process_pending_primary(pool, &config, &client, &treasury_address, batch_size).await?;
    let after = count_settled(pool).await.unwrap_or(before);
    Ok(after.saturating_sub(before))
}

async fn count_settled(pool: &PgPool) -> Option<usize> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM order_items WHERE on_chain_status IN ('submitted', 'confirmed')",
    )
    .fetch_one(pool)
    .await
    .ok()
    .map(|n| n as usize)
}

// ═══════════════════════════════════════════════════════════════
// ── CORE SETTLEMENT LOGIC ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// One full pass: query → group by contract → simulate → broadcast →
/// confirm. Mirrors `service::process_pending_settlements`, but `from`
/// is the treasury for every transfer.
async fn process_pending_primary(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
    treasury_address: &str,
    batch_size: usize,
) -> Result<(), String> {
    let pending = fetch_pending_primary_items(pool, batch_size).await?;
    if pending.is_empty() {
        tracing::debug!("⛓️ No primary-issuance items pending settlement");
        return Ok(());
    }

    tracing::info!(
        "⛓️ Found {} primary order items pending on-chain settlement",
        pending.len()
    );

    // Group by contract — one tx per ERC-1155 clone per cycle.
    let mut groups: std::collections::HashMap<String, Vec<PendingPrimaryItem>> =
        std::collections::HashMap::new();
    for item in pending {
        groups
            .entry(item.chain_contract_address.clone())
            .or_default()
            .push(item);
    }

    for (contract_address, group) in groups {
        let item_ids: Vec<Uuid> = group.iter().map(|i| i.order_item_id).collect();
        let batch_id = create_batch_record(pool, group.len() as i32, "primary").await?;

        // Phase 1 — Reserve.
        match reserve_items(pool, &item_ids, batch_id).await {
            Ok(true) => {}
            Ok(false) => {
                tracing::warn!(
                    "⛓️ Reservation race lost for primary contract {} — another worker has these items",
                    contract_address
                );
                update_batch_status(
                    pool,
                    batch_id,
                    "failed",
                    None,
                    None,
                    None,
                    None,
                    Some("reservation race lost"),
                )
                .await?;
                continue;
            }
            Err(e) => {
                update_batch_status(pool, batch_id, "failed", None, None, None, None, Some(&e))
                    .await?;
                continue;
            }
        }

        // Validate treasury can cover every transfer in the group before
        // we burn a nonce. Cheap insurance vs. a revert mid-batch.
        if let Err(e) = check_treasury_balance(
            client,
            &config.rpc_url,
            &contract_address,
            treasury_address,
            &group,
        )
        .await
        {
            tracing::error!(
                "⛓️ Treasury balance check failed for {}: {} — leaving items pending for next cycle",
                contract_address,
                e
            );
            update_batch_status(pool, batch_id, "failed", None, None, None, None, Some(&e)).await?;
            reset_items_to_pending(pool, &item_ids).await?;
            continue;
        }

        // Phase 2 — Simulate via eth_call. Catches predictable reverts
        // (KYC missing, 80% ownership cap, supply paused) without nonce burn.
        let calldata = match encode_calldata(treasury_address, &group) {
            Ok(c) => c,
            Err(e) => {
                update_batch_status(pool, batch_id, "failed", None, None, None, None, Some(&e))
                    .await?;
                reset_items_to_pending(pool, &item_ids).await?;
                continue;
            }
        };

        if let Err(e) = simulate(
            client,
            &config.rpc_url,
            treasury_address,
            &contract_address,
            &calldata,
        )
        .await
        {
            tracing::warn!(
                "⛓️ Pre-broadcast simulation failed for primary contract {}: {} — releasing reservation",
                contract_address,
                e
            );
            update_batch_status(pool, batch_id, "failed", None, None, None, None, Some(&e)).await?;
            increment_attempt_counts(pool, &item_ids).await?;
            reset_items_to_pending(pool, &item_ids).await?;
            continue;
        }

        // Phase 3 — Broadcast.
        let tx_hash = match broadcast(
            pool,
            config,
            client,
            treasury_address,
            &contract_address,
            &calldata,
        )
        .await
        {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(
                    "⛓️ ❌ Primary broadcast failed for contract {}: {}",
                    contract_address,
                    e
                );
                update_batch_status(pool, batch_id, "failed", None, None, None, None, Some(&e))
                    .await?;
                increment_attempt_counts(pool, &item_ids).await?;
                reset_items_to_pending(pool, &item_ids).await?;
                continue;
            }
        };

        update_batch_status(
            pool,
            batch_id,
            "submitted",
            Some(&tx_hash),
            None,
            None,
            None,
            None,
        )
        .await?;
        update_items_status(pool, &item_ids, "submitted", Some(&tx_hash), Some(batch_id)).await?;

        // Phase 4 — Best-effort receipt poll.
        match wait_for_receipt(client, &config.rpc_url, &tx_hash, 20, 3).await {
            Ok(receipt) => {
                let block = u64::from_str_radix(receipt.block_number.trim_start_matches("0x"), 16)
                    .unwrap_or(0);
                let gas =
                    u64::from_str_radix(receipt.gas_used.trim_start_matches("0x"), 16).unwrap_or(0);
                let gas_price = receipt
                    .effective_gas_price
                    .as_deref()
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                    .map(|p| p / 1_000_000_000);

                if receipt.status == "0x1" {
                    tracing::info!(
                        "⛓️ ✅ Primary batch confirmed: tx={}, contract={}, items={}",
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
                    update_items_status(
                        pool,
                        &item_ids,
                        "confirmed",
                        Some(&receipt.transaction_hash),
                        Some(batch_id),
                    )
                    .await?;
                } else {
                    let msg = format!(
                        "Primary settlement reverted: tx={}",
                        receipt.transaction_hash
                    );
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
                    increment_attempt_counts(pool, &item_ids).await?;
                    reset_items_to_pending(pool, &item_ids).await?;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "⛓️ Primary receipt poll timed out for tx={}: {} — handing off to reconciler",
                    tx_hash,
                    e
                );
            }
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── DATABASE QUERIES ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Pull eligible primary items: order completed, T+1 delay elapsed,
/// buyer wallet bound + asset on-chain. `FOR UPDATE SKIP LOCKED` so
/// concurrent workers (or admin manual run) cannot pick the same row.
async fn fetch_pending_primary_items(
    pool: &PgPool,
    limit: usize,
) -> Result<Vec<PendingPrimaryItem>, String> {
    // Net-of-P2P settlement quantity:
    //   tokens_to_send = LEAST(oi.tokens_quantity, investments.tokens_owned)
    // Reason: between the primary purchase being recorded and chain
    // settlement firing, the buyer may have sold some tokens via the
    // P2P marketplace. Their `investments.tokens_owned` already nets
    // those sells. Sending the full original primary qty (1,882 in the
    // launch case) would push the recipient over the contract's 80%
    // ownership cap and revert the entire batch.
    //
    // The `tokens_to_send <= 0` rows are filtered out (HAVING) so the
    // worker doesn't broadcast a no-op transfer.
    let rows = sqlx::query_as::<_, (Uuid, String, String, i32)>(
        r#"SELECT
            oi.id,
            buyer.chain_wallet_address,
            a.chain_contract_address,
            LEAST(oi.tokens_quantity, COALESCE(inv.tokens_owned, oi.tokens_quantity))::int4
                AS tokens_to_send
        FROM order_items oi
        JOIN orders o ON o.id = oi.order_id
        JOIN users buyer ON buyer.id = o.user_id
        JOIN assets a ON a.id = oi.asset_id
        LEFT JOIN investments inv
               ON inv.user_id = o.user_id AND inv.asset_id = oi.asset_id
        WHERE oi.on_chain_status = 'pending'
          AND oi.on_chain_batch_id IS NULL
          AND o.status = 'completed'
          AND (o.settle_eligible_at IS NULL OR o.settle_eligible_at <= NOW())
          AND buyer.chain_wallet_address IS NOT NULL
          AND buyer.chain_wallet_address <> ''
          AND a.chain_contract_address IS NOT NULL
          AND a.chain_contract_address <> ''
          AND LEAST(oi.tokens_quantity, COALESCE(inv.tokens_owned, oi.tokens_quantity)) > 0
        ORDER BY o.completed_at ASC NULLS LAST, oi.id ASC
        LIMIT $1
        FOR UPDATE OF oi SKIP LOCKED"#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query failed: {}", e))?;

    Ok(rows
        .into_iter()
        .map(
            |(order_item_id, buyer_wallet, chain_contract_address, quantity)| PendingPrimaryItem {
                order_item_id,
                buyer_wallet,
                chain_contract_address,
                quantity,
            },
        )
        .collect())
}

/// Atomically claim a slice of pending items into a batch. Returns
/// false if any row was already claimed by another worker.
async fn reserve_items(pool: &PgPool, item_ids: &[Uuid], batch_id: Uuid) -> Result<bool, String> {
    let affected = sqlx::query(
        r#"UPDATE order_items SET on_chain_batch_id = $1
           WHERE id = ANY($2)
             AND on_chain_status = 'pending'
             AND on_chain_batch_id IS NULL"#,
    )
    .bind(batch_id)
    .bind(item_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to reserve items: {}", e))?
    .rows_affected();
    Ok(affected as usize == item_ids.len())
}

async fn reset_items_to_pending(pool: &PgPool, item_ids: &[Uuid]) -> Result<(), String> {
    sqlx::query(
        r#"UPDATE order_items SET
            on_chain_status = 'pending',
            on_chain_tx_hash = NULL,
            on_chain_batch_id = NULL
        WHERE id = ANY($1)"#,
    )
    .bind(item_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to reset items: {}", e))?;
    Ok(())
}

async fn increment_attempt_counts(pool: &PgPool, item_ids: &[Uuid]) -> Result<(), String> {
    sqlx::query(
        "UPDATE order_items SET settle_attempt_count = settle_attempt_count + 1 WHERE id = ANY($1)",
    )
    .bind(item_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to bump attempt count: {}", e))?;
    Ok(())
}

async fn update_items_status(
    pool: &PgPool,
    item_ids: &[Uuid],
    status: &str,
    tx_hash: Option<&str>,
    batch_id: Option<Uuid>,
) -> Result<(), String> {
    sqlx::query(
        r#"UPDATE order_items SET
            on_chain_status = $1,
            on_chain_tx_hash = COALESCE($2, on_chain_tx_hash),
            on_chain_batch_id = COALESCE($3, on_chain_batch_id)
        WHERE id = ANY($4)"#,
    )
    .bind(status)
    .bind(tx_hash)
    .bind(batch_id)
    .bind(item_ids)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update item statuses: {}", e))?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── ON-CHAIN HELPERS ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

fn encode_calldata(treasury: &str, group: &[PendingPrimaryItem]) -> Result<String, String> {
    let triples: Vec<(&str, &str, i32)> = group
        .iter()
        .map(|i| (treasury, i.buyer_wallet.as_str(), i.quantity))
        .collect();
    encode_settle_batch_calldata_raw(&triples)
}

async fn simulate(
    client: &Client,
    rpc_url: &str,
    treasury: &str,
    contract_address: &str,
    calldata: &str,
) -> Result<(), String> {
    let params = serde_json::json!([
        { "from": treasury, "to": contract_address, "data": calldata },
        "latest"
    ]);
    rpc_call(client, rpc_url, "eth_call", params).await?;
    Ok(())
}

async fn broadcast(
    pool: &PgPool,
    config: &ChainConfig,
    client: &Client,
    treasury: &str,
    contract_address: &str,
    calldata: &str,
) -> Result<String, String> {
    let gas_estimate = estimate_gas(
        client,
        &config.rpc_url,
        treasury,
        contract_address,
        calldata,
    )
    .await?;
    let gas_limit = gas_estimate + (gas_estimate / 5); // +20% headroom
    let gas_price = get_gas_price(client, &config.rpc_url).await?;

    let nonce = reserve_nonce(pool, client, &config.rpc_url, treasury).await?;

    match sign_and_send_in_process(
        config,
        client,
        contract_address,
        calldata,
        nonce,
        gas_limit,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            tracing::info!(
                "⛓️ Primary settlement TX broadcast: hash={}, nonce={}, gas_limit={}",
                tx_hash,
                nonce,
                gas_limit
            );
            Ok(tx_hash)
        }
        Err(e) => {
            if let Err(rb) = release_nonce(pool, treasury, nonce).await {
                tracing::error!("⛓️ Failed to roll back nonce {}: {}", nonce, rb);
            }
            Err(e)
        }
    }
}

/// Resolve the on-chain address that holds the asset supply.
///
/// **Key-derived only.** The signer's address IS the treasury — they're
/// the same wallet by construction: `mintTo` at `deployAsset()` is now
/// also locked to the signer's address (see `admin::blockchain::resolve
/// _settlement_address`). This invariant guarantees the signer can
/// always move tokens it just minted.
///
/// Reading env `CHAIN_SETTLEMENT_ADDRESS` here was a regression — it
/// allowed env to silently override key-derivation, leading to mintTo
/// addresses whose key was never persisted. We don't repeat that.
fn resolve_treasury_address(config: &ChainConfig) -> String {
    super::signing::format_address(&config.signer.address())
}

/// Verify treasury holds enough tokens to cover every transfer in the
/// group. Calls the standard ERC-1155 `balanceOf(account, id)` view.
/// Catches the most common cause of revert (attempted re-tokenization
/// of an already-distributed asset) cheaply, before nonce burn.
async fn check_treasury_balance(
    client: &Client,
    rpc_url: &str,
    contract_address: &str,
    treasury: &str,
    group: &[PendingPrimaryItem],
) -> Result<(), String> {
    // ASSET_TOKEN_ID is fixed at 1 in POOOLAssetToken. The contract
    // does not expose any other id, so we don't bother encoding it
    // dynamically here.
    let token_id = 1u64;

    // selector(balanceOf(address,uint256)) = 0x00fdd58e
    let selector = "00fdd58e";
    let owner = pad_address(treasury)?;
    let id_padded = format!("{:064x}", token_id);
    let calldata = format!("0x{}{}{}", selector, owner, id_padded);

    let resp = rpc_call(
        client,
        rpc_url,
        "eth_call",
        serde_json::json!([
            { "to": contract_address, "data": calldata },
            "latest"
        ]),
    )
    .await?;
    let hex = resp
        .as_str()
        .ok_or_else(|| "balanceOf: bad RPC reply".to_string())?;
    let trimmed = hex.trim_start_matches("0x");
    // Empty (0x) = call hit an EOA / non-contract / wrong selector. Surface
    // an actionable error rather than the cryptic Rust parse failure.
    if trimmed.is_empty() {
        return Err(format!(
            "balanceOf returned empty (0x) for treasury {} on contract {} — \
             treasury_address mismatch (CHAIN_SETTLEMENT_ADDRESS env vs. mintTo \
             at deploy?) or contract does not implement ERC-1155 at that address",
            treasury, contract_address
        ));
    }
    let balance = u128::from_str_radix(trimmed, 16)
        .map_err(|e| format!("balanceOf parse failed (response={}): {}", hex, e))?;

    let needed: u128 = group.iter().map(|i| i.quantity as u128).sum();
    if balance < needed {
        return Err(format!(
            "treasury balance {} < required {} on contract {}",
            balance, needed, contract_address
        ));
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── PLATFORM-SETTING READERS ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════
// ── ELIGIBILITY HOOKS (called from non-chain code paths) ──────
// ═══════════════════════════════════════════════════════════════

/// Mark an order's items as eligible for primary on-chain settlement.
///
/// Call this immediately after flipping `orders.status` to `'completed'`
/// (single-order admin approval, escrow release, etc.). Sets
/// `orders.settle_eligible_at = NOW() + chain_primary_settle_delay_secs`
/// and lifts each `order_items.on_chain_status` from NULL → `'pending'`,
/// **only** for items where the buyer has a bound wallet AND the asset
/// has a deployed contract. Items missing either prerequisite stay NULL
/// and are picked up later by `mark_user_eligible_after_wallet_bind` /
/// `mark_asset_eligible_after_tokenization`.
///
/// Idempotent: re-running on an already-flagged order is a no-op.
/// Takes `&mut PgConnection` so callers can run inside their own
/// transaction (`&mut **tx`) — keeping completion + eligibility flip
/// atomic.
pub async fn mark_order_eligible(
    conn: &mut sqlx::PgConnection,
    order_id: Uuid,
    delay_secs: i64,
) -> Result<(), String> {
    sqlx::query(
        r#"UPDATE orders
           SET settle_eligible_at = COALESCE(
               settle_eligible_at,
               COALESCE(completed_at, NOW()) + make_interval(secs => $1::double precision)
           )
           WHERE id = $2"#,
    )
    .bind(delay_secs)
    .bind(order_id)
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("Failed to set settle_eligible_at: {}", e))?;

    sqlx::query(
        r#"UPDATE order_items oi
           SET on_chain_status = 'pending'
           FROM orders o, users u, assets a
           WHERE oi.order_id = $1
             AND oi.on_chain_status IS NULL
             AND oi.order_id = o.id
             AND o.user_id = u.id
             AND oi.asset_id = a.id
             AND u.chain_wallet_address IS NOT NULL
             AND u.chain_wallet_address <> ''
             AND a.chain_contract_address IS NOT NULL
             AND a.chain_contract_address <> ''"#,
    )
    .bind(order_id)
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("Failed to mark order items eligible: {}", e))?;
    Ok(())
}

/// Lift NULL → 'pending' for any of `user_id`'s previously-completed
/// primary order items, now that they have a bound wallet. Call from
/// the SIWE wallet-bind endpoint after `users.chain_wallet_address`
/// is set.
pub async fn mark_user_eligible_after_wallet_bind(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<u64, String> {
    let n = sqlx::query(
        r#"UPDATE order_items oi
           SET on_chain_status = 'pending'
           FROM orders o, assets a
           WHERE o.user_id = $1
             AND oi.order_id = o.id
             AND o.status = 'completed'
             AND oi.on_chain_status IS NULL
             AND oi.asset_id = a.id
             AND a.chain_contract_address IS NOT NULL
             AND a.chain_contract_address <> ''"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to lift items for user: {}", e))?
    .rows_affected();
    Ok(n)
}

/// Lift NULL → 'pending' for completed primary order items targeting
/// `asset_id`, now that the asset has a deployed contract. Call from
/// the admin tokenization-confirmation flow after
/// `assets.chain_contract_address` is set.
pub async fn mark_asset_eligible_after_tokenization(
    pool: &PgPool,
    asset_id: Uuid,
) -> Result<u64, String> {
    let n = sqlx::query(
        r#"UPDATE order_items oi
           SET on_chain_status = 'pending'
           FROM orders o, users u
           WHERE oi.asset_id = $1
             AND oi.order_id = o.id
             AND o.status = 'completed'
             AND oi.on_chain_status IS NULL
             AND o.user_id = u.id
             AND u.chain_wallet_address IS NOT NULL
             AND u.chain_wallet_address <> ''"#,
    )
    .bind(asset_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to lift items for asset: {}", e))?
    .rows_affected();
    Ok(n)
}

/// Read `chain_primary_settle_delay_secs` from `platform_settings`.
/// Used by `mark_order_eligible` callers that don't already have it.
pub async fn read_settle_delay_secs(pool: &PgPool) -> i64 {
    read_setting_u64(
        pool,
        "chain_primary_settle_delay_secs",
        86_400,
        0,
        7 * 86_400,
    )
    .await as i64
}

async fn read_setting_bool(pool: &PgPool, key: &str, default: bool) -> bool {
    sqlx::query_scalar::<_, String>("SELECT value FROM platform_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(default)
}

async fn read_setting_u64(pool: &PgPool, key: &str, default: u64, min: u64, max: u64) -> u64 {
    sqlx::query_scalar::<_, String>("SELECT value FROM platform_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

async fn read_setting_usize(
    pool: &PgPool,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> usize {
    sqlx::query_scalar::<_, String>("SELECT value FROM platform_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
        .clamp(min, max)
}
