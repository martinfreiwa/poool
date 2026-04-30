# Page Audit: Community

Date: 2026-04-28
Status: fixed_pending_runtime_recheck
Auditor: ChatGPT/Codex
Page URL: `/community`
Template: `frontend/platform/community.html`
JavaScript: `frontend/platform/static/js/community-feed.js`, `frontend/platform/static/js/community-circles.js`, `frontend/platform/static/js/community-amas.js`, `frontend/platform/static/js/htmx-init.js`
CSS: `frontend/platform/static/css/community.css`, `frontend/platform/static/css/leaderboard.css`, shared dashboard bundle
Backend Routes: `backend/src/main.rs`, `backend/src/community/routes.rs`, `backend/src/community/service.rs`, `backend/src/community/circles.rs`, `backend/src/storage/routes.rs`

---

## Summary

The authenticated `/community` shell and its feed, announcements, circle, AMA, challenge, notification, bookmark, poll, hashtag, profile, upload, and moderation/reporting API surfaces are substantially implemented. Global CSRF middleware covers mutating fetches, and core write paths use backend authentication plus community database persistence.

2026-04-28 fix pass resolved the documented code issues in the working tree. The page still needs authenticated browser/E2E recheck because the local backend/build environment was not available for runtime verification during this pass.

Fixes applied:

- Replaced audited profile/circle API renderers with DOM construction and `textContent`.
- Exposed Search, Notifications, and Saved posts from the community topbar and enabled topbar search.
- Replaced `/community/feed?...` search-result links with registered profile modal and `/community/post/:id` behavior.
- Propagated community feed/announcement partial loader errors instead of rendering backend failures as empty states.
- Moved direct shared-post metadata lookup to the community database and added direct post rendering from `SSR_POST_ID`.
- Added dialog semantics, Escape close, focus trap/restoration, close labels, and composer icon labels for community modals/controls.

---

## Tested Scope

