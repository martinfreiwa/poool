# Admin Community Audit Log

Purpose: Verify admin Community audit log listing, filtering, detail integrity, CSV export, authorization, and sensitive-data handling.

Prerequisites:
- Admin account has Community audit permissions.
- Disposable Community mutations exist or can be created.

Pages and endpoints covered:
- `/admin/community/index.html`
- `/api/admin/community/audit-log`
- `/api/admin/community/audit-log.csv`

Steps:
1. Perform a disposable Community admin mutation, such as hide/unhide a test post.
2. Open the admin Community overview or audit log entry point.
3. Load audit log rows and verify actor, action, target type, target ID, timestamp, and summary.
4. Filter by actor, action, target type, status/date where available.
5. Verify pagination and sorting.
6. Export CSV and verify headers, row count, escaping, and no secrets/session data.
7. Verify normal users and read-only admins without export permission cannot access restricted audit data.
8. Verify audit rows are append-only from the UI perspective.

Expected Result:
- Every tested admin mutation creates an audit row.
- CSV export matches filtered data and is safe to open in spreadsheet tools.
- Audit log does not expose passwords, session tokens, private notes beyond intended admin scope, or raw PII unnecessarily.

Edge Cases:
- Zero audit rows.
- Many audit rows.
- Filter returns no rows.
- CSV values beginning with `=`, `+`, `-`, or `@`.
- Target deleted after audit row was created.
- Export during session expiry.

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
