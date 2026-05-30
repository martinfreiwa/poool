# Community Circle Space, Settings, Resources, and Ops

Purpose: Verify circle detail pages, circle owner/admin settings, resources, member operations, reports, join requests, bans, analytics, challenges, onboarding, and ops alerts.

Prerequisites:
- User is logged in on `http://localhost:8888`.
- Test data includes at least one public circle, one private circle, one owner/admin user, one member, and one non-member when possible.
- Use only disposable test circles and test resources for destructive or lifecycle actions.

Pages and endpoints covered:
- `/community/circle/:slug`
- `/community/circle/:slug/settings`
- `/api/community/circles/by-slug/:slug`
- `/api/community/circles/:id`
- `/api/community/circles/:id/posts`
- `/api/community/circles/:id/members`
- `/api/community/circles/:id/join`
- `/api/community/circles/:id/request`
- `/api/community/circles/:id/requests`
- `/api/community/circles/:id/resources`
- `/api/community/circles/:id/resources/manage`
- `/api/community/circles/:id/resources/upload`
- `/api/community/circles/:id/analytics`
- `/api/community/circles/:id/manage`
- `/api/community/circles/:id/reports`
- `/api/community/circles/:id/reports/:report_id/action`
- `/api/community/circles/:id/reports/bulk-action`
- `/api/community/circles/:id/ops-alerts`
- `/api/community/circles/:id/ops-alerts/:alert_id/action`
- `/api/community/circles/:id/challenges`
- `/api/community/circles/:id/onboarding`

Steps:
1. Open a public circle detail page from My Circles or Discover.
2. Verify hero, banner/avatar fallback, name, description, member count, privacy/token-gate state, posts, announcements, events, resources, and member list.
3. Open the same circle as a non-member and verify join/request controls match public/private/token-gated state.
4. Join a public test circle, reload, and verify member state persists.
5. Request access to a private test circle, cancel the request, then request again if allowed.
6. As owner/admin, open circle settings.
7. Save test-safe changes to name, description, tags, rules, disclaimers, public/private state, and token-gate state.
8. Reload settings and detail pages; verify values persisted.
9. Invite a test user, then accept and decline invites from the invited account.
10. Approve and decline pending join requests.
11. Promote/demote a member where role controls are exposed.
12. Kick, ban, and unban a disposable test member.
13. Upload a circle resource file, edit metadata, update lifecycle state, create a new version, review/restore versions, and verify access rules.
14. Verify circle analytics, reports queue, bulk report action, and ops-alert action states for owner/admin.
15. Verify non-owner/non-admin users cannot access settings, resources manage, reports, ops alerts, role changes, bans, kicks, transfer, or delete.

Expected Result:
- Circle details and settings reflect the correct role and membership state.
- Owner/admin changes persist after reload and are blocked for unauthorized users.
- Resources and versions preserve metadata, lifecycle, and access rules.
- Join/request/invite/role/ban/kick flows show clear state transitions.
- Destructive actions only affect disposable test data and are auditable.

Edge Cases:
- Duplicate name or slug.
- Invalid slug characters and reserved slugs.
- Empty or over-limit name, description, tags, rules, and disclaimers.
- Public-to-private transition with existing non-members and pending requests.
- Private-to-public transition with pending requests.
- Token-gated circle where the user lacks required asset ownership.
- Invite an existing member, banned user, blocked user, self, or nonexistent user.
- Approve a request that was already cancelled or declined.
- Transfer ownership to self, non-member, banned member, or last remaining member.
- Upload unsupported resource type, oversized file, empty file, corrupted file, or duplicate version.
- Restore stale resource version after newer version exists.
- Delete a circle with members, posts, resources, reports, and pending requests.

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
