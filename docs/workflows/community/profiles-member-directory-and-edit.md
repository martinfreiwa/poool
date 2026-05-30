# Community Profiles, Member Directory, and Profile Edit

Purpose: Verify profile pages, own-profile edit page, follow graph, member search, privacy toggles, moderation history, verified-owner requests, and profile safety states.

Prerequisites:
- User is logged in on `http://localhost:8888`.
- Seed data includes at least two community users, one with posts/comments/media and one other account for follow/unfollow checks.
- Use test-safe profile values and restore them when the pass is complete.
- A small valid image is available for avatar/banner/verified-owner evidence upload checks.

Pages and endpoints covered:
- `/community/me`
- `/community/me/edit`
- `/community/u/:user_id`
- `/community?tab=members`
- `/api/community/profile/me`
- `/api/community/profile`
- `/api/community/profile/:id`
- `/api/community/profile/:id/posts`
- `/api/community/profile/:id/comments`
- `/api/community/profile/:id/followers`
- `/api/community/profile/:id/following`
- `/api/community/profile/:id/media`
- `/api/community/profile/:id/activity`
- `/api/community/profile/me/analytics`
- `/api/community/profile/me/moderation-log`
- `/api/community/profile/banner`
- `/api/community/follow/:id`
- `/api/community/members`
- `/api/community/verified-owner-requests`

Steps:
1. Open `/community/me`.
2. Verify profile header: avatar, banner, display name, reputation/level, follower/following counts, badges, and own-profile edit CTA.
3. Switch profile tabs: `Posts`, `Comments`, `Followers`, `Following`, `Media`, `Circle`, `Activity`, plus own-only `Analytics` and `Settings` when visible.
4. Verify each profile tab has loading, empty, and populated states and no console errors.
5. Open another user through `/community/u/:user_id`.
6. Verify the follow button appears for another user, not for own profile.
7. Follow and unfollow the test user; verify count/button state updates and persists after reload.
8. Verify blocked/muted/banned/limited-visibility users show the correct state if seed data exists.
9. Use the member directory via `/api/community/members`, `/community?tab=members`, or the visible member entry point from Community search.
10. Search members by name, change sort/page when available, and verify results link to user profiles.
11. Open `/community/me/edit`.
12. Verify `Profile basics`: avatar upload, bio, flair, character counters, save action, and status region.
13. Save test-safe bio/flair values, reload `/community/me/edit`, then `/community/me`, and verify persistence.
14. Upload a valid avatar/banner image if the environment supports local storage; verify it renders after reload.
15. Upload invalid or oversized image files and verify clear rejection without stale preview.
16. Verify `Privacy & visibility`: leaderboard visibility, DM permission, public profile toggle, and status copy.
17. Change only test-safe privacy toggles, save, reload, and verify persistence through `GET /api/community/profile/me`.
18. Verify `Moderation history` loads and handles empty state.
19. Verify `Request Verified Owner badge`: asset selector, note field, evidence upload, submit action, validation errors, and success state.
20. Submit a verified-owner request only with disposable/test data; record request ID for cleanup or admin review workflow.
21. Verify Danger Zone actions are discoverable but do not execute destructive behavior unless explicitly requested.
22. Verify logged-out access redirects to login for own-profile and edit pages; public profile visibility follows the profile privacy setting.

Expected Result:
- Own profile, other-user profile, edit page, and member discovery all load reliably.
- Profile tab state, follow state, and saved profile settings persist across reloads.
- Privacy controls map to API state and do not expose hidden data.
- Upload and verified-owner flows provide clear validation and success/error states.
- Moderation history and limited-visibility banners are visible when the account state requires them.
- No sensitive personal data or moderation details leak to unauthorized users.

Notes:
- The registered public profile page route is `/community/u/:user_id`. If any UI still links to `/community/profile?user=...`, treat that as a route/linking defect unless a compatibility route is intentionally added.
- `/community?tab=members` is linked from Community empty/getting-started states. If it does not activate a real members panel, treat it as a route/linking defect or add the intended panel.
- Do not change legal/KYC identity data from this workflow.
- Treat downloaded or exported profile/community data as sensitive and do not commit it.
- If a test account is community-banned or shadowbanned, run the ban-appeal checks in the safety/admin workflows before treating post visibility as broken.

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
