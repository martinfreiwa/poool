# Leaderboard API Reference

Investor-ranking endpoints. All routes require a valid session cookie unless
noted; admin-only routes additionally check `is_admin`.

Base URL (production): `https://platform.poool.app`
Base URL (local dev):  `http://localhost:8888`

## Authentication

Session cookie set by `/auth/login`. CSRF token required on PUT/POST (sent as
header `X-CSRF-Token` — value mirrored from the `csrf_token` cookie by the
frontend interceptor in `frontend/platform/components/head.html`).

## Rate Limiting

Per-user, per-endpoint token bucket. Keys: `lb:get:{user_id}`, `lb:prefs:{user_id}`,
`lb:refresh:{user_id}`. A 429 response includes `Retry-After` in seconds.

---

## GET /api/leaderboard

List rankings for a given metric + timeframe.

### Query parameters

| Name | Type | Default | Notes |
|------|------|---------|-------|
| `metric` | enum | `invested` | One of: `invested`, `assets`, `roi`, `affiliates`, `revenue`, `highest_inv`. Unknown values fall back to `invested`. |
| `timeframe` | enum | `alltime` | One of: `alltime`, `weekly`, `monthly`. |
| `page` | int | `1` | 1-based. |
| `per_page` | int | `25` | Clamped to `1..=100`. |
| `search` | string | — | Case-insensitive `display_name ILIKE '%…%'`. |
| `tier_id` | uuid | — | Filter by user's portfolio tier. |

### Response (200)

```json
{
  "rankings": [
    {
      "rank": 1,
      "user_id": "9f3a…",
      "display_name": "Alice K.",
      "avatar_url": "https://storage.googleapis.com/…",
      "tier_id": "…",
      "tier_name": "Premium",
      "tier_badge_color": null,
      "metric_value": 12500000,
      "metrics": {
        "total_invested_cents": 12500000,
        "asset_count": 18,
        "portfolio_roi_bps": 1340,
        "affiliate_count": 7,
        "referral_revenue_cents": 480000,
        "highest_investment_cents": 5000000
      },
      "asset_mix": [
        { "category": "real_estate", "pct": 0.55 },
        { "category": "commodities", "pct": 0.30 },
        { "category": "art",         "pct": 0.15 }
      ]
    }
  ],
  "page": 1,
  "per_page": 25,
  "total": 142,
  "total_pages": 6,
  "last_updated": "2026-05-16T13:00:00Z",
  "metric": "invested",
  "timeframe": "alltime"
}
```

`asset_mix` is populated **only for top-3 rows** to keep query cost bounded.
Rows 4+ have `"asset_mix": null`.

### Caching

- `Cache-Control: private, max-age=30`
- `ETag` derived from `last_updated` + query params. If client sends
  `If-None-Match`, server returns `304 Not Modified` with no body when matched.

### Errors

| Status | Reason |
|--------|--------|
| 401 | No session |
| 429 | Rate limit exceeded (bucket `lb:get`) |
| 500 | DB error — logged + Sentry |

---

## GET /api/leaderboard/me

Return the authenticated user's rank under the given metric + timeframe.

### Query parameters

Same as `GET /api/leaderboard` but `page`/`per_page`/`search`/`tier_id` ignored.

### Response (200)

```json
{
  "rank": 47,
  "metric_value": 245000,
  "metric": "invested",
  "timeframe": "alltime",
  "total_visible_participants": 142,
  "visible": true,
  "percentile": 0.67
}
```

`visible: false` is returned if the user has opted out — `rank` will be `null`
in that case.

### Errors

| Status | Reason |
|--------|--------|
| 401 | No session |
| 500 | DB error |

---

## GET /api/leaderboard/preferences

Read the authenticated user's visibility preferences.

### Response (200)

```json
{
  "visible": true,
  "show_avatar": true,
  "display_name": "Alice K."
}
```

`display_name` is `null` until the user sets a custom value (default render
falls back to first name + last-initial in the table).

---

## PUT /api/leaderboard/preferences

Partial update — any field omitted from the body is left unchanged.

### Request body

