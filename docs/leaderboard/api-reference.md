# Leaderboard API Reference

Routes registered in `backend/src/leaderboard/mod.rs`. All responses are JSON
unless otherwise noted. All write endpoints require a valid session cookie
(`poool_session`) and CSRF token (`X-CSRF-Token` header matching the
`csrf_token` cookie). All routes are per-user rate-limited (60 req/min per
endpoint key — see [Runbook → Rate limits](./runbook.md#rate-limits)).

## Common response codes

| Status | Meaning |
|---|---|
| `200 OK` | Success. Body is JSON. |
| `304 Not Modified` | ETag matched the request's `If-None-Match` — body is empty. Browsers reuse the cached payload. |
| `401 Unauthorized` | No valid session cookie. |
| `403 Forbidden` | Authenticated but lacks the role for this endpoint (admin-only routes). |
| `405 Method Not Allowed` | Refresh endpoint accessed via GET. |
| `429 Too Many Requests` | Rate limit exceeded. `Retry-After` header tells you when to retry. |
| `500 Internal Server Error` | Unexpected backend failure. The error is logged + sent to Sentry. |

---

## Endpoints

### `GET /api/leaderboard`

Public listing of ranked investors for a given metric + timeframe.

**Query parameters**

| Param | Type | Default | Notes |
|---|---|---|---|
| `metric` | string | `invested` | One of `invested`, `assets`, `roi`, `affiliates`, `revenue`, `highest_inv`. Unknown values fall back to `invested` (audited in `unknown_metric_falls_back_to_invested`). |
| `timeframe` | string | `alltime` | One of `alltime`, `weekly`, `monthly`. Weekly/Monthly recompute at query time, all-time reads precomputed scores. |
| `page` | int ≥ 1 | `1` | 1-indexed. Values `< 1` are clamped. |
| `per_page` | int 1..=100 | `10` | Hard cap at 100. |
| `tier_id` | int (1..=5) | none | Filter to a specific tier. `NULL` tier rows default to Intro (1). |
| `search` | string | none | ILIKE `%search%` against the resolved display name. `%` and `_` act as wildcards. SQL-safe (parameterized). |

**Response headers**

```
ETag: "lb-<hex>"
Cache-Control: private, max-age=30
```

The ETag is a hash of `(last_updated, metric, timeframe, page, per_page, tier_id, search)`. Send it back in `If-None-Match` to short-circuit subsequent requests.

**Response body**

```json
{
  "rankings": [
    {
      "rank": 1,
      "display_name": "Alice Schmidt",
      "avatar_url": "https://cdn.example/avatars/alice.png",
      "tier_name": "Premium",
      "tier_badge_color": "#0000FF",
      "metric_value": 686679500,
      "is_current_user": false,
      "metrics": {
        "total_invested_cents": 686679500,
        "asset_count": 9,
        "portfolio_roi_bps": 721,
        "affiliate_count": 0,
        "referral_network_value_cents": 0,
        "highest_investment_cents": 250000000
      },
      "asset_mix": [
        { "asset_type": "real_estate", "invested_cents": 680000000, "asset_count": 8 },
        { "asset_type": "commodity",   "invested_cents":   6679500, "asset_count": 1 }
      ]
    }
  ],
  "my_rank": {
    "rank": null,
    "metric_value": 0,
    "metrics": { "total_invested_cents": 0, "asset_count": 0, "portfolio_roi_bps": 0, "affiliate_count": 0, "referral_network_value_cents": 0, "highest_investment_cents": 0 }
  },
  "total_participants": 4,
  "metric_type": "invested",
  "timeframe": "alltime",
  "last_updated": "2026-05-16T13:42:01Z",
  "has_more": false
}
```

**Field notes**

- `display_name`: resolution order is `leaderboard_preferences.display_name` → `user_profiles.display_name` → `first_name + last_initial`. Pseudonym placeholders no longer appear (see [Visibility Semantics](./visibility-semantics.md)).
- `avatar_url`: `null` unless **both** `visible=true` AND `show_avatar=true`. The viewer always sees their own avatar.
- `metric_value`: the integer behind the active sort metric. Unit depends on `metric_type` (cents for monetary, count for `assets`/`affiliates`, bps for `roi`).
- `metrics.portfolio_roi_bps`: investment-weighted **target** yield, not realized return. UI labels it as "Avg Target Yield" to avoid confusion.
- `metrics.referral_network_value_cents`: sum of referees' active investment value (network volume), not commission earned. UI labels it as "Network Volume".
- `asset_mix`: populated only for the **top-3** rows. Position 4+ returns an empty array — performance contract enforced by `asset_mix_only_populated_for_top_3`.
- `is_current_user`: always `false` in this endpoint (the viewer is excluded from the listing). Surfaced via `my_rank` instead.
- `my_rank.rank`: `null` for users with no `leaderboard_scores` row. `metric_value` and `metrics.*` default to `0`.
- `last_updated`: ISO-8601 UTC of the last successful refresh. May be `null` on weekly/monthly mode (always-live aggregation).
- `has_more`: `(offset + rankings.len()) < total_participants`. Don't rely on `rankings.len() == per_page` — that breaks on the last full page (regression test: `has_more_false_when_total_is_exact_multiple_of_per_page`).

---

### `GET /api/leaderboard/me`

Returns the viewer's own rank + metrics for the given metric + timeframe. Same query params as `/api/leaderboard` (only `metric` and `timeframe` are used). Always 1 row.

**Response body**

```json
{
  "rank": 5,
  "metric_value": 4200000,
  "metrics": { /* same shape as above */ }
}
```

Returns `MyRank::default()` (all zeros, `rank: null`) if the user has no `leaderboard_scores` row — never 500.

---

### `GET /api/leaderboard/preferences`

Returns the current viewer's leaderboard preferences.

**Response body**

```json
{
  "visible": true,
  "show_avatar": false,
  "display_name": "AlphaTrader"
}
```

`display_name` is `null` when unset. `visible` defaults to `false` for new users (opt-in).

---

### `PUT /api/leaderboard/preferences`

Partial update — any field omitted from the body keeps its current DB value (`COALESCE` for booleans; CASE/sentinel for nullable display_name).

**Request body** (all fields optional)

```json
{
  "visible": true,
  "show_avatar": false,
  "display_name": "AlphaTrader"
}
```

To clear `display_name` send `"display_name": ""` (empty string is normalized to NULL after trim).

**Side effects**

- **Audit log**: every successful PUT writes a row to `audit_logs` with `action="leaderboard.prefs.update visible=... show_avatar=... display_name_set=..."`, `entity_type="leaderboard_preferences"`, and the request's `X-Forwarded-For` + `User-Agent`. Audit-log write failures are warned but do not block the response.
- **Visibility flip**: toggling `visible: true → false` removes the user from public listings on the next `GET /api/leaderboard` for any viewer. The user themselves still sees their rank via `my_rank`.
- **Cross-page sync**: this endpoint and `POST /api/settings/leaderboard` write to the same `leaderboard_preferences` row, so toggling on either Settings or the Leaderboard page is universal.

**Response body**: the merged preferences (same shape as GET).

---

### `POST /api/leaderboard/refresh`

**Admin-only.** Triggers an immediate refresh of `leaderboard_scores` for all active users, then warms the `last_updated` cache. Used to bypass the 15-minute background cadence after a manual data correction.

**Request**: empty body. Requires the `X-CSRF-Token` header.

**Response**

```json
{ "status": "success", "message": "Leaderboard scores refreshed." }
```

- Non-admin caller → `403 Forbidden`.
- GET on this URL → `405 Method Not Allowed` (regression test: `refresh_get_returns_405`).
- Refresh failure → `500`, error logged + Sentry-captured.

---

## OpenAPI snippet

The leaderboard routes use plain JSON without a code-first OpenAPI generator (utoipa). If you need a machine-readable spec for client codegen, drop the following into a `openapi.yaml`:

```yaml
openapi: 3.0.3
info:
  title: POOOL Leaderboard API
  version: "1.0"
paths:
  /api/leaderboard:
    get:
      summary: Get ranked leaderboard listing
      parameters:
        - { in: query, name: metric, schema: { type: string, enum: [invested, assets, roi, affiliates, revenue, highest_inv] } }
        - { in: query, name: timeframe, schema: { type: string, enum: [alltime, weekly, monthly] } }
        - { in: query, name: page, schema: { type: integer, minimum: 1 } }
        - { in: query, name: per_page, schema: { type: integer, minimum: 1, maximum: 100 } }
        - { in: query, name: tier_id, schema: { type: integer, minimum: 1, maximum: 5 } }
        - { in: query, name: search, schema: { type: string } }
        - { in: header, name: If-None-Match, schema: { type: string } }
      responses:
        "200": { description: OK, headers: { ETag: { schema: { type: string } }, Cache-Control: { schema: { type: string } } } }
        "304": { description: Not Modified }
        "401": { description: Unauthorized }
        "429": { description: Rate limited, headers: { Retry-After: { schema: { type: integer } } } }
  /api/leaderboard/me:
    get: { summary: Get current viewer's rank, responses: { "200": { description: OK } } }
  /api/leaderboard/preferences:
    get: { summary: Get viewer's preferences, responses: { "200": { description: OK } } }
    put:
      summary: Update viewer's preferences (partial)
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                visible: { type: boolean }
                show_avatar: { type: boolean }
                display_name: { type: string, nullable: true }
      responses:
        "200": { description: OK }
        "429": { description: Rate limited }
  /api/leaderboard/refresh:
    post:
      summary: Admin manual refresh
      responses:
        "200": { description: OK }
        "403": { description: Forbidden — admin role required }
        "405": { description: Method Not Allowed (GET) }
```
