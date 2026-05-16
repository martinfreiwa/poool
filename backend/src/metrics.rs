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

/// Spawn a 60-second loop that periodically refreshes gauges that
/// otherwise wouldn't update outside of mutation paths (compliance
/// alerts, withdraw queue depth, etc.).
pub async fn run_metrics_refresh_worker(pool: sqlx::PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    interval.tick().await;
    loop {
        interval.tick().await;
        refresh_compliance_alert_gauge(&pool).await;
    }
}
