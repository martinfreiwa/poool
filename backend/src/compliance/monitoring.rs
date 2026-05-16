//! Transaction monitoring rule engine (P0-1).
//!
//! Runs a small catalogue of pattern-detection rules across the
//! `wallet_transactions` + `withdrawal_requests` tables and opens
//! `compliance_alerts` (reusing the queue introduced in migration 182)
//! for each match.
//!
//! Rules
//! -----
//! Each rule lives in [`Rule::all()`] and is identified by a stable
//! `code` (mirrored by a row in `transaction_monitoring_rules`).
//! Adding a rule = adding a new variant + an `impl`. The engine
//! handles dispatch, dedup and alert creation.
//!
//! Dedup
//! -----
//! `transaction_monitoring_findings` has a `UNIQUE (user_id, rule_code,
//! window_end)` constraint. Two runs over the same data set therefore
//! cannot double-emit. We bucket windows on the hour for consistency.

use chrono::{DateTime, Timelike, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

const WORKER_INTERVAL_MIN: u64 = 60;

/// One match emitted by the engine. Stored verbatim in
/// `transaction_monitoring_findings` and copied into `compliance_alerts`
/// for triage.
#[derive(Debug, Clone)]
pub struct Finding {
    /// User the pattern applies to.
    pub user_id: Uuid,
    /// Stable rule code (matches `transaction_monitoring_rules.code`).
    pub rule_code: &'static str,
    /// Alert severity — defaults to the rule's stored severity but a
    /// rule may upgrade it based on the detected value (e.g. a
    /// >$100k single deposit).
    pub severity: &'static str,
    /// One-line description for the alerts table.
    pub summary: String,
    /// Structured evidence used by the admin UI / regulator export.
    pub details: Value,
    /// Lookback window the rule used.
    pub window_start: DateTime<Utc>,
    /// Window end — also the dedup key.
    pub window_end: DateTime<Utc>,
}

/// Run the worker forever. Cadence is read live from `platform_settings`
/// (`tx_monitoring_interval_minutes`) on each tick.
pub async fn run_monitoring_worker(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(
        WORKER_INTERVAL_MIN * 60,
    ));
    interval.tick().await;
    loop {
        interval.tick().await;
        match run_once(&pool).await {
            Ok(n) if n > 0 => tracing::info!(findings = n, "Transaction monitoring run produced findings"),
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Transaction monitoring worker error: {}", e);
                sentry::capture_message(
                    &format!("Transaction monitoring worker error: {}", e),
                    sentry::Level::Error,
                );
            }
        }
    }
}

/// Single monitoring pass. Returns the number of new findings emitted.
/// Public so admins can trigger it on demand.
pub async fn run_once(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let now = Utc::now();
    // Bucket the window end on the hour so two runs that fall within
    // the same hour share the dedup key.
    let window_end = floor_to_hour(now);

    let rules = load_enabled_rules(pool).await?;
    let mut findings_count: i64 = 0;

    for rule in rules {
        let findings = match rule.code.as_str() {
            "large_deposit" => rule_large_deposit(pool, &rule, window_end).await?,
            "rapid_deposits" => rule_rapid_deposits(pool, &rule, window_end).await?,
            "structuring_deposits" => rule_structuring(pool, &rule, window_end).await?,
            "withdraw_new_bank" => rule_withdraw_new_bank(pool, &rule, window_end).await?,
            "velocity_spike" => rule_velocity_spike(pool, &rule, window_end).await?,
            other => {
                tracing::warn!(rule = %other, "Unknown monitoring rule in DB, skipping");
                continue;
            }
        };

        for finding in findings {
            if persist_finding(pool, &finding).await {
                crate::metrics::SCREENING_RUNS_TOTAL
                    .with_label_values(&["hit"])
                    .inc();
                findings_count += 1;
            }
        }
    }

    Ok(findings_count)
}

#[derive(Debug, Clone)]
struct RuleRow {
    code: String,
    severity: String,
    config: Value,
}

