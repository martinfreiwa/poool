//! KYC retention worker — Phase 4.1 (GwG §8 + DSGVO Art. 5(1)(e)).
//!
//! Periodic job that:
//!   1. Finds `kyc_documents` whose `retention_until` is in the past.
//!   2. Deletes the underlying GCS object.
//!   3. Soft-deletes the DB row (`deleted_at`, `deletion_reason`).
//!   4. Logs the run to `kyc_retention_runs` for the BSI C5 ORG-08
//!      attestation evidence.
//!
//! The `arm_kyc_retention_for_user(uuid, integer)` SQL function (added
//! in migration 200) is the *trigger* — it is called from the DSGVO
//! user-delete handler or the admin off-boarding flow to set
//! `users.business_relationship_ended_at` and compute the per-document
//! `retention_until`.
//!
//! See `docs/storage/04-compliance-and-retention.md` for the runbook.
//!
//! # Dry-run mode
//! Real production deletes are destructive. `run_retention_worker(.., true)`
//! returns the same summary without touching either GCS or the DB —
//! intended for canaries + the daily ops dashboard preview. Real
//! deletes go via `run_retention_worker(.., false)`, gated behind an
//! admin token in the route layer.

use crate::error::AppError;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Outcome of a single retention-worker invocation.
#[derive(Debug, Clone, Serialize)]
pub struct RetentionSummary {
    /// Run-row UUID for cross-referencing logs.
    pub run_id: Uuid,
    /// Total `kyc_documents` rows the worker considered (includes
    /// active rows whose retention_until is still in the future).
    pub rows_considered: i32,
    /// Subset that were past their retention deadline.
    pub rows_due: i32,
    /// Successfully soft-deleted in the DB (GCS delete may have failed
    /// — see `gcs_deletes_failed`; the audit trail is still preserved).
    pub rows_deleted: i32,
    /// GCS objects deleted successfully.
    pub gcs_deletes_ok: i32,
    /// GCS object deletes that failed (auth, 404, network). The DB row
    /// is still soft-deleted; an operator must follow up via the
    /// reconciler to clean up.
    pub gcs_deletes_failed: i32,
    /// `success` | `partial` | `failed`.
    pub status: &'static str,
    /// True when the worker was invoked in observation-only mode.
    pub dry_run: bool,
}

/// Open a run row, return its id.
async fn start_run(pool: &PgPool, dry_run: bool, note: Option<&str>) -> Result<Uuid, AppError> {
    let row: (Uuid,) = sqlx::query_as(
        r#"INSERT INTO kyc_retention_runs (status, dry_run, note)
           VALUES ('running', $1, $2)
           RETURNING id"#,
    )
    .bind(dry_run)
    .bind(note)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

async fn finalize_run(
    pool: &PgPool,
    run_id: Uuid,
    status: &str,
    summary: &RetentionSummary,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE kyc_retention_runs
           SET status = $2,
               finished_at = NOW(),
               rows_considered = $3,
               rows_due = $4,
               rows_deleted = $5,
               gcs_deletes_ok = $6,
               gcs_deletes_failed = $7
           WHERE id = $1"#,
    )
    .bind(run_id)
    .bind(status)
    .bind(summary.rows_considered)
    .bind(summary.rows_due)
    .bind(summary.rows_deleted)
    .bind(summary.gcs_deletes_ok)
    .bind(summary.gcs_deletes_failed)
    .execute(pool)
    .await?;
    Ok(())
}

/// One row queued for deletion by the worker.
struct DueRow {
    id: Uuid,
    gcs_path: String,
}

/// Classify an `AppError` for the GCS error-kind label. Bounded set
/// keeps Prometheus cardinality predictable.
fn kind_for_gcs_error(e: &AppError) -> &'static str {
    let s = e.to_string().to_lowercase();
    if s.contains("auth") {
        "auth"
    } else if s.contains("404") || s.contains("not found") {
        "not_found"
    } else if s.contains("timeout") {
        "timeout"
    } else {
        "other"
    }
}

