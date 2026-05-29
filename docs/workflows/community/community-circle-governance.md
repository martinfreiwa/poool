# Community Circle Governance

Purpose: Verify Circle membership governance: roles, moderators, ownership transfer, privacy, token gates, invites, kicks, bans, and destructive circle actions.

Prerequisites:
- User is logged in as circle owner.
- Test circle has at least one member, one admin/moderator candidate, one non-member, and one banned-user candidate.
- Use a disposable circle for delete/transfer checks.

Pages and endpoints covered:
- `/community/circles`
- `/community/circle/:slug`
- `/community/circle/:slug/settings`
- `/api/community/circles/:id`
- `/api/community/circles/:id/invite`
- `/api/community/circles/:id/kick/:user_id`
- `/api/community/circles/:id/roles`
- `/api/community/circles/:id/transfer`
- `/api/community/circles/:id/privacy`
- `/api/community/circles/:id/token-gate`
- `/api/community/circles/:id/moderator/:user_id`
- `/api/community/circles/:id/bans`
- `/api/community/circles/:id/bans/:user_id`

Steps:
1. Open Circle settings as owner and verify Members/Governance controls load.
2. Invite a non-member test user and verify invite state.
3. Promote a member to admin/moderator and verify role label, permissions, and persistence.
4. Demote the same user and verify permissions are removed.
5. Set/unset moderator status if separate from role.
6. Kick a disposable member and verify membership, feed access, and member list update.
7. Ban the same test user and verify they cannot join, request, or view restricted content.
8. Unban the user and verify normal join/request behavior recovers.
9. Change privacy public/private and verify discoverability, join button, and existing member access.
10. Configure token gate with valid test criteria and verify eligible/ineligible behavior.
11. Transfer ownership to a disposable member in a disposable circle and verify old/new owner permissions.
12. Delete only a disposable circle and verify direct routes, discover lists, and admin views reflect deletion.

Expected Result:
- Governance actions are owner/admin-protected and auditable.
- Role and ownership changes immediately affect available controls.
- Privacy and token-gate changes affect discovery/join/access consistently.
- Bans override invites, requests, and joins.
- Destructive actions require clear confirmation and only affect disposable test data.

Edge Cases:
- Transfer to self, non-member, banned user, or deleted user.
- Kick owner or last admin.
- Ban current owner.
- Invite banned, blocked, existing member, or nonexistent user.
- Private circle with pending requests when switched public.
- Token gate with invalid asset/threshold.
- Delete circle with posts, resources, reports, requests, and members.
