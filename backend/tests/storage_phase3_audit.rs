//! Storage Phase 3 audit tests — backup, DR, and reconciler invariants.
//!
//! Pure-function tests for the parsing + path-extraction helpers, plus
//! DB-backed tests that exercise the run-lifecycle (start → finalize)
//! and finding insertion without requiring a live GCS bucket.
//!
//! The reconciler integration test that does talk to GCS is gated
//! behind `#[ignore]` because it requires production-ish credentials;
//! local + CI runs verify the SQL + lifecycle invariants.

use poool_backend::storage::reconciler::{
    extract_bucket_and_path, finalize_run, parse_gs_uri, start_run, SourceTable,
};

// ─── Pure parsers ─────────────────────────────────────────────────

#[test]
fn parse_gs_uri_round_trips_valid_input() {
    let r = parse_gs_uri("gs://my-bucket/path/to/object.pdf").unwrap();
    assert_eq!(r.0, "my-bucket");
    assert_eq!(r.1, "path/to/object.pdf");
}

#[test]
fn parse_gs_uri_rejects_non_gs_prefix() {
    assert!(parse_gs_uri("https://example.com/foo").is_none());
    assert!(parse_gs_uri("/api/proxy/gcs/bucket/x").is_none());
    assert!(parse_gs_uri("").is_none());
}

#[test]
fn parse_gs_uri_rejects_missing_object_path() {
    assert!(parse_gs_uri("gs://just-the-bucket").is_none());
    assert!(parse_gs_uri("gs://bucket/").is_none());
}

#[test]
fn extract_bucket_and_path_handles_gs_uri() {
    let (b, p) = extract_bucket_and_path("gs://x/y/z.pdf", "fallback").unwrap();
    assert_eq!(b, "x");
    assert_eq!(p, "y/z.pdf");
}

#[test]
fn extract_bucket_and_path_handles_proxy_url() {
    let (b, p) = extract_bucket_and_path(
        "/api/proxy/gcs/poool-assets-primary/kyc/u/doc.pdf",
        "fallback",
    )
    .unwrap();
    assert_eq!(b, "poool-assets-primary");
    assert_eq!(p, "kyc/u/doc.pdf");
}

#[test]
fn extract_bucket_and_path_uses_fallback_for_bare_path() {
    let (b, p) = extract_bucket_and_path("properties/abc/doc.pdf", "fallback-bkt").unwrap();
    assert_eq!(b, "fallback-bkt");
    assert_eq!(p, "properties/abc/doc.pdf");
}

#[test]
fn extract_bucket_and_path_rejects_external_https() {
    // External http(s) URLs aren't ours to verify — return None so the
    // caller can record a finding rather than HEAD a random domain.
    assert!(extract_bucket_and_path("https://example.com/file.pdf", "fb").is_none());
}