- Reviewed `frontend/platform/community.html`.
- Reviewed community partials: feed, post card/list, announcements, circle, AMA, and challenges.
- Reviewed `community-feed.js`, `community-circles.js`, `community-amas.js`, `htmx-init.js`, mobile navigation, and shared head CSRF behavior.
- Reviewed route registration in `backend/src/main.rs` and `backend/src/community/routes.rs`.
- Reviewed storage upload route for post images.
- Reviewed community schema migrations for posts, comments, reactions, reports, notifications, circles, bookmarks, polls, hashtags, XP, and badges.
- Checked existing `tests/e2e/test_community.py`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/community` | Authenticated SSR shell via `serve_protected`. |
| URL | `/community/post/:id` | Public/mixed share page using the same template. |
| HTMX partial | `/community/partials/:tab` | Feed, announcements, circle, AMA, challenges. |
| HTMX partial | `/community/partials/feed/list` | Server-rendered feed list. |
| HTMX partial | `/community/partials/announcements/list` | Server-rendered announcement list. |
| API | `/api/community/feed`, `/api/community/search`, `/api/community/trending-assets` | Feed/search/sidebar data. |
| API | `/api/community/posts*` | Create, detail, update, delete, report, reactions, comments. |
| API | `/api/community/profile*`, `/api/community/follow/:id` | Profile modal and follow actions. |
| API | `/api/community/circles*`, `/api/community/invites*` | Circle, invite, join request, settings actions. |
| API | `/api/community/notifications*` | Notification tab and unread state. |
| API | `/api/community/bookmarks*`, `/api/community/hashtags*`, `/api/community/posts/:id/poll*` | Saved posts, hashtag feeds, native polls. |
| API | `/api/upload/post-image` | Community post image upload. |
| Database | community migrations under `database/community/` | Posts, comments, reports, profiles, circles, notifications, bookmarks, polls, hashtags. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Audit Result |
|--------|---------------------|-------------------|-----------------|----------------|--------------|
| Topbar Feed tab | `.community-tab-btn[data-tab="community-feed-tab"]` | HTMX load feed partial. | Yes. | Yes. | Static verified. |
| Topbar Announcements tab | `.community-tab-btn[data-tab="community-announcements-tab"]` | HTMX load announcement partial. | Yes. | Yes. | Static verified. |
| Topbar My Circle tab | `.community-tab-btn[data-tab="community-circle-tab"]` | HTMX load circle partial and initialize JS. | Yes. | Yes. | Static verified; XSS/rendering gaps below. |
| Topbar Expert AMAs tab | `.community-tab-btn[data-tab="community-ama-tab"]` | HTMX load AMA partial and JS. | Yes. | Yes. | Static verified. |
| Topbar Challenges tab | `.community-tab-btn[data-tab="community-challenges-tab"]` | HTMX load challenges partial. | Yes. | Yes. | Static verified. |
| Topbar search field | `.lb-topbar-search input` | Search community. | Disabled. | Search API exists. | Dead visible control. |
| Hidden Search tab | `#community-search-tab` | Search posts/users. | Alpine code exists. | API exists. | Not reachable from topbar; result links route to unregistered `/community/feed`. |
| Hidden Notifications tab | `#community-notifications-tab` | List and mark notifications read. | Alpine code exists. | API exists. | Not reachable from topbar. |
| Hidden Saved tab | `#community-saved-tab` | Load bookmarks. | Function exists. | API exists. | Not reachable from topbar. |
| Feed filters | `#feed-btn-all`, `#feed-btn-following`, `#sort-btn-fresh`, `#sort-btn-hot` | Update hidden inputs and HTMX reload. | Yes. | Feed query supports mode/sort. | Static verified. |
| Post composer | `#post-content-input`, `#fb-post-type-select`, `#submit-post-btn` | Create post. | Yes. | `POST /api/community/posts`. | Static verified. |
| Image upload | `#post-image-file-input` | Upload post image, show previews. | Yes. | `POST /api/upload/post-image`. | Static verified; no runtime GCS/local verification. |
| Poll creator | `#poll-creator`, poll inputs, expiry select | Attach poll to post. | Yes. | Poll create/results/vote routes exist. | Static verified. |
| Reaction button | `.feed-reaction-btn` | Toggle reaction. | Yes. | `POST /api/community/posts/:id/reactions`. | Static verified. |
| Comments button/input | `#comments-section-*`, `#comment-input-*` | Load and post comments. | Yes. | `GET/POST /api/community/posts/:id/comments`. | Static verified. |
| Share button | inline clipboard handler | Copy `/community/post/:id`. | Yes. | Share page route exists. | Partial; share route has DB mismatch risk. |
| Bookmark button | `.feed-bookmark-btn` | Toggle bookmark. | Yes. | Bookmark APIs exist. | Static verified. |
| Report modal | `#report-post-modal` | Submit content report. | Yes. | `POST /api/community/posts/:id/report`. | Static verified; modal a11y incomplete. |
| Profile modal | `#user-profile-modal` | Load profile and follow/unfollow. | Yes. | Profile/follow APIs exist. | Static verified; unsafe badge/avatar rendering. |
| Edit profile modal | `#edit-profile-modal` | Save bio. | Function exists. | `PUT /api/community/profile`. | Modal open path unclear; profile card links to `/settings`. |
| Trending assets | `#trending-assets-container` | Link to registered property/commodity detail route. | Yes. | API now returns `detail_url`. | Prior PAGE-ISSUE-0004 fixed; browser recheck still needed. |
| Trending tags | `#trending-hashtags-container` | Filter feed by hashtag. | Yes. | Hashtag APIs exist. | Static verified. |
| Circle create/settings/invite modals | `#create-circle-modal`, `#circle-settings-modal`, `#invite-modal` | Create/update/delete/copy invite. | Yes. | Circle APIs exist. | Static verified; modal a11y and unsafe rendering gaps. |
| Circle leaderboard join/request | `handleJoinCircle`, `handleRequestJoinCircle` | Join/request circle. | Yes. | APIs exist. | Static verified; unsafe rendering gap. |
| Pending invite/request actions | accept/decline/approve/decline handlers | Mutate invites and requests. | Yes. | APIs exist. | Static verified. |
| AMA question form | `#ama-question-input`, `submitQuestion()` | Submit AMA question. | Yes. | AMA APIs exist. | Static verified. |
| Notifications read actions | `markAllAsRead`, `handleClick` | Mark notifications read and follow link. | Yes. | APIs exist. | Hidden from navigation. |

