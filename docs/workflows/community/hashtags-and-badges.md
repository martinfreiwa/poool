# Community Hashtags and Badges

Purpose: Verify Community discovery/detail pages for hashtags and badges, including empty states, deep links, and safety around generated links.

Prerequisites:
- User is logged in.
- Seed data includes posts with hashtags and at least one Community badge when possible.

Pages and endpoints covered:
- `/community/hashtag/:tag`
- `/community/badge/:id`
- `/api/community/hashtags/:tag`
- `/api/community/hashtags/trending`
- `/api/community/hashtags/suggest`
- `/api/community/badges/:id`

Steps:
1. Open a hashtag from a feed post.
2. Verify the hashtag page banner, post count, feed list, and back navigation.
3. Open a direct hashtag URL with the same tag and verify the same state.
4. Search or type a hashtag in the composer and verify suggestions.
5. Verify trending hashtags load where surfaced in the Community UI.
6. Open a badge detail page from a profile or badge list.
7. Verify badge icon, name, code, description, holder count, recent holders, and holder profile links.
8. Open a badge page directly and verify protected access behavior.
9. Verify empty states for hashtags with no posts and badges with no holders.

Expected Result:
- Hashtag and badge detail pages load directly and from links.
- Hashtag filters return the expected posts and do not break feed pagination.
- Badge holder lists link to the correct profiles.
- Empty and not-found states are explicit.
- Tags, badge names, and holder names render safely as text.

Edge Cases:
- Unknown hashtag.
- Unknown badge ID.
- Hashtag with uppercase, spaces, emoji, URL encoding, punctuation, and very long text.
- Hashtag from hidden/deleted posts.
- Badge with no holders.
- Badge granted then revoked.
- Duplicate badge code in admin seed data if codes are displayed.
- Logged-out direct access.
- Malicious hashtag or badge display text containing HTML/script.

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
