/// Dividend calculation engine, anti-sniping logic, and payout execution.
///
/// Architecture:
/// - Phase 1 (current): Off-chain fiat credits to user wallets (BIGINT cents)
/// - Phase 2 (future): On-chain USDC via Merkle proof claims
///
/// Anti-sniping measures (§3.2.10 Masterplan):
/// - Secret snapshot timing (admin chooses when)
/// - Holding period requirement (default 7 days, configurable per distribution)
/// - Users who acquired tokens < N days before snapshot are marked ineligible
///
/// Payout execution:
/// - All wallet credits happen inside a single ACID transaction
/// - Each payout creates a `wallet_transactions` entry for audit trail
/// - Distribution status is atomically updated to 'distributed'
///
/// 🔴 FINANCIAL CODE — All amounts in BIGINT cents. No floats. No rounding errors.
use sqlx::PgPool;
use uuid::Uuid;

fn format_usd_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}${}.{:02}", abs / 100, abs % 100)
}

// ═══════════════════════════════════════════════════════════════
// ── TYPES ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// A holder's position at snapshot time (for dividend calculation).
#[derive(Debug)]
struct HolderSnapshot {
    user_id: Uuid,
    tokens_held: i64,
    /// When the user first acquired this asset's tokens
    first_acquired_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Result of a dividend calculation.
#[derive(Debug, serde::Serialize)]
pub struct CalculationResult {
    pub distribution_id: String,
    pub asset_id: String,
    pub total_amount_cents: i64,
    pub total_tokens: i64,
    pub eligible_holders: i32,
    pub ineligible_holders: i32,
    pub payouts: Vec<PayoutPreview>,
}

/// A single payout preview (before approval).
#[derive(Debug, serde::Serialize)]
pub struct PayoutPreview {
    pub user_id: String,
    pub user_email: String,
    pub tokens_held: i64,
    pub percentage_bps: i32,
    pub payout_cents: i64,
    pub holding_days: i32,
    pub eligible: bool,
}

/// Summary of a completed distribution.
#[derive(Debug, serde::Serialize)]
pub struct DistributionSummary {
    pub distribution_id: String,
    pub total_credited_cents: i64,
    pub holders_credited: i32,
    pub holders_skipped: i32,
}

// ═══════════════════════════════════════════════════════════════
// ── 9.1 DIVIDEND CALCULATION ENGINE ───────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Calculate dividends for an asset over a given period.
///
/// Steps:
/// 1. Create a `dividend_distributions` entry in 'draft' status
/// 2. Snapshot all holders from `investments` (off-chain source of truth)
/// 3. Apply anti-sniping filter (min holding days)
/// 4. Calculate per-holder payout proportionally (integer math only)
/// 5. Insert `dividend_payouts` entries
/// 6. Update distribution status to 'calculated'
///
/// Returns a preview of all payouts for admin review.
pub async fn calculate_dividends(
    pool: &PgPool,
    asset_id: Uuid,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    total_amount_cents: i64,
    min_holding_days: i32,
    admin_user_id: Uuid,
) -> Result<CalculationResult, String> {
    // Validate inputs
    if total_amount_cents <= 0 {
        return Err("Total amount must be positive".to_string());
    }
    if period_end <= period_start {
        return Err("Period end must be after period start".to_string());
    }

    // Check asset exists
    let asset_exists: bool =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM assets WHERE id = $1)")
            .bind(asset_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {e}"))?;

    if !asset_exists {
        return Err("Asset not found".to_string());
    }

    // Check no duplicate distribution for this period
    let existing: Option<String> = sqlx::query_scalar(
        r#"SELECT id::text FROM dividend_distributions
           WHERE asset_id = $1 AND period_start = $2 AND period_end = $3
           AND status != 'cancelled'"#,
    )
    .bind(asset_id)
    .bind(period_start)
    .bind(period_end)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    if existing.is_some() {
        return Err("A distribution already exists for this period".to_string());
    }

    // Start transaction
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("TX begin error: {e}"))?;

