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
