# Admin Community Operations

Purpose: Verify admin-only Community management pages and APIs that affect the user-facing Community experience.

Prerequisites:
- Admin account with `community.view` and `community.manage` where needed.
- Normal user account for authorization checks.
- Disposable posts, comments, users, circles, announcements, AMAs, challenges, and badges.

Pages and endpoints covered:
- `/admin/community/index.html`
- `/admin/community/posts.html`
- `/admin/community/post-detail.html`
- `/admin/community/comments.html`
- `/admin/community/reports.html`
- `/admin/community/users.html`
- `/admin/community/user-detail.html`
- `/admin/community/circles.html`
- `/admin/community/circle-detail.html`
- `/admin/community/challenges.html`
- `/admin/community/announcements.html`
- `/admin/community/amas.html`
- `/admin/community/badges.html`
- `/admin/community/leaderboard.html`
- `/admin/community/appeals.html`
- `/admin/community/settings.html`
- `/admin/community/verified-owner-requests.html`
- `/api/admin/community/stats`
- `/api/admin/community/posts`
- `/api/admin/community/posts/:id`
- `/api/admin/community/posts/:id/hide`
- `/api/admin/community/posts/:id/lock`
- `/api/admin/community/posts/:id/tags`
- `/api/admin/community/comments`
- `/api/admin/community/comments/:id`
- `/api/admin/community/comments/:id/hide`
- `/api/admin/community/comments/:id/pin`
- `/api/admin/community/reports`
- `/api/admin/community/reports/:id/action`
- `/api/admin/community/users`
- `/api/admin/community/users/:id/detail`
- `/api/admin/community/users/:id/ban`
- `/api/admin/community/users/:id/warn`
- `/api/admin/community/users/:id/mute`
- `/api/admin/community/users/:id/shadowban`
- `/api/admin/community/users/:id/mod-notes`
- `/api/admin/community/ops-alerts`
- `/api/admin/community/ops-alerts/:id/action`
- `/api/admin/community/announcements`
- `/api/admin/community/amas`
- `/api/admin/community/amas/:id`
- `/api/admin/community/amas/:id/status`
- `/api/admin/community/amas/:id/questions/:qid/answer`
- `/api/admin/community/amas/:id/questions/:qid/feature`
- `/api/admin/community/challenges`
- `/api/admin/community/challenges/:id/toggle`
- `/api/admin/community/circles`
- `/api/admin/community/circles/:id`
- `/api/admin/community/circles/:id/transfer`
- `/api/admin/community/circles/:id/members/:user_id`
- `/api/admin/community/verified-owner-requests`
- `/api/admin/community/verified-owner-requests/:id`

Steps:
1. Log in as an admin with `community.view` only and verify read-only pages load but mutations are blocked.
2. Log in as an admin with `community.manage`.
3. Verify overview stats, leaderboard, and settings load.
4. Create, list, and validate an announcement using disposable content.
5. Review posts: open detail, hide/unhide, lock/unlock, and update tags on disposable content.
6. Review comments: hide, pin/unpin, delete disposable comments.
7. Process reports with each supported action.
8. Moderate a test user: warn, mute, shadowban, community-ban, update mod notes, then revert.
9. Submit a ban appeal as the affected user, then approve/reject it as admin.
10. Create/toggle a challenge and verify user-facing Challenges tab reflects state.
11. Create/update a badge and grant/revoke it to a test user.
12. Create/update/delete/transfer a disposable circle, remove a test member, and verify deleted circles disappear from My Circles, Discover, and direct URLs.
13. Create an AMA, update its status through `draft`, `scheduled`, `accepting_questions`, `live`, `closed`, and `archived`, answer user questions, and feature/unfeature questions.
14. Review verified-owner requests and verify user-facing request state updates.
15. Verify audit log and CSV export.

Expected Result:
- Admin read and write permissions are separated.
- Admin mutations affect only intended disposable data.
- User-facing Community surfaces reflect admin state changes after reload.
- Audit logs capture state-changing actions.
- CSV/export endpoints do not expose secrets beyond intended admin data.

Edge Cases:
- Normal user opens admin pages and APIs.
- `community.view` admin attempts write actions.
- Stale moderation/report/appeal target.
- Duplicate announcement, badge, challenge, or AMA.
- Hide, lock, delete, grant, revoke, approve, or reject the same target twice.
- Admin deletes circle with posts, members, resources, and pending requests.
- Audit export with zero rows and many rows.
- Admin mutation with missing/invalid CSRF token.
