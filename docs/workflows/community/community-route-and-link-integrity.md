# Community Route and Link Integrity

Purpose: Verify all Community links, tab query states, admin links, notification links, profile links, and generated URLs resolve to registered routes or intentionally supported compatibility routes.

Prerequisites:
- Local backend is running on `http://localhost:8888`.
- User is logged in.
- Admin account is available for admin link checks.
- Seed data includes posts, profiles, circles, badges, hashtags, notifications, DMs, reports, and admin rows when possible.

Pages and endpoints covered:
- `/community`
- `/community?tab=feed`
- `/community?tab=announcements`
- `/community?tab=circle`
- `/community?tab=challenges`
- `/community?tab=ama`
- `/community?tab=search`
- `/community?tab=dms`
- `/community?tab=notifications`
- `/community?tab=saved`
- `/community?tab=members`
- `/community/post/:id`
- `/community/u/:user_id`
- `/community/me`
- `/community/me/edit`
- `/community/circle/:slug`
- `/community/circles`
- `/community/circle/:slug/settings`
- `/community/badge/:id`
- `/community/hashtag/:tag`
- `/settings/notifications/community`
- `/admin/community/*`

Steps:
1. Crawl visible links from `/community`, Community partials, profile pages, circle pages, hashtag pages, badge pages, and admin Community pages.
2. Verify each internal Community URL returns a successful page, a protected redirect, or an intentional not-found state.
3. Open every supported `?tab=` value and verify URL state, active state, visible panel, focus state, and mobile bottom-nav state match.
4. Verify generated post, profile, circle, badge, hashtag, notification, DM, admin detail, and report links are URL-encoded.
5. Open links emitted from notifications and DMs and verify they do not point to legacy routes.
6. Verify admin sidebar links and admin row detail links resolve to registered admin pages.
7. Check stale generated links after deleting disposable posts/circles/comments/reports/users and verify graceful not-found or hidden-state behavior.
8. Verify logged-out access to protected links redirects to `/auth/login` and preserves safe redirect intent where supported.

Expected Result:
- Community links use registered canonical routes.
- Direct tab URLs activate the intended panel without stale panels competing visually.
- Generated links survive special characters in slugs, tags, and IDs through correct URL encoding.
- Legacy or unsupported links are recorded as defects and not silently accepted by workflows.

Known local audit notes:
- `frontend/platform/static/js/community-dms.js` builds profile links as `/community/profile?user=...`; the registered route is `/community/u/:user_id`.
- `frontend/platform/partials/community_post_list.html` and `frontend/platform/partials/community_feed.html` link to `/community?tab=members`; verify whether a members client tab is intentionally implemented. If not, treat these as route/linking defects.

Edge Cases:
- Hashtag containing spaces, punctuation, emoji, or URL-reserved characters.
- Circle slug renamed after link was generated.
- Badge revoked after badge detail link was generated.
- Notification link points to hidden/deleted content.
- Admin detail link missing query `id`.
- Browser back/forward after tab switches.
