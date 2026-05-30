# Community Post Q&A and Official Answers

Purpose: Verify Q&A post status, official-answer marking, question-focused UI states, authorization, and feed/detail consistency.

Prerequisites:
- User is logged in.
- Seed data includes a Q&A-style post or the test user can create one.
- Test includes post owner, circle owner/admin where relevant, and normal commenter.

Pages and endpoints covered:
- `/community/post/:id`
- `/community/circle/:slug`
- `/api/community/posts/:id`
- `/api/community/posts/:id/qa-status`
- `/api/community/posts/:id/comments`
- `/api/community/comments/:id/official-answer`

Steps:
1. Create or open a Q&A post.
2. Add at least two disposable comments from different users.
3. As the authorized owner/admin, mark one comment as the official answer.
4. Verify the official-answer badge/state appears on feed card, post detail, and circle feed where applicable.
5. Change Q&A status with `/qa-status` through the UI path if exposed.
6. Reload and verify status and official answer persist.
7. Unmark or replace the official answer if supported and verify only one official answer remains.
8. Verify unauthorized users cannot mark official answers or change Q&A status.
9. Verify deleted/hidden official-answer comments do not leave broken UI.

Expected Result:
- Q&A status and official answer state are consistent across all post surfaces.
- Authorization rules match product ownership/admin rules.
- Official-answer content is escaped and accessible.
- Status changes are idempotent and recoverable.

Edge Cases:
- Mark own comment vs another user's comment.
- Mark comment from blocked/banned user.
- Mark deleted or hidden comment.
- Mark official answer twice.
- Change status after post is locked.
- Non-Q&A post receives Q&A status request.

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
