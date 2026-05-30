# Community Trending and Discovery Widgets

Purpose: Verify trending posts, trending assets, trending hashtags, discover widgets, empty states, and link integrity across Community.

Prerequisites:
- User is logged in.
- Seed data includes enough posts, hashtags, and assets to produce non-empty trending results when possible.

Pages and endpoints covered:
- `/community?tab=feed`
- `/community/hashtag/:tag`
- `/api/community/trending`
- `/api/community/trending-assets`
- `/api/community/hashtags/trending`
- `/api/community/search`

Steps:
1. Open Community Feed and locate trending widgets.
2. Verify trending posts load with title/body excerpt, count signals, and links to `/community/post/:id`.
3. Verify trending assets load with asset identity and links to the correct marketplace/property route.
4. Verify trending hashtags load and link to `/community/hashtag/:tag`.
5. Click each widget type and verify destination content matches the clicked item.
6. Compare one widget result with `/api/community/search` or direct detail API to ensure IDs and labels align.
7. Verify empty trending responses show non-blocking empty states.
8. Verify hidden/deleted/private content does not appear in trending results for unauthorized users.

Expected Result:
- Trending widgets are supplemental and never block the primary feed.
- Links are correct, encoded, and safe.
- Trending output respects visibility, moderation, and privacy rules.

Edge Cases:
- No trending posts/assets/hashtags.
- Hashtag with special characters or URL encoding.
- Trending post deleted after widget renders.
- Trending asset unavailable or sold out.
- User is blocked/muted by an author in trending results.

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
