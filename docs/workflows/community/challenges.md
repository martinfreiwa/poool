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
