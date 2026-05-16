# Observability — Prometheus + Grafana

The Rust backend exposes a Prometheus text endpoint at `GET /metrics`
(see `backend/src/metrics.rs`). This directory holds the scrape config,
alert rules and Grafana dashboard JSON.

## Files

- [`prometheus/scrape.yml`](prometheus/scrape.yml) — `scrape_configs`
  block for the backend `/metrics` endpoint. Drop into your existing
  `prometheus.yml`.
- [`prometheus/alerts.yml`](prometheus/alerts.yml) — alert rules covering
  deposit/withdrawal lifecycle anomalies, compliance backlog and HTTP
  error budget. Load via `rule_files:` in Prometheus.
- [`grafana/poool-wallet.json`](grafana/poool-wallet.json) — importable
  Grafana dashboard with the headline KPIs:
  deposits/withdrawals over time, open compliance alerts by severity,
  reconciliation findings, screening outcome rate, 5xx error rate.

## Quick wiring

1. Point Prometheus at the backend:
   ```yaml
   # prometheus.yml
   rule_files:
     - /etc/prometheus/rules/poool-alerts.yml
   scrape_configs:
     # ...paste the contents of scrape.yml here
   ```

2. Mount the alert rules where Prometheus expects them:
   ```bash
   cp infra/prometheus/alerts.yml /etc/prometheus/rules/poool-alerts.yml
   curl -X POST http://prometheus:9090/-/reload
   ```

3. Import the dashboard in Grafana:
   `Dashboards → Import → Upload JSON file → grafana/poool-wallet.json`,
   then select your Prometheus datasource.

## Metric catalogue

| Metric | Type | Labels | Source |
|--------|------|--------|--------|
| `wallet_deposits_total` | Counter | `outcome`, `currency` | wallet/routes.rs + reconciliation |
| `wallet_withdrawals_total` | Counter | `outcome` | wallet/routes.rs + admin/withdrawals.rs |
| `wallet_amount_cents_total` | Counter | `kind`, `outcome` | helper `record_deposit`/`record_withdrawal` |
| `wallet_reconciliation_findings` | Gauge | `finding` | wallet/reconciliation.rs |
| `compliance_alerts_open` | Gauge | `severity` | metrics-refresh worker |
| `compliance_screening_runs_total` | Counter | `outcome` | compliance/rescreening.rs |
| `http_requests_total` | Counter | `path`, `method`, `status_class` | (reserved — wire from a middleware) |
| `http_request_duration_seconds` | Histogram | `path`, `method` | (reserved — wire from a middleware) |

Note: HTTP request/duration metrics declare the labels but no middleware
is currently wired in. Adding a `tower::Layer` that observes
`HTTP_REQUESTS_TOTAL` + `HTTP_DURATION_SECONDS` is straightforward — see
`backend/src/metrics.rs` for the static handles.

## Alert severity legend

- **page** — pager-grade, 24/7. Customer impact in progress or imminent.
- **ticket** — Slack ping / ticket; review during business hours.
- **info** — observe-only signal, no SLA.
