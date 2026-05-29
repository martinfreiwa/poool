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
