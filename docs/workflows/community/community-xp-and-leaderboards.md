# Community XP and Leaderboards

Purpose: Verify user XP summary, XP history, global leaderboard, circle leaderboard, rank visibility preferences, and admin-awarded XP effects.

Prerequisites:
- User is logged in.
- Seed data includes users with different XP totals and at least one circle with members.
- If admin XP award is tested, use a disposable test account.

Pages and endpoints covered:
- `/community?tab=circle`
- `/community/me`
- `/community/u/:user_id`
- `/admin/community/leaderboard.html`
- `/api/community/xp`
- `/api/community/xp/history`
- `/api/community/leaderboard`
- `/api/community/circles/leaderboard`
- `/api/community/profile/me/analytics`
- `/api/admin/community/leaderboard`
- `/api/admin/community/users/:id/xp`

Steps:
1. Open My Circles and verify XP summary loads.
2. Open XP history and verify pagination, timestamps, reason labels, and point deltas.
3. Open own profile analytics and verify XP/rank values align with `/api/community/xp`.
4. Open global leaderboard and verify ordering, tie behavior, current-user marker, and profile links.
5. Open circle leaderboard and verify circle/user totals align with member data.
6. Toggle leaderboard visibility in profile settings if available and verify public visibility changes as expected.
7. As admin, award disposable XP to a test user.
8. Verify user XP summary, history, global leaderboard, circle leaderboard, and admin leaderboard reflect the award after reload.
9. Verify negative, huge, duplicate, or malformed XP award attempts are rejected or audited according to admin rules.
10. Verify logged-out users and normal users cannot call admin XP endpoints.

Expected Result:
- XP totals, history, and leaderboard ranks are internally consistent.
- Privacy settings affect public visibility without deleting underlying history.
- Admin XP awards are auditable and affect only intended users.
- Empty and zero-XP states are explicit.

Edge Cases:
- User with zero XP.
- User hidden from leaderboard.
- Tie rank across multiple users.
- XP history with many pages.
- Admin award with negative, zero, very large, or non-numeric amount.
- Award to deleted/banned/shadowbanned user.
- Circle leaderboard for a circle with one or zero members.
