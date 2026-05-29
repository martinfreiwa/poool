# Community Edge-Case Matrix

Purpose: Provide one cross-cutting checklist for unusual Community states that must be covered across all workflows.

Use this after the feature-specific workflows. Each row should be validated against every relevant Community page, partial, API, and modal.

| Category | Cases | Expected Result |
|----------|-------|-----------------|
| Authentication | Logged out, expired session, session revoked in another tab, direct partial route | Redirect or 401/403 without partial writes. |
| Authorization | Non-owner, non-member, member, moderator/admin, owner, `community.view`, `community.manage` | Only eligible controls render; direct API calls are blocked when unauthorized. |
| CSRF | Missing, invalid, stale, valid | Mutations reject invalid CSRF and do not modify data. |
| Rate limiting | Rapid posts, comments, reactions, follows, reports, circle joins, invites, DMs | Stable rate-limit message; no server panic or duplicate data. |
| Empty states | No posts, comments, followers, circles, requests, challenges, AMAs, notifications, DMs, badges, hashtag results | Intentional empty state with next action where useful. |
| Error states | 400, 401, 403, 404, 409, 422, 429, 500 | Clear user-visible error; UI recovers on retry/reload. |
| Network | Offline, slow response, cancelled request, malformed JSON, old schema missing fields | Loading/error states remain stable; no blank panels. |
| HTMX/client state | Rapid tab switches, browser back/forward, stale partial response, invalid `tab` query | URL, active tab, and visible panel remain aligned. |
| Data safety | HTML/script in user names, bios, posts, comments, hashtags, circle names, poll options, resource names | Content is escaped or sanitized; scripts never execute. |
| Uploads | SVG, wrong MIME, oversized, empty, corrupted, multiple image limit, duplicate resource version | Clear validation; no stale preview or orphaned UI state. |
| Concurrency | Two tabs edit same post/profile/circle/resource; duplicate submit click; retry after timeout | Last-write/conflict behavior is explicit; duplicate actions are idempotent or rejected. |
| Moderation | Banned, muted, shadowbanned, warned, hidden post, locked post, deleted post, pending appeal | User-facing restrictions and banners match backend state. |
| Privacy | Private profile, DMs disabled, blocked user, muted user, private/token-gated circle | Hidden data stays hidden; allowed read paths still work. |
| Destructive actions | Delete post/comment/circle/resource, leave circle, transfer ownership, revoke badge | Confirmation/gating present; disposable data only; audit where expected. |
| Mobile/a11y | Narrow viewport, zoom 200%, keyboard-only, modal focus trap, screen-reader labels | Controls remain usable; no overlap or focus loss. |

