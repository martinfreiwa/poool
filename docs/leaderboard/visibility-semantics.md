# Visibility Semantics

The single source of truth for what `leaderboard_preferences.visible` and
`show_avatar` mean in production, and what every observer sees in each
state combination.

This is a privacy-relevant contract — changes here affect what users can
infer about each other. If you change behavior, update this doc *first*
and link the PR back here.

---

## The two-bit privacy model

Every user has exactly two boolean preferences plus one optional string:

| Field | Type | Default for new users | Affects |
|---|---|---|---|
| `visible` | bool | `false` (opt-in) | Whether the user appears in the public listing at all |
| `show_avatar` | bool | `false` | Whether their `users.avatar_url` is exposed |
| `display_name` | nullable text | `NULL` | Public-facing handle override |

Both bools default to `false` so a brand-new investor is never in the
public listing without explicit opt-in.

---

## What each observer sees

There are three observer roles:

1. **The user themselves** (the viewer) — looking at their own data.
2. **Another logged-in user** — looking at the public listing.
3. **Anonymous / unauthenticated** — currently `401 Unauthorized` on all leaderboard endpoints; the entire API requires a session.

| State `(visible, show_avatar)` | The user themselves | Another logged-in user |
|---|---|---|
| `(false, *)` | "Your Standing" card shows their rank + metrics. Listing rows: **other users only**, never themselves. | User does NOT appear in the listing at all. They contribute to no rank, no count. |
| `(true, false)` | Same as above — viewer never appears in their own listing. | User appears in the listing with their resolved display name. **Avatar is `null`** — initials are rendered client-side instead. |
| `(true, true)` | Same as above. | User appears with display name + `users.avatar_url`. |

Key invariant: **the viewer never appears in their own public listing.** This is enforced at the SQL level (`m.user_id <> $current_user`) — see `visible_viewer_excluded_from_own_listing` test.

---

## Display name resolution

When `visible = true`, the resolved `display_name` is the first non-null of:

1. `leaderboard_preferences.display_name` — user-chosen public handle
2. `user_profiles.display_name` — account-level display name
3. `user_profiles.first_name + ' ' + LEFT(last_name, 1) + '.'` — e.g. "Alice S."
4. Literal `'Investor'` — fallback when no profile data at all

Test coverage: `display_name_preference_overrides_user_profile` confirms step 1 wins over step 2.

---

## What the user controls vs what they don't

### User controls

- **Whether they appear** (`visible`)
- **Whether their photo appears** (`show_avatar`)
- **Their handle** (`display_name`)

### User does NOT control

- **Their tier** — derived from 12-month invested volume via `user_tiers`. Visible whenever the user is visible.
- **Their metric values** — the actual `total_invested_cents`, `asset_count`, `portfolio_roi_bps` etc. These are the whole point of the leaderboard; opting in means exposing them.
- **Their rank** — derived from metric values.

If a user wants to hide metric values from the leaderboard, they must set `visible = false`. There is no "show me but not my numbers" mode by design — that would make the leaderboard meaningless.

---

## Visibility changes — what happens when

### `visible: false → true` (opt-in)

- Their `leaderboard_preferences` row is upserted with `visible = true`.
- An `audit_logs` row is written: `action="leaderboard.prefs.update visible=Some(true) …"`.
- **Next** call to `GET /api/leaderboard` (any viewer) sees them in the listing.
- The browser's existing 30s `Cache-Control` may delay this — the user's own `If-None-Match` doesn't apply because their viewer-id-keyed ETag changes the moment a new user joins the cohort. Other viewers' caches expire within 30s.

### `visible: true → false` (opt-out)

