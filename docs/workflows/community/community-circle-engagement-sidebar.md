# Community Circle Engagement Sidebar

Purpose: Verify Circle detail sidebar modules: announcements, events, resources, challenges, onboarding, rules/about content, and partial failures.

Prerequisites:
- User is logged in.
- Seed data includes one circle with sidebar content and one circle without sidebar content.
- Test both member and non-member views when possible.

Pages and endpoints covered:
- `/community/circle/:slug`
- `/api/community/circles/by-slug/:slug`
- `/api/community/circles/:id/announcements`
- `/api/community/circles/:id/events`
- `/api/community/circles/:id/resources`
- `/api/community/circles/:id/challenges`
- `/api/community/circles/:id/onboarding`
- `/api/community/circles/:id/onboarding/:step`

Steps:
1. Open a circle detail page with seeded sidebar content.
2. Verify about/rules text, announcements, events, resources, challenges, and onboarding modules render in the expected order.
3. Click each sidebar item and verify route, anchor, modal, or download behavior.
4. Complete an onboarding step and verify progress/state persists after reload.
5. Open the same circle as a non-member and verify restricted modules hide or show locked states according to access rules.
6. Open a circle with no sidebar content and verify empty states do not collapse the page layout awkwardly.
7. Simulate or observe one failed sidebar API while other modules still render.
8. Verify HTML/script in sidebar-provided content is escaped or sanitized.

Expected Result:
- Sidebar modules load independently and do not block the main feed.
- Member/non-member visibility matches circle access rules.
- Onboarding actions persist and are idempotent.
- Empty and partial-failure states are visible and recoverable.

Edge Cases:
- Unknown slug.
- Circle deleted after page shell loads.
- Empty announcements/events/resources/challenges/onboarding.
- Onboarding step already completed.
- Resource unavailable or unauthorized.
- Very long rules/about/sidebar titles.

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