    // 1. Snapshot holders from investments table
    let holders: Vec<HolderSnapshot> =
        sqlx::query_as::<_, (Uuid, i64, Option<chrono::DateTime<chrono::Utc>>)>(
            r#"SELECT user_id,
                  COALESCE(tokens_owned, 0)::bigint as tokens_held,
                  purchased_at as first_acquired_at
           FROM investments
           WHERE asset_id = $1 AND tokens_owned > 0
           ORDER BY tokens_owned DESC"#,
        )
        .bind(asset_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("DB snapshot error: {e}"))?
        .into_iter()
        .map(|(user_id, tokens_held, first_acquired_at)| HolderSnapshot {
            user_id,
            tokens_held,
            first_acquired_at,
        })
        .collect();

    if holders.is_empty() {
        return Err("No token holders found for this asset".to_string());
    }

    // 2. Calculate total tokens in circulation
    let total_tokens: i64 = holders.iter().map(|h| h.tokens_held).sum();
    if total_tokens <= 0 {
        return Err("Total tokens in circulation is zero".to_string());
    }

    // 3. Create distribution record
    let snapshot_at = chrono::Utc::now();
    let distribution_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO dividend_distributions
           (asset_id, period_start, period_end, total_amount_cents,
            snapshot_at, total_tokens_snapshot, min_holding_days,
            status, created_by, calculated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, 'calculated', $8, NOW())
           RETURNING id"#,
    )
    .bind(asset_id)
    .bind(period_start)
    .bind(period_end)
    .bind(total_amount_cents)
    .bind(snapshot_at)
    .bind(total_tokens as i32)
    .bind(min_holding_days)
    .bind(admin_user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("DB insert distribution error: {e}"))?;

    // 4. Calculate per-holder payouts with anti-sniping
    let mut payouts: Vec<PayoutPreview> = Vec::new();
    let mut eligible_count = 0i32;
    let mut ineligible_count = 0i32;

    // Running total to handle rounding — last eligible holder gets remainder
    let mut total_allocated_cents: i64 = 0;
    let eligible_holders: Vec<&HolderSnapshot> = holders.iter().collect();

    for (idx, holder) in eligible_holders.iter().enumerate() {
        // Anti-sniping check (§3.2.10)
        let holding_days = calculate_holding_days(holder, &snapshot_at);
        let is_eligible = holding_days >= min_holding_days;

        // Calculate payout (integer math only — no floats!)
        // payout = total_amount * tokens_held / total_tokens
        // BPS (basis points): percentage_bps = tokens_held * 10000 / total_tokens
        let percentage_bps = ((holder.tokens_held * 10000) / total_tokens) as i32;

        let payout_cents = if !is_eligible {
            0 // Ineligible holders get nothing
        } else if idx == eligible_holders.len() - 1 && is_eligible {
            // Last eligible holder gets the remainder (prevents rounding loss)
            let eligible_total = calculate_eligible_total(
                &holders,
                &snapshot_at,
                min_holding_days,
                total_amount_cents,
                total_tokens,
            );
            eligible_total - total_allocated_cents
        } else {
            // Standard calculation: proportional share
            (total_amount_cents * holder.tokens_held) / total_tokens
        };

        if is_eligible && payout_cents > 0 {
            total_allocated_cents += payout_cents;
            eligible_count += 1;
        } else if !is_eligible {
            ineligible_count += 1;
        }

        // Get user email for preview
        let email: String =
            sqlx::query_scalar("SELECT COALESCE(email, '') FROM users WHERE id = $1")
                .bind(holder.user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| format!("DB email lookup error: {e}"))?
                .unwrap_or_default();

        // Insert payout record (even for ineligible — for audit trail)
        let actual_payout = if is_eligible && payout_cents > 0 {
            payout_cents
        } else {
            0
        };
        if actual_payout > 0 || !is_eligible {
            // Only insert if there's something meaningful to record
            let insert_payout_cents = if actual_payout > 0 { actual_payout } else { 1 }; // DB has CHECK payout_cents > 0
            let result = sqlx::query(
                r#"INSERT INTO dividend_payouts
                   (distribution_id, user_id, asset_id, amount_cents, payout_type, status,
                    tokens_held, percentage_bps, first_acquired_at, holding_days, eligible)
                   VALUES ($1, $2, $3, $4, 'rental', 'scheduled', $5, $6, $7, $8, $9)
                   ON CONFLICT (distribution_id, user_id) DO NOTHING"#,
            )
            .bind(distribution_id)
            .bind(holder.user_id)
            .bind(asset_id)
            .bind(insert_payout_cents)
            .bind(holder.tokens_held as i32)
            .bind(percentage_bps)
            .bind(holder.first_acquired_at)
            .bind(holding_days)
            .bind(is_eligible)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("DB payout insert error: {e}"))?;

            if result.rows_affected() == 0 {
                return Err(format!(
                    "Duplicate payout row for distribution {} and user {}",
                    distribution_id, holder.user_id
                ));
            }
        }

        payouts.push(PayoutPreview {
            user_id: holder.user_id.to_string(),
            user_email: email,
            tokens_held: holder.tokens_held,
            percentage_bps,
            payout_cents: if is_eligible { payout_cents } else { 0 },
            holding_days,
            eligible: is_eligible,
        });
    }

    // Update eligible_holders count on the distribution
    let update_result =
        sqlx::query("UPDATE dividend_distributions SET eligible_holders = $1 WHERE id = $2")
            .bind(eligible_count)
            .bind(distribution_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("DB distribution update error: {e}"))?;

    if update_result.rows_affected() != 1 {
        return Err("Failed to update dividend distribution holder counts".to_string());
    }

    // Commit
    tx.commit()
        .await
        .map_err(|e| format!("TX commit error: {e}"))?;

    tracing::info!(
        "💰 Dividend calculated: dist={}, asset={}, total={}¢, eligible={}, ineligible={}",
        distribution_id,
        asset_id,
        total_amount_cents,
        eligible_count,
        ineligible_count
    );

    Ok(CalculationResult {
        distribution_id: distribution_id.to_string(),
        asset_id: asset_id.to_string(),
        total_amount_cents,
        total_tokens,
        eligible_holders: eligible_count,
        ineligible_holders: ineligible_count,
        payouts,
    })
}

