//! Prometheus metrics — central registry + helpers (P0-5).
//!
//! Exposed at `GET /metrics` for Prometheus scraping. Counters/gauges
//! are declared once via `once_cell::sync::Lazy` so every call site
//! gets a cheap `.inc()` / `.observe()` against the shared registry.
//!
//! Naming follows Prometheus convention: snake_case, unit-suffix
//! (`_total`, `_seconds`, `_bytes`). New metrics belong in one of the
//! existing buckets:
//!
//!   wallet_deposits_total{outcome="initiated|submitted|expired|paid|...", currency="USD"}
//!   wallet_withdrawals_total{outcome="requested|approved|rejected|frozen"}
//!   wallet_amount_cents_total{kind="deposit|withdrawal", outcome="..."}
//!   wallet_reconciliation_findings{kind="...", state="..."}
//!   compliance_alerts_open{severity="..."}
//!   compliance_screening_runs_total{outcome="clear|hit|error|skipped"}
//!   http_requests_total{path, method, status_class}
//!   http_request_duration_seconds{path, method}
//!
//! Histograms use the default exponential buckets; tune as needed when
//! we have a baseline.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec_with_registry, register_gauge_vec_with_registry,
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, CounterVec,
    Encoder, GaugeVec, HistogramVec, IntCounterVec, Registry, TextEncoder,
};

/// Process-wide Prometheus registry. Initialised on first access.
pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// Outcomes for `wallet_deposits_total`.
#[allow(missing_docs)]
pub mod deposit_outcome {
    /// User completed step 1 — deposit_requests row created, reference issued.
    pub const INITIATED: &str = "initiated";
    /// User completed step 2 — proof uploaded, awaiting admin verification.
    pub const SUBMITTED: &str = "submitted";
    /// Admin confirmed the wire — wallet credited.
    pub const PAID: &str = "paid";
    /// Auto-expired past the processing window.
    pub const EXPIRED: &str = "expired";
    /// Any other terminal failure (rejected, system error, etc.).
    pub const FAILED: &str = "failed";
}

/// Outcomes for `wallet_withdrawals_total`.
#[allow(missing_docs)]
pub mod withdraw_outcome {
    /// User submitted the request, balance frozen, awaiting admin.
    pub const REQUESTED: &str = "requested";
    /// Admin approved → funds released to bank.
    pub const APPROVED: &str = "approved";
    /// Admin rejected → balance restored.
    pub const REJECTED: &str = "rejected";
    /// Blocked by withdrawal-safety pipeline (KYC, 2FA, daily cap, velocity).
    pub const BLOCKED_SAFETY: &str = "blocked_safety";
    /// Blocked because available balance was insufficient.
    pub const BLOCKED_FUNDS: &str = "blocked_funds";
    /// User cancelled before admin review (P1-4).
    pub const CANCELLED: &str = "cancelled";
}

/// Counter for deposit lifecycle events by outcome and currency.
pub static DEPOSITS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "wallet_deposits_total",
        "Count of deposit lifecycle events.",
        &["outcome", "currency"],
        REGISTRY
    )
    .expect("register wallet_deposits_total")
});

/// Counter for withdrawal lifecycle events by outcome.
pub static WITHDRAWALS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "wallet_withdrawals_total",
        "Count of withdrawal lifecycle events.",
        &["outcome"],
        REGISTRY
    )
    .expect("register wallet_withdrawals_total")
});

/// Counter for total wallet amount processed in cents by kind and outcome.
pub static WALLET_AMOUNT_CENTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "wallet_amount_cents_total",
        "Sum of amounts (in cents) processed by lifecycle outcome.",
        &["kind", "outcome"],
        REGISTRY
    )
    .expect("register wallet_amount_cents_total")
});

/// Gauge for latest wallet reconciliation finding counts by finding type.
pub static RECONCILIATION_FINDINGS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec_with_registry!(
        "wallet_reconciliation_findings",
        "Latest per-run counts from the reconciliation worker.",
        &["finding"],
        REGISTRY
    )
    .expect("register wallet_reconciliation_findings")
});

