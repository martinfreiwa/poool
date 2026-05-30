# Community Browser Workflow Run — 2026-05-29

Purpose: Document the browser execution of Community workflows against local `http://localhost:8888`.

Run metadata:
- Date/time: 2026-05-29 17:03 WAT.
- Browser target: Codex in-app browser.
- Backend: already running on port `8888`.
- Account state: logged in as `support@traffic-creator.com`.
- Scope: non-mutating browser pass. Navigation, tab activation, visibility checks, URL checks, page-shell checks, and console inspection were executed. Post/comment/message/admin mutations were not executed.

## Workflows Exercised

| Workflow | Browser Coverage | Result |
|----------|------------------|--------|
| Shell and Tab Navigation | `/community`, all primary tab URLs, client-panel URLs | Partial pass; client panels expose defects below. |
| Route and Link Integrity | Community tab URLs, profile/settings/admin URLs | Partial pass; `saved`, `members`, and admin index defects below. |
| Feed | `/community`, `/community?tab=feed` | Pass with console/API defect below. |
| Announcements | `/community?tab=announcements` | Pass. |
| Circles | `/community?tab=circle`, `/community/circles` | Pass. |
| Challenges | `/community?tab=challenges` | Pass. |
| Expert AMAs | `/community?tab=ama` | Pass. |
| Search, Messages, Notifications | `/community?tab=search`, `/community?tab=dms`, `/community?tab=notifications` | Partial pass; feed remains visible behind client panels. |
| Saved Posts and Bookmarks | `/community?tab=saved` | Fail; falls back to Feed and no saved panel exists. |
| Profiles and Profile Edit | `/community/me`, `/community/me/edit` | Pass for page load/shell. |
| Notification Settings Page | `/settings/notifications/community` | Pass for page load. |
| Admin Community Operations | `/admin/community/`, `/admin/community/index.html`, `/admin/community/posts.html`, `/admin/community/settings.html` | Partial pass; slash route works, `index.html` route fails. |

## Detailed Results

### Community shell
- `/community` loaded as `Community - POOOL`.
- Active tab: `Feed`.
- Visible panel: `community-feed-tab`.
- Auth state was valid; no login form appeared.

### HTMX-backed tabs
- `/community?tab=feed`: active `Feed`, visible `community-feed-tab`.
- `/community?tab=announcements`: active `Announcements`, visible `community-announcements-tab`.
- `/community?tab=circle`: active `My Circles`, visible `community-circle-tab`.
- `/community?tab=challenges`: active `Challenges`, visible `community-challenges-tab`.
- `/community?tab=ama`: active `Expert AMAs`, visible `community-ama-tab`.

### Client-side tabs
- `/community?tab=search`: active `Search`, visible panels `community-search-tab` and `community-feed-tab`.
- `/community?tab=notifications`: active `Notifications`, visible panels `community-notifications-tab` and `community-feed-tab`.
- `/community?tab=dms`: active `Messages`, visible panels `community-dms-tab` and `community-feed-tab`.

Expected: only the requested client panel should be visible as the primary panel.

Observed defect:
- `#community-content-area` is hidden, but `#community-feed-tab` remains visible in the DOM alongside the client panel.
- This confirms the earlier workflow note in `shell-and-tabs.md` and `search-messages-notifications.md`.

### Saved and members tab URLs
- `/community?tab=saved` keeps active tab `Feed`.
- `/community?tab=saved` has no `#community-saved-tab`.
- `/community?tab=members` keeps active tab `Feed`.
- `/community?tab=members` has no `#community-members-tab`.

Observed defect:
- Both URLs are linked from Community/profile surfaces but do not activate a dedicated panel.
- Treat this as a route/linking/product-state defect unless these are intentionally feed fallbacks.

### Profile pages
- `/community/me` loaded and redirected to `/community/me?tab=posts`.
- Page title: `Support Admin | Community - POOOL`.
- Main heading: `Support Admin`.
- `/community/me/edit` loaded.
- Page title: `Community Profile - POOOL`.
- Main heading: `Community Profile`.

### Notification settings
- `/settings/notifications/community` loaded.
- Page title: `Community Notifications - POOOL`.
- Main heading: `Community Notifications`.

### Admin Community
- `/admin/community/` loaded.
- Page title: `Community Dashboard | Admin | POOOL`.
- Main heading: `Community Overview`.
- `/admin/community/posts.html` loaded and canonicalized to `/admin/community/posts`.
- `/admin/community/settings.html` loaded and canonicalized to `/admin/community/settings`.
- `/admin/community/index.html` canonicalized to `/admin/community/index` and returned `404 Page not found`.

Observed defect:
- The workflow index lists `/admin/community/index.html`, and backend routes register it, but browser navigation ends at `/admin/community/index` with a 404.
- `/admin/community/` works as the usable overview route.

### Console/API observation
During Community page checks, the browser reported repeated errors:

```text
Response Status Error Code 400 from /community/partials/feed/list?page=2&feed_mode=all&sort_by=fresh
HTMX Response Error [400]: /community/partials/feed/list?page=2&feed_mode=all&sort_by=fresh Failed to deserialize query string: duplicate field `feed_mode`
```

Observed defect:
- Feed pagination or load-more is producing duplicate `feed_mode` query parameters.
- The visible page still loads, but pagination/load-more should be fixed or covered with a focused feed regression.

## Not Executed

The following workflow areas were not executed in this pass because they mutate data or require destructive/admin state changes:
- Creating/editing/deleting posts.
- Comment creation/edit/delete.
- Reactions, bookmarks, poll votes, reports.
- Sending DMs.
- Changing notification preferences.
- Joining/leaving circles, invites, bans, role changes, ownership transfer, token gates.
- Resource uploads/versioning.
- Admin moderation actions, badge grants, XP awards, settings changes, audit export.
- Ban appeal submission/review.

## Screenshot Status

Attempted viewport screenshots for:
- `/community?tab=search`
- `/admin/community/index.html`

Result:
- Browser screenshot capture timed out with `Page.captureScreenshot`.
- The run is documented from DOM, URL, active-tab, visible-panel, title, heading, and console-log checks instead.

## Follow-Up Defects To Track

1. Fix client panel visibility so Search, Notifications, and Messages hide `community-feed-tab`.
2. Decide and implement behavior for `/community?tab=saved`.
3. Decide and implement behavior for `/community?tab=members` or remove links to it.
4. Fix `/admin/community/index.html` canonicalization/route behavior or update workflows to use `/admin/community/`.
5. Fix feed pagination/load-more duplicate `feed_mode` query string.
