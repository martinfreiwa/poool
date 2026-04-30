# POOOL Build And Test Gate - 2026-04-27

Run time: 2026-04-27 12:23 CEST

## Overall Status

**Partial pass: Rust/backend checks passed; browser-backed E2E is blocked by local Playwright launch.**

Rust formatting, unit tests, and clippy passed. The already-running local backend responded with HTTP 200 for both `/` and `/health`. Non-browser Python/API/static checks passed.

The broad Python gate did not complete because Playwright could not launch Chromium in this macOS sandbox and the Firefox fallback aborted in headless mode before the first browser-backed test executed. No production application code was modified by this gate.

## Commands Run

| Command | Status | Evidence |
|---------|--------|----------|
| `cd backend && cargo fmt --check` | Pass | Completed with exit code 0. |
| `cd backend && cargo test` | Pass | 202 Rust tests passed; 0 failed. |
| `cd backend && cargo clippy` | Pass | Completed with exit code 0. |
| `curl -sS -o /tmp/poool_root_check.txt -w '%{http_code}' http://localhost:8888/` | Pass | Returned `200`. |
| `curl -sS -o /tmp/poool_health_check.txt -w '%{http_code}' http://localhost:8888/health` | Pass | Returned `200`. |
| `pgrep -fl 'poool-backend\|cargo run' \|\| true` | Blocked | macOS process lookup failed with `sysmond service not found`; HTTP probes confirmed backend availability instead. |
| `python3 -m pytest tests/ --maxfail=1 -vv` | Blocked | 6 tests passed, then Playwright setup failed before `tests/e2e/test_admin_asset_tokenize.py::test_admin_asset_tokenize_permissions_csrf_mock_deploy_and_ui`. |
| `python3 -m pytest tests/adhoc tests/admin --maxfail=1 -vv` | Pass | 6 non-browser tests passed. |
| `python3 -m pytest tests/e2e/test_public_property.py -m smoke --maxfail=1 -vv` | Blocked | Browser setup failed before the first selected public property smoke test executed. |

## Failure / Blocker Details

### Playwright Browser Launch Blocker

The Python E2E gate is blocked before page execution:

- Chromium failed with `bootstrap_check_in org.chromium.Chromium.MachPortRendezvousServer... Permission denied (1100)`.
- Firefox fallback launched, then exited in headless mode with `signal=SIGABRT`.

This appears environment-specific because the failure happens in browser fixture setup before application navigation or assertions. It currently blocks browser confidence for admin asset tokenize and public property pages in this local automation run.

### Process Lookup Limitation

`pgrep` could not inspect local processes because `sysmond` was unavailable in the sandbox. This did not block the gate because direct HTTP probes to the running backend returned 200.

## Backend Domains Covered

- `admin`: route contract/static tests passed for affiliate applications; admin asset tokenize browser coverage is blocked by Playwright setup.
- `assets`: Rust public asset slug/model tests passed; public property browser smoke is blocked by Playwright setup.
- `auth`: rate limiter Rust tests passed.
- `blockchain`: encoding helper Rust tests passed.
- `cart/checkout/payments`: financial arithmetic, platform fee, and FX Rust tests passed.
- `common`: financial invariant, reconciliation, currency, and sanitization Rust tests passed.
- `community`: validation and sanitization Rust tests passed.
- `developer`: draft validation Rust tests passed.
- `dividends`: payout and eligibility Rust tests passed.
- `ipfs`: config and URI Rust tests passed.
- `marketplace`: background, charts, matching, orderbook, P2P, settlement, validation, websocket, and model Rust tests passed.
- `storage`: MIME validation and legacy GCS URL rewrite Rust tests passed.
- `support`: `tests/adhoc/test_support.py::test_support_endpoints` passed.

## Pages / Routes Affected

- `/` returned HTTP 200.
- `/health` returned HTTP 200.
- `/admin/asset-tokenize` browser E2E did not execute because Playwright setup failed.
- `/p/:slug` public property smoke did not execute because Playwright setup failed.
- `/admin/affiliate-applications` static/route-contract tests passed.
- Support endpoints passed via `tests/adhoc/test_support.py`.

## Production Readiness Impact

- **Deployability:** No Rust formatting, compile, unit-test, or clippy blocker was found.
- **Backend confidence:** High for covered Rust domains and non-browser API/static checks.
- **Browser confidence:** Blocked in this local environment; browser-backed regressions cannot be ruled out from this run.
- **Production risk:** No application regression was identified. The remaining blocker is verification infrastructure, not a confirmed product failure.
- **Other automations:** Page audits and E2E coverage automations should treat browser evidence from this run as unavailable until Playwright can launch.

## Recommended Next Action

Run the browser-backed E2E gate in an environment where Playwright can launch, or fix the local macOS sandbox/browser permission issue. Prioritize rerunning `python3 -m pytest tests/ --maxfail=1 -vv` and `python3 -m pytest tests/e2e/test_public_property.py -m smoke --maxfail=1 -vv` after that environment issue is cleared.