/// Gauge for currently open compliance alerts by severity.
pub static COMPLIANCE_ALERTS_OPEN: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec_with_registry!(
        "compliance_alerts_open",
        "Number of compliance_alerts rows still open, split by severity.",
        &["severity"],
        REGISTRY
    )
    .expect("register compliance_alerts_open")
});

/// Counter for compliance screening runs by outcome.
pub static SCREENING_RUNS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "compliance_screening_runs_total",
        "Sanctions re-screening runs by outcome.",
        &["outcome"],
        REGISTRY
    )
    .expect("register compliance_screening_runs_total")
});

/// Counter for HTTP requests by route, method, and status class.
pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "http_requests_total",
        "Total HTTP requests by route + method + status class.",
        &["path", "method", "status_class"],
        REGISTRY
    )
    .expect("register http_requests_total")
});

/// Histogram for HTTP request latency in seconds by route and method.
pub static HTTP_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "http_request_duration_seconds",
        "HTTP request latency in seconds, bucketed by route + method.",
        &["path", "method"],
        // Default exponential buckets are usable; tune once we have data.
        prometheus::DEFAULT_BUCKETS.to_vec(),
        REGISTRY
    )
    .expect("register http_request_duration_seconds")
});

/// Convenience: record one deposit-lifecycle event with its amount.
pub fn record_deposit(outcome: &str, currency: &str, amount_cents: i64) {
    DEPOSITS_TOTAL.with_label_values(&[outcome, currency]).inc();
    if amount_cents > 0 {
        WALLET_AMOUNT_CENTS_TOTAL
            .with_label_values(&["deposit", outcome])
            .inc_by(amount_cents as f64);
    }
}

/// Convenience: record one withdrawal-lifecycle event with its amount.
pub fn record_withdrawal(outcome: &str, amount_cents: i64) {
    WITHDRAWALS_TOTAL.with_label_values(&[outcome]).inc();
    if amount_cents > 0 {
        WALLET_AMOUNT_CENTS_TOTAL
            .with_label_values(&["withdrawal", outcome])
            .inc_by(amount_cents as f64);
    }
}

/// Convenience: record one screening run.
pub fn record_screening(outcome: &str) {
    SCREENING_RUNS_TOTAL.with_label_values(&[outcome]).inc();
}

/// Snapshot the reconciliation report into Prometheus gauges so Grafana
/// can read both "current state" and "rate of change" through alerts.
pub fn record_reconciliation_snapshot(
    deposits_expired: i64,
    deposits_stuck_no_proof: i64,
    deposits_stuck_with_proof: i64,
    withdrawals_stuck: i64,
) {
    RECONCILIATION_FINDINGS
        .with_label_values(&["deposits_expired"])
        .set(deposits_expired as f64);
    RECONCILIATION_FINDINGS
        .with_label_values(&["deposits_stuck_no_proof"])
        .set(deposits_stuck_no_proof as f64);
    RECONCILIATION_FINDINGS
        .with_label_values(&["deposits_stuck_with_proof"])
        .set(deposits_stuck_with_proof as f64);
    RECONCILIATION_FINDINGS
        .with_label_values(&["withdrawals_stuck"])
        .set(withdrawals_stuck as f64);
}

/// Refresh the `compliance_alerts_open` gauge by severity. Called from a
/// lightweight background task so the gauge tracks DB state without
/// requiring every alert mutation to update it directly.
pub async fn refresh_compliance_alert_gauge(pool: &sqlx::PgPool) {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT severity, COUNT(*)::bigint
             FROM compliance_alerts
            WHERE closed_at IS NULL
            GROUP BY severity"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Zero out all known severities first so a vanished severity doesn't
    // keep its stale value forever.
    for sev in ["low", "medium", "high", "critical"] {
        COMPLIANCE_ALERTS_OPEN.with_label_values(&[sev]).set(0.0);
    }
    for (sev, count) in rows {
        COMPLIANCE_ALERTS_OPEN
            .with_label_values(&[&sev])
            .set(count as f64);
    }
}