---

## Findings And Fix Status

### Fixed High - Circle/profile renderers interpolate community-controlled values into `innerHTML`

Locations:

- `frontend/platform/static/js/community-feed.js:288`
- `frontend/platform/static/js/community-feed.js:299`
- `frontend/platform/static/js/community-circles.js:160`
- `frontend/platform/static/js/community-circles.js:194`
- `frontend/platform/static/js/community-circles.js:510`
- Backend source fields: `backend/src/community/circles.rs:49`, `backend/src/community/circles.rs:124`

Problem:

Several frontend renderers build HTML strings from data returned by community APIs and assign those strings with `innerHTML`. Profile badges use `b.name` and `b.icon`; circle renderers use `c.name`, `c.avatar_emoji`, `m.role`, and join-request `req.user_name`. Circle creation/update stores name, description, and emoji without escaping or strict validation in the user-facing create/update paths, so an authenticated user can potentially persist markup that later renders in another authenticated user's community page.

Expected:

Use DOM construction and `textContent` for all API-provided profile, badge, circle, invite, and request fields. Validate circle names/descriptions/emoji server-side for length and allowable display content.

Evidence:

Static review found `container.innerHTML = html` patterns populated from community API JSON. Node syntax checks passed, but no browser XSS fixture test exists for profile badges, circle names, or join requests.

Recommended fix:

Replace string template renderers in community profile/circle modules with DOM APIs. Add a Playwright fixture that creates or stubs a circle/profile response containing HTML-like payloads and asserts no script/attribute execution and escaped text rendering.

Fix:

`community-feed.js` now renders profile badges, avatar initials, and image previews with DOM APIs and text nodes. `community-circles.js` now renders member rows, circle leaderboard rows, pending invites, and join requests with DOM APIs and event listeners instead of API-populated HTML strings.

Remaining:

Add browser XSS fixture coverage with hostile profile/circle payloads.

### Fixed Medium - Search, notifications, and saved posts are shipped but unreachable from page navigation

Locations:

- `frontend/platform/community.html:67`
- `frontend/platform/community.html:310`
- `frontend/platform/community.html:470`
- `frontend/platform/components/investor-topbar.html` community variant

Problem:

The template includes full Search, Notifications, and Saved Posts panels, and the backend APIs for search, notifications, and bookmarks exist. The topbar community variant exposes only Feed, Announcements, My Circle, Expert AMAs, and Challenges. The visible topbar search input is disabled. Because the hidden panels are never reachable through normal navigation, users cannot access shipped functionality and automated E2E coverage does not exercise it.

Expected:

Either add real navigation for Search/Notifications/Saved and initialize their data loaders, or remove the hidden panels until the product intentionally exposes them. The visible search input should not be disabled if the search panel is considered part of the page.

Evidence:

Static review found hidden panels and active APIs, but no `.community-tab-btn` targets for `community-search-tab`, `community-notifications-tab`, or `community-saved-tab`.

Recommended fix:

Decide the intended community navigation set, then wire topbar buttons or a secondary nav for these panels. Add browser coverage for each exposed tab.

Fix:

The community topbar now exposes Search, Notifications, and Saved tabs. The visible topbar search input is enabled and opens the Search tab.

Remaining:

Authenticated browser coverage should verify all exposed tabs load and retain accessible focus/URL state.

### Fixed Medium - Search result links point to an unregistered `/community/feed` path

Locations:

- `frontend/platform/community.html:111`
- `frontend/platform/community.html:141`
- Backend routes: `backend/src/main.rs:835`

