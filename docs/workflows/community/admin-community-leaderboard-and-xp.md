# Admin Community Leaderboard and XP Awards

Purpose: Verify admin leaderboard visibility, XP award action, ranking updates, user detail links, and audit/authorization behavior.

Prerequisites:
- Admin account has Community leaderboard/XP permissions.
- Disposable test user exists.
- Record test user's starting XP.

Pages and endpoints covered:
- `/admin/community/leaderboard.html`
- `/admin/community/user-detail.html`
- `/api/admin/community/leaderboard`
- `/api/admin/community/users/:id/xp`
- `/api/community/xp`
- `/api/community/xp/history`
- `/api/community/leaderboard`

Steps:
1. Open `/admin/community/leaderboard.html`.
2. Verify rows show user identity, XP, rank, status, and user-detail links.
3. Award a small disposable XP amount to a test user.
4. Verify admin leaderboard updates.
5. Open the user's admin detail page and verify XP-related state.
6. Log in as the test user and verify `/api/community/xp`, XP history, and public leaderboard update.
7. Restore the test user's original state if supported or document the disposable award.
8. Try invalid award amounts and unauthorized users.

Expected Result:
- Admin and user-facing XP views agree after award.
- XP award validation prevents invalid values.
- XP awards are auditable and scoped to the intended user.

Edge Cases:
- Award zero, negative, decimal, huge, or non-numeric XP.
- Award to deleted/banned/shadowbanned user.
- Award repeated rapidly.
- Public leaderboard hidden preference.
- Read-only admin tries award.
