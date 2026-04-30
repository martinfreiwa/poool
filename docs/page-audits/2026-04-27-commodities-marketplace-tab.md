# Page Audit: Commodities Tab Fragment

Date: 2026-04-27
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/commodities-marketplace/tab`
Template: Manual HTML fragment in `backend/src/assets/routes.rs`
JavaScript: `frontend/platform/static/js/commodities-marketplace.js`, `frontend/platform/static/js/marketplace-search.js`, `frontend/platform/static/js/property-card.js`
CSS: `frontend/platform/static/css/marketplace.css`, `frontend/platform/static/css/property-card.css`
Backend Routes: `backend/src/assets/mod.rs`, `backend/src/assets/routes.rs`

---

## Summary

The commodities tab fragment is registered, session-protected, and returns commodity cards for `available`, `funded`, and invalid-tab fallback requests. The 2026-04-28 fix pass addressed the documented DB-error handling, swapped-card filter contract, repeated search/filter listener binding, and click-only card navigation issues. A follow-up verification pass also fixed the parent commodities template runtime error, added explicit `data-yield` filtering data, restored the mobile More Filters control, and passed focused desktop/mobile browser E2E plus formatter/build/static checks.

---

## Fix Pass Update - 2026-04-28

Fixed:

- `PAGE-ISSUE-0393`: `api_commodities_tab` now returns a logged 500 error fragment instead of silently converting DB failures into an empty category.
- `PAGE-ISSUE-0394`: swapped commodity cards now include `data-price`, `data-duration`, `data-area`, and `data-commodity-type` attributes aligned with the active filters.
- `PAGE-ISSUE-0395`: marketplace search/filter initialization is now idempotent and resolves the current grid after HTMX swaps.
- `PAGE-ISSUE-0396`: swapped commodity cards now expose semantic `.property-card-link` anchors for detail navigation.

Remaining issues:

- No open commodities tab issues remain from this audit.

Verification follow-up:

- Browser/mobile/keyboard recheck passed on 2026-04-28 with `tests/e2e/test_commodities_marketplace.py`.
- Isolated backend build verification passed on 2026-04-28 with `CARGO_TARGET_DIR=/tmp/poool-commodities-run CARGO_INCREMENTAL=0 cargo check --quiet`.
- `cargo fmt --check`, JS syntax checks, and focused static regression tests passed on 2026-04-28.

---

## Tested Scope

- Reviewed `/commodities-marketplace/tab` route registration and handler.
- Reviewed parent `/commodities-marketplace` HTMX tab buttons and filter controls.
- Reviewed shared commodity-marketplace, marketplace-search, and property-card JavaScript.
- Reviewed `assets`, `asset_images`, `orders`, and `order_items` database dependencies used by the fragment.
- Ran JS syntax checks and HTTP smoke tests with unauthenticated and temporary authenticated sessions.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/commodities-marketplace/tab?tab=available` | HTMX fragment for open commodity funding cards. |
| URL | `/commodities-marketplace/tab?tab=funded` | HTMX fragment for funded commodity cards. |
| URL | `/commodities-marketplace/tab?tab=exited` | Handler supports it, but the parent page does not expose an Exited tab. |
| Parent page | `/commodities-marketplace` | Supplies tab buttons, filters, and target wrapper. |
| Parent template | `frontend/platform/commodities-marketplace.html` | Initial card markup differs from the tab fragment. |
| Backend route registration | `backend/src/assets/mod.rs` | Registers `GET /commodities-marketplace/tab`. |
| Backend handler | `backend/src/assets/routes.rs` | `api_commodities_tab`. |
| JS | `frontend/platform/static/js/commodities-marketplace.js` | Reinitializes search and card behavior after HTMX swaps. |
| JS | `frontend/platform/static/js/marketplace-search.js` | Owns filter/search listeners for `#property-grid`. |
| JS | `frontend/platform/static/js/property-card.js` | Initializes image fallback, dots, keyboard dot navigation, and swipe. |
| Database table | `assets` | Commodity card source and funding-status filter. |
| Database table | `asset_images` | Card image arrays. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Available tab | `#filter-bar-tab-available` | HTMX loads available commodity cards into `#commodities-content-wrapper`. | Yes, via HTMX and active-class JS. | Yes, maps to `funding_open` and `funding_in_progress`. | Authenticated HTTP returned 200 with cards. |
| Funded tab | `#filter-bar-tab-funded` | HTMX loads funded commodity cards. | Yes. | Yes, maps to `funded`. | Authenticated HTTP returned 200 with funded card. |
| Exited tab | Handler supports `tab=exited` | Should expose exited commodity cards if product wants that state selectable. | No visible parent button. | Yes. | Unverified through UI; direct route supported. |
| Invalid tab fallback | `?tab=bogus` | Should reject or clearly normalize invalid tab. | N/A. | Partially, defaults to available. | Authenticated HTTP returned available cards with 200. |
| Empty category state | Handler empty response | Should distinguish true empty state from backend failure. | HTMX can render returned HTML. | Partially. | DB failures would be masked as empty because of `unwrap_or_default()`. |
| Commodity card | `.property-card` in fragment | Navigate to `/commodity/:slug`. | Click handler only. | Yes, commodity detail route exists. | Mouse click path likely works; keyboard card activation is not wired. |
| Previous/next image | `.property-nav-prev`, `.property-nav-next` | Cycle card images without navigating card. | Yes, inline handlers call `cardPrevImage` / `cardNextImage`. | No backend needed. | Static wiring present. |
| Image dots | `.property-dot` | Select image and support keyboard after property-card initialization. | Yes, via delegated listeners. | No backend needed. | Static wiring present after `property-card.js` HTMX listener. |
| Search input/button | `#filter-bar-search-input`, `#filter-bar-search-btn` | Filter current swapped cards. | Partially; listeners are rebound after swaps. | No backend needed. | Works in principle, but repeated swaps add duplicate listeners. |
| Location filter | `#filter-bar-location-select` | Filter cards by location. | Partially. | No backend needed. | Fragment cards have `data-location`; options are Bali/property-specific. |
| Investment type filter | `#filter-bar-investment-select` | Filter cards by duration. | Partially. | No backend needed. | Fragment cards omit `data-duration` and duration IDs, so this degrades after swaps. |
| Property type filter | `#filter-bar-property-select` | Filter by asset/property type. | Weakly. | No backend needed. | Options are property-specific and not meaningful for commodity cards. |
| Extra price/yield filters | Dynamically built `#commodities-extra-filters` | Filter by card price and yield. | Partially, but no visible trigger exists in parent template. | No backend needed. | `data-price` is missing from swapped cards, so price filters would hide cards incorrectly if triggered. |

