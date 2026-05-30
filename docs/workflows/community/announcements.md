# Community Announcements

Purpose: Verify announcements loading, category filters, list refresh behavior, and announcement detail links.

Prerequisites:
- User is logged in.
- Seed data includes announcements in at least one category when possible.

Pages and endpoints covered:
- `/community?tab=announcements`
- `/community/partials/announcements`
- `/community/partials/announcements/list`
- `/api/admin/community/announcements`

Steps:
1. Open `/community?tab=announcements`.
2. Verify the Announcements tab is active and the list loads.
3. Click category filters: `All`, `New Commodities`, `Dividends`, `Platform Updates`, `Market News`, and `Farm Updates`.
4. Verify each category request updates only the announcements list area.
5. Open an announcement detail or linked post if available.
6. Return to Announcements and verify the previous layout is still stable.
7. Verify empty category behavior when no results exist.

Expected Result:
- Announcement categories load through `/community/partials/announcements/list`.
- Filtering does not reload the whole Community shell.
- Empty categories show a stable empty state.
- Links route to the expected announcement/post target.

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
