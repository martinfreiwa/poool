# Leaderboard Runbook

Operations playbook for the on-call engineer. Walks through the three
incident classes that have actually happened in development (stale data,
refresh failure, hot user-reported bug) plus the routine deployment and
test workflows.

---

## TL;DR

- **Stale leaderboard** → trigger manual refresh via admin UI button OR `POST /api/leaderboard/refresh`.
- **Refresh job silently broken** → check Sentry for `Background leaderboard refresh failed:` events, then `tail -f` the backend logs filtered to `leaderboard refresh`.
- **User claims they're invisible / still visible** → check `leaderboard_preferences` row directly in DB. See [Visibility Semantics](./visibility-semantics.md) for what the value means.
- **Deploy** → ship code, then if you changed default-visibility behavior, run the [Migration Guide](./migration-guide.md).

---

## Architecture cheat sheet

```
┌──────────────────────────┐
│ Browser (HTMX-free SPA)  │  caches via ETag + max-age=30
└──────────┬───────────────┘
           │ GET /api/leaderboard?metric=…&timeframe=…
           ▼
┌──────────────────────────┐    rate-limit (60 req/min/user)
│ leaderboard/routes.rs    │    ETag short-circuit (cached MAX(computed_at))
└──────────┬───────────────┘
           │ service::get_rankings
           ▼
┌──────────────────────────┐    CTEs over leaderboard_scores
│ leaderboard/service.rs   │    + user_profiles + tiers + lp
└──────────┬───────────────┘    + asset-mix enrichment for top-3
           │
           ▼
┌──────────────────────────┐
│ Postgres: leaderboard_*  │  refreshed every 15 min by background tokio task
└──────────────────────────┘
```

---

## Routine procedures

### Trigger a manual refresh

When the 15-min background tick is too slow (e.g. a user just made a large investment and wants to see their rank update immediately).

**Via UI** (admin only):

1. Open `/leaderboard` while signed in with an admin role.
2. Click **Refresh now** in the "Global Tier Listings" header.
3. Wait for the status pill to show "Leaderboard refreshed."

**Via curl**:

```sh
# Use a real admin session cookie + matching CSRF token.
curl -X POST https://api.poool.example/api/leaderboard/refresh \
  -H "Cookie: poool_session=<token>; csrf_token=<csrf>" \
  -H "X-CSRF-Token: <csrf>"
# → { "status": "success", "message": "Leaderboard scores refreshed." }
```

The handler runs `refresh_all_scores` synchronously (≈100-500ms on a healthy DB up to ~10k users). It's idempotent (`refresh_is_idempotent_on_repeat` test guards this) — safe to re-run.

### Clear the `last_updated` cache

The in-process cache lives in `AppState.leaderboard_last_refresh`. It hydrates from `SELECT MAX(computed_at) FROM leaderboard_scores` on cold start and gets stamped by every refresh.

To force a re-read **without** restarting the backend:

```sh
# Easiest: trigger a manual refresh — it writes a fresh timestamp.
curl -X POST /api/leaderboard/refresh -H "..."
```

Restarting the backend also drops the cache. If you suspect the cache is wrong but the DB is right, log on with `RUST_LOG=poool_backend::leaderboard=debug` and look for the next "leaderboard refresh OK" line.

### Invalidate browser caches

Browsers honor `Cache-Control: private, max-age=30` and the `ETag`. If you ship a backend change that alters response shape, bump the JS module version (`leaderboard.js?v=NN` in the HTML templates) — the next page-load fetches a fresh asset that may re-issue requests.

ETag mismatches automatically invalidate browser caches without any manual intervention.

---

## Incident response

### Symptom: Leaderboard shows yesterday's numbers

1. **Confirm**: hit `/api/leaderboard` and check the `last_updated` field. If it's >30 min old, refresh is broken.
2. **Check Sentry** for `Background leaderboard refresh failed:` issues.
3. **Check backend logs**:
   ```sh
   journalctl -u poool-backend -f | grep -E "leaderboard|refresh"
   ```
   Look for the periodic `leaderboard refresh OK elapsed_ms=…` lines.
4. **Run manual refresh** (above). If it fails with the same error, the DB schema or constraints likely changed.
5. **Read the error carefully** — most failures come from:
   - **Foreign-key constraint** (e.g. a `users.id` referenced by `leaderboard_scores` got hard-deleted instead of soft-deleted). Fix the orphan, re-run.
   - **DB lock timeout** (long-running migration in progress). Wait it out.
   - **Disk full** in the rare case of unbounded growth — `leaderboard_snapshots` retains forever; consider a prune job.

### Symptom: User says they enabled "Show on Leaderboard" but still don't appear

1. **Check the DB row** directly:
   ```sql
   SELECT visible, show_avatar, display_name, updated_at
   FROM leaderboard_preferences
   WHERE user_id = '<uuid>';
   ```
   If `visible = false` → the toggle didn't save (check Sentry for PUT failures; check audit_logs row presence).