- Same audit-log row pattern.
- Listing query re-ranks via `ROW_NUMBER()` over the now-smaller visible set on the next request. **Ranks of remaining visible users shift up** — important for UX expectations.
- The user themselves still sees their own rank in "Your Standing" via the separate `my_rank` query (which does NOT apply the visibility filter for the requesting user's own row).

### `show_avatar: false → true`

- `audit_logs` row.
- Other viewers' next request includes `avatar_url` for this user. The browser may not have the image cached and will fetch it from `users.avatar_url`.

### `display_name: '' → 'Foo'`

- `audit_logs` row. **The value itself is NOT logged** — only the fact a `display_name` was set (`display_name_set=true`).
- The new name appears on the next listing fetch.

### `display_name: 'Foo' → ''`

- The empty string is normalized to `NULL` after trim.
- Resolution falls back to `user_profiles.display_name` etc.

---

## Edge cases & gotchas

### User has no `user_profiles` row

Then resolution lands on `'Investor'`. Unhelpful but never crashes. Audit fix `display_name_preference_overrides_user_profile` documents the chain.

### User is `visible = true` but never invested

They have no `leaderboard_scores` row → they don't appear in the listing regardless of `visible`. Setting `visible = true` for an inactive user is a no-op until they invest.

### User is `visible = true` but their account is suspended

`refresh_all_scores` (step 0b) deletes their `leaderboard_scores` row on every refresh tick. They disappear on the next 15-min refresh, even if `visible` is still `true`. Re-instate the account and they reappear after the next refresh.

### Two pages, two settings — guaranteed in sync?

Yes. The settings page (`/settings → Leaderboard section`) and the leaderboard page's Visibility card both `PUT`/`POST` to the same `leaderboard_preferences` row. Cross-tab live updates are NOT implemented (no WebSocket) but on page-load both views read the same DB state. Verified by the round-trip test in conversation history (Setting → Leaderboard and reverse).

### Avatar URL leakage paranoia

`avatar_url` is set to `None` server-side when `(visible && show_avatar) || is_current` is false. Test `avatar_url_hidden_when_show_avatar_false` guards this. The browser never sees an URL it shouldn't have.

### Pseudonym pattern from pre-2026-05-16

Older code anonymized hidden users as `'Investor #' || substring(user_id::text, 1, 6)`. **This is gone.** Hidden users are filtered out entirely. The string `"Investor #"` should never appear in any production API response. Regression-guarded by `pseudonym_pattern_never_appears_in_response`.

---

## SQL — what the listing query actually does

The simplified shape (full version in `service::get_rankings_alltime`):

```sql
WITH visible AS (
  SELECT ls.*
  FROM leaderboard_scores ls
  LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
  WHERE ls.rank_invested IS NOT NULL          -- has a score for this metric
    AND COALESCE(lp.visible, false) = true    -- opted in
    AND ls.user_id <> $current_user           -- not the viewer themselves
),
ranked AS (
  SELECT *,
    ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) AS rank
  FROM visible
)
SELECT
  rank, total_invested_cents, …,
  COALESCE(lp.display_name, up.display_name, …, 'Investor') AS full_name,
  CASE WHEN lp.show_avatar THEN u.avatar_url END AS avatar_url
FROM ranked
JOIN users u ON …
LEFT JOIN user_profiles up ON …
LEFT JOIN leaderboard_preferences lp ON …
WHERE (tier_id_filter IS NULL OR tier_id = tier_id_filter)
  AND (search IS NULL OR full_name ILIKE '%' || search || '%')
ORDER BY rank
LIMIT $per_page OFFSET $offset
```

Three things to notice:

1. **The `<>` predicate excludes self.** Removing this would make the viewer appear in their own listing — undesired UX.
2. **`ROW_NUMBER()` re-ranks within the visible set.** Without this the listing would inherit the precomputed `rank_invested` which counts hidden users, producing gaps (1, 4, 7, …).
3. **`COALESCE(lp.visible, false)`** treats users with NO preferences row as hidden. New users without a `leaderboard_preferences` row are invisible by default.

---

## Audit trail

Every `PUT /api/leaderboard/preferences` writes a row to `audit_logs`. Inspect a user's history:

```sql
SELECT created_at, action, ip_address, user_agent
FROM audit_logs
WHERE actor_user_id = '<uuid>'
  AND action LIKE 'leaderboard.prefs.update%'
ORDER BY created_at DESC
LIMIT 20;
```

Example output:

```
2026-05-16 13:42:01+00 | leaderboard.prefs.update visible=Some(true) show_avatar=Some(false) display_name_set=true | 203.0.113.42 | Mozilla/5.0 …
2026-05-16 13:38:14+00 | leaderboard.prefs.update visible=Some(false) show_avatar=None display_name_set=false      | 203.0.113.42 | Mozilla/5.0 …
```

The `action` string records WHAT changed (which fields were sent) but
NOT the new `display_name` value — names can be PII and the immutable
audit log isn't the right home for them. Look up the current value in
`leaderboard_preferences.display_name` directly.