---

## Frontend Findings

### P2 - Swapped Cards Lose Filter Contract

Location:

- Template: `frontend/platform/commodities-marketplace.html:162`
- Backend fragment: `backend/src/assets/routes.rs:1055`
- JS: `frontend/platform/static/js/commodities-marketplace.js:151`
- JS: `frontend/platform/static/js/marketplace-search.js:126`

Problem:

Initial commodity cards include `data-price`, while tab-fragment cards do not. The shared filters also look for `data-duration` or duration IDs that neither initial nor swapped commodity cards provide consistently. After a tab swap, extra price filters parse missing `data-price` as `0`, and duration filters cannot reliably read the card duration.

Expected:

The tab fragment should emit the same card contract as the initial page, including `data-price`, `data-duration`, and any selectors the active filters read.

Evidence:

Static review found `data-price` in the initial template but not in the fragment string. Authenticated HTTP for `tab=available` returned cards without `data-price`.

Recommended fix:

Extract a shared commodity-card renderer or align the manual fragment markup with the initial template and filter JS. Add a regression test that fetches each tab and asserts required card data attributes exist.

Fix status:

Fixed locally on 2026-04-28. The fragment now emits the filter data attributes covered by the static regression test.

### P2 - HTMX Swaps Rebind Search And Filter Listeners

Location:

- JS: `frontend/platform/static/js/commodities-marketplace.js:37`
- JS: `frontend/platform/static/js/marketplace-search.js:17`
- JS: `frontend/platform/static/js/marketplace-search.js:202`

Problem:

After every HTMX swap, `commodities-marketplace.js` calls `window._initMarketplaceSearch()`. The initializer attaches new listeners to the same search input, search button, clear button, native selects, and document-level dropdown event without a guard or teardown. Repeated tab changes can trigger duplicate filtering work and stale closures that point at old `#property-grid` nodes.

Expected:

Search/filter initialization should be idempotent or delegated so one listener set operates on the current grid.

Evidence:

Static review shows no `data-wired` guard, listener cleanup, or event delegation in `initMarketplaceSearch()`.

Recommended fix:

Make marketplace search initialization idempotent and have filter handlers resolve the current grid at event time, or use delegated document-level handlers with a page-scope guard.

Fix status:

Fixed locally on 2026-04-28. The search initializer now guards repeated binding and resolves the active grid after swaps.

### P3 - Commodity Cards Are Not Keyboard-Accessible Links

Location:

- Template: `frontend/platform/commodities-marketplace.html:162`
- Backend fragment: `backend/src/assets/routes.rs:1055`

Problem:

Cards navigate through `onclick` on a `<div>`. The card itself is not focusable, has no semantic link role, and has no Enter/Space key activation. Image dots are keyboard-enabled by `property-card.js`, but the primary card navigation is mouse-only unless a nested control is focused.

