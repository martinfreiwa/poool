# Storage — Observability Runbook

Covers the **fifth leg** of the storage hardening pass: Prometheus
metrics, Grafana dashboards, alert thresholds, and the tracing-span
contract for upload + retention hot paths.

Implementation lives in `backend/src/metrics.rs` and is exposed via
the existing `GET /metrics` endpoint (Prometheus text exposition).

---

## Storage Metric Catalog

| Metric | Type | Labels | What it tells you |
|---|---|---|---|
| `storage_uploads_total` | counter | `class`, `outcome` | Per-class upload attempts + final outcome (`ok`, `quota_exceeded`, `av_infected`, `mime_mismatch`, `gcs_error`, `svg_rejected`) |
| `storage_upload_bytes_total` | counter | `class` | Bytes accepted (outcome=ok only); drives the growth-rate alerts |
| `storage_upload_duration_seconds` | histogram | `class` | End-to-end upload latency; SLO source |
| `storage_gcs_errors_total` | counter | `op`, `kind` | Per-operation error stream (`upload`/`download`/`delete`/`head`/`sign` × `auth`/`not_found`/`timeout`/`other`) |
| `storage_av_outcomes_total` | counter | `outcome` | ClamAV scan verdicts (`clean`/`infected`/`error`/`not_yet_scanned`) |
| `storage_retention_due` | gauge | `bucket` | Live count of KYC docs past retention deadline awaiting worker |
| `storage_reconcile_findings_open` | gauge | `kind`, `severity` | Live count of open (unacknowledged) reconciler findings |
| `storage_quota_used_bytes` | gauge | `class` | Aggregate bytes consumed per class across all users |

Gauges that don't update on hot paths are refreshed every 60s by
`refresh_storage_gauges` (called from the existing
`run_metrics_refresh_worker`).

---

## Alert Rules

Drop into `infra/prometheus/alerts/storage.yml`. Severities map to the
PagerDuty escalation policy (Pxx).

```yaml
groups:
- name: storage.rules
  interval: 1m
  rules:

  # P1: any GCS errors trending up — error-budget burn
  - alert: StorageGcsErrorRateHigh
    expr: |
      sum by (op,kind) (rate(storage_gcs_errors_total[5m])) > 0.05
    for: 10m
    labels: { severity: P1, owner: storage }
    annotations:
      summary: "GCS {{ $labels.op }} errors ({{ $labels.kind }}) > 0.05/s for 10m"
      runbook: docs/storage/05-observability.md#triage-gcs-errors

  # P0: malware detected in any user upload
  - alert: StorageAvInfected
    expr: increase(storage_av_outcomes_total{outcome="infected"}[5m]) > 0
    for: 0s   # fire immediately
    labels: { severity: P0, owner: storage }
    annotations:
      summary: "Infected file uploaded — quarantined; investigate"
      runbook: docs/storage/02-antivirus-scanning.md#triage-infected

  # P1: retention worker not keeping up with deadlines
  - alert: StorageRetentionBacklog
    expr: storage_retention_due > 0
    for: 2h
    labels: { severity: P1, owner: compliance }
    annotations:
      summary: "{{ $value }} KYC docs past retention deadline >2h"
      runbook: docs/storage/04-compliance-and-retention.md#triage

  # P0: a missing-object reconciler finding implies data loss
  - alert: StorageReconcileDataLoss
    expr: |
      sum by (kind) (storage_reconcile_findings_open{kind="missing_object",severity="critical"}) > 0
    for: 0s
    labels: { severity: P0, owner: storage }
    annotations:
      summary: "Reconciler reports {{ $value }} missing GCS objects"
      runbook: docs/storage/03-backup-and-disaster-recovery.md#triage-runbook

  # P2: quota for any class is approaching the per-user cap (proxy via
  # aggregate growth rate)
  - alert: StorageGrowthSpike
    expr: |
      rate(storage_upload_bytes_total[1h]) > 5 * rate(storage_upload_bytes_total[7d] offset 7d)
    for: 30m
    labels: { severity: P2, owner: storage }
    annotations:
      summary: "{{ $labels.class }} upload rate 5× the trailing 7d baseline"

  # P3: upload latency tail regressing
  - alert: StorageUploadLatencyP99High
    expr: |
      histogram_quantile(0.99, sum by (class,le) (rate(storage_upload_duration_seconds_bucket[10m]))) > 8
    for: 30m
    labels: { severity: P3, owner: storage }
    annotations:
      summary: "p99 upload latency for {{ $labels.class }} > 8s for 30m"

  # P3: AV scanner falling behind (objects served before scan completed)
  - alert: StorageAvNotYetScanned
    expr: |
      rate(storage_av_outcomes_total{outcome="not_yet_scanned"}[5m]) > 0.5
    for: 15m
    labels: { severity: P3, owner: storage }
    annotations:
      summary: "AV scanner lag — {{ $value }}/s NOT-YET-SCANNED reads"
```

