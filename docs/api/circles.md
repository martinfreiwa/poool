# Community Circles API Reference

Multi-circle membership + discovery + moderation. All routes require a
valid session cookie. Mutations (POST/PUT/DELETE) additionally require
the `X-CSRF-Token` header (value mirrored from the `csrf_token` cookie).

Base URL (production): `https://platform.poool.app`
Base URL (local dev):  `http://localhost:8888`

## Authentication & CSRF

Same pattern as `docs/api/leaderboard.md`. Session via cookie set by
`/auth/login`; CSRF via header on mutating verbs. Missing CSRF returns
**403 Forbidden** with `{ "error": "CSRF token validation failed" }`.

## Multi-circle rules (2026-05-16 rework)

- A user can be a member of **any number of circles simultaneously**.
- `UNIQUE(circle_id, user_id)` still prevents joining the same circle
  twice (idempotent — second `POST /:id/join` returns 200, not 409).
- `community_profiles.circle_id` = the user's **primary** circle. Set
  on the first join, cleared if that circle is left/banned. Subsequent
  joins do not change primary unless the user is currently primary-less.
- Bans persist independently of membership: `circle_bans` rows survive
  unban-and-rejoin scenarios; the join handler always checks bans.

## Role hierarchy

`owner > admin > moderator > member`. Owners can promote/demote, transfer,
delete. Admins can ban + approve requests. Moderators can ban (members
only — cannot ban peers or higher ranks).

---

## GET /api/community/circles/discover

Three curated lists for the discovery page. No query parameters.

### Response (200)

```json
{
  "featured": [ { "id": "…", "slug": "founder-circle", "name": "Founder Circle", "description": "…", "avatar_emoji": "🟢", "banner_url": null, "member_count": 42, "max_members": 50, "is_public": true, "is_featured": true, "recent_post_count": 14 } ],
  "trending": [ /* up to 10 by recent_post_count DESC */ ],
  "new":      [ /* up to 10 created in last 30 days */ ]
}
```

`recent_post_count` is refreshed every `POOOL_CIRCLE_TRENDING_REFRESH_SECS`
seconds (default 300) by the `circle_trending_refresh_worker` background
task.

---

## GET /api/community/circles/search

Search public circles by name + description (pg_trgm ILIKE). Paginated.

### Query parameters

| Name | Type | Default | Notes |
|------|------|---------|-------|
| `q` | string | — | Empty → returns `{ results: [], total: 0 }` without a DB hit. |
| `page` | int | `1` | 1-based. |
| `per_page` | int | `10` | Clamped to `1..=50`. |

### Response (200)

```json
{
  "results": [ /* CircleCardRow[] */ ],
  "page": 1, "per_page": 10, "total": 27, "total_pages": 3
}
```

---

## GET /api/community/circles/by-slug/:slug

Resolve a slug to the full circle row + viewer's role.

### Response (200)

```json
{
  "circle": { /* full Circle struct: id, slug, name, description, owner_id, avatar_emoji, banner_url, member_count, total_xp, level, level_name, is_public, max_members, created_at, updated_at, token_gate_*, is_featured, featured_at, recent_post_count */ },
  "my_role": "owner"  // or "admin" | "moderator" | "member" | null (not a member)
}
```

### Errors

| Status | Reason |
|--------|--------|
| 404 | No circle with that slug |
| 401 | No session |

---

## GET /api/community/me/circles

Every circle the viewer is a member of, with role badge.

### Response (200)

```json
{
  "circles": [
    { "circle": { /* CircleCardRow */ }, "role": "owner" },
    { "circle": { /* CircleCardRow */ }, "role": "moderator" },
    …
  ]
}
```

Ordered by `joined_at DESC`. The viewer's **primary** circle is the one
on `community_profiles.circle_id` — it's not flagged in this payload;
the discover UI labels it client-side.

---

## POST /api/community/circles/:id/join

Join a public circle. For private circles, use
`/api/community/circles/:id/request` instead.

- **Idempotent.** Joining a circle the user is already in returns 200.
- **Ban-aware.** Banned users get 403 `"You are banned from this circle."`
- **Capacity-aware.** Full circles return 400 `"This circle is full."`
- **Privacy-aware.** Private circles return 400 `"This circle is private."`

### Errors

| Status | Reason |
|--------|--------|
| 400 | private / full / invalid |
| 403 | banned |
| 404 | circle not found |

