/// On-chain pending-pile alert (separate from fund conservation).
/// Alerts if too many trades have been waiting in `on_chain_status='pending'`
/// for too long — symptom of a stalled / mis-configured chain worker.
pub async fn check_pending_pile(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let row: (i64, i64) = sqlx::query_as(
        r#"SELECT
              COUNT(*)::bigint AS total,
              COALESCE(EXTRACT(EPOCH FROM NOW() - MIN(executed_at))::bigint, 0) AS oldest_age_secs
           FROM trade_history
           WHERE on_chain_status = 'pending'"#,
    )
    .fetch_one(pool)
    .await?;
    let (total_pending, oldest_age_secs) = row;

    // Thresholds — tune from prod data. Values picked to balance noise
    // vs catching genuine stalls within the next settlement cycle.
    const HIGH_COUNT: i64 = 100;
    const STALE_AGE_SECS: i64 = 1800; // 30 min — well over the 5min settlement interval

    if total_pending >= HIGH_COUNT {
        sentry::capture_message(
            &format!(
                "On-chain pending pile high: {} trades pending settlement",
                total_pending
            ),
            sentry::Level::Warning,
        );
    }
    if oldest_age_secs >= STALE_AGE_SECS && total_pending > 0 {
        sentry::capture_message(
            &format!(
                "On-chain pending pile stale: oldest pending trade is {}s old (count={})",
                oldest_age_secs, total_pending
            ),
            sentry::Level::Error,
        );
    }
    Ok(())
}

/// Conservation-of-funds invariant worker.
///
/// Once per hour, asserts that the SUM of all user wallet balances plus the
/// platform-fee wallet balance equals the SUM of (deposits − withdrawals)
/// across the lifetime of the system. Any drift indicates a settlement bug,
/// missed fee credit, double-debit, or external manual edit.
///
/// We also assert per-wallet that `held_balance_cents <= balance_cents`
/// (you can't have more funds in escrow than in the account), and per-asset
/// that `SUM(tokens_owned) == tokens_total - tokens_unsold`.
///
/// On drift, the worker emits a CRITICAL Sentry message with the delta and
/// stops alerting on subsequent cycles for the same delta (alert dedup) so
/// operators can investigate without flooding noise.
///
/// This is preventative — the worker does NOT auto-correct. Drift requires
/// human investigation; auto-correcting would mask root cause.
use sqlx::PgPool;

const POLL_INTERVAL_SECS: u64 = 3600; // 1 hour
const ALERT_THRESHOLD_CENTS: i64 = 100; // ignore drift < $1.00 (rounding noise)