2. **Check leaderboard_scores has a row**:
   ```sql
   SELECT total_invested_cents, rank_invested
   FROM leaderboard_scores
   WHERE user_id = '<uuid>';
   ```
   If no row → user has no active investments AND no referrals → won't appear regardless of `visible`. Verify their investment status.
3. **Run a manual refresh** — could be that a new investment hasn't been picked up by the 15-min job yet.
4. **Sanity-check the listing query**:
   ```sql
   SELECT user_id FROM leaderboard_scores ls
   LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
   WHERE COALESCE(lp.visible, false) = true
     AND ls.rank_invested IS NOT NULL
   ORDER BY ls.total_invested_cents DESC LIMIT 20;
   ```
   The user's ID should be in the result if they expect to be public.

### Symptom: User says someone else's name appears with their data

This shouldn't happen. The flow is:
1. `leaderboard_scores` is keyed by `user_id` (UNIQUE constraint).
2. `display_name` resolution: `lp.display_name` → `up.display_name` → `up.first_name + last_initial`.

If you really see swapped data:
- Check `audit_logs` for an unexpected `leaderboard.prefs.update` row.
- Check `user_profiles` for both users — possibly a profile-merge bug elsewhere.
- File a P0 — privacy data leak.

### Symptom: `429 Too Many Requests` from a power user

Rate-limit is **60 req/min per user, per endpoint key**. The endpoint key is `lb:get`, `lb:prefs`, or `lb:refresh`. A power user hammering F5 should not normally hit this — investigate whether their frontend is in a retry loop.

To override per-environment, set in `.env`:
```
POOOL_RATE_LIMIT_DISABLED=true   # disables ALL limiters; dev only
```

To raise just the leaderboard limit, edit `backend/src/lib.rs` (`leaderboard_rate_limiter` initialization). The literal `60` is intentional — bump cautiously.

### Symptom: ETag never seems to match (no 304 responses)

Likely causes:
- The frontend is sending the ETag in the wrong header (must be `If-None-Match`, not `If-Match`).
- The user just toggled visibility → `lp.visible` change forces the listing to recompute; ETag is still stable but the BODY changes. Make sure the test isn't comparing the ETag across a visibility flip.
- The query parameters differ between requests — even a `?page=1` vs `?page=01` would hash differently. Normalize on the client.

---

## Rate limits

| Endpoint | Key | Cap |
|---|---|---|
| `GET /api/leaderboard` | `lb:get:<user_id>` | 60 req / 60s |
| `PUT /api/leaderboard/preferences` | `lb:prefs:<user_id>` | 60 req / 60s |
| `POST /api/leaderboard/refresh` | `lb:refresh:<user_id>` | 60 req / 60s |

When Redis is configured (`REDIS_URL`), limits are global across backend instances. Otherwise they're in-memory per-instance — adjust horizontal scaling accordingly.

To increase a single user's allowance (e.g. for a load test):
```sql
-- Whitelist via flag (NOT IMPLEMENTED — needs a small middleware change).
-- Easier: temporarily set POOOL_RATE_LIMIT_DISABLED=true on a canary instance.
```

---

## Observability

### Tracing

Every handler has `#[tracing::instrument]`. Filter by:

```sh
RUST_LOG=poool_backend::leaderboard=debug,poool_backend=info cargo run
```

Useful event signatures to grep:
- `Server listening on http://0.0.0.0:PORT` — backend up
- `leaderboard refresh OK elapsed_ms=N` — happy 15-min tick
- `Error refreshing leaderboard scores` — refresh job failure (also Sentry-captured)
- `audit log for leaderboard prefs failed` — audit row write failed (non-blocking; investigate but user save still succeeded)

### Sentry

Three Sentry events come from leaderboard code paths:

| Message prefix | Source | Severity |
|---|---|---|
| `Background leaderboard refresh failed:` | tokio task in `lib.rs` | Error |
| `Manual leaderboard refresh failed:` | admin POST `/api/leaderboard/refresh` handler | Error |
| `Leaderboard listing failed:` | GET `/api/leaderboard` handler | Error |
| `Leaderboard prefs update failed:` | PUT `/api/leaderboard/preferences` handler | Error |

Configure a Sentry alert rule on **any** event matching `message:"leaderboard"` and route to the on-call channel.

### Audit log

Every preference change writes a row to the immutable `audit_logs` table.

Query recent prefs changes:
```sql
SELECT actor_user_id, action, ip_address, user_agent, created_at
FROM audit_logs
WHERE action LIKE 'leaderboard.prefs.update%'
ORDER BY created_at DESC
LIMIT 50;
```

Sample action string:
```
leaderboard.prefs.update visible=Some(true) show_avatar=Some(false) display_name_set=true
```

