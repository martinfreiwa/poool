//! Wallet reconciliation background worker.
//!
//! Three jobs run on the same hourly tick:
//!
//! 1. [`expire_stale_deposits`] — `deposit_requests.status='pending'` rows
//!    whose `expires_at < NOW()` have not been credited and the wire is
//!    almost certainly never coming. Transition them to `expired` so
//!    they stop polluting admin dashboards and free their unique
//!    `provider_reference` for re-use.
//!
//! 2. [`flag_stuck_pending_deposits`] — deposits that are still `pending`,
//!    past the admin-configured processing window, AND have no proof
//!    attached. These need a human chase: either the user wired without
//!    proof, or the user submitted proof but our reconciliation pipeline
//!    failed to match the incoming wire. A counter is logged + an audit
//!    log entry written; future iterations can surface this in Slack /
//!    PagerDuty.
//!
//! 3. [`flag_stuck_withdrawals`] — `withdrawal_requests.status='pending'`
//!    rows older than the operational SLA (default 48h). Funds are
//!    frozen in the user's wallet; leaving them in limbo erodes trust.
//!    Logs + audit entry so an admin can dig in.
//!
//! All checks are read-mostly with small, indexed scans. Worst-case
//! pathological volumes are bounded by `LIMIT 500` per run.

use sqlx::PgPool;
use tracing::{info, warn};

/// Stats returned for observability + admin dashboards.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ReconciliationReport {
    /// Deposits transitioned from `pending` to `expired` this run.
    pub deposits_expired: i64,
    /// Pending deposits past their processing window without proof.
    pub deposits_stuck_no_proof: i64,
    /// Pending deposits with proof attached but >24h unverified — needs
    /// admin review.
    pub deposits_stuck_with_proof: i64,
    /// Pending withdrawals older than the SLA threshold.
    pub withdrawals_stuck: i64,
}

const RECONCILIATION_INTERVAL_SECS: u64 = 3600; // hourly
const WITHDRAWAL_SLA_HOURS: i64 = 48;
const DEPOSIT_STUCK_HOURS: i64 = 24;
const SCAN_LIMIT: i64 = 500;

/// Spawn-target for `tokio::spawn` in `lib.rs`. Loops forever; errors are
/// logged + reported to Sentry but do not stop the worker.
pub async fn run_reconciliation_worker(pool: PgPool) {
    let mut interval =
        tokio::time::interval(std::time::Duration::from_secs(RECONCILIATION_INTERVAL_SECS));
    // Skip the immediate tick — give the server a few seconds to settle.
    interval.tick().await;

    loop {
        interval.tick().await;
        match run_once(&pool).await {
            Ok(report) => {
                if report.deposits_expired > 0
                    || report.deposits_stuck_no_proof > 0
                    || report.deposits_stuck_with_proof > 0
                    || report.withdrawals_stuck > 0
                {
                    info!(
                        deposits_expired = report.deposits_expired,
                        deposits_stuck_no_proof = report.deposits_stuck_no_proof,
                        deposits_stuck_with_proof = report.deposits_stuck_with_proof,
                        withdrawals_stuck = report.withdrawals_stuck,
                        "Wallet reconciliation run produced findings"
                    );
                }
            }
            Err(e) => {
                tracing::error!("Wallet reconciliation worker error: {}", e);
                sentry::capture_message(
                    &format!("Wallet reconciliation worker error: {}", e),
                    sentry::Level::Error,
                );
            }
        }
    }
}

/// Single reconciliation pass. Public so admin / tests can trigger it
/// on demand without waiting for the background tick.
pub async fn run_once(pool: &PgPool) -> Result<ReconciliationReport, sqlx::Error> {
    let mut report = ReconciliationReport::default();

    report.deposits_expired = expire_stale_deposits(pool).await?;
    let (no_proof, with_proof) = count_stuck_deposits(pool).await?;
    report.deposits_stuck_no_proof = no_proof;
    report.deposits_stuck_with_proof = with_proof;
    report.withdrawals_stuck = count_stuck_withdrawals(pool).await?;

    // Mirror the run findings into Prometheus gauges so Grafana shows
    // both the latest state and rate-of-change.
    crate::metrics::record_reconciliation_snapshot(
        report.deposits_expired,
        report.deposits_stuck_no_proof,
        report.deposits_stuck_with_proof,
        report.withdrawals_stuck,
    );
    if report.deposits_expired > 0 {
        crate::metrics::record_deposit(crate::metrics::deposit_outcome::EXPIRED, "USD", 0);
    }

    Ok(report)
}

/// Mark expired pending deposits. Returns rows affected.
pub async fn expire_stale_deposits(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, i64>(
        r#"
        WITH expired AS (
            UPDATE deposit_requests
               SET status = 'expired', updated_at = NOW()
             WHERE status IN ('pending', 'requested')
               AND expires_at IS NOT NULL
               AND expires_at < NOW()
               AND id IN (
                   SELECT id FROM deposit_requests
                    WHERE status IN ('pending', 'requested')
                      AND expires_at IS NOT NULL
                      AND expires_at < NOW()
                    ORDER BY expires_at ASC
                    LIMIT $1
               )
            RETURNING id, user_id, amount_cents
        ),
        audit AS (
            INSERT INTO audit_logs (action, entity_type, entity_id, new_state)
            SELECT 'deposit.auto_expired', 'deposit_request', id,
                   jsonb_build_object('amount_cents', amount_cents, 'reason', 'expires_at_passed')
              FROM expired
            RETURNING 1
        )
        SELECT COUNT(*)::bigint FROM expired
        "#,
    )
    .bind(SCAN_LIMIT)
    .fetch_one(pool)
    .await?;

    if rows > 0 {
        info!(rows, "Auto-expired stale deposit requests");
    }
    Ok(rows)
}

/// Stuck deposits split by whether the user uploaded proof.
///
/// `(no_proof, with_proof)`:
///   - `no_proof`   — user never followed through, possibly never wired
///   - `with_proof` — wire & proof submitted but admin hasn't matched it;
///                    operationally this is the more interesting bucket.
async fn count_stuck_deposits(pool: &PgPool) -> Result<(i64, i64), sqlx::Error> {
    let row: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE proof_gcs_path IS NULL)::bigint AS no_proof,
            COUNT(*) FILTER (WHERE proof_gcs_path IS NOT NULL)::bigint AS with_proof
          FROM deposit_requests
         WHERE status IN ('pending', 'requested')
           AND created_at < NOW() - ($1 || ' hours')::interval
        "#,
    )
    .bind(DEPOSIT_STUCK_HOURS.to_string())
    .fetch_one(pool)
    .await?;
    if row.0 > 0 || row.1 > 0 {
        warn!(
            no_proof = row.0,
            with_proof = row.1,
            stuck_threshold_hours = DEPOSIT_STUCK_HOURS,
            "Stuck pending deposits detected"
        );
    }
    Ok(row)
}

async fn count_stuck_withdrawals(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
          FROM withdrawal_requests
         WHERE status = 'pending'
           AND created_at < NOW() - ($1 || ' hours')::interval
        "#,
    )
    .bind(WITHDRAWAL_SLA_HOURS.to_string())
    .fetch_one(pool)
    .await?;
    if count > 0 {
        warn!(
            count,
            sla_hours = WITHDRAWAL_SLA_HOURS,
            "Withdrawals exceeded review SLA"
        );
    }
    Ok(count)
}