Problem:

Search user and post result cards navigate to `/community/feed?profile=...` and `/community/feed?post=...`. The registered page routes are `/community` and `/community/post/:id`; no `/community/feed` route is registered. If the hidden search panel becomes reachable, search result clicks can send authenticated users to a 404 or wrong surface.

Expected:

Post search results should link to `/community/post/:id` or update `/community?post=:id` if the shell implements post focusing. Profile results should either open the existing profile modal or route to a registered profile page.

Evidence:

Route registration review found `/community`, `/community/post/:id`, and `/community/partials/*`, but no `/community/feed`.

Recommended fix:

Align search result URLs with registered routes and add a route-contract test for search result navigation.

Fix:

User search results now call the existing profile modal, and post search results navigate to `/community/post/:id`. Static grep confirmed `/community/feed` is no longer present in the touched community UI.

Remaining:

Add route-contract/browser coverage for search result clicks.

### Fixed Medium - Community partial routes mask backend failures as empty states

Locations:

- `backend/src/main.rs:2535`
- `backend/src/main.rs:2568`

Problem:

The feed and announcement HTMX partial handlers call backend data loaders and then use `unwrap_or_default()`. A community database outage, query/schema error, or user-bridge failure renders a normal empty feed or empty announcement list instead of a visible failure state or operationally distinct response.

Expected:

Partial handlers should return an error response or an explicit degraded/error partial. Empty content should be distinguishable from backend failure.

Evidence:

Static review found `get_feed_data(...).await.unwrap_or_default()` and `get_announcements(...).await.unwrap_or_default()`.

Recommended fix:

Propagate `AppError` or render a dedicated retry/error partial. Add tests for real empty feed vs community database failure.

Fix:

The feed and announcement HTMX handlers now return `Result`, propagate loader failures, validate announcement categories, and load announcements from the configured community database.

Remaining:

Add tests distinguishing legitimate empty content from community database/query failures.

### Fixed Medium - `/community/post/:id` share page reads post metadata from the core database

Locations:

- `backend/src/main.rs:2598`
- `backend/src/main.rs:2612`

Problem:

The share/OG route for community posts queries `posts` through `state.db`, while most community post reads use `state.community_db` via `get_community_pool`. In deployments where community tables live only in the community database, direct share pages cannot resolve post metadata and will render without the intended OG title/description/image. The frontend also sets `window.SSR_POST_ID`, but no code path consumes it to focus or load the shared post.

Expected:

Use the community pool for post data, core DB only for author profile enrichment, and make `SSR_POST_ID` drive a visible post detail or focused feed state.

Evidence:

Static route review found `page_community_post` querying `posts` on `state.db`; `community-feed.js` has no `SSR_POST_ID` usage.

Recommended fix:

Move share-post lookup to `state.community_db`, keep author lookup on `state.db`, and add direct `/community/post/:id` E2E for an existing seeded post.

Fix:

The share route now reads post metadata from `state.community_db`, preserves core DB only for author enrichment, returns 404 for unavailable posts, and the frontend consumes `SSR_POST_ID` to render the direct post view.

Remaining:

Add direct `/community/post/:id` E2E with a seeded community post and OG metadata assertions.

### Fixed Low - Modals and icon-only controls need accessibility hardening

Locations:

- `frontend/platform/community.html:507`
- `frontend/platform/community.html:545`
- `frontend/platform/community.html:579`
- `frontend/platform/partials/community_circle.html`
- `frontend/platform/partials/community_feed.html:89`

Problem:

Community report/profile/circle modals are displayed with style toggles but lack `role="dialog"`, `aria-modal`, focus trapping, Escape close, and focus restoration. Several icon-only controls rely on `title` but do not have stable accessible labels. Error/success handling often uses `alert()`, which is not a durable page state.

Expected:

Modals should use accessible dialog semantics, labeled buttons, focus management, Escape close, and visible/live-region status messages.

Evidence:

