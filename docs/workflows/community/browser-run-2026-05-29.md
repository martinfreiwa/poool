# Community Browser Workflow Run — 2026-05-29

Purpose: Document the browser execution of Community workflows against local `http://localhost:8888`.

Run metadata:
- Date/time: 2026-05-29 17:42 WAT.
- Browser target: Codex in-app browser.
- Backend: `cargo run` on port `8888`.
- Account state: logged in as `support@traffic-creator.com`.
- Scope: started as a non-mutating browser pass, then expanded through disposable-data E2E coverage for Community and Admin Community workflows. Navigation, tab activation, visibility checks, URL checks, page-shell checks, admin route checks, feed partial checks, and representative mutating workflows were executed.
- Follow-up route smoke: repeated at 17:50 WAT after the service-worker navigation fallback fix.

## Workflows Exercised

| Workflow | Browser Coverage | Result |
|----------|------------------|--------|
| Shell and Tab Navigation | `/community`, primary tab URLs, client-panel URLs | Pass. |
| Route and Link Integrity | Community tab URLs, admin community index/canonical URLs | Pass. |
| Feed | `/community`, `/community?tab=feed`, feed page-2 partial | Pass. |
| Announcements | Topbar click to `/community?tab=announcements` | Pass. |
| Circles | Topbar click to `/community?tab=circle` | Pass. |
| Challenges | Topbar click to `/community?tab=challenges` | Pass. |
| Expert AMAs | Topbar click to `/community?tab=ama` | Pass. |
| Search, Messages, Notifications | `/community?tab=search`, `/community?tab=dms`, `/community?tab=notifications` | Pass. |
| Saved Posts and Bookmarks | `/community?tab=saved` | Pass. |
| Members | `/community?tab=members` | Pass. |
| Circle Q&A Feed Filter | `/community/circle/:slug`, Q&A filter entrypoint | Pass after adding the visible Q&A tab affordance. |
| Admin Community Operations | `/admin/community/index`, `/admin/community/` | Pass. |
| Admin Community Mobile Dialogs | `/admin/community/challenges`, mobile viewport | Pass after hiding the desktop admin sidebar at mobile width. |
| Community route smoke | Challenges, AMAs, Search, Messages, Notifications, Saved, Members, Circles, own profile, profile edit | Pass after service-worker abort handling fix. |

## Detailed Results

### Community shell and direct tabs
- `/community?tab=feed`: active `Feed`, visible `community-feed-tab`.
- `/community?tab=search`: active `Search`, visible `community-search-tab`; feed content hidden.
- `/community?tab=notifications`: active `Notifications`, visible `community-notifications-tab`; feed content hidden.
- `/community?tab=dms`: active `Messages`, visible `community-dms-tab`; feed content hidden.
- `/community?tab=saved`: active `Saved`, visible `community-saved-tab`, saved feed container loaded.
- `/community?tab=members`: active `Members`, visible `community-members-tab`, 30 member rows loaded.

### HTMX-backed topbar tabs
- Clicking `Announcements` activated `community-announcements-tab` and pushed `/community?tab=announcements`.
- Clicking `My Circles` activated `community-circle-tab` and pushed `/community?tab=circle`.
- Clicking `Challenges` activated `community-challenges-tab` and pushed `/community?tab=challenges`.
- Clicking `Expert AMAs` activated `community-ama-tab` and pushed `/community?tab=ama`.
- Clicking `Feed` returned to `community-feed-tab` and pushed `/community?tab=feed`.

### Admin Community
- `/admin/community/index` redirected to `/admin/community/`.
- `/admin/community/` loaded with title `Community Dashboard | Admin | POOOL`.
- Main heading: `Community Overview`.

### Feed partial and pagination query
- `/community/partials/feed/list?page=2&feed_mode=all&sort_by=fresh` returned feed HTML.
- The previous duplicate `feed_mode` deserialization failure was not reproduced after forcing the feed pagination sentinel to use its own URL query via `hx-include="this"`.

### Route smoke and service-worker fallback
- A fast route-smoke pass briefly reproduced `offline.html` for several Community routes while the backend was still reachable.
- The service worker previously treated every navigation `fetch()` failure as a real offline state.
- Fixed by letting `AbortError` from superseded navigations bubble instead of rendering the offline shell.
- Repeated route smoke loaded all checked Community routes without offline fallback:
  - `/community?tab=challenges`
  - `/community?tab=ama`
  - `/community?tab=search`
  - `/community?tab=dms`
  - `/community?tab=notifications`
  - `/community?tab=saved`
  - `/community?tab=members`
  - `/community/circles`
  - `/community/me`
  - `/community/me/edit`

### Follow-up E2E workflow pass
- Ran the broader Community E2E group covering feed accessibility, Feed UI, DMs, Notifications, and Admin Reports.
- Found an outdated Circle Settings test that still expected the removed in-page `#circle-settings-modal`. Updated it to the current `/community/circle/:slug/settings` workflow.
- Found that Playwright route mocks were being bypassed by an already registered service worker. E2E browser contexts now block service workers so workflow tests exercise the current app/API directly.
- Found `pwa-install.js` crashed when service worker registration is unavailable or blocked. Added a defensive null-registration guard.
- Found the Community page did not render the global inbox-bell host, so the unread badge/dropdown workflow could not start. Added the bell markup to the Community topbar.
- Verification passed:
  - `python3 -m pytest tests/e2e/test_community.py tests/e2e/test_community_feed_ui.py tests/e2e/test_community_dm_flow.py tests/e2e/test_community_notifications_ui.py tests/e2e/test_admin_community_reports.py -q` → 27 passed.
  - `python3 -m pytest tests/test_community_tab_contract_static.py tests/test_community_circles_phase8_static.py tests/e2e/test_community_circle_settings_ui.py -q` → 32 passed, 1 skipped.