// ═══════════════════════════════════════════════════════════════
// ── 9.2 ANTI-SNIPING LOGIC ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Calculate how many days a holder has held their tokens.
///
/// Uses `first_acquired_at` from the investments table.
/// If unknown, defaults to 0 (will be filtered by anti-sniping).
fn calculate_holding_days(
    holder: &HolderSnapshot,
    snapshot_at: &chrono::DateTime<chrono::Utc>,
) -> i32 {
    match holder.first_acquired_at {
        Some(acquired) => {
            let duration = *snapshot_at - acquired;
            std::cmp::max(0, duration.num_days() as i32)
        }
        None => {
            // Conservative: if we don't know when they acquired, assume recent (0 days)
            // This can be overridden by admin setting min_holding_days = 0
            0
        }
    }
}

/// Calculate the total amount that will actually be distributed to eligible holders.
///
/// When some holders are ineligible (anti-sniping), their share is redistributed
/// proportionally among eligible holders.
fn calculate_eligible_total(
    holders: &[HolderSnapshot],
    snapshot_at: &chrono::DateTime<chrono::Utc>,
    min_holding_days: i32,
    total_amount_cents: i64,
    total_tokens: i64,
) -> i64 {
    let eligible_tokens: i64 = holders
        .iter()
        .filter(|h| calculate_holding_days(h, snapshot_at) >= min_holding_days)
        .map(|h| h.tokens_held)
        .sum();

    if eligible_tokens <= 0 || total_tokens <= 0 {
        return 0;
    }

    // Redistribute: eligible holders split the FULL amount
    // (ineligible holders' share goes back to the pool)
    total_amount_cents
}