Static review found direct `style.display` modal toggles and inline close handlers without dialog semantics or focus management.

Recommended fix:

Introduce a small community modal helper or shared ds-modal behavior and convert report/profile/circle modals to use it. Add keyboard/mobile Playwright checks.

Fix:

Community modals now declare dialog semantics, use Escape close, trap focus, restore focus when opened through the helper, and expose labeled close controls. Composer icon buttons and preview remove controls now have accessible labels.

Remaining:

Run keyboard/mobile Playwright checks for report, profile, create-circle, invite, and circle-settings modals.

---

## Security Findings

- Global CSRF middleware is active for mutating requests and injects tokens into fetch/HTMX requests through shared head JavaScript.
- Authenticated community write routes require a session and use backend validation/moderation for core post/comment paths.
- The main security concern found in this run is authenticated stored/DOM XSS exposure from user/community-controlled API fields rendered through `innerHTML`.
- No monetary or financial mutation logic is present on this page.

---

## Database Findings

- Community tables exist for the audited page surfaces: posts, comments, reactions, content reports, community profiles, circles, notifications, bookmarks, polls, hashtags, XP ledger, and badges.
- Post creation is transactional and includes profile creation, post insert, hashtag linking, optional poll insert, and profile post count update.
- Poll voting is transactional for delete/insert vote behavior.
- Feed and announcement partials should stop converting database errors into empty states.

---

## Test Results

| Command | Result | Notes |
|---------|--------|-------|
| `node --check frontend/platform/static/js/community-feed.js` | Pass | No syntax errors after fix pass. |
| `node --check frontend/platform/static/js/community-circles.js` | Pass | No syntax errors after fix pass. |
| `node --check frontend/platform/static/js/community-amas.js` | Pass | No syntax errors. |
| `node --check frontend/platform/static/js/htmx-init.js` | Pass | No syntax errors. |
| `curl -I http://localhost:8888/community` | Blocked | No local backend was listening on port 8888. |
| `python3 -m pytest tests/e2e/test_community.py -q` | Blocked | Test harness stopped before running tests because `/health` was unreachable on port 8888. |
| `rustfmt backend/src/main.rs --edition 2021 --check` | Pass | No formatting changes needed for the touched backend file. |
| Static route grep for `/community/feed` | Pass | No remaining `/community/feed` references in touched community UI files. |
| `cd backend && cargo check` | Blocked | Stuck waiting on the shared build-directory file lock from another active build. |

---

## Missing Tests

- Authenticated browser E2E for feed post creation with text, image, poll, reaction, comment, bookmark, report, and visible error states.
- Authenticated browser E2E for announcements, My Circle create/update/delete/join/request/invite flows, AMA question/upvote, and Challenges loading.
- XSS fixture tests for profile badges, circle names/emojis, join-request names, post content, comments, and link previews.
- Direct `/community/post/:id` route test with seeded community DB post and OG metadata assertions.
- Failure-mode tests for community DB unavailable/query error vs legitimate empty feed.
- Accessibility tests for community modals, icon-only buttons, tab keyboard behavior, and mobile layout.

---

## Recommended Fix Order

1. Replace unsafe community profile/circle `innerHTML` renderers with DOM/text rendering and add XSS fixtures.
2. Decide whether Search, Notifications, and Saved Posts are part of the page, then expose or remove them.
3. Fix search result routing and direct shared-post route behavior.
4. Stop masking community partial backend failures as empty states.
5. Harden modal accessibility and add authenticated browser coverage.

---

## Final Status

`fixed_pending_runtime_recheck`

Severity counts:

- Open High: 0
- Open Medium: 0
- Open Low: 0
- Fixed in this pass: 1 High, 4 Medium, 1 Low

Reason: The audited code issues were fixed locally. Remaining work is runtime verification: authenticated browser/E2E coverage for feed, comments, bookmarks, reports, circle flows, AMAs, notifications, direct shared posts, keyboard/modal behavior, and XSS fixtures once the backend/build lock is clear.