```json
{
  "visible": false,
  "show_avatar": false,
  "display_name": "Anonymous"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `visible` | bool | When `false`, user is filtered out of public rankings entirely (server-side WHERE clause). |
| `show_avatar` | bool | When `false`, avatar field returned as `null` in rankings even if `visible=true`. |
| `display_name` | string \| null | 1–60 chars, trimmed. `null` resets to the platform-default render. |

### Response (200)

Returns the new full state — same shape as `GET /preferences`. Also writes an
audit-log row with `actor_user_id`, IP, UA.

### Errors

| Status | Reason |
|--------|--------|
| 400 | `display_name` too long / invalid |
| 401 | No session |
| 429 | Rate limit exceeded (bucket `lb:prefs`) |

---

## POST /api/leaderboard/refresh

**Admin-only.** Triggers `refresh_all_scores()` immediately instead of waiting
for the 15-minute background tick.

### Response (200)

```json
{
  "status": "refreshed",
  "computed_at": "2026-05-16T13:24:11Z",
  "rows_upserted": 1428
}
```

### Errors

| Status | Reason |
|--------|--------|
| 401 | No session |
| 403 | Caller is not admin |
| 429 | Rate limit exceeded (bucket `lb:refresh`) |
| 500 | Refresh job failed — logged + Sentry |

---

## Metric semantics

Computed from production tables on each refresh. All cent values are
`bigint` (avoid float drift).

| Metric | Source | Notes |
|--------|--------|-------|
| `total_invested_cents` | `SUM(investments.purchase_value_cents WHERE status='active')` | Lifetime capital deployed. |
| `asset_count` | `COUNT(DISTINCT investments.asset_id WHERE status='active')` | Diversification proxy. |
| `portfolio_roi_bps` | Weighted ROI across active positions, in basis points (1bp = 0.01%). | Negative values rendered with `-` prefix client-side. |
| `affiliate_count` | `COUNT(DISTINCT referral_tracking.referred_user_id)` | Direct referrals only (no second-degree). |
| `referral_revenue_cents` | `SUM(referral_tracking.commission_cents WHERE paid_at IS NOT NULL)` | Lifetime paid affiliate revenue. |
| `highest_investment_cents` | `MAX(investments.purchase_value_cents)` | Largest single position. |

## Timeframe semantics

- `alltime` reads precomputed `rank_*` columns on `leaderboard_scores`
  (O(log n) lookup via per-metric indexes).
- `weekly` filters `investments.purchased_at >= NOW() - INTERVAL '7 days'` and
  recomputes `ROW_NUMBER()` at query time.
- `monthly` filters with `INTERVAL '30 days'`. Same recomputation path.

`weekly` and `monthly` therefore do **not** consult the precomputed ranks —
they are recalculated per request, which scales O(n log n) over the active
investment set. Materialised views are a planned optimisation for >10k users.

## Privacy contract

- A user with `leaderboard_preferences.visible = false` is NEVER returned by
  `GET /api/leaderboard` — they are filtered at the DB level, not blanked.
- `show_avatar = false` returns `avatar_url: null` in rankings.
- The caller is always excluded from their own listing (use `/me` to fetch
  their own rank).
- Metric values for hidden users remain queryable only via `/me`.

## Schema reference

- Tables: `leaderboard_scores`, `leaderboard_preferences`
- Migrations: `023_leaderboard.sql`, `024_leaderboard_display_name.sql`,
  `025_leaderboard_metrics.sql`, `046_leaderboard_cleanup.sql`

## Operations

### Background refresh worker

- **Cadence:** every `POOOL_LEADERBOARD_REFRESH_SECS` seconds (default `900`,
  floor `60`). Each tick refreshes ALL three precomputed tables under one
  Postgres advisory lock: `leaderboard_scores` (all-time),
  `leaderboard_scores_weekly`, `leaderboard_scores_monthly`.
- **Cluster safety:** the advisory lock (`LEADERBOARD_REFRESH_LOCK_KEY = 0x10ADBD0001`)
  ensures only one instance refreshes per tick, even with horizontal scaling.
  Instances that miss the lock log `leaderboard refresh skipped` at DEBUG and
  wait for the next tick.
- **Startup jitter:** random 0–300s sleep before the first tick to stagger
  multi-instance fleets.

### Structured log keys for ops dashboards

The refresh worker emits one of these per tick — grep / alert on the
`metric_name` field:

| `metric_name`                              | Level | Fields                                          | Meaning                                    |
|---------------------------------------------|-------|-------------------------------------------------|--------------------------------------------|
| `leaderboard_refresh_duration_ms`           | INFO  | `elapsed_ms`, `timeframes`                      | Successful refresh of all 3 timeframes.    |
| `leaderboard_refresh_failure`               | ERROR | `consecutive_failures`, `error`, `elapsed_ms`   | Refresh failed for at least one timeframe. |
| `leaderboard_refresh_consecutive_failures`  | WARN  | `value`                                         | ≥3 fails in a row — page on-call.          |

### Sentry alert recipe

Add this rule in the Sentry project that ingests platform errors:

- **Condition:** `event.message` contains `Background leaderboard refresh failed`
- **Frequency:** more than 2 occurrences in 30 minutes
- **Action:** page on-call (Slack `#platform-alerts` + PagerDuty)

This catches sustained refresh failure without firing on a single transient
DB hiccup (which the worker swallows + retries on the next tick).

### Manual refresh

Admins can force a refresh from the page UI (button is hidden for non-admins)
or via the API: `POST /api/leaderboard/refresh`. Useful when an investment
batch lands close to a "release moment" and the next scheduled tick is too
far away to be visible to users.

### Performance characteristics

- **Read latency (all timeframes):** O(log n) index lookup on
  `rank_*` column → typically <20ms for page 1 of 25 rows. Identical cost
  whether `timeframe=alltime`, `weekly`, or `monthly` after the
  pre-computation refactor (migration 168).
- **Refresh latency:** ~100–500ms per timeframe table on a 50k-user fleet
  (3 ROW_NUMBER passes over the active investment set). Surfaced via
  `leaderboard_refresh_duration_ms` log so regressions trigger an alert.
- **ETag short-circuit:** 304 Not Modified returned in ~5ms when
  `If-None-Match` matches the current `last_updated` hash.

## See also

- Source: `backend/src/leaderboard/{routes.rs, service.rs, models.rs}`
- Frontend: `frontend/platform/leaderboard.html`, `static/js/leaderboard.js`
- Tests: `backend/tests/leaderboard_integration.rs` (18 tests),
  `backend/tests/leaderboard_http.rs` (11 tests),
  `tests/e2e/test_leaderboard.py` (7 tests). **Total: 36.**
- Migrations: `023`, `024`, `025`, `046`, `168` (precomputed weekly/monthly),
  `176` (column rename), `177` (snapshots).
- Audit close-out: `docs/page-audits/2026-05-11-leaderboard-closeout.md`,
  this file's gap-closure pass: 2026-05-16.
