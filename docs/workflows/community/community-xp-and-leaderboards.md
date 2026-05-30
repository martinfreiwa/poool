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