async fn load_enabled_rules(pool: &PgPool) -> Result<Vec<RuleRow>, sqlx::Error> {
    let rows: Vec<(String, String, Value)> = sqlx::query_as(
        "SELECT code, severity, config FROM transaction_monitoring_rules WHERE enabled = TRUE",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(code, severity, config)| RuleRow {
            code,
            severity,
            config,
        })
        .collect())
}

fn floor_to_hour(t: DateTime<Utc>) -> DateTime<Utc> {
    t.date_naive()
        .and_hms_opt(t.time().hour(), 0, 0)
        .map(|nd| chrono::DateTime::<Utc>::from_naive_utc_and_offset(nd, Utc))
        .unwrap_or(t)
}

fn cfg_i64(rule: &RuleRow, key: &str, default: i64) -> i64 {
    rule.config
        .get(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(default)
}

fn rule_severity(rule: &RuleRow) -> &'static str {
    match rule.severity.as_str() {
        "critical" => "critical",
        "high" => "high",
        "low" => "low",
        _ => "medium",
    }
}

// ── Rule implementations ────────────────────────────────────────

/// Single deposit ≥ threshold (default $10k). Emits at most one finding
/// per (user, hour-window) — repeated large deposits trip
/// `rapid_deposits` separately.
async fn rule_large_deposit(
    pool: &PgPool,
    rule: &RuleRow,
    window_end: DateTime<Utc>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let threshold = cfg_i64(rule, "threshold_cents", 1_000_000);
    let window_start = window_end - chrono::Duration::hours(1);

    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        r#"
        SELECT d.user_id, MAX(d.amount_cents)
          FROM deposit_requests d
         WHERE d.created_at >= $1 AND d.created_at < $2
           AND d.amount_cents >= $3
         GROUP BY d.user_id
        "#,
    )
    .bind(window_start)
    .bind(window_end)
    .bind(threshold)
    .fetch_all(pool)
    .await?;

    let severity = rule_severity(rule);
    Ok(rows
        .into_iter()
        .map(|(user_id, amount)| Finding {
            user_id,
            rule_code: "large_deposit",
            // Critical when ≥10× the threshold (typically $100k+).
            severity: if amount >= threshold.saturating_mul(10) {
                "critical"
            } else {
                severity
            },
            summary: format!(
                "Single deposit of {} (threshold {})",
                format_usd(amount),
                format_usd(threshold)
            ),
            details: serde_json::json!({
                "max_amount_cents": amount,
                "threshold_cents": threshold,
            }),
            window_start,
            window_end,
        })
        .collect())
}

/// N+ deposits within `window_hours`. Catches deposit spam, automated
/// onboarding abuse, and brute-force structuring attempts.
async fn rule_rapid_deposits(
    pool: &PgPool,
    rule: &RuleRow,
    window_end: DateTime<Utc>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let count_threshold = cfg_i64(rule, "count_threshold", 5);
    let window_hours = cfg_i64(rule, "window_hours", 24);
    let window_start = window_end - chrono::Duration::hours(window_hours);

    let rows: Vec<(Uuid, i64, i64)> = sqlx::query_as(
        r#"
        SELECT user_id, COUNT(*)::bigint, COALESCE(SUM(amount_cents), 0)::bigint
          FROM deposit_requests
         WHERE created_at >= $1 AND created_at < $2
           AND status IN ('pending', 'requested', 'paid')
         GROUP BY user_id
         HAVING COUNT(*) >= $3
        "#,
    )
    .bind(window_start)
    .bind(window_end)
    .bind(count_threshold)
    .fetch_all(pool)
    .await?;

    let severity = rule_severity(rule);
    Ok(rows
        .into_iter()
        .map(|(user_id, count, total)| Finding {
            user_id,
            rule_code: "rapid_deposits",
            severity,
            summary: format!(
                "{} deposits totaling {} in the last {}h",
                count,
                format_usd(total),
                window_hours
            ),
            details: serde_json::json!({
                "deposit_count": count,
                "total_amount_cents": total,
                "window_hours": window_hours,
                "threshold_count": count_threshold,
            }),
            window_start,
            window_end,
        })
        .collect())
}

