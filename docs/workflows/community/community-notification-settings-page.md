# Community Notification Settings Page

Purpose: Verify the dedicated Community notification settings page, preference load/save behavior, fallback states, and alignment with Community tab preferences.

Prerequisites:
- User is logged in.
- Record the starting notification preference state and restore it after the pass.

Pages and endpoints covered:
- `/settings/notifications/community`
- `/community?tab=notifications`
- `/api/community/notifications/preferences`

Steps:
1. Open `/settings/notifications/community` directly.
2. Verify all preference rows render with current server state.
3. Toggle one test-safe preference and save.
4. Reload the settings page and verify persistence.
5. Open `/community?tab=notifications` and verify the same preference state is reflected there.
6. Toggle the preference from the Community notifications panel and verify the settings page reflects the change.
7. Restore the original preference state.
8. Verify loading, empty/fallback, validation-error, network-error, and session-expired states.

Expected Result:
- The dedicated settings page and Community notifications panel share the canonical preference API.
- Saves persist once and do not overwrite unrelated preferences.
- Legacy notification preference endpoints are not used.
- User receives clear feedback for success and failure.

Edge Cases:
- Preference object has unknown keys.
- Preference object is missing a key.
- Rapid save clicks.
- Server rejects malformed boolean values.
- Logged-out direct access.
- Network failure after optimistic UI update.

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
