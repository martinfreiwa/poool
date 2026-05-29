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