/// Classic structuring: 3+ deposits each in the [lower, upper] band
/// (default $8k-$10k) within 7 days. Each one is just below the CTR
/// threshold — telltale evasion pattern.
async fn rule_structuring(
    pool: &PgPool,
    rule: &RuleRow,
    window_end: DateTime<Utc>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let upper = cfg_i64(rule, "upper_cents", 1_000_000);
    let lower = cfg_i64(rule, "lower_cents", 800_000);
    let count_threshold = cfg_i64(rule, "count_threshold", 3);
    let window_hours = cfg_i64(rule, "window_hours", 168);
    let window_start = window_end - chrono::Duration::hours(window_hours);

    let rows: Vec<(Uuid, i64, i64)> = sqlx::query_as(
        r#"
        SELECT user_id, COUNT(*)::bigint, COALESCE(SUM(amount_cents), 0)::bigint
          FROM deposit_requests
         WHERE created_at >= $1 AND created_at < $2
           AND amount_cents >= $3 AND amount_cents < $4
           AND status IN ('pending', 'requested', 'paid')
         GROUP BY user_id
         HAVING COUNT(*) >= $5
        "#,
    )
    .bind(window_start)
    .bind(window_end)
    .bind(lower)
    .bind(upper)
    .bind(count_threshold)
    .fetch_all(pool)
    .await?;

    let severity = rule_severity(rule);
    Ok(rows
        .into_iter()
        .map(|(user_id, count, total)| Finding {
            user_id,
            rule_code: "structuring_deposits",
            severity,
            summary: format!(
                "{} deposits between {} and {} (total {}) — possible structuring",
                count,
                format_usd(lower),
                format_usd(upper),
                format_usd(total)
            ),
            details: serde_json::json!({
                "deposit_count": count,
                "lower_cents": lower,
                "upper_cents": upper,
                "total_amount_cents": total,
                "window_hours": window_hours,
            }),
            window_start,
            window_end,
        })
        .collect())
}

/// Withdrawal sent to a bank account added in the last `cooldown_hours`.
/// A common money-mule pattern is: deposit → add new bank → withdraw
/// before the platform notices.
async fn rule_withdraw_new_bank(
    pool: &PgPool,
    rule: &RuleRow,
    window_end: DateTime<Utc>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let cooldown_hours = cfg_i64(rule, "cooldown_hours", 168);
    let window_start = window_end - chrono::Duration::hours(1);

    let rows: Vec<(Uuid, Uuid, i64, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT w.user_id, w.id, w.amount_cents, w.created_at, pm.created_at
          FROM withdrawal_requests w
          JOIN payment_methods pm ON pm.id = w.payment_method_id
         WHERE w.created_at >= $1 AND w.created_at < $2
           AND pm.created_at > w.created_at - ($3::int || ' hours')::interval
        "#,
    )
    .bind(window_start)
    .bind(window_end)
    .bind(cooldown_hours as i32)
    .fetch_all(pool)
    .await?;

    let severity = rule_severity(rule);
    Ok(rows
        .into_iter()
        .map(|(user_id, wd_id, amount, wd_at, pm_at)| {
            let pm_age_hours = (wd_at - pm_at).num_hours();
            Finding {
                user_id,
                rule_code: "withdraw_new_bank",
                severity,
                summary: format!(
                    "Withdrawal of {} to a bank added {}h earlier",
                    format_usd(amount),
                    pm_age_hours
                ),
                details: serde_json::json!({
                    "withdrawal_id": wd_id.to_string(),
                    "amount_cents": amount,
                    "payment_method_age_hours": pm_age_hours,
                    "cooldown_hours_setting": cooldown_hours,
                }),
                window_start,
                window_end,
            }
        })
        .collect())
}