pub async fn run_invariant_worker(pool: &PgPool) {
    tracing::info!(
        "🔍 Fund-conservation invariant worker starting (interval={}s, threshold={}¢)",
        POLL_INTERVAL_SECS,
        ALERT_THRESHOLD_CENTS
    );

    // Initial delay so the worker doesn't run during boot-time DB churn
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    let mut last_reported_drift: i64 = 0;

    loop {
        match check_fund_conservation(pool).await {
            Ok(report) => {
                tracing::info!(
                    "🔍 Fund conservation: wallets={}, deposits-withdrawals={}, drift={} (threshold {}¢)",
                    report.total_wallet_balances,
                    report.total_net_deposits,
                    report.drift_cents,
                    ALERT_THRESHOLD_CENTS
                );

                if report.drift_cents.abs() > ALERT_THRESHOLD_CENTS
                    && report.drift_cents != last_reported_drift
                {
                    tracing::error!(
                        "🚨 FUND CONSERVATION VIOLATED: drift={}¢ (wallets={}, net_deposits={})",
                        report.drift_cents,
                        report.total_wallet_balances,
                        report.total_net_deposits
                    );
                    sentry::capture_message(
                        &format!(
                            "CRITICAL: Fund conservation drift detected. \
                             Δ={}¢. wallets={}¢, deposits-withdrawals={}¢. \
                             Investigate settlement, withdrawal, or fee paths.",
                            report.drift_cents,
                            report.total_wallet_balances,
                            report.total_net_deposits
                        ),
                        sentry::Level::Fatal,
                    );
                    last_reported_drift = report.drift_cents;
                }

                if let Err(e) = check_per_wallet_invariants(pool).await {
                    tracing::error!("Per-wallet invariant check failed: {}", e);
                }
                if let Err(e) = check_token_invariants(pool).await {
                    tracing::error!("Token invariant check failed: {}", e);
                }
                if let Err(e) = check_pending_pile(pool).await {
                    tracing::error!("Pending-pile check failed: {}", e);
                }

                let _ = persist_report(pool, &report).await;
            }
            Err(e) => {
                tracing::error!("Invariant check query failed: {}", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

#[derive(Debug)]
struct ConservationReport {
    total_wallet_balances: i64,
    total_net_deposits: i64,
    drift_cents: i64,
}

async fn check_fund_conservation(pool: &PgPool) -> Result<ConservationReport, sqlx::Error> {
    // Sum of all USD cash-wallet balances (user + platform_fee).
    let total_wallet_balances: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance_cents), 0)::bigint
         FROM wallets
         WHERE currency = 'USD' AND wallet_type IN ('cash', 'platform_fee')",
    )
    .fetch_one(pool)
    .await?;

    // Net deposits — only count completed deposit/withdrawal transactions.
    let total_net_deposits: i64 = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(
                CASE
                    WHEN type = 'deposit' THEN amount_cents
                    WHEN type = 'withdrawal' THEN -amount_cents
                    ELSE 0
                END
            ), 0)::bigint
           FROM wallet_transactions
           WHERE status = 'completed'"#,
    )
    .fetch_one(pool)
    .await?;

    let drift_cents = total_wallet_balances - total_net_deposits;

    Ok(ConservationReport {
        total_wallet_balances,
        total_net_deposits,
        drift_cents,
    })
}

/// Per-wallet sanity: balance >= held_balance for every wallet.
/// Violation = silent over-hold or accounting bug.
async fn check_per_wallet_invariants(pool: &PgPool) -> Result<(), sqlx::Error> {
    let bad_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM wallets
         WHERE balance_cents < held_balance_cents",
    )
    .fetch_one(pool)
    .await?;

    if bad_count > 0 {
        tracing::error!(
            "🚨 {} wallets have held_balance > balance (over-hold drift)",
            bad_count
        );
        sentry::capture_message(
            &format!(
                "CRITICAL: {} wallets have held_balance_cents > balance_cents",
                bad_count
            ),
            sentry::Level::Fatal,
        );
    }
    Ok(())
}

/// Per-asset token sanity: SUM(tokens_owned) <= tokens_total. Strict equality
/// would also need tokens_unsold tracking; we assert the weaker bound.
async fn check_token_invariants(pool: &PgPool) -> Result<(), sqlx::Error> {
    let over_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM (
               SELECT a.id, a.tokens_total, COALESCE(SUM(i.tokens_owned), 0)::bigint AS owned
               FROM assets a
               LEFT JOIN investments i
                 ON i.asset_id = a.id AND i.status != 'exited'
               GROUP BY a.id, a.tokens_total
               HAVING COALESCE(SUM(i.tokens_owned), 0) > a.tokens_total
           ) t"#,
    )
    .fetch_one(pool)
    .await?;

    if over_count > 0 {
        tracing::error!(
            "🚨 {} assets have SUM(tokens_owned) > tokens_total — token over-issuance",
            over_count
        );
        sentry::capture_message(
            &format!(
                "CRITICAL: {} assets have over-issued tokens (sum_owned > total)",
                over_count
            ),
            sentry::Level::Fatal,
        );
    }
    Ok(())
}

async fn persist_report(pool: &PgPool, report: &ConservationReport) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO marketplace_drift_metrics (metric_type, value)
         VALUES ('fund_conservation_drift_cents', $1),
                ('total_wallet_balances_cents', $2),
                ('total_net_deposits_cents', $3)",
    )
    .bind(report.drift_cents)
    .bind(report.total_wallet_balances)
    .bind(report.total_net_deposits)
    .execute(pool)
    .await
    .map(|_| ())
}
