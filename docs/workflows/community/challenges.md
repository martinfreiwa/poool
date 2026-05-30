# Community Challenges

Purpose: Verify the Challenges tab loads active challenges, renders progress correctly, handles submissions/votes, and covers empty/error states.

Prerequisites:
- User is logged in.
- Backend endpoints `/api/community/challenges`, `/api/community/challenges/:id/submit`, `/api/community/challenges/:id/submissions`, and `/api/community/challenges/submissions/:sid/vote` are reachable.
- Seed data includes at least one active challenge when possible.
- Use disposable test submissions only.

Pages and endpoints covered:
- `/community?tab=challenges`
- `/community/partials/challenges`
- `/api/community/challenges`
- `/api/community/challenges/:id/submit`
- `/api/community/challenges/:id/submissions`
- `/api/community/challenges/submissions/:sid/vote`

Steps:
1. Open `/community?tab=challenges`.
2. Verify the `Challenges` tab is active and `/community/partials/challenges` has loaded.
3. Verify loading state is replaced by either challenge cards or a clear empty state.
4. For each visible challenge, verify title, description, reward/progress metadata, progress bar, and action state.
5. Verify progress values do not exceed 100% visually.
6. Open a challenge submissions view if the UI exposes one.
7. Submit a valid test challenge entry only for a disposable challenge.
8. Verify the submitted entry appears in `/api/community/challenges/:id/submissions` or the UI list.
9. Vote on another user's challenge submission.
10. Toggle the vote if supported and verify count/state updates once.
11. Refresh the page and confirm the same tab opens directly.
12. Simulate or inspect API empty state when no challenges are available.
13. Simulate or inspect API failure state and verify retry behavior.

Expected Result:
- Challenges load through the tab route and API.
- Progress calculations are stable, including zero-target and over-target cases.
- Valid submissions and votes persist and do not duplicate.
- Empty and error states are understandable and do not break the page.
- No console errors occur during initial load, retry, or direct URL load.

Edge Cases:
- No active challenges.
- Challenge starts in the future.
- Challenge already ended.
- Submit with empty body, missing proof, invalid URL, or over-limit text.
- Submit twice to the same challenge.
- Vote on own submission.
- Vote on deleted or hidden submission.
- Submission list with zero entries and with many entries.
- Challenge reward/progress fields missing or null.
- API returns 409 for duplicate submission or duplicate vote.

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
