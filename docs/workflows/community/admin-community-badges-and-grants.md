# Admin Community Badges and Grants

Purpose: Verify badge CRUD, user badge grants/revocations, badge detail page effects, duplicate handling, and auditability.

Prerequisites:
- Admin account has Community badge permissions.
- Disposable badge and test user are available.

Pages and endpoints covered:
- `/admin/community/badges.html`
- `/community/badge/:id`
- `/community/u/:user_id`
- `/api/admin/community/badges`
- `/api/admin/community/badges/:id`
- `/api/admin/community/users/:id/badge`
- `/api/admin/community/users/:id/badge/:badge_id`
- `/api/community/badges/:id`

Steps:
1. Open admin badges page and verify existing badges load.
2. Create a disposable badge with valid name, code, icon, and description.
3. Edit the badge and verify changes persist.
4. Open the public badge detail page and verify public fields and holder list.
5. Grant the badge to a test user.
6. Verify the user's profile shows the badge and the badge holder list includes the user.
7. Revoke the badge and verify profile/detail pages update.
8. Try duplicate code, missing fields, oversized text, invalid icon, and malformed color/icon values.
9. Verify normal users cannot call admin badge endpoints.

Expected Result:
- Badge create/update/grant/revoke operations persist and are reflected on public pages.
- Duplicate grants are idempotent or rejected clearly.
- Badge text/icon fields render safely.
- Admin actions are audited.

Edge Cases:
- Grant badge to nonexistent, banned, or deleted user.
- Revoke badge user does not have.
- Delete/update badge with existing holders if supported.
- Badge with zero holders.
- Very long badge name/code.

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