/// Render the full metrics registry as Prometheus text exposition.
pub fn render() -> Result<String, prometheus::Error> {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer)?;
    String::from_utf8(buffer).map_err(|e| prometheus::Error::Msg(e.to_string()))
}

/// Axum middleware that records request count + latency by matched
/// route. Uses `MatchedPath` (the route template like
/// `/api/wallet/transactions/:id`) instead of the raw URI to keep
/// Prometheus cardinality bounded.
pub async fn http_metrics_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "unmatched".to_string());
    let method = request.method().as_str().to_string();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let status = response.status().as_u16();
    let status_class = match status {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    };
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&path, &method, status_class])
        .inc();
    HTTP_DURATION_SECONDS
        .with_label_values(&[&path, &method])
        .observe(start.elapsed().as_secs_f64());
    response
}

/// Spawn a 60-second loop that periodically refreshes gauges that
/// otherwise wouldn't update outside of mutation paths (compliance
/// alerts, withdraw queue depth, etc.).
pub async fn run_metrics_refresh_worker(pool: sqlx::PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    interval.tick().await;
    loop {
        interval.tick().await;
        refresh_compliance_alert_gauge(&pool).await;
        refresh_storage_gauges(&pool).await;
    }
}

// ─── Storage subsystem metrics (Phase 5) ───────────────────────────
//
// Counters: per-upload events with class+outcome dimensions.
// Histograms: upload duration so SLOs can be set per class.
// Gauges: snapshot-style state (open reconciler findings, retention
// backlog, quota usage) refreshed by `refresh_storage_gauges` from the
// 60-second worker above.
//
// Naming follows the existing convention: `storage_*_total` for
// counters, `storage_*_seconds` for histograms, `storage_*` for
// gauges. Label cardinality is bounded — `class` enumerates over the
// 6 `QuotaClass` values, `outcome` over a small closed set.

/// Counter for storage upload events by class + outcome.
/// Outcome ∈ {ok, quota_exceeded, av_infected, mime_mismatch, gcs_error, svg_rejected}.
pub static STORAGE_UPLOADS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "storage_uploads_total",
        "Count of upload attempts by class + outcome.",
        &["class", "outcome"],
        REGISTRY
    )
    .expect("register storage_uploads_total")
});

/// Counter for upload bytes accepted (outcome=ok only). Tracks growth
/// rate per class.
pub static STORAGE_UPLOAD_BYTES_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "storage_upload_bytes_total",
        "Bytes successfully uploaded, by class.",
        &["class"],
        REGISTRY
    )
    .expect("register storage_upload_bytes_total")
});

/// Histogram for upload duration in seconds, per class. Buckets are
/// tuned for "small image to 20 MB doc" range — adjust once we have
/// production data.
pub static STORAGE_UPLOAD_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "storage_upload_duration_seconds",
        "End-to-end upload duration in seconds, by class.",
        &["class"],
        // 50ms .. 16s exponential covers realistic image + doc uploads.
        vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0],
        REGISTRY
    )
    .expect("register storage_upload_duration_seconds")
});

/// Counter for GCS API errors by operation + error kind. Used by the
/// "storage error budget" alert.
pub static STORAGE_GCS_ERRORS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "storage_gcs_errors_total",
        "GCS API errors by operation (upload|download|delete|head|sign) and kind (auth|404|timeout|other).",
        &["op", "kind"],
        REGISTRY
    )
    .expect("register storage_gcs_errors_total")
});

/// Counter for AV scan outcomes by terminal verdict.
pub static STORAGE_AV_OUTCOMES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec_with_registry!(
        "storage_av_outcomes_total",
        "AV scan terminal outcomes (clean|infected|error|not_yet_scanned).",
        &["outcome"],
        REGISTRY
    )
    .expect("register storage_av_outcomes_total")
});

/// Gauge for the current count of KYC docs past their retention
/// deadline but not yet deleted (worker backlog). Refreshed every 60s.
pub static STORAGE_RETENTION_DUE: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec_with_registry!(
        "storage_retention_due",
        "KYC docs past retention deadline awaiting worker deletion.",
        &["bucket"],
        REGISTRY
    )
    .expect("register storage_retention_due")
});

