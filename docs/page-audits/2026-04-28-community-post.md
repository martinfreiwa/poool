# Page Audit: Community Post

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/community/post/:id`
Template: `frontend/platform/community.html`
JavaScript: `frontend/platform/static/js/community-feed.js`
CSS: `frontend/platform/static/css/community.css`, `frontend/platform/static/css/leaderboard.css`
Backend Routes: `backend/src/main.rs`, `backend/src/community/routes.rs`, `backend/src/community/service.rs`

---

## Summary

`/community/post/:id` is not a working post-detail page. The page route accepts a post UUID and injects `window.SSR_POST_ID`, but the template and JavaScript never consume that value. Runtime smoke showed a valid post URL returning the generic Community shell, generic OG metadata, and the default feed HTMX target instead of the requested post. Visible post-card actions also have route-contract problems: fetch-based POST actions do not send the required CSRF token, and the Like button sends a `reaction_type` value rejected by the database constraint.

2026-04-28 fix pass: the tracked implementation issues are fixed in the working tree. `/community/post/:id` now uses community DB visibility rules for OG lookup, returns a 404 status shell for unavailable posts, disables default feed autoload on direct post URLs, and renders a selected-post detail state from `/api/community/posts/:id`. Community state-changing fetches now attach CSRF headers, reactions use the allowed `fire` taxonomy with server-side validation/count response, comment creation/count updates are transactional, duplicate reports return durable IDs, poll choices render as buttons, comment input is labelled, and profile modal user data is rendered without unsafe `innerHTML`/style URL injection.

Final status remains `needs_recheck` because authenticated browser/runtime verification is still pending.

---

## Fix Pass Update

Fixed:

- PAGE-ISSUE-0507: direct community post URL now has a selected-post render path instead of generic feed autoload.
- PAGE-ISSUE-0508: `page_community_post` reads post OG data from `state.community_db` and uses the core DB only for author enrichment.
- PAGE-ISSUE-0509: community POST/PUT/DELETE fetches in `community-feed.js` now use a local CSRF header helper.
- PAGE-ISSUE-0510: post-card reactions now use `fire`, and the backend validates allowed reaction values before insert.
- PAGE-ISSUE-0511: comment insert plus `posts.comment_count` update now run in one transaction.
- PAGE-ISSUE-0512: post-card semantics were improved with labels/ARIA and poll option buttons; profile modal unsafe rendering was also cleaned up.

Remaining issues:

- Authenticated browser E2E still needs to verify direct post rendering, comments, reactions, bookmarks, reports, poll voting, keyboard/mobile behavior, and not-found behavior.

---

## Tested Scope

- Static review of the page route, shared community template, feed partial, post-card partial, and community feed JavaScript.
- Backend review of post detail, reaction, comment, report, bookmark, and poll routes.
- Database review of community `posts`, `comments`, `reactions`, `content_reports`, `bookmarks`, and poll tables.
- Runtime curl smoke against a local backend on `localhost:8888`.
- Targeted JavaScript syntax check and targeted Rust community tests.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/community/post/:id` | Registered as `page_community_post` in `backend/src/main.rs`. |
| Template | `frontend/platform/community.html` | Shared community shell; sets `window.SSR_POST_ID`. |
| Partial | `frontend/platform/partials/community_feed.html` | Default HTMX feed wrapper. |
| Partial | `frontend/platform/partials/community_post_list.html` | Includes post cards for feed results. |
| Partial | `frontend/platform/partials/community_post_card.html` | Visible post controls for reactions, comments, share, bookmark, report, and poll container. |
| JS | `frontend/platform/static/js/community-feed.js` | Owns post actions, comments, profile modal, bookmarks, reports, poll voting, and feed reload behavior. |
| Backend page route | `GET /community/post/:id` | Returns community shell with context; does not render the selected post. |
| Backend API route | `GET /api/community/posts/:id` | Returns JSON post detail from community DB. |
| Backend API route | `POST /api/community/posts/:id/reactions` | Requires auth and CSRF; frontend sends no CSRF and sends `like`. |
| Backend API route | `GET/POST /api/community/posts/:id/comments` | Fetches and creates comments. POST lacks frontend CSRF header. |
| Backend API route | `POST /api/community/posts/:id/report` | Creates moderation report. Frontend lacks CSRF header. |
| Backend API route | `POST /api/community/posts/:id/bookmark` | Toggles bookmark. Frontend lacks CSRF header. |
| Backend API route | `GET /api/community/posts/:id/poll`, `POST /api/community/posts/:id/poll/vote` | Poll read/vote. Vote lacks frontend CSRF header. |
| Database table | `posts` | Community DB post records and denormalized counts. |
| Database table | `comments` | Post comments. |
| Database table | `reactions` | Constraint allows `fire`, `insightful`, `clap`, `green`; not `like`. |
| Database table | `content_reports` | One report per post/reporter. |
| Database table | `bookmarks` | Referenced by route; migration exists elsewhere in community set. |
| Database table | `polls`, `poll_options`, `poll_votes` | Poll display and voting. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Direct post page | `GET /community/post/:id` | Render or focus exactly the requested post. | No; `SSR_POST_ID` is set but unused. | Partial; JSON detail route exists. | Valid post URL returned generic shell/feed. |
| Feed tab | `#community-content-area[hx-get="/community/partials/feed"]` | Load feed content into shell. | Yes via HTMX. | Yes, protected partial. | Direct post page still points at feed, not detail. |
| Post author | `.feed-post-author[onclick="openUserProfile(...)"]` | Open profile modal. | Yes. | Yes, profile API. | Static only; modal has unsafe badge/avatar HTML patterns from broader community page. |
| Reaction button | `button[onclick="toggleReaction(..., 'like')"]` | Toggle reaction and sync count. | Broken; no CSRF and invalid reaction type. | Backend route exists but DB rejects `like`. | CSRF smoke returned 403 before auth. |
| Comments button/stat | `onclick="toggleComments(...)"` | Load comments and show section. | Mostly wired. | GET comments requires auth. | Not browser-tested with auth fixture. |
| Comment textarea | `#comment-input-{id}` | Enter comment. | Wired to `submitComment`. | POST comments exists. | Missing CSRF header; not accessible-labeled. |
| Comment Post button | `onclick="submitComment(...)"` | Persist comment and reload list. | Broken for CSRF. | Backend validates and creates comment. | Runtime not safely submitted. |
| Share button | inline `navigator.clipboard.writeText` | Copy canonical post URL. | Yes. | No backend required. | Static OK; fallback is `alert`. |
| Bookmark button | `#bookmark-btn-{id}` | Check/toggle saved state. | Broken for POST CSRF; GET status wired. | Backend routes exist. | Static only. |
| Report button | `onclick="openReportModal(...)"` | Open report modal. | Wired. | Backend route exists. | POST missing CSRF; modal a11y weak. |
| Poll container | `#poll-container-{id}` | Fetch and render poll if present. | GET wired; vote POST lacks CSRF. | Backend routes exist. | Static only. |
| Hashtag/mention links | `rendered_content | safe` spans with HTMX attrs | Filter feed by tag/mention. | Server-generated HTMX. | Feed list route must support params. | Not verified in runtime. |
| Link preview | `.feed-post-link-preview` | Open external URL. | Native link. | Preview stored by backend. | Static only; needs URL allowlist review separately. |