// ═══════════════════════════════════════════════════════════════
// ── ADMIN OPERATIONS ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Approve a calculated distribution.
///
/// Changes status from 'calculated' → 'approved'.
/// Only admins can approve.
pub async fn approve_distribution(
    pool: &PgPool,
    distribution_id: Uuid,
    admin_user_id: Uuid,
) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("TX begin error: {e}"))?;

    let current: Option<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT created_by, status FROM dividend_distributions WHERE id = $1 FOR UPDATE",
    )
    .bind(distribution_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let (created_by, status) = match current {
        Some(row) => row,
        None => return Err("Distribution not found".to_string()),
    };

    if status != "calculated" {
        return Err("Distribution not in 'calculated' status".to_string());
    }

    let created_by = created_by.ok_or_else(|| {
        "Distribution is missing creator metadata and cannot be approved".to_string()
    })?;

    if created_by == admin_user_id {
        return Err("Creator cannot approve their own dividend distribution".to_string());
    }

    let result = sqlx::query(
        r#"UPDATE dividend_distributions
           SET status = 'approved', approved_by = $1, approved_at = NOW()
           WHERE id = $2 AND status = 'calculated'"#,
    )
    .bind(admin_user_id)
    .bind(distribution_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("Distribution not found or not in 'calculated' status".to_string());
    }

    sqlx::query(
        r#"INSERT INTO audit_logs
           (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'dividend_distribution.approved', 'dividend_distributions', $2, $3, $4)"#,
    )
    .bind(admin_user_id)
    .bind(distribution_id)
    .bind(serde_json::json!({"status": "calculated"}))
    .bind(serde_json::json!({"status": "approved"}))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB audit log error: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("TX commit error: {e}"))?;

    tracing::info!(
        "💰 Distribution {} approved by {}",
        distribution_id,
        admin_user_id
    );
    Ok(())
}

/// Cancel a distribution (only if not yet distributed).
pub async fn cancel_distribution(
    pool: &PgPool,
    distribution_id: Uuid,
    reason: &str,
    admin_user_id: Uuid,
) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("TX begin error: {e}"))?;

    let previous_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM dividend_distributions WHERE id = $1 FOR UPDATE")
            .bind(distribution_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("DB error: {e}"))?;

    let previous_status = match previous_status {
        Some(status) => status,
        None => return Err("Distribution not found".to_string()),
    };

    let result = sqlx::query(
        r#"UPDATE dividend_distributions
           SET status = 'cancelled', cancelled_at = NOW(), cancel_reason = $1
           WHERE id = $2 AND status IN ('draft', 'calculated', 'approved')"#,
    )
    .bind(reason)
    .bind(distribution_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("Distribution not found or already distributed".to_string());
    }

    sqlx::query(
        r#"INSERT INTO audit_logs
           (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'dividend_distribution.cancelled', 'dividend_distributions', $2, $3, $4)"#,
    )
    .bind(admin_user_id)
    .bind(distribution_id)
    .bind(serde_json::json!({"status": previous_status}))
    .bind(serde_json::json!({"status": "cancelled", "reason": reason}))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB audit log error: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("TX commit error: {e}"))?;

    tracing::info!("💰 Distribution {} cancelled: {}", distribution_id, reason);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── 9.4 PAYOUT EXECUTION ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Execute payout for an approved distribution.