---

## POST /api/community/circles/leave

Leave a specific circle. Body is required since users may belong to
multiple circles.

### Request body

```json
{ "circle_id": "…" }
```

### Errors

| Status | Reason |
|--------|--------|
| 400 | not a member / circle owner cannot leave |

---

## POST /api/community/circles/:id/moderator/:user_id

Promote (or demote) a member to **moderator**. Owner-only.

### Request body

```json
{ "moderator": true }   // or false to demote back to member
```

### Errors

| Status | Reason |
|--------|--------|
| 403 | actor is not the owner / CSRF |
| 400 | target is not a member / target is the owner |

Logs to `community_audit_logs` as `circle.promote_moderator` or
`circle.demote_moderator` with `actor_role`, `previous_role`, `new_role`.

---

## POST /api/community/circles/:id/bans

Ban a member. Owner / admin / moderator. Moderators cannot ban peers
or higher ranks (returns 403).

### Request body

```json
{ "user_id": "…", "reason": "Spam" }
```

`reason` optional. The ban is permanent unless `expires_at` is set
later via direct DB write (no API yet).

Side effects:
- Inserts into `circle_bans` (UPSERT on conflict).
- Removes from `circle_members`.
- Decrements `circles.member_count`.
- Clears `community_profiles.circle_id` if that circle was primary.
- Writes `circle.ban_member` audit log with actor + target roles + reason.

### Errors

| Status | Reason |
|--------|--------|
| 403 | not authorized / CSRF / cannot ban peer-or-higher |
| 400 | cannot ban the circle owner |

---

## GET /api/community/circles/:id/bans

List active bans (`expires_at IS NULL OR expires_at > NOW()`).
Owner / admin / moderator only.

### Response (200)

```json
{
  "bans": [
    { "banned_user_id": "…", "banned_by": "…", "reason": "Spam",
      "banned_at": "2026-05-16T10:00:00Z", "expires_at": null }
  ]
}
```

---

## DELETE /api/community/circles/:id/bans/:user_id

Unban a user. Owner / admin. Moderators cannot unban.

Writes `circle.unban_member` audit log.

---

## PUT /api/community/profile/banner

Save the Facebook-style cover-photo URL on the caller's profile. The
actual image upload happens via `/api/upload/post-image` (returns URL);
this endpoint just persists the reference.

### Request body

```json
{ "banner_url": "https://storage.googleapis.com/poool/banners/…" }
```

Pass `banner_url: null` (or empty string) to clear. URLs are trimmed +
length-capped at 1024 chars.

### Response (200)

```json
{ "success": true, "banner_url": "https://…" }
```

---

## Background workers

- `circle_trending_refresh_worker` (every 5 min default) — refreshes
  `circles.recent_post_count` from `posts` table. Drives the Discover
  > Trending sort order.
- `circle_invite_expiry_worker` (every hour) — expires pending invites.
- `circle_retry_worker` (every 30 min) — auto-joins referred users to
  referrer's primary circle.

Override worker interval: `POOOL_CIRCLE_TRENDING_REFRESH_SECS=<n>` env
var, floor 60s.

## Audit trail

All ban/promote/demote/unban actions write to `community_audit_logs`
with `actor_user_id`, `action`, `entity_type=circle`, `entity_id`,
`target_user_id`, and a JSON `details` blob. See `docs/CODEBASE_ARCHITECTURE.md`
for the audit log schema.

## Tests

- `backend/tests/community_circles_rework.rs` — 9 SQL-level integration
  tests (multi-join, ban-rejoin, slug-unique, discover sections, etc.).
  Run with `COMMUNITY_DATABASE_URL=… cargo test --test community_circles_rework -- --ignored`.
- `backend/tests/community_circles_http.rs` — 10 HTTP-router tests
  (route mounted, CSRF rejected, auth required). Run with
  `DATABASE_URL=… cargo test --test community_circles_http -- --ignored`.

## See also

- Source: `backend/src/community/{routes.rs, circles.rs, background.rs, audit.rs}`
- Frontend: `frontend/platform/partials/community_circle.html`,
  `community-circle-settings.html`, `community-profile.html`
- Frontend JS: `static/js/community-circles-discover.js`,
  `community-circle-settings.js`, `community-profile.js`
- Migrations: `database/community/045_circles_rework.sql`,
  `046_profile_banner.sql`
