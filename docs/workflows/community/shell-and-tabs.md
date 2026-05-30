# Community Shell and Tab Navigation

Purpose: Verify the Community page shell, topbar tab navigation, URL state, protected access, and HTMX/client-side tab loading.

Prerequisites:
- Local backend is running on `http://localhost:8888`.
- PostgreSQL is running and the local `poool` database is reachable.
- A valid test account exists and can log in.

Steps:
1. Log in with a test account.
2. Open `http://localhost:8888/community`.
3. Verify the sidebar, Community topbar, search entry, notification entry, and tab bar render.
4. Click each visible topbar tab: `Feed`, `Announcements`, `My Circles`, `Challenges`, and `Expert AMAs`.
5. For each tab, verify the active tab state, `aria-selected`, loaded content, and URL query parameter.
6. Open direct URLs for each tab: `?tab=feed`, `?tab=announcements`, `?tab=circle`, `?tab=challenges`, and `?tab=ama`.
7. Trigger the topbar search field and verify the Search client-side panel opens.
8. Trigger Messages and Notifications entry points when available and verify their client-side panels open without replacing the HTMX content incorrectly.
9. Open direct client-panel URLs: `?tab=search`, `?tab=notifications`, `?tab=dms`, `?tab=saved`, and `?tab=members`.
10. For each client-panel URL, verify the client panel is the only visible primary panel and the feed HTMX panel is not visually competing with it.
11. Resize to mobile width and verify the mobile Community tab controls expose every same destination.
12. Log out, then open `/community` and `/community/partials/feed`.

Expected Result:
- Community shell loads for authenticated users.
- Every tab loads the matching panel or partial.
- URL state and active tab state stay in sync.
- Client-panel deep links do not leave stale Feed content visible or focusable.
- Protected Community routes redirect logged-out users to `/auth/login`.
- Tab switching does not duplicate panels, leave stale active states, or create console errors.

Known local audit note:
- During the 2026-05-29 browser pass, direct `?tab=search`, `?tab=notifications`, and `?tab=dms` activated the correct topbar button but left `community-feed-tab` visible in the DOM. Treat this as a regression candidate until fixed and rechecked.
- `/community?tab=members` is linked from Community content but is not part of the visible topbar map in the last audit. Verify it activates an intentional member directory panel or classify it as a route/linking defect.

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
