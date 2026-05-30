# Community Search, Messages, and Notifications

Purpose: Verify client-side Community panels for Search, Messages, Notifications, and notification preferences.

Prerequisites:
- User is logged in.
- Seed data includes searchable posts/users when possible.
- If testing Messages, use test accounts only.
- If testing notification preferences, record the starting state and restore it after the pass.

Pages and endpoints covered:
- `/community?tab=search`
- `/community?tab=dms`
- `/community?tab=notifications`
- `/api/community/search`
- `/api/community/notifications`
- `/api/community/notifications/unread-count`
- `/api/community/notifications/read-all`
- `/api/community/notifications/:id/read`
- `/api/community/notifications/preferences`
- `/api/community/dms/threads`
- `/api/community/dms/threads/:id/messages`

Steps:
1. Open `/community?tab=search`.
2. Search for a known post keyword, hashtag, and user name.
3. Switch search type filters between `All`, `Posts`, and `Users`.
4. Apply date and minimum-engagement filters for post search.
5. Clear filters and verify results reset.
6. Verify Search direct URL hides or de-emphasizes the feed panel so only Search is the active primary panel.
7. Open `/community?tab=dms` or trigger the Messages entry point.
8. Verify thread list, empty state, selected-thread state, compose state, unread state, and blocked/permission state.
9. Send a test-safe message only between test accounts.
10. Reload `/community?tab=dms` and verify the message/thread state persists without duplicates.
11. Open `/community?tab=notifications` or trigger the Notifications entry point.
12. Verify notification list, unread state, mark-all-read action, empty state, and preference controls.
13. Toggle a test-safe notification preference, reload, and verify persistence.
14. Verify Notifications direct URL hides or de-emphasizes the feed panel so only Notifications is the active primary panel.
15. Verify Messages direct URL hides or de-emphasizes the feed panel so only Messages is the active primary panel.
16. Mark one notification read and verify unread count updates.
17. Mark all notifications read and verify zero-unread state.

Expected Result:
- Search results match query and filter state.
- Messages and notifications open as client-side panels without breaking HTMX tabs.
- Test messages persist and do not duplicate after reload.
- Empty states and network errors are clear.
- Read/unread counts update consistently.
- Notification preferences persist and affect future notification behavior where testable.
- Direct URLs for client panels keep URL, active button, visible panel, focus, and feed visibility in sync.

Edge Cases:
- Search empty query.
- Search special characters, hashtag, mention, URL fragments, and date range where `from` is after `to`.
- DM user has disabled messages from strangers.
- DM recipient is blocked, muted, deleted, or nonexistent.
- Empty thread list.
- Very long message, empty message, duplicate send click, and expired session during send.
- Notification already read.
- Read-all with zero unread notifications.
- Preference save with stale session, missing CSRF, and network failure.

Known local audit note:
- During the 2026-05-29 browser pass, direct client-panel URLs activated the correct button but left `community-feed-tab` visible too. Recheck this workflow after fixing tab visibility.
- `frontend/platform/static/js/community-dms.js` currently builds profile links as `/community/profile?user=...`; the registered route is `/community/u/:user_id`. Treat that as a route/linking defect unless compatibility routing is intentionally added.