---

## Frontend Findings

### P1 - Direct post URL does not render the requested post

Location:

- Template: `frontend/platform/community.html`
- JS: `frontend/platform/static/js/community-feed.js`
- Backend: `backend/src/main.rs`

Problem:

`page_community_post` injects `ssr_post_id`, but the template only writes it to `window.SSR_POST_ID`. No JavaScript references `SSR_POST_ID`, and the page still loads `/community/partials/feed` into `#community-content-area`. A direct post URL therefore behaves like `/community`, not a post-detail view.

Expected:

`/community/post/:id` should either server-render the selected post, HTMX-load a selected-post partial, or client-fetch `/api/community/posts/:id` and render a single post/detail state with clear not-found handling.

Evidence:

Runtime smoke for a valid community post returned `page_http=200`, `window.SSR_POST_ID = '3a09814c-4768-444b-8fac-b2e01eff0fb9'`, generic `og:title` of `Community - POOOL`, and `#community-content-area hx-get="/community/partials/feed"`.

Recommended fix:

Create a dedicated detail partial/template or add a startup branch in `community-feed.js` that detects `window.SSR_POST_ID`, fetches `/api/community/posts/:id`, renders the selected post card/detail, updates empty/not-found states, and prevents default feed autoload for detail routes.

### P1 - Post-card mutations omit required CSRF headers