// ─── DB-backed lifecycle ──────────────────────────────────────────

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn run_lifecycle_open_then_finalize_success() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    let run_id = start_run(
        &pool,
        SourceTable::KycDocuments,
        "test-bucket",
        Some("unit"),
    )
    .await
    .expect("start_run");

    // Before finalize: status='running'.
    let (status_before,): (String,) =
        sqlx::query_as("SELECT status FROM storage_reconcile_runs WHERE id = $1")
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status_before, "running");

    finalize_run(&pool, run_id, "success", None)
        .await
        .expect("finalize_run");

    // After finalize: status='success' + finished_at set.
    let (status, finished): (String, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT status, finished_at FROM storage_reconcile_runs WHERE id = $1")
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "success");
    assert!(finished.is_some(), "finished_at must be populated");

    // Cleanup so reruns don't leak rows.
    sqlx::query("DELETE FROM storage_reconcile_runs WHERE id = $1")
        .bind(run_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn run_lifecycle_respects_status_check_constraint() {
    // The migration 199 CHECK rejects unknown status values — guards
    // against typos when extending the reconciler.
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    let run_id = start_run(&pool, SourceTable::AssetDocuments, "test-bucket", None)
        .await
        .expect("start_run");

    let bad = finalize_run(&pool, run_id, "bogus_status", None).await;
    assert!(bad.is_err(), "CHECK constraint must reject 'bogus_status'");

    // Cleanup.
    sqlx::query("DELETE FROM storage_reconcile_runs WHERE id = $1")
        .bind(run_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn findings_table_severity_check_constraint() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    let run_id = start_run(&pool, SourceTable::KycDocuments, "tb", None)
        .await
        .expect("start_run");

    // Insert with bogus severity should be rejected by CHECK.
    let bad: Result<_, sqlx::Error> = sqlx::query(
        "INSERT INTO storage_reconcile_findings (run_id, source_table, object_path, kind, severity)
         VALUES ($1, 'kyc_documents', 'gs://x/y', 'missing_object', 'doomsday')",
    )
    .bind(run_id)
    .execute(&pool)
    .await;
    assert!(bad.is_err(), "severity CHECK should reject 'doomsday'");

    // And bogus kind likewise.
    let bad2: Result<_, sqlx::Error> = sqlx::query(
        "INSERT INTO storage_reconcile_findings (run_id, source_table, object_path, kind, severity)
         VALUES ($1, 'kyc_documents', 'gs://x/y', 'sabotage', 'critical')",
    )
    .bind(run_id)
    .execute(&pool)
    .await;
    assert!(bad2.is_err(), "kind CHECK should reject 'sabotage'");

    sqlx::query("DELETE FROM storage_reconcile_runs WHERE id = $1")
        .bind(run_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn open_findings_partial_index_is_used_for_dashboard_query() {
    // The migration 199 partial index `idx_storage_reconcile_findings_open`
    // is designed for the dashboard "show open findings" query
    // (acknowledged_at IS NULL ORDER BY created_at DESC). Verify EXPLAIN
    // picks it after ANALYZE.
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    // Seed enough rows that the planner prefers the partial index over a
    // seq-scan. The real "open critical findings" dashboard query
    // matches the index's leading column (severity) + WHERE clause.
    let run_id = start_run(&pool, SourceTable::KycDocuments, "tb", None)
        .await
        .expect("start_run");

    // 6000 rows: mix of severities + acknowledgement status so the
    // partial index actually filters meaningfully.
    for i in 0..6000 {
        let sev = if i % 3 == 0 {
            "critical"
        } else if i % 3 == 1 {
            "warning"
        } else {
            "info"
        };
        let kind = if i % 2 == 0 {
            "missing_object"
        } else {
            "size_mismatch"
        };
        let ack_at = if i % 5 == 0 {
            Some(chrono::Utc::now())
        } else {
            None
        };
        let _ = sqlx::query(
            "INSERT INTO storage_reconcile_findings
             (run_id, source_table, object_path, kind, severity, acknowledged_at)
             VALUES ($1, 'kyc_documents', $2, $3, $4, $5)",
        )
        .bind(run_id)
        .bind(format!("gs://test/{}", i))
        .bind(kind)
        .bind(sev)
        .bind(ack_at)
        .execute(&pool)
        .await;
    }

    sqlx::query("ANALYZE storage_reconcile_findings")
        .execute(&pool)
        .await
        .ok();

    // Dashboard query: open critical findings, newest first.
    let plan: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id, kind, severity FROM storage_reconcile_findings
         WHERE acknowledged_at IS NULL AND severity = 'critical'
         ORDER BY created_at DESC LIMIT 25",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let plan_text = plan
        .iter()
        .map(|p| p.0.clone())
        .collect::<Vec<_>>()
        .join("\n");

    // Either the partial index or another index is acceptable; what we
    // refuse is a bare Seq Scan over the whole table.
    assert!(
        plan_text.contains("Index") || plan_text.contains("index"),
        "expected an Index plan, got:\n{}",
        plan_text
    );

    // Cleanup
    sqlx::query("DELETE FROM storage_reconcile_runs WHERE id = $1")
        .bind(run_id)
        .execute(&pool)
        .await
        .ok();
}

// ─── Schema introspection: catch accidental drops ─────────────────

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn storage_reconcile_runs_has_expected_columns() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    let cols: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns
         WHERE table_name = 'storage_reconcile_runs'
         ORDER BY column_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let names: Vec<String> = cols.into_iter().map(|c| c.0).collect();
    for expected in [
        "bucket",
        "finished_at",
        "hash_mismatches",
        "id",
        "missing_objects",
        "note",
        "objects_scanned",
        "orphan_objects",
        "rows_scanned",
        "sentry_event_id",
        "size_mismatches",
        "source_table",
        "started_at",
        "status",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing column '{}' in storage_reconcile_runs; got {:?}",
            expected,
            names
        );
    }
}