Expected:

The card title or whole card should be an `<a href="/commodity/:slug">` with an accessible name, or the card should have explicit focus/keyboard behavior. A real anchor is preferable for open-in-new-tab, copy-link, and assistive technology support.

Evidence:

Static review found no anchor wrapper in initial or fragment card markup.

Recommended fix:

Use the existing property-card anchor pattern or add a visible/focusable title link while keeping gallery controls from triggering navigation.

Fix status:

Fixed and browser-verified on 2026-04-28 for swapped tab cards.

---

## Backend Findings

### P1 - Database Failures Are Reported As Empty Commodity Categories

Location:

- Backend: `backend/src/assets/routes.rs:928`

Problem:

The tab query uses `.unwrap_or_default()` after `fetch_all()`. Any database error, schema mismatch, type decode failure, or connection issue returns an empty vector and the user sees "No commodities found in this category." Operators also lose a reliable HTTP failure signal.

Expected:

The handler should log and return a 5xx or safe error fragment when the database read fails, while preserving the normal empty state for successful zero-row queries.

Evidence:

Static review confirmed `.fetch_all(&state.db).await.unwrap_or_default()`. HTTP smoke verified the empty/category fragment path is the only fallback branch.

Recommended fix:

Replace `unwrap_or_default()` with explicit `match`, log the database error with route/tab context, and return an HTMX-safe error state plus 500 status. Add an HTTP test that injects or simulates a DB failure if practical.

Fix status:

Fixed locally on 2026-04-28. Static regression coverage added in `tests/test_commodities_tab_static.py`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated fragment request | `curl /commodities-marketplace/tab?tab=available` without cookie | 401 or redirect-safe auth failure. | Returned `401 Unauthorized`. | Pass |
| Authenticated available tab | Created temporary test user/session and requested `tab=available`. | 200 HTML fragment with available commodity cards. | Returned 200 with `property-card` markup. | Pass |
| Authenticated funded tab | Requested `tab=funded` with same session. | 200 HTML fragment with funded commodity cards or empty state. | Returned 200 with funded commodity card. | Pass |
| Invalid tab fallback | Requested `tab=bogus` with same session. | Defined behavior. | Returned available cards with 200. | Pass with product caveat |
| Parent page authenticated load | Requested `/commodities-marketplace` with same session. | 200 full HTML page. | Returned 200 full page. | Pass |
| Browser smoke | Ran focused authenticated Playwright coverage for parent page, filters, HTMX tab swap, keyboard link navigation, and mobile viewport. | Browser opens for console/network check. | `tests/e2e/test_commodities_marketplace.py` passed 2/2 against a current local backend on port 8894. | Pass |

---

## Security Findings

- Authentication is enforced for the fragment.
- CSRF is not required because the route is a read-only GET.
- Manually rendered asset text fields are escaped with `html_escape`, including slug, title, location, and image URL values.
- No sensitive user, wallet, or KYC data is returned by this fragment.
- The database failure masking issue is primarily reliability/observability, but it can hide production incidents from users and monitoring.

---

## Database Findings

- The fragment depends on `assets.asset_type`, `assets.funding_status`, `assets.total_value_cents`, `assets.tokens_total`, `assets.tokens_available`, `assets.annual_yield_bps`, `assets.capital_appreciation_bps`, `assets.term_months`, `assets.land_size_sqm`, and `asset_images.image_url`.
- Monetary display uses integer cents from `total_value_cents`; no financial mutation happens on this route.
- The query is read-only and does not need a transaction.
- DB read errors are not propagated because of `.unwrap_or_default()`.

---

## Missing Tests

- HTTP integration test for unauthenticated `GET /commodities-marketplace/tab`.
- HTTP integration test for authenticated `available`, `funded`, `exited`, and invalid tab behavior.
- Fragment contract test asserting required card selectors and data attributes after each tab response.
- Browser/E2E test that clicks tabs, then verifies search, clear, location/duration filters, image arrows/dots, and card navigation.
- Accessibility test for keyboard operation of tab buttons, image controls, and commodity-card navigation.

---

## Recommended Fix Order

1. Stop masking DB errors in `api_commodities_tab`; return a clear safe error fragment with a 5xx status.
2. Align swapped commodity card markup with the initial page card contract, especially filter data attributes and shared design classes.
3. Make marketplace search/filter initialization idempotent after HTMX swaps.
4. Replace click-only card navigation with semantic links or keyboard-accessible behavior.
5. Add targeted HTTP and browser coverage for tab swaps and filters.

---

## Final Status

`fixed`

Reason: The documented code issues are fixed and verified with focused static, build, formatter, and authenticated desktop/mobile browser coverage.