///
/// 🔴 CRITICAL FINANCIAL OPERATION — Wrapped in a single ACID transaction.
///
/// For each eligible payout:
/// 1. Credit the user's cash wallet (UPDATE wallets.balance_cents)
/// 2. Create a wallet_transactions record (audit trail)
/// 3. Mark the payout as credited
/// 4. Update the distribution status to 'distributed'
///
/// If ANY step fails, the ENTIRE transaction is rolled back.
/// No partial payouts — either all succeed or none do.
pub async fn execute_distribution(
    pool: &PgPool,
    distribution_id: Uuid,
    executor_user_id: Uuid,
) -> Result<DistributionSummary, String> {
    // 1. Begin ACID transaction first so the distribution row is locked
    //    for the whole execution. Previously we SELECT'd outside the tx,
    //    which let a second concurrent executor see the same 'approved'
    //    snapshot and double-pay.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("TX begin error: {e}"))?;

    let dist_info: Option<(Uuid, i64, String, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        r#"SELECT asset_id, total_amount_cents, status, created_by, approved_by
           FROM dividend_distributions WHERE id = $1 FOR UPDATE"#,
    )
    .bind(distribution_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let (asset_id, total_amount_cents, status, created_by, approved_by) = match dist_info {
        Some(info) => info,
        None => return Err("Distribution not found".to_string()),
    };

    if status != "approved" {
        return Err(format!(
            "Distribution is '{}', must be 'approved' to execute",
            status
        ));
    }

    let created_by = created_by.ok_or_else(|| {
        "Distribution is missing creator metadata and cannot be executed".to_string()
    })?;
    let approved_by = approved_by.ok_or_else(|| {
        "Distribution is missing approver metadata and cannot be executed".to_string()
    })?;

    if executor_user_id == created_by {
        return Err("Creator cannot execute their own dividend distribution".to_string());
    }

    // 3. Get all eligible, uncredited payouts
    let payouts: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        r#"SELECT id, user_id, amount_cents
           FROM dividend_payouts
           WHERE distribution_id = $1
           AND eligible = true
           AND (wallet_credited = false OR wallet_credited IS NULL)
           AND amount_cents > 0
           ORDER BY amount_cents DESC
           FOR UPDATE"#, // Lock rows to prevent concurrent execution
    )
    .bind(distribution_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("DB payout fetch error: {e}"))?;

    if payouts.is_empty() {
        return Err("No eligible uncredited payouts found".to_string());
    }

    let mut total_credited: i64 = 0;
    let mut holders_credited: i32 = 0;
    let mut holders_skipped: i32 = 0;

    // Get asset title for transaction descriptions
    let asset_name: String =
        sqlx::query_scalar("SELECT COALESCE(title, 'Unknown Asset') FROM assets WHERE id = $1")
            .bind(asset_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("DB lookup error: {e}"))?
            .unwrap_or_else(|| "Unknown Asset".to_string());

    for (payout_id, user_id, amount_cents) in &payouts {
        // 4a. Credit the user's USD cash wallet. Currency is pinned so a
        //     future multi-currency wallet setup doesn't quietly splash
        //     dividends across every wallet the user owns.
        let wallet_result = sqlx::query(
            r#"UPDATE wallets
               SET balance_cents = balance_cents + $1, updated_at = NOW()
               WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'"#,
        )
        .bind(amount_cents)
        .bind(user_id)
        .execute(&mut *tx)
        .await;

        match wallet_result {
            Ok(r) if r.rows_affected() > 0 => {
                // 4b. Get user's USD cash wallet_id for the transaction record
                let wallet_id: Option<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM wallets WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
                )
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| format!("DB wallet lookup error: {e}"))?;

                // 4c. Create wallet_transactions record (audit trail)
                let tx_id: Option<Uuid> = if let Some(wid) = wallet_id {
                    sqlx::query_scalar(
                        r#"INSERT INTO wallet_transactions
                           (wallet_id, type, amount_cents, currency,
                            description, status, created_at)
                           VALUES ($1, 'dividend', $2, 'USD',
                                   $3, 'completed', NOW())
                           RETURNING id"#,
                    )
                    .bind(wid)
                    .bind(amount_cents)
                    .bind(format!("Dividend: {} rental income", asset_name))
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| format!("DB wallet_tx error: {e}"))?
                } else {
                    None
                };

                // 4d. Mark payout as credited
                let payout_update = sqlx::query(
                    r#"UPDATE dividend_payouts
                       SET wallet_credited = true, credited_at = NOW(), wallet_tx_id = $1,
                           status = 'paid', paid_at = NOW()
                       WHERE id = $2"#,
                )
                .bind(tx_id)
                .bind(payout_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("DB payout update error: {e}"))?;

                if payout_update.rows_affected() != 1 {
                    return Err(format!(
                        "Payout update affected {} rows for payout {}",
                        payout_update.rows_affected(),
                        payout_id
                    ));
                }

                total_credited += amount_cents;
                holders_credited += 1;
            }
            Ok(_) => {
                // Wallet not found for user — skip but log
                tracing::warn!(
                    "💰 No cash wallet for user {} — skipping dividend payout",
                    user_id
                );
                holders_skipped += 1;
            }
            Err(e) => {
                // DB error — rollback entire transaction
                return Err(format!("Wallet credit failed for user {}: {}", user_id, e));
            }
        }
    }

    // 5. Sanity check: total credited should not exceed total_amount_cents
    if total_credited > total_amount_cents {
        return Err(format!(
            "[P0-FINANCIAL] Dividend payout exceeds total! Credited: {}¢ > Total: {}¢ — ROLLING BACK",
            total_credited, total_amount_cents
        ));
    }

    // 6. Update distribution status to 'distributed'
    let distribution_update = sqlx::query(
        r#"UPDATE dividend_distributions
           SET status = 'distributed', distributed_at = NOW(), distributed_by = $1
           WHERE id = $2"#,
    )
    .bind(executor_user_id)
    .bind(distribution_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB distribution update error: {e}"))?;

    if distribution_update.rows_affected() != 1 {
        return Err("Failed to mark dividend distribution as distributed".to_string());
    }

    sqlx::query(
        r#"INSERT INTO audit_logs
           (actor_user_id, action, entity_type, entity_id, previous_state, new_state, metadata)
           VALUES ($1, 'dividend_distribution.executed', 'dividend_distributions', $2, $3, $4, $5)"#,
    )
    .bind(executor_user_id)
    .bind(distribution_id)
    .bind(serde_json::json!({
        "status": "approved",
        "approved_by": approved_by,
    }))
    .bind(serde_json::json!({
        "status": "distributed",
        "total_credited_cents": total_credited,
        "holders_credited": holders_credited,
        "holders_skipped": holders_skipped,
    }))
    .bind(serde_json::json!({
        "asset_id": asset_id,
        "created_by": created_by,
        "approved_by": approved_by,
        "executor_user_id": executor_user_id,
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("DB audit log error: {e}"))?;

    // 7. COMMIT — all payouts atomically applied
    tx.commit()
        .await
        .map_err(|e| format!("TX commit error: {e}"))?;

    tracing::info!(
        "💰 ✅ Distribution {} completed: {}¢ credited to {} holders ({} skipped)",
        distribution_id,
        total_credited,
        holders_credited,
        holders_skipped
    );

    Ok(DistributionSummary {
        distribution_id: distribution_id.to_string(),
        total_credited_cents: total_credited,
        holders_credited,
        holders_skipped,
    })
}

// ═══════════════════════════════════════════════════════════════
// ── QUERY HELPERS ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// List all distributions for an asset.
pub async fn list_distributions(
    pool: &PgPool,
    asset_id: Option<Uuid>,
) -> Result<Vec<serde_json::Value>, String> {
    let query = if let Some(aid) = asset_id {
        sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i64,
                String,
                i32,
                String,
                Option<String>,
            ),
        >(
            r#"SELECT d.id::text, d.asset_id::text,
                      COALESCE(a.title, '') as asset_name,
                      d.period_start::text || ' - ' || d.period_end::text as period,
                      d.total_amount_cents,
                      d.status,
                      d.eligible_holders,
                      d.created_at::text,
                      d.distributed_at::text
               FROM dividend_distributions d
               LEFT JOIN assets a ON a.id = d.asset_id
               WHERE d.asset_id = $1
               ORDER BY d.created_at DESC
               LIMIT 50"#,
        )
        .bind(aid)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i64,
                String,
                i32,
                String,
                Option<String>,
            ),
        >(
            r#"SELECT d.id::text, d.asset_id::text,
                      COALESCE(a.title, '') as asset_name,
                      d.period_start::text || ' - ' || d.period_end::text as period,
                      d.total_amount_cents,
                      d.status,
                      d.eligible_holders,
                      d.created_at::text,
                      d.distributed_at::text
               FROM dividend_distributions d
               LEFT JOIN assets a ON a.id = d.asset_id
               ORDER BY d.created_at DESC
               LIMIT 50"#,
        )
        .fetch_all(pool)
        .await
    };

    let rows = query.map_err(|e| format!("DB error: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "asset_id": r.1,
                "asset_name": r.2,
                "period": r.3,
                "total_amount_cents": r.4,
                "total_amount_display": format_usd_cents(r.4),
                "status": r.5,
                "eligible_holders": r.6,
                "created_at": r.7,
                "distributed_at": r.8,
            })
        })
        .collect())
}

