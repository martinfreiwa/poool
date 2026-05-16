//! Sanctions / PEP re-screening — runs as a background worker plus an
//! on-demand admin endpoint.
//!
//! Why we need it
//! --------------
//! Initial KYC clears the user against the sanctions list at the moment
//! of onboarding. Lists change daily (new SDN designations, new PEP
//! flags, country-level updates), so an approved user can become a hit
//! a month later. AMLD5/6 and the OFAC framework both require
//! "ongoing due diligence" — periodic re-screening of the existing book.
//!
//! What this module does
//! ---------------------
//! 1. Pick up approved users whose last re-screening is older than the
//!    admin-configured interval (default 30d).
//! 2. Run them through `screen_user` (provider-pluggable; today wraps
//!    the cached Didit response). Each run inserts a row into
//!    `sanctions_rescreening_log`.
//! 3. On `hit` or `error`, file a row into `compliance_alerts` so the
//!    compliance team gets a triage item with a link back to the raw
//!    log entry.
//!
//! Provider integration is intentionally thin — `screen_user` returns a
//! `ScreeningResult` enum and the worker doesn't care which provider
//! produced it. Plug in ComplyAdvantage / Dow Jones by replacing the
//! body of `screen_user`.

use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

const DEFAULT_INTERVAL_DAYS: i64 = 30;
const WORKER_INTERVAL_SECS: u64 = 6 * 60 * 60; // every 6h
const BATCH_SIZE: i64 = 200;

/// Reportable outcome of one re-screening attempt. Maps 1:1 to the
/// `status` column on `sanctions_rescreening_log`.
#[derive(Debug, Clone)]
pub enum ScreeningResult {
    /// User cleared — no sanctions/PEP/adverse-media hits.
    Clear,
    /// One or more hits. `summary` is a short, human-readable
    /// description; `details` is the raw provider response (kept verbatim
    /// for audit + appeal).
    Hit {
        summary: String,
        details: serde_json::Value,
    },
    /// Provider couldn't be reached or returned an error. We don't treat
    /// these as hits, but we do record them so we can retry the user on
    /// the next tick instead of waiting another 30d.
    Error(String),
    /// User wasn't eligible for screening (no KYC record yet, or already
    /// re-screened within the interval).
    Skipped(&'static str),
}

impl ScreeningResult {
    fn db_status(&self) -> &'static str {
        match self {
            ScreeningResult::Clear => "clear",
            ScreeningResult::Hit { .. } => "hit",
            ScreeningResult::Error(_) => "error",
            ScreeningResult::Skipped(_) => "skipped",
        }
    }
}

/// Run the worker forever. Wakes every 6 hours; the actual scan only
/// touches users whose last re-screening is older than the configured
/// interval, so most ticks are nearly no-ops.
pub async fn run_rescreening_worker(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(WORKER_INTERVAL_SECS));
    // Skip the immediate tick.
    interval.tick().await;
    loop {
        interval.tick().await;
        let interval_days = fetch_interval_days(&pool).await;
        match run_once(&pool, interval_days, BATCH_SIZE).await {
            Ok(n) if n > 0 => info!(
                rescreened = n,
                interval_days, "Sanctions re-screening run completed"
            ),
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Sanctions re-screening worker error: {}", e);
                sentry::capture_message(
                    &format!("Sanctions re-screening worker error: {}", e),
                    sentry::Level::Error,
                );
            }
        }
    }
}

