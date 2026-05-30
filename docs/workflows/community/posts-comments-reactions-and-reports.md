# Community Posts, Comments, Reactions, and Reports

Purpose: Verify post detail pages, comments, reactions, bookmarks, poll voting, owner edit/delete controls, report flow, and user-generated-content safety.

Prerequisites:
- User is logged in on `http://localhost:8888`.
- Seed data includes at least one normal post, one poll post, one announcement post, one post by the test user, and one post by another user when possible.
- Use disposable test posts for edit/delete checks.

Pages and endpoints covered:
- `/community/post/:id`
- `/community/partials/feed/list`
- `/api/community/posts`
- `/api/community/posts/:id`
- `/api/community/posts/:id/report`
- `/api/community/posts/:id/qa-status`
- `/api/community/posts/:id/comments`
- `/api/community/posts/:id/reactions`
- `/api/community/posts/:id/bookmark`
- `/api/community/posts/:id/bookmark/status`
- `/api/community/posts/:id/poll`
- `/api/community/posts/:id/poll/vote`
- `/api/community/comments/:id`
- `/api/community/comments/:id/reactions`

Steps:
1. Open `/community?tab=feed` and identify a visible post card.
2. Verify each post card renders author identity, timestamp, post type badge, body, images/link preview/poll content if present, reaction count, comment count, bookmark action, and report action.
3. Open the post detail route via `/community/post/:id`.
4. Verify the detail page preserves the same content, reaction state, comment state, and any announcement/poll badges.
5. Toggle a reaction on the post, verify count and `aria-pressed` update, then toggle back if the workflow should leave data unchanged.
6. Toggle bookmark state and verify it persists after reload or returns a clear error.
7. Open comments, submit a short test comment with a unique marker, and verify it appears once.
8. Submit invalid comments: empty body, whitespace-only body, and over-limit body. Verify clear validation errors and no duplicate comments.
9. For a poll post, vote once and verify results update; reload and verify vote state persists.
10. For a post owned by the test user, open the owner menu and verify `Edit` and `Delete` are present.
11. Edit a disposable post, save, reload, and verify the updated body persists.
12. Try invalid edit content: empty and over-limit. Verify validation error and original content remains.
13. Delete only a disposable post. Verify confirmation, removal from feed, and direct detail route behavior after deletion.
14. For a post by another user, verify owner edit/delete controls are absent.
15. Open the report modal, verify reason options and note counter, submit a test report against disposable content, and verify success state through `/api/community/posts/:id/report`.
16. Try invalid report submission without reason or with over-limit note. Verify validation blocks or server rejects safely.
17. Verify user-generated content is escaped: test content containing `<script>`, inline event handlers, and HTML tags should display as text or sanitized allowed markup only.
18. Verify a logged-out browser cannot mutate reactions, comments, bookmarks, votes, reports, edits, or deletes.

Expected Result:
- Post detail and feed card state agree.
- Comments, reactions, bookmarks, and poll votes update once and persist as expected.
- Owner-only actions are only available to the owner and are authorization-protected server-side.
- Report flow records the report or shows clear validation/authorization errors.
- User-generated text cannot execute HTML or script.
- Empty, validation-error, authorization-error, and network-error states are visible and recoverable.

Notes:
- Do not delete non-disposable seed posts.
- If test reports enter a real moderation queue, record the created report ID and clean it up through admin tooling after the pass.
- If the post includes financial language, verify the investment disclaimer appears where required.

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
