# Page Audit: Community Announcements Partial

Date: 2026-04-28
Status: fixed_runtime_recheck_blocked
Auditor: ChatGPT/Codex
Page URL: `/community/partials/announcements/list`
Template: `frontend/platform/partials/community_announcements_list.html`
JavaScript: `frontend/platform/static/js/community-announcements.js`
CSS: `frontend/platform/static/css/community.css`
Backend Routes: `backend/src/main.rs`

---

## Summary

The announcements list fragment is registered and renders a protected MiniJinja partial. The initial audit found four implementation issues; the 2026-04-28 fix pass resolved all four in code and added static regression coverage.

Final implementation status is fixed. Runtime status remains blocked because authenticated browser/curl verification could not be completed without a listening local backend, and repeated full `cargo check` attempts could not complete cleanly while the workspace was already running many Cargo jobs.

---

## Fix Pass - 2026-04-28

Fixed:

- PAGE-ISSUE-0495: Removed the legacy client-side list fetch/render path from `community-announcements.js`; HTMX/server rendering now owns announcement loading and filtering.
- PAGE-ISSUE-0496: Normalized public filter query values and list-template category checks to the backend enum: `new_commodity`, `dividend`, `platform_update`, `market_news`, and `farm_update`.
- PAGE-ISSUE-0497: Changed `community_announcements_list_htmx` to return `Result<axum::response::Response, AppError>`, validate category values, and propagate `get_announcements` errors instead of `unwrap_or_default()`.
- PAGE-ISSUE-0498: Replaced the inline clickable `span` with a semantic `button` and delegated JavaScript handler.

Added coverage:

- `tests/test_community_tab_contract_static.py::test_announcement_fragment_uses_server_rendered_contract`

Remaining documented issues:

- No remaining implementation issues from PAGE-ISSUE-0495 through PAGE-ISSUE-0498.
- Remaining work is verification-only: authenticated browser/runtime recheck is still needed for the Announcements tab, category filters, server failure behavior, and keyboard behavior.
- Full `cargo check` was retried with an isolated target directory and `CARGO_BUILD_JOBS=1`; the tool session ended without a normal completion signal while compiling dependencies in a workspace already running many Cargo jobs.
- Local curl smoke tests stayed blocked because `localhost:8888` was not accepting connections.

---

## Tested Scope

- Reviewed tracker entry `community.partial-announcements`.
- Reviewed `frontend/platform/partials/community_announcements.html`.
- Reviewed `frontend/platform/partials/community_announcements_list.html`.
- Reviewed `frontend/platform/static/js/community-announcements.js`.
- Reviewed `backend/src/main.rs` route registration and `community_announcements_list_htmx`.
- Reviewed `backend/src/community/service.rs::get_announcements`.
- Reviewed `backend/src/community/routes.rs` announcement category validation and admin create/list behavior.
- Reviewed existing community/admin announcement E2E coverage.
- Ran a JavaScript syntax check.
- Attempted curl and targeted E2E runtime checks; both were blocked because no backend was running on `localhost:8888`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/community/partials/announcements/list` | Protected HTMX fragment endpoint. |
| Parent tab template | `frontend/platform/partials/community_announcements.html` | Filter buttons and HTMX target container. |
| List template | `frontend/platform/partials/community_announcements_list.html` | Announcement cards and empty state. |
| JS | `frontend/platform/static/js/community-announcements.js` | Delegated View in Feed handler; no announcement data fetching. |
| CSS | `frontend/platform/static/css/community.css` | Announcement card/filter styling. |
| Backend page route | `GET /community/partials/announcements/list` | Registered in `backend/src/main.rs`. |
| Backend handler | `community_announcements_list_htmx` | Validates category values and propagates announcement query errors. |
| Backend service | `backend/src/community/service.rs::get_announcements` | Reads `posts` joined to `announcement_categories`. |
| Database table | `posts` | Stores announcement post content and counters. |
| Database table | `announcement_categories` | Stores category used by filters. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| All filter | `.ann-filter-btn`, `hx-get="/community/partials/announcements/list"` | Load all announcements into `#community-announcements-container`. | HTMX wired; legacy JS no longer fetches list data. | Yes. | Fixed statically; runtime recheck pending. |
| New Commodities filter | `hx-get="/community/partials/announcements/list?category=new_commodity"` | Load new commodity announcements. | HTMX wired with backend enum value. | Yes. | Fixed statically; runtime recheck pending. |
| Dividends filter | `?category=dividend` | Load dividend announcements. | HTMX wired with backend enum value. | Yes. | Fixed statically; runtime recheck pending. |
| Platform Updates filter | `?category=platform_update` | Load platform update announcements. | HTMX wired with backend enum value. | Yes. | Fixed statically; runtime recheck pending. |
| Market News filter | `?category=market_news` | Load market news announcements. | HTMX wired; category matches admin contract. | Yes. | Static review passes. |
| Farm Updates filter | `?category=farm_update` | Load farm update announcements. | HTMX wired with backend enum value. | Yes. | Fixed statically; runtime recheck pending. |
| Loading state | `#community-announcements-container` initial child | Show loading while HTMX loads. | HTMX trigger wired. | Yes. | Unverified at runtime. |
| Empty state | list template lines 1-11 | Show no-announcements message only when query succeeds with no rows. | Server-rendered. | Backend now propagates query failures. | Fixed statically; runtime failure-state recheck pending. |
| Announcement card | `.ann-card` | Show category, date, sanitized body, reactions, comments. | Server-rendered. | Yes. | Category labels fixed statically. |
| Pinned badge | `.ann-card-pin` | Mark pinned announcements. | Server-rendered. | Yes. | Static review passes. |
| View in Feed | `button.ann-read-more[data-community-ann-read-more]` | Navigate to Feed tab. | Delegated JS handler. | No backend needed. | Fixed statically; keyboard/browser recheck pending. |

