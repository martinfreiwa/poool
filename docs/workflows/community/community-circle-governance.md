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
