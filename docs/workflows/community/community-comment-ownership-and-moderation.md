# Community Comment Ownership and Moderation

Purpose: Verify own-comment edit/delete, comment reactions, official/pinned/hidden/deleted states, admin moderation, and authorization boundaries.

Prerequisites:
- User is logged in.
- Admin account exists for moderation checks.
- Disposable post and comments are available.

Pages and endpoints covered:
- `/community/post/:id`
- `/admin/community/comments.html`
- `/api/community/posts/:id/comments`
- `/api/community/comments/:id`
- `/api/community/comments/:id/reactions`
- `/api/community/comments/:id/official-answer`
- `/api/admin/community/comments`
- `/api/admin/community/comments/:id`
- `/api/admin/community/comments/:id/hide`
- `/api/admin/community/comments/:id/pin`

Steps:
1. Add a disposable comment to a post.
2. Edit the comment as the author and verify the updated text persists.
3. Try invalid edits: empty, too long, and HTML/script content.
4. React to the comment and verify count/state update once.
5. Delete the comment as the author and verify the UI changes to removed state or removes it.
6. Add a second disposable comment for admin checks.
7. As admin, open `/admin/community/comments.html` and locate the comment.
8. Hide/unhide the comment and verify user-facing visibility changes.
9. Pin/unpin the comment and verify ordering/badge behavior.
10. Delete only a disposable comment through admin and verify direct post view handles it.
11. Verify non-author users cannot edit/delete someone else's comment and normal users cannot call admin comment endpoints.

Expected Result:
- Own-comment actions are available only to the author.
- Admin moderation actions are separate from user ownership actions.
- Hidden, pinned, deleted, and official-answer states render consistently.
- Comment text is safe against HTML/script injection.

Edge Cases:
- Edit deleted comment.
- Delete hidden or pinned comment.
- Pin multiple comments if only one pin is allowed.
- React to hidden/deleted comment.
- Comment belongs to locked/deleted post.
- Admin action repeated twice.

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
