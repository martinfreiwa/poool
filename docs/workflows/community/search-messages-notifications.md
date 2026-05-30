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
- DM active-thread profile links should point to `/community/u/:user_id`; this is now the implemented route contract.

Required Workflow Fields Appendix:

Roles: Community User; Admin moderator only for ownership, moderation, or operational escalation branches.

Primary pages: Community pages and endpoints listed above; admin community pages only where the workflow explicitly includes moderation or operations.

Backend/API surfaces: Community routes and services under `backend/src/community/**`; admin community routes under `backend/src/admin/**` where this workflow includes moderation, grants, settings, reports, or audit review. See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for exact route-to-workflow mappings.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Happy path | The workflow reaches the visible final state and persists after page reload. |
| Authorization boundary | Logged-out, wrong-role, non-owner, banned, or muted actors are redirected, blocked, or receive `401`/`403` without partial writes. |
| Validation failure | Missing, malformed, duplicate, stale, or out-of-state input is rejected with recoverable UI feedback. |
| Reload/readback | The affected community/admin page is reloaded after mutation and reflects database/API state, not stale client state. |
| Cleanup | Disposable `Workflow Test` content, uploads, grants, reports, or moderation state can be removed, reverted, or intentionally retained with a note. |

Negative Cases: Use the edge cases above plus unauthorized direct API access, duplicate submit, stale record, hidden/deleted content access, network failure, and unsafe user-generated content. Upload branches must reject invalid file type, oversize files, missing storage objects, and inaccessible storage links.

Audit / DB / Financial Checks: Admin moderation, grants, settings, reports, appeals, and destructive actions must write community/admin audit rows with actor, action, target, timestamp, prior/new state where available, and redacted sensitive values. Community XP, badges, reports, notifications, and saved/bookmark rows must persist once and remain idempotent on duplicate requests. Community workflows do not move money; if an asset/investment reference is shown, verify it remains read-only here and any monetary values stay integer cents in the owning investor/admin workflow.

Cleanup: Delete or hide disposable posts/comments/uploads where policy allows, undo test reactions/bookmarks/follows/mutes/blocks, revert badge/grant/settings/moderation changes, remove temporary files, and retain audit logs unless the environment is disposable and the cleanup runbook explicitly truncates them.