`display_name_set=true` means the request body included a `display_name` field (the actual value is NOT logged — privacy). Use `leaderboard_preferences.display_name` for the current value.

### Metrics worth scraping

The `tracing` events emit structured fields you can collect with Vector / Promtail / OpenTelemetry:

| Field | From | Use it for |
|---|---|---|
| `elapsed_ms` | refresh job logs | Alert if p95 > 5000 (refresh slowdown) |
| `user_id`, `metric`, `timeframe`, `page` | get_rankings spans | Top-N-most-requested combinations |
| `error` | all `tracing::error!` calls | Aggregate failure rate per handler |

---

## Test execution

### Run all leaderboard tests

```sh
DATABASE_URL=postgres://martin@localhost/poool \
  cargo test --test leaderboard_integration \
             --test leaderboard_http \
             --test leaderboard_roi_precision \
             --test leaderboard_production_audit \
  -- --ignored --test-threads=1
```

**Total: 36 tests.** All `#[ignore]`d because they hit a live Postgres.

| Suite | Count | Covers |
|---|---|---|
| `leaderboard_integration` | 11 | refresh, pagination, has_more derivation, tier filter, prefs partial update, visibility filter, self-exclusion |
| `leaderboard_http` | 11 | full Axum router via oneshot — me/refresh auth, prefs partial, ETag round-trip, Cache-Control header, audit-log row, rate-limit |
| `leaderboard_roi_precision` | 1 | weighted-bps ROI math is exact on mixed yields |
| `leaderboard_production_audit` | 13 | edge cases + security: empty dataset, per_page caps, asset-mix enrichment contract, SQL injection in search, pseudonym leak regression, avatar hidden, unknown metric fallback |

### When a test fails

1. Re-run with `RUST_LOG=debug` and `--nocapture`:
   ```sh
   DATABASE_URL=… cargo test --test leaderboard_integration -- --ignored --nocapture
   ```
2. Common false-positives:
   - **Stale test data** from a previously-failed run polluting the cohort. Each test starts with a per-uuid cleanup pass but a `panic!` between insert+cleanup leaves orphans. Manually run:
     ```sql
     DELETE FROM leaderboard_preferences WHERE display_name LIKE 'HMTEST_%' OR display_name LIKE 'PROD_%';
     DELETE FROM leaderboard_scores ls WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = ls.user_id);
     ```
   - **Background refresh job overwriting test fixtures** — only an issue if the test takes longer than the 15-min refresh tick AND you're running against a real environment, not local.

---

## Deployment

1. **Standard deploy** (no schema or default-visibility change):
   - Push → CI runs the full leaderboard test suite (36 tests).
   - On green, deploy via the normal `gh workflow run deploy.yml`.
   - Backend restart drops the `last_updated` cache; first request re-hydrates from `SELECT MAX(computed_at)`.
   - No DB migration step needed.

2. **Schema change** (e.g. adding a new column to `leaderboard_scores`):
   - Add SQL migration to `database/` and update the inline `refresh_all_scores` query in `service.rs`.
   - Coordinate so the old backend (pre-deploy) doesn't write to the new column — additive only.
   - Run migration first, then deploy. Background refresh on next 15-min tick picks up the new column.

3. **Default-visibility change** (e.g. flipping `visible` default from `false` → `true`):
   - This affects PRIVACY. Follow the [Migration Guide](./migration-guide.md) exactly.

---

## Useful one-liners

```sh
# Count visible users in the leaderboard
psql $DB -c "SELECT COUNT(*) FROM leaderboard_preferences WHERE visible = true;"

# Top 5 with full display name + tier
psql $DB <<'SQL'
SELECT ls.rank_invested, COALESCE(lp.display_name, up.display_name, up.first_name) AS name,
       t.name AS tier, ls.total_invested_cents/100 AS eur
FROM leaderboard_scores ls
LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
LEFT JOIN user_profiles up ON up.user_id = ls.user_id
LEFT JOIN user_tiers ut ON ut.user_id = ls.user_id
LEFT JOIN tiers t ON t.id = COALESCE(ut.tier_id, 1)
WHERE COALESCE(lp.visible, false) = true
ORDER BY ls.rank_invested ASC LIMIT 5;
SQL

# Inspect last refresh timestamp
psql $DB -c "SELECT MAX(computed_at) FROM leaderboard_scores;"

# Force-set a single user to visible (for support/debug; audit-log it manually)
psql $DB -c "INSERT INTO leaderboard_preferences (user_id, visible, show_avatar) VALUES ('<uuid>', true, false) ON CONFLICT (user_id) DO UPDATE SET visible = true;"

# Recent rate-limit hits (if Redis-backed — needs redis-cli access)
redis-cli --scan --pattern "rate:lb:*" | head -20
```
