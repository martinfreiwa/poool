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
