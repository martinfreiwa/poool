# Villa-Returns — Cut-Over Runbook (Phase 3)

> Operational runbook for moving from legacy `asset_financials` to the Villa-Returns layer in production. Each step gated by stakeholder sign-off.

---

## Current state (post-Phase 1+2)

- **`platform_settings.villa_returns.enabled`** = `on` (default) | `shadow` | `off`
- **`assets.villa_returns_pilot`** = `TRUE` (default per migration 146) per asset
- Read path: investor performance API reads from `villa_operations_*` when both `enabled='on'` AND `pilot=TRUE`. Otherwise returns zeros (no legacy fallback).
- Write path: publish handler shadow-writes to `asset_financials` when `enabled != 'on'`.
- Background: daily NAV snapshot via `VILLA_NAV_SNAPSHOT_ENABLED=true` env var (Cloud Run).

---

## Phase 3 step sequence

### Step C1 — Shadow-write running (DONE)

- ✅ Backend writes to both `villa_operations_log` AND `asset_financials` when `enabled='shadow'`.
- ✅ Verified in dev: row published in shadow mode appears in both tables; row published in 'on' mode appears only in new layer.

### Step C2 — Pilot cut-over (DONE in code)

- ✅ Per-asset `villa_returns_pilot` flag wired into investor read gate.
- **Operational plan:**
  1. Set `platform_settings.villa_returns.enabled = 'shadow'` in prod.
  2. Flip 1 production villa's `villa_returns_pilot = TRUE`; all others `FALSE`.
  3. Monitor for 14 days:
     - Reads: pilot asset returns Villa-Returns data; others return zeros.
     - Writes: every publish creates rows in BOTH tables. Diff them daily.
  4. If divergence < 0.5% across 14 days → expand to 25% of villas.
  5. After another 14 days → expand to 100%.
  6. Then proceed to C3.

### Step C3 — Full read cut-over (no-op for this implementation)

The original plan called for "remove legacy fallback from villa_performance.rs". This implementation never wired a legacy read fallback (`off` mode returns zeros, not legacy data). Step is effectively a NOP — no code to remove.

If a future legacy-fallback gets added (e.g. for analytics or compat with other consumers), revisit this step.

### Step C4 — Multi-currency expansion (PARTIAL)

- ✅ Wallet & wallet_transactions inserts route through `assets.payout_currency` (no longer hardcoded `'USD'`).
- ⚠️ Distribute endpoint still hard-fails for currencies other than `USD` or `IDR`. To enable EUR/USDT:
  1. Populate `fx_rates_daily` for the new currency pair (e.g. IDR→EUR).
  2. Extend the match block in `api_admin_villa_operations_distribute` to handle the new currency by looking up the latest snapshot rate + converting `distributable_idr_cents`.
  3. Test with a single EUR-denominated asset before broader rollout.

### Step C5 — Drop legacy `asset_financials` (PENDING — gated)

- Migration files staged with `.PENDING` suffix (NOT picked up by runner):
  - `database/rollback/148_rename_asset_financials_deprecated.sql.PENDING`
  - `database/rollback/149_drop_asset_financials_deprecated.sql.PENDING`
- **Required before applying 148 (rename):**
  - 90 consecutive days of zero reads on `asset_financials` (verify via `pg_stat_user_tables.seq_scan + idx_scan` delta).
  - Shadow-write disabled (`enabled='on'` in prod).
  - Stakeholder sign-off (the user) on this runbook entry.
- **Required before applying 149 (drop):**
  - Migration 148 executed at least 30 days ago.
  - GCS cold-storage backup verified.

### Step C6 — Per-investor jurisdiction tax (OPEN SCOPE)

- Per-asset `assets.withholding_tax_bps` works today.
- Per-investor jurisdiction layer was Q10-deferred and is open scope — coordinate with legal/compliance.

---

## Rollback procedures

### If divergence > 0.5% during C2 pilot:

1. `UPDATE platform_settings SET value='off' WHERE key='villa_returns.enabled';`
2. Investigate divergence: query `villa_operations_current` vs `asset_financials` for diffs.
3. Fix root cause; reset to `'shadow'` mode.

### If a published row needs to be removed (emergency):

1. Append-only triggers prevent DELETE. Use the supersession chain — create a new row with corrected values + `supersedes_id` set.
2. For wallet credits already paid: run **top-up endpoint** with the corrected row for positive delta; negative deltas absorbed per Q11 policy.

### If C5 rename causes app errors:

1. `ALTER TABLE asset_financials__deprecated_YYYYMMDD RENAME TO asset_financials;`
2. Re-enable shadow-write: set `enabled='shadow'` in `platform_settings`.
3. Diagnose the offending code path; commit fix.

---

## Monitoring

| Signal | What it tells you |
|---|---|
| `villa_market_prices_daily` row count vs days since start | Daily NAV snapshot job is running |
| `audit_logs WHERE action='villa_ops.publish'` count | Publish rate per period |
| `notifications WHERE type='payout' AND created_at > NOW() - INTERVAL '7 days'` | Payouts hitting investors |
| `pg_stat_user_tables.seq_scan + idx_scan` for `asset_financials` | Reads against legacy table; should trend to zero |
| Reserve fund balance (per asset) | Reserve_pct_bps applied correctly per period |

---

## Sign-off log

| Date | Phase 3 step | Approver | Notes |
|---|---|---|---|
| _pending_ | C2 pilot to 1 asset | _user_ | Awaiting prod readiness |
| _pending_ | C2 pilot expansion (25%) | _user_ | After 14 days monitoring |
| _pending_ | C2 pilot expansion (100%) | _user_ | After another 14 days |
| _pending_ | C5 rename migration 148 | _user_ | After 90 days no reads |
| _pending_ | C5 drop migration 149 | _user_ | 30 days after rename |
