# Admin Community Leaderboard and XP Awards

Purpose: Verify admin leaderboard visibility, XP award action, ranking updates, user detail links, and audit/authorization behavior.

Prerequisites:
- Admin account has Community leaderboard/XP permissions.
- Disposable test user exists.
- Record test user's starting XP.

Pages and endpoints covered:
- `/admin/community/leaderboard.html`
- `/admin/community/user-detail.html`
- `/api/admin/community/leaderboard`
- `/api/admin/community/users/:id/xp`
- `/api/community/xp`
- `/api/community/xp/history`
- `/api/community/leaderboard`

Steps:
1. Open `/admin/community/leaderboard.html`.
2. Verify rows show user identity, XP, rank, status, and user-detail links.
3. Award a small disposable XP amount to a test user.
4. Verify admin leaderboard updates.
5. Open the user's admin detail page and verify XP-related state.
6. Log in as the test user and verify `/api/community/xp`, XP history, and public leaderboard update.
7. Restore the test user's original state if supported or document the disposable award.
8. Try invalid award amounts and unauthorized users.

Expected Result:
- Admin and user-facing XP views agree after award.
- XP award validation prevents invalid values.
- XP awards are auditable and scoped to the intended user.

Edge Cases:
- Award zero, negative, decimal, huge, or non-numeric XP.
- Award to deleted/banned/shadowbanned user.
- Award repeated rapidly.
- Public leaderboard hidden preference.
- Read-only admin tries award.

Required Workflow Fields Appendix:

Roles: Admin, Community User for readback where the admin action changes visible community state.

Primary pages: Admin Community pages listed above, plus affected public/community readback pages when applicable.

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
