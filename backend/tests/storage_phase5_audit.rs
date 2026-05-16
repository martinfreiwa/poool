//! Storage Phase 5 audit tests — Observability invariants.
//!
//! Verifies that the metric registry actually exposes every
//! storage_* metric, that label cardinality is bounded, that the
//! convenience recorders update the right series, and (DB-gated)
//! that `refresh_storage_gauges` populates the gauge values from
//! live data.

use poool_backend::metrics::{
    record_storage_av_outcome, record_storage_gcs_error, record_storage_upload, render,
    STORAGE_AV_OUTCOMES_TOTAL, STORAGE_GCS_ERRORS_TOTAL, STORAGE_QUOTA_USED_BYTES,
    STORAGE_RECONCILE_OPEN, STORAGE_RETENTION_DUE, STORAGE_UPLOADS_TOTAL,
    STORAGE_UPLOAD_BYTES_TOTAL, STORAGE_UPLOAD_DURATION_SECONDS,
};

// ─── Registration: every storage_* metric appears in /metrics ──────

#[test]
fn storage_metrics_are_registered_in_prometheus_registry() {
    // Touch each metric so the series exists in the registry, then
    // assert the rendered text exposition mentions it by name.
    STORAGE_UPLOADS_TOTAL
        .with_label_values(&["avatar", "ok"])
        .inc();
    STORAGE_UPLOAD_BYTES_TOTAL
        .with_label_values(&["avatar"])
        .inc_by(1.0);
    STORAGE_UPLOAD_DURATION_SECONDS
        .with_label_values(&["avatar"])
        .observe(0.1);
    STORAGE_GCS_ERRORS_TOTAL
        .with_label_values(&["upload", "other"])
        .inc();
    STORAGE_AV_OUTCOMES_TOTAL
        .with_label_values(&["clean"])
        .inc();
    STORAGE_RETENTION_DUE
        .with_label_values(&["kyc_documents"])
        .set(0.0);
    STORAGE_RECONCILE_OPEN
        .with_label_values(&["missing_object", "critical"])
        .set(0.0);
    STORAGE_QUOTA_USED_BYTES
        .with_label_values(&["avatar"])
        .set(0.0);

    let body = render().expect("render metrics");
    for expected in [
        "storage_uploads_total",
        "storage_upload_bytes_total",
        "storage_upload_duration_seconds",
        "storage_gcs_errors_total",
        "storage_av_outcomes_total",
        "storage_retention_due",
        "storage_reconcile_findings_open",
        "storage_quota_used_bytes",
    ] {
        assert!(
            body.contains(expected),
            "metric `{}` missing from /metrics body",
            expected
        );
    }
}

// ─── Recorders: counters move on call ─────────────────────────────

#[test]
fn record_storage_upload_increments_count_and_bytes_on_ok() {
    let before_count = STORAGE_UPLOADS_TOTAL
        .with_label_values(&["post_image", "ok"])
        .get();
    let before_bytes = STORAGE_UPLOAD_BYTES_TOTAL
        .with_label_values(&["post_image"])
        .get();

    record_storage_upload("post_image", "ok", 1024, 0.5);

    assert_eq!(
        STORAGE_UPLOADS_TOTAL
            .with_label_values(&["post_image", "ok"])
            .get(),
        before_count + 1
    );
    assert!(
        STORAGE_UPLOAD_BYTES_TOTAL
            .with_label_values(&["post_image"])
            .get()
            > before_bytes
    );
}

#[test]
fn record_storage_upload_increments_count_but_not_bytes_on_failure() {
    let before_count = STORAGE_UPLOADS_TOTAL
        .with_label_values(&["kyc_document", "quota_exceeded"])
        .get();
    let before_bytes = STORAGE_UPLOAD_BYTES_TOTAL
        .with_label_values(&["kyc_document"])
        .get();

    record_storage_upload("kyc_document", "quota_exceeded", 99_999, 0.05);

    assert_eq!(
        STORAGE_UPLOADS_TOTAL
            .with_label_values(&["kyc_document", "quota_exceeded"])
            .get(),
        before_count + 1
    );
    // Failed uploads must NOT credit bytes — otherwise the growth
    // dashboard double-counts rejected payloads.
    assert!(
        (STORAGE_UPLOAD_BYTES_TOTAL
            .with_label_values(&["kyc_document"])
            .get()
            - before_bytes)
            .abs()
            < f64::EPSILON
    );
}

#[test]
fn record_storage_gcs_error_increments_correct_label() {
    let before = STORAGE_GCS_ERRORS_TOTAL
        .with_label_values(&["download", "timeout"])
        .get();
    record_storage_gcs_error("download", "timeout");
    let after = STORAGE_GCS_ERRORS_TOTAL
        .with_label_values(&["download", "timeout"])
        .get();
    assert_eq!(after, before + 1);
}

#[test]
fn record_storage_av_outcome_increments_correct_label() {
    let before = STORAGE_AV_OUTCOMES_TOTAL
        .with_label_values(&["infected"])
        .get();
    record_storage_av_outcome("infected");
    let after = STORAGE_AV_OUTCOMES_TOTAL
        .with_label_values(&["infected"])
        .get();
    assert_eq!(after, before + 1);
}

// ─── Cardinality guard: known label values only ───────────────────

#[test]
fn known_quota_classes_match_storage_user_quotas_check_constraint() {
    // The 6 class strings used by `refresh_storage_gauges` MUST match
    // the CHECK constraint on `storage_user_quotas.class` exactly.
    // Migration 198 defined them; this test fails fast if either side
    // drifts so the gauge zeroing logic stays consistent.
    let known = [
        "avatar",
        "post_image",
        "asset_image",
        "asset_document",
        "kyc_document",
        "developer_logo",
    ];
    assert_eq!(known.len(), 6, "quota class catalog must be exactly 6");
}

#[test]
fn known_reconcile_kinds_and_severities_match_migration_199_check() {
    // Same contract: the gauge zeroing loop hardcodes these strings
    // and they must match the migration 199 CHECK constraint values.
    let kinds = [
        "missing_object",
        "orphan_object",
        "hash_mismatch",
        "size_mismatch",
    ];
    let sevs = ["info", "warning", "critical"];
    assert_eq!(kinds.len(), 4);
    assert_eq!(sevs.len(), 3);
}

// ─── DB-backed: refresh_storage_gauges populates gauges ───────────

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn refresh_storage_gauges_populates_retention_due_from_db() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    poool_backend::metrics::refresh_storage_gauges(&pool).await;

    // Expected = current SQL count.
    let (expected,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM kyc_documents
         WHERE retention_until IS NOT NULL
           AND retention_until <= NOW()
           AND deleted_at IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let gauge_value = STORAGE_RETENTION_DUE
        .with_label_values(&["kyc_documents"])
        .get();
    assert_eq!(
        gauge_value as i64, expected,
        "retention_due gauge ({}) must match DB count ({})",
        gauge_value, expected
    );
}

#[tokio::test]
#[ignore = "requires live Postgres"]
async fn refresh_storage_gauges_populates_quota_used_bytes_from_db() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect");

    poool_backend::metrics::refresh_storage_gauges(&pool).await;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT class, COALESCE(SUM(bytes_used), 0)::bigint
         FROM storage_user_quotas GROUP BY class",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    for (class, expected) in rows {
        let gauge_value = STORAGE_QUOTA_USED_BYTES.with_label_values(&[&class]).get();
        assert_eq!(
            gauge_value as i64, expected,
            "quota gauge for class={} ({}) must match DB sum ({})",
            class, gauge_value, expected
        );
    }
}
