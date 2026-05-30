# Community Ban Appeals

Purpose: Verify banned-user appeal submission, appeal validation, admin review, status transitions, and user-facing recovery or rejection states.

Prerequisites:
- A disposable test user can be community-banned.
- Admin account has Community moderation permissions.
- Do not use real user moderation records.

Pages and endpoints covered:
- `/community`
- `/community/me`
- `/admin/community/appeals.html`
- `/api/community/profile/me`
- `/api/community/appeals`
- `/api/admin/community/appeals`
- `/api/admin/community/appeals/:id/review`

Steps:
1. Community-ban a disposable test user through admin tooling.
2. Log in as the banned user and open `/community`.
3. Verify the ban banner appears and normal mutation controls are blocked or hidden.
4. Submit an appeal with valid text.
5. Verify duplicate appeal behavior before review.
6. Try invalid appeal text: empty, too short, too long, and HTML/script.
7. Log in as admin and open `/admin/community/appeals.html`.
8. Filter appeals by pending/approved/rejected and open the new appeal.
9. Reject the appeal with notes and verify banned user sees the expected state.
10. Submit or reuse another disposable appeal and approve it.
11. Verify the user ban state, profile state, and Community access update after approval.
12. Verify every review action is audited and blocked for non-admin users.

Expected Result:
- Banned users see a clear appeal path without gaining unauthorized Community mutation access.
- Appeal text is validated and safely rendered.
- Admin review transitions are explicit, persisted, and auditable.
- Approved appeals restore only the intended account state.

Edge Cases:
- User not banned submits appeal.
- Already pending appeal.
- Already reviewed appeal.
- Appeal target user deleted.
- Admin review with missing notes if notes are required.
- Session expires during submission or review.

Required Workflow Fields Appendix:

Roles: Community User, Moderated/Reported User, Admin moderator where escalation is required.

Primary pages: Community pages/endpoints listed above, plus admin community moderation pages when escalation applies.

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
