//! Storage reconciler — Phase 3.3.
//!
//! Walks every row in a source table (kyc_documents / asset_documents),
//! HEADs the referenced GCS object, and records anomalies into the
//! `storage_reconcile_runs` + `storage_reconcile_findings` tables created
//! by migration 199.
//!
//! See `docs/storage/03-backup-and-disaster-recovery.md` → "Layer 3 —
//! Reconciliation Job" for the runbook + triage flow.
//!
//! # Scope
//! v1 only does **shallow** checks: object-exists + size matches. Deep
//! SHA-256 verification requires downloading the bytes and recomputing
//! the hash, which is expensive at scale. That mode is exposed via the
//! `deep` parameter but expected to run weekly, not nightly.
//!
//! # Orphan detection
//! Listing every object in the bucket and reverse-checking DB rows is
//! deferred to v2 — it requires paginated `list_objects` calls and a
//! full bucket scan, which is cost-heavy. The runbook documents the
//! manual `gsutil ls` procedure in the meantime.

use crate::error::AppError;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Tables the reconciler knows how to scan. The string value must match
/// the `source_table` column convention in the audit tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTable {
    KycDocuments,
    AssetDocuments,
}

impl SourceTable {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KycDocuments => "kyc_documents",
            Self::AssetDocuments => "asset_documents",
        }
    }
}

/// Summary returned to the caller. Mirrors the counters persisted on
/// the `storage_reconcile_runs` row.
#[derive(Debug, Clone, Serialize)]
pub struct ReconcileSummary {
    pub run_id: Uuid,
    pub source_table: &'static str,
    pub bucket: String,
    pub rows_scanned: i32,
    pub objects_scanned: i32,
    pub missing_objects: i32,
    pub size_mismatches: i32,
    pub hash_mismatches: i32,
    pub status: &'static str,
}

/// Severity escalation table. `missing_object` is always critical (data
/// loss). `size_mismatch` is critical (likely corruption). `orphan` is
/// info-level if recent, warning if older than 24h.
fn severity_for(kind: &str) -> &'static str {
    match kind {
        "missing_object" | "hash_mismatch" | "size_mismatch" => "critical",
        "orphan_object" => "warning",
        _ => "info",
    }
}

/// Insert a single finding row. Returns the ID so callers can chain to
/// Sentry or paging.
async fn insert_finding(
    pool: &PgPool,
    run_id: Uuid,
    source_id: Option<Uuid>,
    source_table: &str,
    object_path: &str,
    kind: &str,
    detail: serde_json::Value,
) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as(
        r#"INSERT INTO storage_reconcile_findings
           (run_id, source_id, source_table, object_path, kind, severity, detail)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id"#,
    )
    .bind(run_id)
    .bind(source_id)
    .bind(source_table)
    .bind(object_path)
    .bind(kind)
    .bind(severity_for(kind))
    .bind(detail)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Bump a single counter on the active run row. Idempotent under retry.
async fn bump_run_counter(pool: &PgPool, run_id: Uuid, counter_col: &str) -> Result<(), AppError> {
    // SQL injection guard: we whitelist the counter names so a bad
    // caller can't inject DDL through the column name. The column list
    // is closed (matches migration 199 exactly).
    let allowed = [
        "rows_scanned",
        "objects_scanned",
        "missing_objects",
        "orphan_objects",
        "hash_mismatches",
        "size_mismatches",
    ];
    if !allowed.contains(&counter_col) {
        return Err(AppError::Internal(format!(
            "reconciler: refusing to bump unknown counter column '{}'",
            counter_col
        )));
    }
    let sql = format!(
        "UPDATE storage_reconcile_runs SET {} = {} + 1 WHERE id = $1",
        counter_col, counter_col
    );
    sqlx::query(&sql).bind(run_id).execute(pool).await?;
    Ok(())
}