---

## Grafana Dashboard Panels

Each panel is implementable from a single PromQL query.

| Panel | Query |
|---|---|
| Uploads per minute (stacked by class) | `sum by (class) (rate(storage_uploads_total{outcome="ok"}[1m]))` |
| Upload failure ratio | `sum(rate(storage_uploads_total{outcome!="ok"}[5m])) / sum(rate(storage_uploads_total[5m]))` |
| Bytes/sec inflow per class | `sum by (class) (rate(storage_upload_bytes_total[1m]))` |
| p50/p95/p99 upload latency | `histogram_quantile(0.5\|0.95\|0.99, sum by (class,le) (rate(storage_upload_duration_seconds_bucket[5m])))` |
| GCS error rate by op | `sum by (op) (rate(storage_gcs_errors_total[5m]))` |
| AV verdicts | `sum by (outcome) (rate(storage_av_outcomes_total[5m]))` |
| Open reconcile findings | `sum by (severity) (storage_reconcile_findings_open)` |
| Retention backlog | `storage_retention_due` |
| Quota used per class (GB) | `storage_quota_used_bytes / 1024 / 1024 / 1024` |

---

## Tracing Span Contract

Every storage hot path opens a span with these fields. Spans bubble
into Sentry traces + Cloud Trace via the existing
`tracing-opentelemetry` bridge.

| Span | Required fields | Optional fields |
|---|---|---|
| `storage.upload` | `class`, `bucket`, `object_path`, `bytes`, `user_id` | `mime_sniffed`, `sha256`, `av_status` |
| `storage.download` | `bucket`, `object_path` | `user_id`, `range_start`, `range_end` |
| `storage.delete` | `bucket`, `object_path`, `reason` | `user_id` |
| `storage.signed_url` | `bucket`, `object_path`, `expires_in_min` | `disposition` |
| `storage.reconcile.run` | `source_table`, `bucket`, `run_id` | (counters surfaced as span events) |
| `storage.retention.run` | `bucket`, `dry_run`, `run_id` | `rows_due`, `rows_deleted` |

`user_id` is the **business** user_id, not the session id. Operator
identity goes in a separate `actor_id` field when present.

---

## Triage Runbooks (linked from alert annotations)

### Triage GCS errors

1. Open the Grafana panel "GCS error rate by op". Pick the dominant op.
2. Check Cloud Run logs for the matching `storage.upload` /
   `storage.download` span. Span errors carry the underlying GCS error
   string verbatim.
3. If `kind="auth"`: rotate the workload identity binding. Likely a
   key expiry or revoked role.
4. If `kind="not_found"`: orphaned DB row — file the reconciler will
   pick up. No immediate action.
5. If `kind="timeout"`: check GCS status page; if green, profile the
   upload — usually a flaky client connection that should have
   retried.
6. If `kind="other"`: pull the raw error from logs, open a Sentry
   issue, escalate.

### Triage retention backlog

See `docs/storage/04-compliance-and-retention.md` → "Triage runbook".

### Triage missing-object findings (P0)

See `docs/storage/03-backup-and-disaster-recovery.md` → "Triage runbook".

---

## SLOs

| SLO | Target | Window | Source |
|---|---|---|---|
| Upload success rate | ≥ 99.5% | 28-day rolling | `1 - sum(rate(storage_uploads_total{outcome!="ok"}[28d])) / sum(rate(storage_uploads_total[28d]))` |
| Upload p95 latency | < 2s (small), < 8s (asset_document) | 28d | `histogram_quantile(0.95, ...)` per class |
| Retention SLA | 100% delete within 24h of deadline | continuous | `storage_retention_due == 0` (alerts after 2h) |
| Reconcile zero data-loss | `missing_object` findings = 0 | continuous | gauge directly |

Error budget burn → trigger an automatic PR-blocking flag once the
28d budget is < 30% remaining.

---

## Verification Checklist

- [ ] `/metrics` returns all 8 storage metrics by name (verified by
      `cargo test storage_metrics_are_registered`).
- [ ] Each metric has at least one observation in production within
      24h of deploy.
- [ ] Alert rules deployed to Prometheus + linted with `promtool
      check rules`.
- [ ] Grafana dashboard imported and at least one screenshot saved to
      `docs/storage/grafana/`.
- [ ] Tracing spans visible in Sentry trace view for a real upload.
- [ ] SLO error-budget burn alert tested via synthetic upload-fail
      injection.