/// Hard-delete the GCS object backing a `kyc_documents` row. Returns
/// `Ok(true)` on success, `Ok(false)` on a benign 404 (object already
/// gone — still counts as success for the worker), or `Err` on a real
/// failure. Default-bucket fallback mirrors the reconciler's parser.
async fn delete_gcs_object(default_bucket: &str, stored_url: &str) -> Result<bool, AppError> {
    let (bucket, path) =
        crate::storage::reconciler::extract_bucket_and_path(stored_url, default_bucket)
            .ok_or_else(|| AppError::Internal(format!("unparseable gcs_path: {}", stored_url)))?;

    match crate::storage::service::delete_object(&bucket, &path).await {
        Ok(()) => Ok(true),
        Err(AppError::NotFound(_)) => Ok(false),
        Err(e) => {
            let es = e.to_string();
            if es.contains("404") || es.to_lowercase().contains("not found") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

/// Top-level entrypoint. Scans, optionally deletes, returns a summary.
/// When `dry_run=true`, neither the GCS object nor the DB row is
/// touched — only the count of due rows is recorded.
pub async fn run_retention_worker(
    pool: &PgPool,
    default_bucket: &str,
    dry_run: bool,
    note: Option<&str>,
) -> Result<RetentionSummary, AppError> {
    let run_id = start_run(pool, dry_run, note).await?;

    // Total rows the worker is responsible for (active + due).
    let (rows_considered,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM kyc_documents WHERE retention_until IS NOT NULL AND deleted_at IS NULL",
    )
    .fetch_one(pool)
    .await?;

    let due_rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, gcs_path FROM kyc_documents
         WHERE retention_until IS NOT NULL
           AND retention_until <= NOW()
           AND deleted_at IS NULL
         ORDER BY retention_until ASC
         LIMIT 1000",
    )
    .fetch_all(pool)
    .await?;

    let rows_due = due_rows.len() as i32;
    let mut rows_deleted = 0i32;
    let mut gcs_deletes_ok = 0i32;
    let mut gcs_deletes_failed = 0i32;

    if !dry_run {
        for (id, gcs_path) in due_rows.iter().map(|(id, p)| (id, p.as_str())) {
            let row = DueRow {
                id: *id,
                gcs_path: gcs_path.to_string(),
            };

            // GCS delete first — if GCS fails the row stays alive so we
            // can retry on the next run instead of orphaning a real
            // file that wasn't actually deleted.
            match delete_gcs_object(default_bucket, &row.gcs_path).await {
                Ok(_) => {
                    gcs_deletes_ok = gcs_deletes_ok.saturating_add(1);
                }
                Err(e) => {
                    tracing::warn!(
                        "retention: GCS delete failed run={} doc={} err={}",
                        run_id,
                        row.id,
                        e
                    );
                    crate::metrics::record_storage_gcs_error(
                        "retention.delete",
                        kind_for_gcs_error(&e),
                    );
                    gcs_deletes_failed = gcs_deletes_failed.saturating_add(1);
                    // Skip the DB soft-delete on GCS failure. The row
                    // will be retried on the next run.
                    continue;
                }
            }

            // DB soft-delete.
            let res = sqlx::query(
                r#"UPDATE kyc_documents
                   SET deleted_at = NOW(),
                       deletion_reason = 'gwg_retention_expired'
                   WHERE id = $1 AND deleted_at IS NULL"#,
            )
            .bind(row.id)
            .execute(pool)
            .await;

            match res {
                Ok(_) => {
                    rows_deleted = rows_deleted.saturating_add(1);
                }
                Err(e) => {
                    tracing::warn!(
                        "retention: DB soft-delete failed run={} doc={} err={}",
                        run_id,
                        row.id,
                        e
                    );
                    gcs_deletes_failed = gcs_deletes_failed.saturating_add(1);
                }
            }
        }
    }

    let status: &'static str = if gcs_deletes_failed > 0 {
        "partial"
    } else {
        "success"
    };

    // Phase 5 observability: each failed GCS delete already incremented
    // `storage_gcs_errors_total{op="retention.delete"}` via the loop
    // above (see `delete_gcs_object` error path). The run-row counters
    // are surfaced as gauges by `refresh_storage_gauges`.

    let summary = RetentionSummary {
        run_id,
        rows_considered: rows_considered as i32,
        rows_due,
        rows_deleted,
        gcs_deletes_ok,
        gcs_deletes_failed,
        status,
        dry_run,
    };
    finalize_run(pool, run_id, status, &summary).await?;
    Ok(summary)
}

/// Convenience wrapper: arm the retention clock for a single user. Used
/// by the DSGVO user-delete handler + the admin off-boarding flow.
/// Returns the number of KYC documents that got their `retention_until`
/// freshly computed by this call.
pub async fn arm_retention_for_user(
    pool: &PgPool,
    user_id: Uuid,
    retention_years: i32,
) -> Result<i32, AppError> {
    let (n,): (i32,) = sqlx::query_as("SELECT arm_kyc_retention_for_user($1, $2)")
        .bind(user_id)
        .bind(retention_years)
        .fetch_one(pool)
        .await?;
    Ok(n)
}
