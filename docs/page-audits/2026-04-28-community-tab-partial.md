# Page Audit: Community Tab Partial

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/community/partials/:tab`
Template: dynamic fragment route serving `frontend/platform/partials/community_feed.html`, `frontend/platform/partials/community_announcements.html`, `frontend/platform/partials/community_circle.html`, `frontend/platform/partials/community_ama.html`, and `frontend/platform/partials/community_challenges.html`
JavaScript: `frontend/platform/static/js/community-feed.js`, `frontend/platform/static/js/community-circles.js`, `frontend/platform/static/js/community-amas.js`, inline Alpine/HTMX handlers in `frontend/platform/community.html` and `frontend/platform/components/investor-topbar.html`
CSS: `frontend/platform/static/css/community.css`, `frontend/platform/static/css/leaderboard.css`, shared design-system bundle
Backend Routes: `backend/src/main.rs`, `backend/src/community/routes.rs`

---

## Summary

The dynamic community tab partial route is registered and protected, and it serves the expected fragment template for `feed`, `announcements`, `circle`, `ama`, and `challenges`.

2026-04-28 fix update: the AMA tab now targets the current `community_ama.html` DOM contract and auto-loads after HTMX swaps; the Challenges tab now has a loaded `communityChallenges()` controller with API/error/empty-state handling; Circle dynamic rows now render user data via DOM nodes and `textContent`; and static plus Playwright regression coverage was added.

2026-04-28 follow-up: unauthenticated partial redirect coverage and seeded Circle modal keyboard/mobile coverage were added to the E2E suite, and Circle modals now expose described dialog semantics, aria-hidden state updates, mobile viewport height bounds, explicit labels, and focus restoration for trigger elements without IDs.

Remaining issue: the expanded authenticated browser suite still needs to execute against a live local backend.

---

## Tested Scope

- Read required project context, development rules, design reference, database schema, security docs, production-readiness standards, and tracker.
- Selected exactly one tracker item: `community.partial-tab` (`/community/partials/:tab`).
- Reviewed the dynamic route and its protected MiniJinja rendering path.
- Reviewed all tab selectors in `components/investor-topbar.html`.
- Reviewed the five tab fragments served by this route.
- Reviewed page scripts used by the fragments: community feed, circles, AMAs, HTMX init, and inline Alpine handlers.
- Reviewed related backend API route registration for circles, XP, AMAs, challenges, feed list, and announcements list.
- Checked existing E2E coverage in `tests/e2e/test_community.py`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/community/partials/:tab` | HTMX fragment route for topbar community tabs |
| Parent page | `frontend/platform/community.html` | Loads the HTMX content host and community scripts |
| Topbar component | `frontend/platform/components/investor-topbar.html` | Buttons call `/community/partials/feed`, `/announcements`, `/circle`, `/ama`, `/challenges` |
| Fragment | `frontend/platform/partials/community_feed.html` | Compose UI, feed filters, feed list HTMX host |
| Fragment | `frontend/platform/partials/community_announcements.html` | Announcement filters and announcement list HTMX host |
| Fragment | `frontend/platform/partials/community_circle.html` | Circle, XP, leaderboard, invites, settings modals |
| Fragment | `frontend/platform/partials/community_ama.html` | AMA loading, hero, questions, question form |
| Fragment | `frontend/platform/partials/community_challenges.html` | Alpine-driven challenge list |
| JS | `frontend/platform/static/js/community-feed.js` | Feed compose, uploads, sidebar widgets, feed helpers |
| JS | `frontend/platform/static/js/community-circles.js` | Circle and XP APIs |
| JS | `frontend/platform/static/js/community-amas.js` | AMA APIs, but DOM contract does not match fragment |
| Backend page route | `GET /community/partials/:tab` | `community_htmx_partial` in `backend/src/main.rs` |
| Backend API route | `GET /api/community/challenges` | Registered in `backend/src/community/routes.rs` |
| Backend API route | `GET /api/community/amas`, `GET /api/community/amas/:id`, `POST /api/community/amas/:id/questions`, `POST /api/community/amas/:id/questions/:qid/upvote` | Registered in `backend/src/community/routes.rs` |
| Backend API route | `/api/community/circles*`, `/api/community/xp*`, `/api/community/invites*` | Registered in `backend/src/community/routes.rs` |
| Database tables | `posts`, `announcements`, `community_profiles`, `circles`, `circle_members`, `circle_join_requests`, `xp_events`, `challenges`, `challenge_progress`, `amas`, `ama_questions` | Inferred from backend community modules and SQL queries |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Feed tab | `.community-tab-btn[data-tab="community-feed-tab"]` | Swap feed fragment into `#community-content-area` | Yes, HTMX | Yes | Unverified runtime; static route OK |
| Announcements tab | `.community-tab-btn[data-tab="community-announcements-tab"]` | Swap announcements fragment | Yes, HTMX | Yes | Existing E2E only checks panel visibility |
| My Circle tab | `.community-tab-btn[data-tab="community-circle-tab"]` | Swap circle fragment and load circle/XP APIs | Yes, via HTMX and `community-circles.js` after swap | Yes | Unverified runtime; static route/API map OK |
| Expert AMAs tab | `.community-tab-btn[data-tab="community-ama-tab"]` | Swap AMA fragment and load AMA data/questions | Fixed locally: JS matches current fragment IDs and auto-loads after swap | Yes | Static contract test passed; browser run pending |
| Challenges tab | `.community-tab-btn[data-tab="community-challenges-tab"]` | Swap challenges fragment and render challenge list | Fixed locally: `communityChallenges()` is defined and loaded | Yes | Static contract test passed; browser run pending |
| Feed filters | `#feed-btn-all`, `#feed-btn-following`, `#sort-btn-fresh`, `#sort-btn-hot` | Update hidden filters and reload feed list | Yes, `community-feed.js` | Yes, feed list route | Covered by separate feed partial audit |
| Create post composer | `#post-content-input`, `#fb-post-type-select`, upload/poll/tag buttons, `#submit-post-btn` | Validate, upload optional images, POST `/api/community/posts`, reload feed | Yes | Yes | Covered by separate feed/parent audits |
| Announcement filters | `.ann-filter-btn` | Load announcement list by category | Yes, HTMX | Yes | Covered by separate announcements partial audit |
| Circle create modal | `#create-circle-modal`, `#circle-name-input`, `handleCreateCircle()` | POST `/api/community/circles` | Yes; dialog semantics and mobile bounds fixed locally | Yes | Static contract passed; browser run pending |
| Circle settings modal | `#circle-settings-modal`, `openCircleSettings()`, `handleSaveCircleSettings()` | PUT circle details and POST privacy | Yes; keyboard/focus/mobile E2E added with seeded API fixtures | Yes | E2E coverage added; browser run pending |
| Circle delete | `handleDeleteCircle()` | DELETE circle | Yes | Yes | Needs protected-owner test coverage |
| Join/request circle | Generated buttons in circle leaderboard | POST join/request endpoints | Fixed locally with DOM-created buttons/listeners | Yes | Static XSS contract test passed; browser run pending |
| AMA question form | `#ama-question-input`, `#ama-question-submit-btn` | Submit question for active AMA | Fixed locally | Yes | Static contract test passed; browser run pending |
| AMA upvote buttons | Generated by `community-amas.js` | POST upvote endpoint | Fixed locally | Yes | Static contract test passed; browser run pending |
| Challenges list | `x-data="communityChallenges()"`, `x-for="ch in challenges"` | Fetch and render active challenges | Fixed locally | Yes | Static contract test passed; browser run pending |