/// Gauge for the current count of open reconciler findings by kind +
/// severity. `acknowledged_at IS NULL` only.
pub static STORAGE_RECONCILE_OPEN: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec_with_registry!(
        "storage_reconcile_findings_open",
        "Open (unacknowledged) reconciler findings by kind + severity.",
        &["kind", "severity"],
        REGISTRY
    )
    .expect("register storage_reconcile_findings_open")
});

/// Gauge for aggregate quota usage in bytes, by class. Sum across all
/// users — Prometheus would not handle per-user labels at scale.
pub static STORAGE_QUOTA_USED_BYTES: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec_with_registry!(
        "storage_quota_used_bytes",
        "Aggregate quota usage in bytes by class (sum across all users).",
        &["class"],
        REGISTRY
    )
    .expect("register storage_quota_used_bytes")
});

/// Convenience: record one upload attempt with its outcome + bytes +
/// elapsed duration. Bytes are only credited on `outcome="ok"`.
pub fn record_storage_upload(class: &str, outcome: &str, bytes: i64, elapsed_secs: f64) {
    STORAGE_UPLOADS_TOTAL
        .with_label_values(&[class, outcome])
        .inc();
    STORAGE_UPLOAD_DURATION_SECONDS
        .with_label_values(&[class])
        .observe(elapsed_secs);
    if outcome == "ok" && bytes > 0 {
        STORAGE_UPLOAD_BYTES_TOTAL
            .with_label_values(&[class])
            .inc_by(bytes as f64);
    }
}

/// Convenience: record one GCS API error.
pub fn record_storage_gcs_error(op: &str, kind: &str) {
    STORAGE_GCS_ERRORS_TOTAL
        .with_label_values(&[op, kind])
        .inc();
}

/// Convenience: record one AV scan outcome.
pub fn record_storage_av_outcome(outcome: &str) {
    STORAGE_AV_OUTCOMES_TOTAL
        .with_label_values(&[outcome])
        .inc();
}

/// Snapshot all storage-subsystem gauges that don't update on hot
/// paths. Called from the 60s metrics refresh worker. Failures are
/// silent — a transient DB blip should not crash the worker.
pub async fn refresh_storage_gauges(pool: &sqlx::PgPool) {
    // Retention backlog: rows past deadline + still alive.
    if let Ok((due,)) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*)::bigint FROM kyc_documents
         WHERE retention_until IS NOT NULL
           AND retention_until <= NOW()
           AND deleted_at IS NULL",
    )
    .fetch_one(pool)
    .await
    {
        STORAGE_RETENTION_DUE
            .with_label_values(&["kyc_documents"])
            .set(due as f64);
    }

    // Open reconciler findings by (kind, severity). Zero out the
    // known cells first so disappeared combinations don't stick.
    for kind in [
        "missing_object",
        "orphan_object",
        "hash_mismatch",
        "size_mismatch",
    ] {
        for sev in ["info", "warning", "critical"] {
            STORAGE_RECONCILE_OPEN
                .with_label_values(&[kind, sev])
                .set(0.0);
        }
    }
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT kind, severity, COUNT(*)::bigint
           FROM storage_reconcile_findings
          WHERE acknowledged_at IS NULL
          GROUP BY kind, severity",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for (kind, sev, count) in rows {
        STORAGE_RECONCILE_OPEN
            .with_label_values(&[&kind, &sev])
            .set(count as f64);
    }

    // Aggregate quota usage by class.
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT class, COALESCE(SUM(bytes_used), 0)::bigint
           FROM storage_user_quotas
          GROUP BY class",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    // Zero out known classes first so a vanished class drops.
    for c in [
        "avatar",
        "post_image",
        "asset_image",
        "asset_document",
        "kyc_document",
        "developer_logo",
    ] {
        STORAGE_QUOTA_USED_BYTES.with_label_values(&[c]).set(0.0);
    }
    for (class, bytes) in rows {
        STORAGE_QUOTA_USED_BYTES
            .with_label_values(&[&class])
            .set(bytes as f64);
    }
}