Location:

- JS: `frontend/platform/static/js/community-feed.js`
- Template: `frontend/platform/partials/community_post_card.html`
- Backend: `backend/src/community/routes.rs`

Problem:

Visible fetch POST actions for reactions, comments, reports, bookmarks, and poll votes send JSON with `credentials: 'same-origin'` but no `X-CSRF-Token`. Local runtime smoke showed the backend rejects these requests with `403` CSRF errors before action logic.

Expected:

All state-changing fetches should include the same CSRF helper used elsewhere in the app, and the UI should show inline failures instead of optimistic success or alert-only errors.

Evidence:

`curl` against `POST /api/community/posts/:id/reactions` after page load returned `403` with `CSRF token missing or invalid`.

Recommended fix:

Centralize a community fetch helper that reads the `csrf_token` cookie and adds `X-CSRF-Token` for all POST/PUT/DELETE requests, then use it for create post, upload where required, reaction, comment, report, bookmark, follow, and poll vote flows.

### P1 - Reaction button sends a value rejected by the backend

Location:

- Template: `frontend/platform/partials/community_post_card.html`
- Backend: `database/community/003_reactions.sql`

Problem:

The visible reaction button calls `toggleReaction('{{ p.id }}', this, 'like')`, but the `reactions.reaction_type` check constraint only accepts `fire`, `insightful`, `clap`, and `green`. Once CSRF/auth are satisfied, the Like button will fail at persistence.

Expected:

The frontend and database should share one reaction taxonomy. Either send an allowed value or migrate the database and backend validation to include `like`.

Evidence:

Static review of the post-card partial and reaction migration. The backend passes the frontend value directly to `service::toggle_reaction`.

Recommended fix:

Pick the canonical reaction set, validate it in Rust before insert, and update post-card controls to send only allowed values.

### P2 - Post action controls need accessible names and keyboard-safe interaction

Location:

- Template: `frontend/platform/partials/community_post_card.html`
- JS: `frontend/platform/static/js/community-feed.js`

Problem:

Several controls are icon-only or clickable non-button UI: bookmark/report buttons rely on `title`, stats are a clickable `div`, poll options are clickable `div`s, and modals have no visible focus management. The comment textarea has no label.

Expected:

Use semantic buttons with `aria-label`, keyboard activation for poll choices, labels for form fields, focus movement into/away from modals, and inline status regions for async results.

Evidence:

Static review of post-card partial and poll renderer.

Recommended fix:

Convert clickable `div`s to buttons, add labels/ARIA, and add modal focus handling while preserving the existing visual style.

---

## Backend Findings

### P1 - Page route reads OG data from the wrong database

Location:

- Backend: `backend/src/main.rs`

Problem:

`page_community_post` queries `posts` through `state.db`, but community posts live in `state.community_db` and are served by community API routes through `get_community_pool`. As a result, valid community posts do not populate post-specific OG metadata.

Expected:

The page route should query the community DB for the post record, then use the core DB only for author profile/avatar enrichment.

Evidence:

Runtime smoke for a post that `GET /api/community/posts/:id` returned successfully still rendered generic Community OG metadata on `/community/post/:id`.

Recommended fix:

Use `get_community_pool`-equivalent access in `page_community_post`, apply the same hidden/shadowban visibility rules as `get_post_detail`, and separately batch/fetch author data from core DB.

### P2 - Comment insert and denormalized counter update are not transactional

Location:

- Backend: `backend/src/community/service.rs`