/// 24h activity exceeds `multiplier` × the 30-day daily average. Only
/// emits when the baseline itself is non-trivial (`min_baseline_cents`)
/// so a user's first deposit doesn't immediately fire.
async fn rule_velocity_spike(
    pool: &PgPool,
    rule: &RuleRow,
    window_end: DateTime<Utc>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let multiplier = cfg_i64(rule, "multiplier", 10);
    let min_baseline = cfg_i64(rule, "min_baseline_cents", 50_000);
    let window_start = window_end - chrono::Duration::hours(24);

    let rows: Vec<(Uuid, i64, i64)> = sqlx::query_as(
        r#"
        WITH recent AS (
            SELECT user_id, COALESCE(SUM(amount_cents), 0)::bigint AS recent_cents
              FROM deposit_requests
             WHERE created_at >= $1 AND created_at < $2
               AND status IN ('pending', 'requested', 'paid')
             GROUP BY user_id
        ),
        baseline AS (
            SELECT user_id,
                   GREATEST(
                       1,
                       (COALESCE(SUM(amount_cents), 0)::bigint / 30)
                   ) AS daily_avg_cents
              FROM deposit_requests
             WHERE created_at >= $2 - INTERVAL '30 days'
               AND created_at < $2
               AND status IN ('pending', 'requested', 'paid')
             GROUP BY user_id
        )
        SELECT r.user_id, r.recent_cents, b.daily_avg_cents
          FROM recent r
          JOIN baseline b ON b.user_id = r.user_id
         WHERE b.daily_avg_cents >= $3
           AND r.recent_cents >= $4 * b.daily_avg_cents
        "#,
    )
    .bind(window_start)
    .bind(window_end)
    .bind(min_baseline)
    .bind(multiplier)
    .fetch_all(pool)
    .await?;

    let severity = rule_severity(rule);
    Ok(rows
        .into_iter()
        .map(|(user_id, recent, baseline)| Finding {
            user_id,
            rule_code: "velocity_spike",
            severity,
            summary: format!(
                "{} deposited in 24h — {}× the 30-day daily average ({})",
                format_usd(recent),
                if baseline > 0 { recent / baseline } else { 0 },
                format_usd(baseline)
            ),
            details: serde_json::json!({
                "recent_24h_cents": recent,
                "daily_avg_cents": baseline,
                "multiplier": multiplier,
            }),
            window_start,
            window_end,
        })
        .collect())
}

// ── Persistence ─────────────────────────────────────────────────

/// Insert the finding row + open a compliance_alerts entry. Returns
/// true when a new row was created (false on dedup hit).
async fn persist_finding(pool: &PgPool, finding: &Finding) -> bool {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("monitoring tx begin failed: {}", e);
            return false;
        }
    };

    let alert_id: Option<Uuid> = sqlx::query_scalar(
        r#"INSERT INTO compliance_alerts (user_id, kind, severity, summary, details)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(finding.user_id)
    .bind(alert_kind(finding.rule_code))
    .bind(finding.severity)
    .bind(&finding.summary)
    .bind(&finding.details)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();

    let inserted: bool = match sqlx::query(
        r#"INSERT INTO transaction_monitoring_findings
                (user_id, rule_code, severity, summary, details, alert_id,
                 window_start, window_end)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (user_id, rule_code, window_end) DO NOTHING"#,
    )
    .bind(finding.user_id)
    .bind(finding.rule_code)
    .bind(finding.severity)
    .bind(&finding.summary)
    .bind(&finding.details)
    .bind(alert_id)
    .bind(finding.window_start)
    .bind(finding.window_end)
    .execute(&mut *tx)
    .await
    {
        Ok(res) => res.rows_affected() == 1,
        Err(e) => {
            tracing::error!("monitoring finding insert failed: {}", e);
            let _ = tx.rollback().await;
            return false;
        }
    };

    // If the finding was a duplicate, undo the alert insert too.
    if !inserted {
        let _ = tx.rollback().await;
        return false;
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("monitoring tx commit failed: {}", e);
        return false;
    }
    true
}

fn alert_kind(rule_code: &str) -> &'static str {
    match rule_code {
        "structuring_deposits" => "structuring",
        "velocity_spike" | "rapid_deposits" => "velocity_anomaly",
        _ => "manual_review",
    }
}

fn format_usd(cents: i64) -> String {
    crate::common::currency::format_usd(cents)
}