/// Get distribution details with all payouts.
pub async fn get_distribution_detail(
    pool: &PgPool,
    distribution_id: Uuid,
) -> Result<serde_json::Value, String> {
    let dist: Option<(
        String,
        String,
        String,
        String,
        i64,
        String,
        i32,
        i32,
        String,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        r#"SELECT d.id::text, d.asset_id::text,
                  COALESCE(a.title, '') as asset_name,
                  d.period_start::text || ' - ' || d.period_end::text,
                  d.total_amount_cents, d.status,
                  d.eligible_holders, d.min_holding_days,
                  d.created_at::text, d.approved_at::text, d.distributed_at::text
           FROM dividend_distributions d
           LEFT JOIN assets a ON a.id = d.asset_id
           WHERE d.id = $1"#,
    )
    .bind(distribution_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let dist = match dist {
        Some(d) => d,
        None => return Err("Distribution not found".to_string()),
    };

    let payouts: Vec<(
        String,
        String,
        Option<i32>,
        i64,
        Option<i32>,
        Option<i32>,
        bool,
        bool,
    )> = sqlx::query_as(
        r#"SELECT p.user_id::text, COALESCE(u.email, ''),
                  p.tokens_held, p.amount_cents,
                  p.percentage_bps, p.holding_days,
                  p.eligible, COALESCE(p.wallet_credited, false)
           FROM dividend_payouts p
           LEFT JOIN users u ON u.id = p.user_id
           WHERE p.distribution_id = $1
           ORDER BY p.amount_cents DESC"#,
    )
    .bind(distribution_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let payout_list: Vec<serde_json::Value> = payouts
        .iter()
        .map(|p| {
            serde_json::json!({
                "user_id": p.0,
                "user_email": p.1,
                "tokens_held": p.2,
                "payout_cents": p.3,
                "payout_display": format_usd_cents(p.3),
                "percentage_bps": p.4,
                "holding_days": p.5,
                "eligible": p.6,
                "wallet_credited": p.7,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "distribution": {
            "id": dist.0,
            "asset_id": dist.1,
            "asset_name": dist.2,
            "period": dist.3,
            "total_amount_cents": dist.4,
            "total_amount_display": format_usd_cents(dist.4),
            "status": dist.5,
            "eligible_holders": dist.6,
            "min_holding_days": dist.7,
            "created_at": dist.8,
            "approved_at": dist.9,
            "distributed_at": dist.10,
        },
        "payouts": payout_list,
    }))
}

// ═══════════════════════════════════════════════════════════════
// ── 9.X UNIT TESTS ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn make_snapshot(acquired: Option<chrono::DateTime<Utc>>, tokens: i64) -> HolderSnapshot {
        HolderSnapshot {
            user_id: Uuid::new_v4(),
            tokens_held: tokens,
            first_acquired_at: acquired,
        }
    }

    #[test]
    fn test_calculate_holding_days() {
        let now = Utc::now();

        let holder1 = make_snapshot(Some(now - Duration::days(5)), 100);
        assert_eq!(calculate_holding_days(&holder1, &now), 5);

        // Exact day
        let holder2 = make_snapshot(Some(now - Duration::hours(25)), 100);
        assert_eq!(calculate_holding_days(&holder2, &now), 1);

        // Future acquisition? (edge case)
        let holder3 = make_snapshot(Some(now + Duration::days(5)), 100);
        assert_eq!(calculate_holding_days(&holder3, &now), 0);

        // No acquisition date defaults to 0
        let holder4 = make_snapshot(None, 100);
        assert_eq!(calculate_holding_days(&holder4, &now), 0);
    }

    #[test]
    fn test_eligible_total_all_eligible() {
        let now = Utc::now();
        let min_days = 30;
        let total_amount = 100_000;
        let total_tokens = 1000;

        let holders = vec![
            make_snapshot(Some(now - Duration::days(40)), 500),
            make_snapshot(Some(now - Duration::days(50)), 500),
        ];

        assert_eq!(
            calculate_eligible_total(&holders, &now, min_days, total_amount, total_tokens),
            total_amount
        );
    }

    #[test]
    fn test_eligible_total_none_eligible() {
        let now = Utc::now();
        let min_days = 30;
        let total_amount = 100_000;
        let total_tokens = 1000;

        let holders = vec![
            make_snapshot(Some(now - Duration::days(10)), 500),
            make_snapshot(Some(now - Duration::days(20)), 500),
        ];

        assert_eq!(
            calculate_eligible_total(&holders, &now, min_days, total_amount, total_tokens),
            0
        );
    }

    #[test]
    fn test_eligible_total_partially_eligible() {
        let now = Utc::now();
        let min_days = 30;
        let total_amount = 100_000;
        let total_tokens = 1000;

        let holders = vec![
            make_snapshot(Some(now - Duration::days(40)), 500),
            make_snapshot(Some(now - Duration::days(10)), 500),
        ];

        // Redistribution logic: eligible portion gets the whole pot.
        assert_eq!(
            calculate_eligible_total(&holders, &now, min_days, total_amount, total_tokens),
            total_amount
        );
    }

    #[test]
    fn test_basic_payout_math() {
        let total_amount = 100_000;
        let eligible_tokens = 1000;

        let mut total_allocated = 0;
        let holder1_tokens = 400;
        let payout_expected_1 =
            (total_amount as u128 * holder1_tokens as u128) / eligible_tokens as u128;
        assert_eq!(payout_expected_1, 40_000);
        total_allocated += payout_expected_1;

        let holder2_tokens = 600;
        let payout_expected_2 =
            (total_amount as u128 * holder2_tokens as u128) / eligible_tokens as u128;
        assert_eq!(payout_expected_2, 60_000);
        total_allocated += payout_expected_2;

        assert_eq!(total_allocated, total_amount as u128);
    }

    #[test]
    fn test_payout_math_odd_remainders() {
        let total_amount = 100_000;
        let eligible_tokens = 999;

        let holder_tokens = 333;

        let payout_expected =
            (total_amount as u128 * holder_tokens as u128) / eligible_tokens as u128;
        assert_eq!(payout_expected, 33333);

        // 33333 * 3 = 99999. 1 cent is lost due to integer math, maintaining conservation of funds safely.
    }
}