---

## Frontend Findings

### P1 - AMA tab JavaScript and fragment DOM contract do not match

Status: fixed locally on 2026-04-28.

Location:

- Template: `frontend/platform/partials/community_ama.html:20`
- JS: `frontend/platform/static/js/community-amas.js:21`

Problem:

The AMA fragment defines `#ama-content`, `#ama-status-badge`, `#ama-title`, `#ama-description`, `#ama-date-time`, `#ama-expert-name`, and `#ama-question-submit-btn`. The loaded AMA script looks for different IDs such as `#ama-hero`, `#ama-questions-section`, `#ama-past-section`, `#ama-hero-badge`, `#ama-hero-title`, `#ama-hero-expert`, `#ama-hero-desc`, `#ama-hero-date-text`, `#ama-submit-question-btn`, `#ama-question-count`, `#ama-question-modal`, and `#ama-question-charcount`.

Expected:

The fragment and script should use one shared DOM contract. Loading the AMA tab should fetch `/api/community/amas`, render an active or empty state, render questions, and allow valid question/upvote actions.

Evidence:

Static selector comparison shows `community-amas.js` dereferences missing elements immediately after API responses. For example, `hero.style.display = 'none'` and `renderHero()` will throw when `#ama-hero` is null. `node --check frontend/platform/static/js/community-amas.js` passes syntax, so this is a runtime contract failure rather than a parse error.

