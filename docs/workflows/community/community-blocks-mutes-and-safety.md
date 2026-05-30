# Community Blocks, Mutes, and Safety Controls

Purpose: Verify user block/mute controls, block/mute lists, content visibility effects, DM restrictions, follow-state cleanup, and safety privacy.

Prerequisites:
- User is logged in.
- Seed data includes at least three test users: actor, target, and unaffected control.
- Use test users only; restore relationship state after the pass.

Pages and endpoints covered:
- `/community/u/:user_id`
- `/community?tab=feed`
- `/community?tab=dms`
- `/api/community/users/:id/block`
- `/api/community/users/:id/mute`
- `/api/community/blocks`
- `/api/community/mutes`
- `/api/community/follow/:id`
- `/api/community/dms/threads`

Steps:
1. Open a target user's profile and verify block/mute controls are discoverable where expected.
2. Follow the target user, then block the target.
3. Verify follow state is removed or blocked according to product rules.
4. Reload feed/search/profile surfaces and verify blocked user content is hidden or clearly restricted.
5. Try opening a DM thread with the blocked user and verify it is blocked with a clear message.
6. Open block list through API/UI and verify the target appears.
7. Unblock the target and verify profile/feed/DM behavior recovers.
8. Mute the target user and verify their content is hidden or de-emphasized without using the stronger block behavior.
9. Open mute list and verify the target appears.
10. Unmute the target and verify normal visibility returns.
11. Verify the target cannot infer private block/mute metadata beyond allowed UI behavior.
12. Verify logged-out and unauthorized direct API calls are rejected.

Expected Result:
- Block and mute state persists after reload.
- Blocked users cannot DM or interact where prohibited.
- Muted users are filtered without leaking private safety metadata.
- Block/mute lists only expose the current user's relationships.
- Self-block, self-mute, duplicate actions, and stale targets are handled safely.

Edge Cases:
- Block self.
- Mute self.
- Block an already blocked user.
- Unblock a user who is not blocked.
- Target is deleted, banned, shadowbanned, or already muted.
- Mutual block.
- Existing DM thread before block.
- Existing comments/reactions from blocked user on visible posts.

Required Workflow Fields Appendix:

Roles: Community User, Moderated/Reported User, Admin moderator where escalation is required.

Primary pages: Community pages/endpoints listed above, plus admin community moderation pages when escalation applies.

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
