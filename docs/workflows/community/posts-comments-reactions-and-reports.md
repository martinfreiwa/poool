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