Recommended fix:

Implemented: `frontend/platform/static/js/community-amas.js` now targets the existing fragment IDs, uses null-safe DOM helpers, renders questions with `textContent`, auto-loads after HTMX swaps, and has static DOM-contract coverage in `tests/test_community_tab_contract_static.py`. Runtime authenticated E2E execution remains pending under the remaining issue below.

### P1 - Challenges tab references an undefined Alpine component

Status: fixed locally on 2026-04-28.

Location:

- Template: `frontend/platform/partials/community_challenges.html:1`
- Backend API: `backend/src/community/routes.rs` registers `GET /api/community/challenges`

Problem:

The Challenges fragment uses `x-data="communityChallenges()"`, but repository search found no JavaScript definition for `communityChallenges`. The fragment includes a small inline script that dispatches `alpine:init`, but it does not register the component or fetch `/api/community/challenges`.

Expected:

The Challenges tab should register an Alpine component, fetch `/api/community/challenges`, render loading/error/empty/success states, and expose user progress safely.

Evidence:

`rg "communityChallenges"` only found the template usage. A Python search over `frontend/platform/**/*.js` found no definitions.

Recommended fix:

Implemented: `frontend/platform/static/js/community-challenges.js` defines `window.communityChallenges`, fetches `/api/community/challenges`, maps `current_value`/`requirement_value` into template fields, exposes retry/error/empty states, and guards zero-target progress calculations. `frontend/platform/community.html` now loads the controller.

### P2 - Circle tab renders circle data through string-built `innerHTML`

Status: fixed locally on 2026-04-28.

Location:

- JS: `frontend/platform/static/js/community-circles.js:154`
- JS: `frontend/platform/static/js/community-circles.js:194`

Problem:

`renderMembers()` and `loadCircleLeaderboard()` build HTML strings and assign `container.innerHTML`. The leaderboard path injects API values such as `c.name` and `c.avatar_emoji` directly into markup. Circle names and emoji are user-created values, so this is an avoidable XSS surface on an authenticated community page.

Expected:

User-controlled values should be rendered with `textContent` or DOM node construction. Static badges/buttons can be created as trusted elements, but user data must not enter HTML strings.

Evidence:

The same script correctly uses `textContent` for several profile and circle fields, but the leaderboard and member list still use template literals and `innerHTML`.

Recommended fix:

Implemented: `frontend/platform/static/js/community-circles.js` now builds member, leaderboard, invite, join-request, and XP history rows with DOM nodes and `textContent`; generated action controls use event listeners instead of inline handlers. Static coverage blocks the previous user-data string interpolation patterns.

### P3 - Dynamic tab coverage is too shallow

Status: partially fixed; needs runtime recheck.

Location:

- Tests: `tests/e2e/test_community.py`

Problem:

Existing community E2E coverage only asserts that `/community` loads and that the Announcements tab panel becomes visible. It does not cover the dynamic route directly, Circle tab API rendering, AMA success/error paths, Challenges rendering, invalid tab handling, mobile behavior, keyboard behavior, console errors, or the child fragment list reloads.

Expected:

Each tab served by `/community/partials/:tab` should have at least one authenticated browser smoke test plus targeted tests for broken/error states.

Evidence:

`tests/e2e/test_community.py` currently has `test_community_feed_load` and `test_community_announcements`; neither covers `/community/partials/circle`, `/community/partials/ama`, `/community/partials/challenges`, or `/community/partials/not-a-tab`.

Recommended fix:

Implemented: `tests/e2e/test_community.py` now includes coverage for Circle, AMA, Challenges, console health through the shared tracker, invalid partial 404, unauthenticated partial redirects, and seeded Circle settings modal keyboard/mobile behavior. Remaining: run the authenticated browser tests against a live backend and add deeper mutation success/failure fixtures as needed.

---

## Backend Findings