---

## Frontend Findings

### P1 - Legacy JS can overwrite the HTMX announcement fragment with the general feed

Status: fixed in the 2026-04-28 fix pass.

Location:

- Template: `frontend/platform/partials/community_announcements.html`
- JS: `frontend/platform/static/js/community-announcements.js`

Problem:

The parent community page loads `community-announcements.js`, and that script initializes after the announcements tab is swapped into `#community-content-area`. It attaches click listeners to the same filter buttons already controlled by HTMX, reads `data-category`, and fetches `/api/community/feed` or `/api/community/feed?category=...`. The filter buttons do not define `data-category`, so the JS path can fetch the unfiltered general feed and replace `#community-announcements-container` with non-announcement posts.

Expected:

The announcements tab should have one controller. Either HTMX owns the fragment completely, or the JS renderer should call the announcements endpoint with a matching category contract and should not race HTMX swaps.

Evidence:

`frontend/platform/static/js/community-announcements.js` lines 56-60 fetch `/api/community/feed`; lines 153-160 attach click listeners and read `data-category`. The filter buttons in `frontend/platform/partials/community_announcements.html` use Alpine state and `hx-get`, but no `data-category`.

Recommended fix:

Remove the legacy client renderer for this tab, or convert it into a tiny HTMX state helper only. If JS remains, add explicit `data-category` values, fetch the announcements endpoint, and prevent duplicate HTMX/JS requests.

### P1 - Announcement category values are inconsistent across admin create, filters, and display

Status: fixed in the 2026-04-28 fix pass.

Location:

- Template: `frontend/platform/partials/community_announcements.html`
- Template: `frontend/platform/partials/community_announcements_list.html`
- Backend: `backend/src/community/routes.rs`

Problem:

Admin announcement creation validates singular categories: `new_commodity`, `dividend`, `platform_update`, `market_news`, and `farm_update`. The public announcements filters send plural values: `new_commodities`, `dividends`, and `platform_updates`. The list template also checks plural values when assigning icons and display labels, so singular rows from the admin API render as the default Platform Update category or disappear behind filters.

Expected:

All announcement create, list, filter, and display code should use one canonical category enum.

Evidence:

`backend/src/community/routes.rs` lines 35-41 define singular categories. `frontend/platform/partials/community_announcements.html` lines 5-10 sends plural query strings. `frontend/platform/partials/community_announcements_list.html` lines 19-30 checks plural categories except `market_news`.

Recommended fix:

Normalize the frontend HTMX query values and template category comparisons to the backend enum, then add regression coverage for each category.