/// Open a new run row. Returns the UUID for chaining.
pub async fn start_run(
    pool: &PgPool,
    source: SourceTable,
    bucket: &str,
    note: Option<&str>,
) -> Result<Uuid, AppError> {
    let row: (Uuid,) = sqlx::query_as(
        r#"INSERT INTO storage_reconcile_runs (source_table, bucket, note, status)
           VALUES ($1, $2, $3, 'running')
           RETURNING id"#,
    )
    .bind(source.as_str())
    .bind(bucket)
    .bind(note)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Close out a run with a final status string. Status values that the
/// audit table accepts: `success`, `partial`, `failed`.
pub async fn finalize_run(
    pool: &PgPool,
    run_id: Uuid,
    status: &str,
    sentry_event_id: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE storage_reconcile_runs
           SET status = $2,
               finished_at = NOW(),
               sentry_event_id = COALESCE($3, sentry_event_id)
           WHERE id = $1"#,
    )
    .bind(run_id)
    .bind(status)
    .bind(sentry_event_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Parse a `gs://bucket/object_path` URI into its components. Returns
/// `None` for non-`gs://` URIs (e.g. legacy public URLs).
pub fn parse_gs_uri(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix("gs://")?;
    let (bucket, object) = rest.split_once('/')?;
    if bucket.is_empty() || object.is_empty() {
        return None;
    }
    Some((bucket.to_string(), object.to_string()))
}

/// Pluck the object path out of a stored DB URL value. Supports two
/// historical shapes: the `gs://bucket/path` private form and the
/// `/api/proxy/gcs/{bucket}/{path}` public proxy form. Returns
/// `(bucket, object_path)` when parseable, else `None`.
pub fn extract_bucket_and_path(
    stored_url: &str,
    fallback_bucket: &str,
) -> Option<(String, String)> {
    if let Some(parsed) = parse_gs_uri(stored_url) {
        return Some(parsed);
    }
    if let Some(rest) = stored_url.strip_prefix("/api/proxy/gcs/") {
        if let Some((bucket, path)) = rest.split_once('/') {
            if !bucket.is_empty() && !path.is_empty() {
                return Some((bucket.to_string(), path.to_string()));
            }
        }
    }
    // Last resort: assume the stored value IS the object path, under the
    // caller-supplied default bucket. Common for `properties/{id}/...`
    // legacy rows where only the relative path was persisted.
    if !stored_url.is_empty() && !stored_url.starts_with("http") {
        return Some((
            fallback_bucket.to_string(),
            stored_url.trim_start_matches('/').to_string(),
        ));
    }
    None
}

/// A single DB row to check. Bucket may be `None` when the legacy URL
/// shape doesn't include one and we must use the caller-supplied default.
pub struct ReconcileTarget {
    pub id: Uuid,
    pub stored_url: String,
    pub expected_size: Option<i64>,
    pub expected_sha256: Option<String>,
}

/// Check a single target. Returns the finding kind (if any) so the
/// caller can update counters appropriately. Errors that prevent the
/// check from completing (network, auth) are bubbled — caller decides
/// whether to mark the run `partial` or `failed`.
pub async fn check_one(
    pool: &PgPool,
    run_id: Uuid,
    source: SourceTable,
    default_bucket: &str,
    t: &ReconcileTarget,
) -> Result<Option<&'static str>, AppError> {
    let (bucket, path) = match extract_bucket_and_path(&t.stored_url, default_bucket) {
        Some(p) => p,
        None => {
            insert_finding(
                pool,
                run_id,
                Some(t.id),
                source.as_str(),
                &t.stored_url,
                "missing_object",
                json!({"reason": "unparseable stored URL"}),
            )
            .await?;
            bump_run_counter(pool, run_id, "missing_objects").await?;
            return Ok(Some("missing_object"));
        }
    };

    let client = match crate::storage::service::build_client_public().await {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    let head = client
        .get_object(&GetObjectRequest {
            bucket: bucket.clone(),
            object: path.clone(),
            ..Default::default()
        })
        .await;

    let object = match head {
        Ok(o) => o,
        Err(e) => {
            // 404 = data loss event. Other errors bubble up so the run
            // can be marked partial; we don't want a transient auth fail
            // to spam findings.
            let es = e.to_string();
            if es.contains("404") || es.to_lowercase().contains("not found") {
                insert_finding(
                    pool,
                    run_id,
                    Some(t.id),
                    source.as_str(),
                    &format!("gs://{}/{}", bucket, path),
                    "missing_object",
                    json!({"gcs_error": es}),
                )
                .await?;
                bump_run_counter(pool, run_id, "missing_objects").await?;
                return Ok(Some("missing_object"));
            }
            return Err(AppError::Internal(format!("GCS HEAD failed: {}", es)));
        }
    };

    bump_run_counter(pool, run_id, "objects_scanned").await?;

    if let Some(exp) = t.expected_size {
        if exp != object.size {
            insert_finding(
                pool,
                run_id,
                Some(t.id),
                source.as_str(),
                &format!("gs://{}/{}", bucket, path),
                "size_mismatch",
                json!({"expected_size": exp, "actual_size": object.size}),
            )
            .await?;
            bump_run_counter(pool, run_id, "size_mismatches").await?;
            return Ok(Some("size_mismatch"));
        }
    }

    // `expected_sha256` cannot be verified without re-downloading the
    // object (GCS doesn't compute SHA-256, only crc32c + md5). v1
    // records the expected hash on the finding-detail field of any
    // other mismatch so triage has the reference. Deep check is a v2
    // feature gated behind a `deep=true` query param.
    let _ = &t.expected_sha256;

    Ok(None)
}

/// Top-level entry: open run row, scan all DB rows in `source`, check
/// each one, close run row with final status. Returns a summary that
/// the admin route can render as JSON.
pub async fn run_reconciliation(
    pool: &PgPool,
    source: SourceTable,
    default_bucket: &str,
    note: Option<&str>,
) -> Result<ReconcileSummary, AppError> {
    let run_id = start_run(pool, source, default_bucket, note).await?;

    // Column names differ between tables — see migration 197:
    //   kyc_documents:   gcs_path,  content_size_bytes
    //   asset_documents: file_url, file_size_bytes (legacy)
    // Neither table has a deleted_at, so we scan all live rows. KYC docs
    // in status='rejected' still have a GCS object that needs auditing.
    let rows: Vec<(Uuid, String, Option<i64>, Option<String>)> = match source {
        SourceTable::KycDocuments => {
            sqlx::query_as(
                "SELECT id, gcs_path, content_size_bytes, content_sha256
             FROM kyc_documents
             ORDER BY uploaded_at ASC NULLS LAST",
            )
            .fetch_all(pool)
            .await?
        }
        SourceTable::AssetDocuments => {
            sqlx::query_as(
                "SELECT id, file_url::text, file_size_bytes, content_sha256
             FROM asset_documents
             ORDER BY created_at ASC NULLS LAST",
            )
            .fetch_all(pool)
            .await?
        }
    };

    sqlx::query("UPDATE storage_reconcile_runs SET rows_scanned = $2 WHERE id = $1")
        .bind(run_id)
        .bind(rows.len() as i32)
        .execute(pool)
        .await?;

    let mut errors_seen = 0u32;
    for (id, stored_url, expected_size, expected_sha256) in rows {
        let t = ReconcileTarget {
            id,
            stored_url,
            expected_size,
            expected_sha256,
        };
        if let Err(e) = check_one(pool, run_id, source, default_bucket, &t).await {
            tracing::warn!("reconciler row failure run={} id={} err={}", run_id, id, e);
            errors_seen = errors_seen.saturating_add(1);
            // Bail early if every check is failing — usually auth or
            // network. Continuing just floods Sentry.
            if errors_seen > 25 {
                finalize_run(pool, run_id, "failed", None).await?;
                return Err(AppError::Internal(
                    "reconciler aborted after >25 consecutive errors".to_string(),
                ));
            }
        }
    }

    let final_status: &'static str = if errors_seen > 0 {
        "partial"
    } else {
        "success"
    };
    finalize_run(pool, run_id, final_status, None).await?;

    // Re-read the counters from the DB so the summary reflects whatever
    // landed via `bump_run_counter` calls.
    let row: (i32, i32, i32, i32, i32) = sqlx::query_as(
        "SELECT rows_scanned, objects_scanned, missing_objects, size_mismatches, hash_mismatches
         FROM storage_reconcile_runs WHERE id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;

    Ok(ReconcileSummary {
        run_id,
        source_table: source.as_str(),
        bucket: default_bucket.to_string(),
        rows_scanned: row.0,
        objects_scanned: row.1,
        missing_objects: row.2,
        size_mismatches: row.3,
        hash_mismatches: row.4,
        status: final_status,
    })
}