- `GET /community/partials/:tab` is registered after the more specific `/community/partials/feed/list` and `/community/partials/announcements/list` routes, so route ordering is safe.
- The route maps only known tab names and returns 404 for unknown tabs.
- The route uses `serve_protected`, so unauthenticated fragment requests should follow the shared protected-page behavior.
- No production-path `unwrap()` or `expect()` was found in `community_htmx_partial`.
- Backend APIs for AMA and Challenges exist, but the frontend fragments do not consume them successfully.
- Separate child-list routes still mask backend data errors with `unwrap_or_default()` for feed and announcements. Those are documented in the feed and announcement partial audits, but they also affect the tab route user experience.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route registration | Reviewed `backend/src/main.rs` route map | `/community/partials/:tab` registered and protected | Route registered and uses `serve_protected` | Pass |
| Tab selector map | Reviewed topbar HTMX buttons | Feed, Announcements, Circle, AMA, Challenges call matching fragment routes | Routes match backend tab names | Pass |
| AMA DOM contract | Compared `community_ama.html` IDs to `community-amas.js` selectors | Selectors should match | Multiple required IDs missing from fragment | Fail |
| Challenges component contract | Searched for `communityChallenges` definition | Alpine component should exist | No definition found | Fail |
| JS syntax | `node --check` for community-feed, community-circles, community-amas | No syntax errors | Passed | Pass |
| Original audit Rust build smoke | `cd backend && cargo check` | Build check passes | Passed during original page audit | Pass |
| Fix-run isolated Rust build smoke | `CARGO_TARGET_DIR=/tmp/poool-community-tab-fix-target-2 cargo check --message-format=short` | Build check passes | Progressed through dependency compilation, then the tool session exited with code -1 and no Rust diagnostics | Blocked |
| Runtime browser/curl | Check local `:8888` listener | App should be reachable for browser/curl smoke | No process listening on `:8888` | Blocked |
| Static regression | `python3 -m pytest tests/test_community_tab_contract_static.py -q` | Static contracts pass | Passed | Pass |
| E2E syntax | `python3 -m py_compile tests/e2e/test_community.py` | Test file compiles | Passed | Pass |
| Modal static contract | `python3 -m pytest tests/test_community_tab_contract_static.py -q` | Circle modal dialog/mobile/focus-helper contract is covered | Passed | Pass |

---

## Security Findings

- Fixed locally: Circle tab dynamic user data no longer renders through string-built `innerHTML`.
- State-changing circle and AMA actions rely on the shared fetch CSRF interceptor. This is acceptable if the interceptor loads, but tests should explicitly cover missing/invalid CSRF for circle create/update/delete/join/request and AMA question/upvote actions.
- The dynamic fragment route itself does not expose admin-only data and rejects unknown tab names.

---

## Database Findings

- The tab route itself performs no direct database writes.
- Circle, AMA, and Challenges APIs depend on community tables. The route/API map is present, but the broken frontend contracts prevent verification that those tables render correctly through the tab route.
- No monetary values or financial database mutations are part of this page.

---

## Missing Tests

- Added locally: authenticated browser smoke for Circle, AMA, Challenges, and invalid tab 404 in `tests/e2e/test_community.py`.
- Added locally: static DOM-contract and XSS-rendering regression checks in `tests/test_community_tab_contract_static.py`.
- Still missing: executed authenticated browser run against a live backend.
- Added locally: explicit unauthenticated redirect checks for `/community/partials/:tab`.
- Still missing: deeper AMA fixture test for question submission/upvote success states.
- Added locally: seeded Circle settings modal keyboard/focus/mobile coverage.
- Still missing: deeper Circle fixture test for no-circle, existing-circle, leaderboard join/request, settings save/delete, and CSRF failures.
- Still missing: mobile and keyboard tests for topbar tab scrolling and remaining icon-only controls.

---

## Recommended Fix Order

1. Run the expanded authenticated Community dynamic-tab E2E tests against a live backend.
2. Add deeper seeded AMA and Circle mutation tests, including CSRF failure cases.
3. Add mobile/keyboard coverage for topbar tab scrolling and remaining icon-only controls.

---

## Final Status

`needs_recheck`

Reason: The documented functional/security/accessibility defects are fixed locally and covered by static or E2E test definitions, but the expanded authenticated browser suite still needs to execute against a live backend.