### P3 - View in Feed control is a clickable span with inline script

Status: fixed in the 2026-04-28 fix pass.

Location:

- Template: `frontend/platform/partials/community_announcements_list.html`

Problem:

The "View in Feed" action is rendered as a `<span>` with `onclick`. It is not a button or link, has no keyboard activation semantics, and keeps inline JavaScript in a server-rendered fragment.

Expected:

Use a `<button type="button">` or `<a>` with an accessible name and delegated JavaScript, or an HTMX/navigation mechanism that works by keyboard.

Evidence:

`frontend/platform/partials/community_announcements_list.html` line 60 renders `span.ann-read-more` with inline `onclick`.

Recommended fix:

Replace with a semantic button and register the tab-switch behavior from shared community JS.

---

## Backend Findings

### P2 - Announcement list DB failures are masked as empty state

Status: fixed in the 2026-04-28 fix pass.

Location:

- Backend: `backend/src/main.rs`

Problem:

`community_announcements_list_htmx` calls `service::get_announcements(...).await.unwrap_or_default()`. Any query failure, schema drift, or database outage returns an empty `posts` vector and the user sees "No announcements found."

Expected:

The route should propagate the error through the existing application error handling or render an explicit safe error fragment. Operators and users should be able to distinguish no content from a backend failure.

Evidence:

`backend/src/main.rs` lines 2574-2576 call `unwrap_or_default()` on the announcement query result.

Recommended fix:

Return `Result<impl IntoResponse, AppError>` from the handler and use `?`, or render a dedicated retry/error partial while logging the root cause.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/community-announcements.js` | JS parses. | Passed with no output. | Pass |
| Static regression | `python3 -m pytest tests/test_community_tab_contract_static.py -q` | Announcement contracts are protected. | 5 passed. | Pass |
| Rust formatting for touched file | `cd backend && rustfmt --edition 2021 --check src/main.rs` | Touched Rust file is formatted. | Passed. | Pass |
| Full cargo check | `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/poool-community-final-check cargo check --manifest-path backend/Cargo.toml` | Backend compiles. | Blocked by concurrent Cargo activity; session ended without a normal pass/fail while compiling dependencies. | Blocked |
| Curl fragment smoke | `curl -i -sS http://localhost:8888/community/partials/announcements/list` | Protected redirect or fragment response. | Failed to connect; backend not running. | Blocked |
| Targeted E2E | `python3 -m pytest tests/e2e/test_community.py::test_community_announcements -q` | Announcement tab smoke runs. | Pytest aborted because `/health` was unreachable. | Blocked |

---

## Security Findings

No direct authorization or XSS finding was confirmed for this fragment. The route is protected through `serve_protected_with_context`, and admin-created announcement content is sanitized before storage. The remaining security-adjacent risk is operational: backend failures are hidden as empty content, which can mask outages or schema regressions.

---

## Database Findings

- `posts` and `announcement_categories` support this fragment.
- The service query joins `posts` to `announcement_categories` and limits results to 50.
- Category integrity is weak at read/filter time because the public fragment accepts arbitrary category strings and does not validate against the same enum used by admin creation.
- Query failures are swallowed by the route, so runtime DB health for this fragment is not observable from the UI.

---

## Missing Tests

- Add an authenticated browser E2E that opens `/community`, switches to Announcements, verifies only announcement posts render, and asserts no general feed posts appear after filter clicks.
- Add filter tests for `new_commodity`, `dividend`, `platform_update`, `market_news`, and `farm_update`.
- Add a forced backend failure test for `/community/partials/announcements/list` that verifies a visible error state instead of the empty state.
- Add a keyboard test for "View in Feed" after it is converted to a semantic control.

---

## Recommended Fix Order

1. Run authenticated browser E2E for announcement tab loading, filters, failure state, and keyboard interaction once a local backend is available.
2. Run full `cargo check` after the current Cargo job backlog clears.
3. Consider adding a dedicated runtime failure-state test that forces `get_announcements` to fail and verifies the HTMX response path.

---

## Final Status

`fixed_runtime_recheck_blocked`

Reason: All documented implementation findings were fixed and covered statically. Runtime/browser recheck remains blocked by no local backend, and full build verification could not complete cleanly while the workspace was busy with other Cargo jobs.