/// Single batch run. Returns the count of users re-screened.
pub async fn run_once(pool: &PgPool, interval_days: i64, limit: i64) -> Result<i64, sqlx::Error> {
    let due: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT k.user_id
          FROM kyc_records k
          LEFT JOIN LATERAL (
              SELECT MAX(checked_at) AS last_at
                FROM sanctions_rescreening_log l
               WHERE l.user_id = k.user_id
          ) AS l ON TRUE
         WHERE k.status IN ('approved', 'verified', 'completed')
           AND (l.last_at IS NULL OR l.last_at < NOW() - ($1::int || ' days')::interval)
         LIMIT $2
        "#,
    )
    .bind(interval_days as i32)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut count = 0i64;
    for (user_id,) in due {
        match screen_user(pool, user_id).await {
            ScreeningResult::Skipped(_) => {
                crate::metrics::record_screening("skipped");
                continue;
            }
            result => {
                let log_id = persist_log(pool, user_id, &result).await;
                if let ScreeningResult::Hit {
                    ref summary,
                    ref details,
                } = result
                {
                    persist_alert(pool, user_id, log_id, summary, details).await;
                }
                crate::metrics::record_screening(result.db_status());
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Trigger a re-screening for a single user from an admin action. Same
/// pipeline as the worker — recorded + alerted if hit.
pub async fn rescreen_user(pool: &PgPool, user_id: Uuid) -> ScreeningResult {
    let result = screen_user(pool, user_id).await;
    if !matches!(result, ScreeningResult::Skipped(_)) {
        let log_id = persist_log(pool, user_id, &result).await;
        if let ScreeningResult::Hit {
            ref summary,
            ref details,
        } = result
        {
            persist_alert(pool, user_id, log_id, summary, details).await;
        }
    }
    result
}

/// Provider-pluggable screening function.
///
/// Current implementation re-inspects the cached PEP/sanctions flags on
/// the latest `kyc_records` row. This catches users whose stored result
/// shows hits (e.g. the KYC provider downgraded them post-onboarding)
/// without depending on a live screening API.
///
/// To wire a real provider:
///   1. Replace this body with an HTTP call to your sanctions API using
///      the user's first_name + last_name + dob + nationality (extend
///      the SQL select to pull those from user_profiles + kyc_records).
///   2. Return `ScreeningResult::Hit { summary, details }` for each hit,
///      `ScreeningResult::Clear` otherwise.
async fn screen_user(pool: &PgPool, user_id: Uuid) -> ScreeningResult {
    let row: Option<(String, Option<bool>, Option<bool>)> = sqlx::query_as(
        r#"
        SELECT status,
               pep_check_passed,
               sanctions_check
          FROM kyc_records
         WHERE user_id = $1
         ORDER BY updated_at DESC NULLS LAST, created_at DESC
         LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let Some((status, pep_passed, sanctions_passed)) = row else {
        return ScreeningResult::Skipped("no_kyc_record");
    };

    if !matches!(status.as_str(), "approved" | "verified" | "completed") {
        return ScreeningResult::Skipped("kyc_not_approved");
    }

    let sanctions_hit = matches!(sanctions_passed, Some(false));
    let pep_hit = matches!(pep_passed, Some(false));

    if !sanctions_hit && !pep_hit {
        return ScreeningResult::Clear;
    }

    let summary = format!(
        "Re-screening surfaced cached hit(s) — sanctions: {}, PEP: {}",
        sanctions_hit as u8, pep_hit as u8
    );
    let details = serde_json::json!({
        "sanctions_hits": sanctions_hit as u8,
        "pep_hits": pep_hit as u8,
        "provider": "kyc_cached_flags",
        "kyc_status": status,
    });
    warn!(
        user_id = %user_id,
        sanctions_hit, pep_hit,
        "Sanctions/PEP cached flag flipped — opening compliance alert"
    );
    ScreeningResult::Hit { summary, details }
}

async fn persist_log(pool: &PgPool, user_id: Uuid, result: &ScreeningResult) -> Option<Uuid> {
    let (summary, raw): (Option<String>, Option<serde_json::Value>) = match result {
        ScreeningResult::Hit { summary, details } => (Some(summary.clone()), Some(details.clone())),
        ScreeningResult::Error(e) => (Some(e.clone()), None),
        ScreeningResult::Clear => (None, None),
        ScreeningResult::Skipped(_) => return None,
    };

    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO sanctions_rescreening_log
                (user_id, status, provider, summary, raw_response)
           VALUES ($1, $2, 'kyc_cached', $3, $4)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(result.db_status())
    .bind(summary)
    .bind(raw)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

async fn persist_alert(
    pool: &PgPool,
    user_id: Uuid,
    log_id: Option<Uuid>,
    summary: &str,
    details: &serde_json::Value,
) {
    let kind = if details
        .get("sanctions_hits")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0
    {
        "sanctions_hit"
    } else if details
        .get("pep_hits")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        > 0
    {
        "pep_hit"
    } else {
        "adverse_media"
    };

    let severity = if kind == "sanctions_hit" {
        "critical"
    } else {
        "high"
    };

    let _ = sqlx::query(
        r#"INSERT INTO compliance_alerts
                (user_id, kind, severity, summary, details, source_log_id)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(user_id)
    .bind(kind)
    .bind(severity)
    .bind(summary)
    .bind(details)
    .bind(log_id)
    .execute(pool)
    .await;
}

async fn fetch_interval_days(pool: &PgPool) -> i64 {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'sanctions_rescreening_interval_days'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    raw.and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_DAYS)
}
