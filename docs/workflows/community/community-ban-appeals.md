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
