# Community Saved Posts and Bookmarks

Purpose: Verify saved-post discovery, bookmark status, bookmark toggles, pagination, empty states, and stale/deleted post handling.

Prerequisites:
- User is logged in.
- Seed data includes at least two bookmarkable posts, one owned by the user and one by another user.
- Use disposable posts for delete/stale-state checks.

Pages and endpoints covered:
- `/community?tab=saved`
- `/api/community/bookmarks`
- `/api/community/posts/:id/bookmark`
- `/api/community/posts/:id/bookmark/status`

Steps:
1. Open `/community?tab=saved` from the profile edit shortcut and by direct URL.
2. Verify the Saved tab/panel is visible, active, and not competing with the Feed panel.
3. Bookmark a normal feed post and verify the action state updates once.
4. Open `/api/community/posts/:id/bookmark/status` through the UI state or a test request and verify it matches the visible bookmark state.
5. Reload `/community?tab=saved` and verify the bookmarked post appears.
6. Unbookmark the same post and verify it disappears from Saved after refresh or list update.
7. Bookmark multiple posts and verify ordering, pagination/load-more behavior, and post detail links.
8. Delete or hide a disposable bookmarked post from another tab/admin path and verify Saved handles the stale item cleanly.
9. Verify logged-out users cannot list bookmarks or mutate bookmark state.

Expected Result:
- Bookmark state is consistent across feed cards, post detail, status API, and Saved.
- Saved empty state is explicit and has a safe route back to Community.
- Deleted, hidden, locked, or inaccessible posts do not leak sensitive content.
- Duplicate bookmark clicks do not create duplicate Saved rows.

Edge Cases:
- Saved list with zero posts.
- Bookmarking an already bookmarked post.
- Unbookmarking a post that is not bookmarked.
- Bookmark status for missing, hidden, deleted, or unauthorized post.
- Direct `?tab=saved` access after session expiry.
- Rapid double-click or network retry during toggle.

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
