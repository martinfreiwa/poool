# Community Asset Reviews

Purpose: Verify asset-linked community circles, asset review creation/update/delete, review listing, upvotes, ownership/eligibility checks, and moderation visibility.

Prerequisites:
- User is logged in.
- Seed data includes at least one marketplace asset with a community circle.
- Use only disposable reviews for mutation checks.

Pages and endpoints covered:
- `/community/circle/:slug`
- `/api/community/assets/:id/circle`
- `/api/community/assets/:id/reviews`
- `/api/community/reviews/:review_id/upvote`

Steps:
1. Open an asset detail page or known asset context that links to Community.
2. Resolve the asset's circle with `/api/community/assets/:id/circle` through the UI flow.
3. Open the linked circle and verify asset identity, access state, and review entry points.
4. List asset reviews and verify author, rating/sentiment if present, body, timestamps, and upvote count.
5. Create or update a disposable review with valid content.
6. Reload and verify the review persists once.
7. Edit the review and verify the update replaces the previous user review rather than duplicating it.
8. Upvote another user's review and verify count/state changes once.
9. Remove the upvote and verify count/state recovers.
10. Delete the disposable review and verify list/detail behavior after deletion.
11. Verify users without required asset access see the expected restriction state if eligibility rules apply.

Expected Result:
- Asset-to-circle lookup is stable and links to the correct circle.
- Review CRUD respects ownership and eligibility rules.
- Upvotes are idempotent and cannot be duplicated through rapid clicks.
- Deleted or hidden reviews do not leak through list views.

Edge Cases:
- Asset has no circle.
- Asset ID is invalid, missing, or unauthorized.
- User tries to review the same asset twice.
- Review body empty, too long, or contains HTML/script.
- Upvote own review.
- Upvote missing/deleted review.
- Review by blocked, muted, banned, or shadowbanned user.

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