### Follow-up mutating workflow pass
- Added and ran an owner-post browser workflow that opens the owner menu, validates blank edit rejection, saves edited content, verifies DB persistence, opens a delete confirmation modal, deletes the post, and verifies the DB row is gone.
- Replaced native own-post `confirm()` deletion with the in-app `delete-post-modal` so the workflow is automatable like Circle deletion.
- Added and ran Poll edge-case workflows for single-choice vote replacement, expired poll UI/API rejection, and wrong-poll option rejection.
- Stabilized the Admin Community Users detail-link workflow after the broad run exposed popup timing/stale-row coupling around the `View` action.
- Verification passed:
  - `python3 -m pytest tests/e2e/test_community_feed_ui.py::test_feed_owner_edit_validation_and_delete_modal -q` → 1 passed.
  - `python3 -m pytest tests/e2e/test_community_polls_ui.py -q` → 5 passed.
  - `python3 -m pytest tests/e2e/test_admin_community_users.py -q` → 2 passed.
  - `python3 -m pytest tests/e2e/test_community*.py tests/e2e/test_admin_community*.py -q` → 103 passed, 1 skipped.

### Final broad workflow verification
- Ran the complete Community/Admin Community browser E2E group after the later browser fixes.
- Coverage included Community shell, accessibility audits, Circle discovery/journey/settings, DMs, feed workflows, follow/block, mobile tabs, notifications, polls, profile tabs, admin AMAs, announcements, badges, challenges, comments, index, moderation, reports, user detail, and users.
- Fixed additional issues found during this pass:
  - Admin mobile viewport: the desktop sidebar still intercepted mobile dialog clicks. Fixed by hiding `.admin-sidebar` and resetting `.admin-main` below `768px`.
  - Accessibility audit readiness: `networkidle` was flaky because Community pages keep background fetches/pollers alive. Replaced it with page-specific DOM readiness checks.
  - Custom E2E helper contexts still allowed service workers. Blocked service workers in those contexts too.
  - Admin Community Users ban success status was overwritten by the table reload. The success status is now restored after `loadUsers()`.
  - Circle Q&A workflow had backend/static coverage but no visible in-circle Q&A entrypoint. Added `All` and `Q&A` feed-view tabs to the Circle page.
  - Static tests for Profile and User Bridge still enforced old implementation details. Updated them to assert the current `cp-*` profile layout and `normalize_email(...)` nullable-email contract.
- Verification passed:
  - `python3 -m pytest tests/test_community*.py tests/admin/test_admin_community*.py -q` → 82 passed.
  - `python3 -m pytest tests/e2e/test_community*.py tests/e2e/test_circles.py tests/e2e/test_admin_community*.py -q` → 105 passed, 1 skipped.
  - `cargo check` → passed.
  - `cargo fmt --check` → passed.
  - `git diff --check` → passed.

## Fixes Applied From This Run

- Added dedicated client panels for Saved and Members.
- Added hidden topbar tab buttons for Saved and Members so deep links and bottom navigation have real tab targets.
- Changed client-tab switching to hide the HTMX feed content area and any late-swapped feed panel.
- Removed reliance on newly assigned `window.*` functions because the browser environment marks `window` as non-extensible.
- Rewired Community tab switching through DOM event listeners and button clicks.
- Replaced the AMA sidebar button's `window.activateCommunityTab('ama')` dependency with `data-community-tab="ama"`.
- Fixed admin community index canonicalization by routing `/admin/community/index` to `/admin/community/`.
- Prevented inherited feed filter parameters from duplicating on pagination/load-more requests.
- Prevented aborted service-worker navigations from being rendered as offline pages during rapid workflow passes.
- Restored the Community inbox-bell workflow by rendering `#inbox-bell-btn` and `#inbox-bell-badge` in the Community topbar.
- Hardened PWA registration when service workers are unavailable.
- Blocked service workers in E2E browser contexts to avoid stale-cache and route-mock interference.
- Blocked service workers in custom Community helper contexts as well.
- Replaced own-post native delete confirmation with an in-app modal and added browser coverage for edit validation, edit persistence, and delete persistence.
- Added Poll edge-case coverage for vote replacement, expired polls, and invalid option ownership.
- Added a visible Circle Q&A feed-view entrypoint and updated static contracts for the current product behavior.
- Fixed admin mobile sidebar click interception.
- Stabilized accessibility audits with page-specific readiness checks.
- Preserved Admin Community Users mutation success status after table reload.
- Stabilized Admin Community Users `View` detail navigation by opening the href directly after re-querying the refreshed table row.

## Not Executed

The following workflow areas were still not executed end-to-end in this browser pass because they require broader cross-role setup, external services, file uploads, or destructive/admin state changes beyond the disposable data used here:
- Sending real DMs beyond UI/rendering coverage.
- Changing notification preferences.
- Joining/leaving circles, invites, bans, role changes, ownership transfer, token gates.
- Resource uploads/versioning with real files.
- Admin badge grants, XP awards, settings changes, audit export.
- Ban appeal submission/review.

## Residual Risk

- Mobile bottom-nav physical tapping could not be forced in the desktop Browser session because the tool's viewport/style mutation path is read-only. The same target buttons now call the same DOM tab buttons used by the verified direct URL and topbar flows.
- Screenshot capture was not used; the run is documented from DOM, URL, active-tab, visible-panel, title, heading, and partial-response checks.
- Mutating workflows are covered where listed in `community-workflow-run-2026-05-29.md`; the remaining cross-role, upload, and destructive/admin mutations listed above still require dedicated disposable-data execution.