Problem:

`service::create_comment` inserts the comment, then separately updates `posts.comment_count`. If the second write fails, comment rows and counters drift.

Expected:

Comment creation and counter update should run in one SQLx transaction, or the count should be maintained by a database trigger.

Evidence:

Static review of `create_comment`.

Recommended fix:

Wrap both writes in one transaction or add a trigger matching the reaction/poll counter pattern.

### P2 - Duplicate reports return synthetic success IDs

Location:

- Backend: `backend/src/community/service.rs`

Problem:

`create_content_report` uses `ON CONFLICT DO NOTHING RETURNING id`, then returns a newly generated UUID when the report already exists. The UI cannot distinguish a new report from a duplicate no-op.

Expected:

Return the existing report ID or a clear `already_reported` response.

Evidence:

Static review of `create_content_report`.

Recommended fix:

Use `ON CONFLICT ... DO UPDATE SET updated_at = content_reports.updated_at RETURNING id` or fetch the existing report after conflict.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Valid direct post page | Selected a visible joined community post ID from local community DB; requested `/community/post/:id`. | Page renders or focuses that post with post-specific metadata. | Returned `200`, generic Community metadata, `window.SSR_POST_ID`, and default feed HTMX. | Fail |
| Missing direct post page | Requested `/community/post/00000000-0000-0000-0000-000000000000`. | 404/not-found state or safe redirect. | Returned `200` generic Community shell with missing UUID in `SSR_POST_ID`. | Fail |
| Post JSON API | Requested `/api/community/posts/:id` for a joined visible post. | JSON for the selected post. | Returned `200` with post JSON. | Pass |
| Reaction CSRF smoke | POSTed reaction without a CSRF header. | Backend rejects unsafe request. | Returned `403` CSRF error. | Pass backend / fail frontend contract |
| Syntax check | Ran `node --check frontend/platform/static/js/community-feed.js`. | No syntax errors. | Passed. | Pass |
| Rust community tests | Ran `cargo test community::tests --manifest-path backend/Cargo.toml`. | Relevant unit tests pass. | 2 passed. | Pass |

---

## Security Findings

- P1: Fetch-based state-changing controls omit CSRF headers, so the current UI cannot complete the protected actions and has inconsistent optimistic UI behavior.
- P2: `page_community_post` visibility rules do not match `get_post_detail`; the page query should use the same hidden/shadowban checks once moved to the community DB.
- P2: Profile badge/avatar rendering still uses `innerHTML`/style URL assignment for profile API data in the shared community controller.
- P2: Duplicate report submissions receive synthetic success IDs, weakening user/admin audit clarity.

---

## Database Findings

- `reactions.reaction_type` allows `fire`, `insightful`, `clap`, and `green`; the post card sends `like`.
- `comments` and `posts.comment_count` can drift because comment creation and count update are separate writes.
- `content_reports` has a useful uniqueness constraint, but the service hides conflicts by returning a random UUID.
- Poll percentage calculation uses `f64` for non-monetary percentages only; this is not a money-rule violation.

---

## Missing Tests

- Browser E2E for `/community/post/:id` with an authenticated fixture verifying the selected post is rendered/focused, not the generic feed.
- Not-found E2E for `/community/post/:id` with an unknown UUID.
- API/UI contract test for reaction taxonomy.
- Authenticated CSRF E2E for reaction, comment, report, bookmark, and poll vote controls.
- Accessibility test for post-card keyboard navigation, poll choices, report/profile modals, and comment form labeling.
- Transaction/regression test for comment count consistency on comment creation failure paths.

---

## Recommended Fix Order

1. Make `/community/post/:id` use the community DB and render/load the requested post with a real not-found state.
2. Add a CSRF-aware fetch helper and migrate all community state-changing fetches to it.
3. Align the reaction taxonomy between post-card UI, Rust validation, and the database constraint.
4. Make comment creation/count update atomic.
5. Repair post-card accessibility and modal focus behavior.

---

## Final Status

`needs_recheck`

Reason: The tracked code fixes are in place, but the page still needs authenticated browser/runtime recheck.
