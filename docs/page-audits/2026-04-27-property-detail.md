# Page Audit: Property detail

Date: 2026-04-27
Status: fixed_e2e_verified
Auditor: ChatGPT/Codex
Page URL: `/property`, `/property/:slug`
Template: `frontend/platform/property.html`
JavaScript: `frontend/platform/static/js/property-detail.js`, `frontend/platform/static/js/property-detail-cart.js`, `frontend/platform/static/js/property-price-sticky.js`, `frontend/platform/static/js/property-detail-mobile.js`, `frontend/platform/static/js/mobile-calculator.js`, `frontend/platform/static/js/mobile-financial.js`, `frontend/platform/static/js/mobile-documents.js`, `frontend/platform/static/js/mobile-faq.js`
CSS: `frontend/platform/static/css/property-detail.css`, `frontend/platform/static/css/property-detail-mobile.css`, `frontend/platform/static/css/mobile-documents.css`, plus related mobile/property card CSS loaded by the page
Backend Routes: `backend/src/assets/mod.rs`, `backend/src/assets/routes.rs`, `backend/src/cart/mod.rs`, `backend/src/cart/routes.rs`, `backend/src/storage/routes.rs`

---

## Summary

The authenticated property detail page renders successfully for a real seeded slug and redirects unauthenticated users to login. The 2026-04-28 remediation fixed the blocking page-contract issues from this audit: desktop and mobile "Add to cart" now send explicit CSRF data and show visible failures, the documents section renders investor-visible `asset_documents`, the document download route allows authenticated investors to download only published investor-visible document types, the contact CTA routes to support with the asset slug, mobile gallery dots call `goToSlide(index)`, and `/property` without a slug redirects to marketplace instead of rendering fallback content.

Final status is `fixed_e2e_verified`.

## Fix Update - 2026-04-28

Fixed:

- Desktop and mobile add-to-cart handlers now append `csrf_token`, send `X-CSRF-Token`, disable buttons while submitting, reject non-OK responses, and surface inline errors instead of blindly redirecting.
- `/property` now requires a concrete asset slug/id, returns a safe missing/error state, redirects no-slug requests to `/marketplace`, and passes real investor-visible documents to the template.
- `frontend/platform/components/property/documents.html` now renders per-asset document download links or a truthful empty state.
- `GET /api/documents/:id/download` now allows authenticated users to download published investor-visible document types while keeping owner/admin-only access for private documents.
- The "Chat us" CTA links to `/support?topic=property&asset=<slug>`.
- Mobile gallery dot clicks now use `goToSlide(index)` instead of the undefined `updateGallery()`.

Follow-up:

- Authenticated browser/mobile E2E coverage was added in `tests/e2e/test_property_detail.py` and passed on 2026-04-28.
- Full `cargo fmt --check` and `cargo check --message-format short` passed on 2026-04-28.
- Local backend startup still emits pre-existing idempotency noise from already-applied migrations and Redis-disabled warnings.

---

## Tested Scope

- Static review of `frontend/platform/property.html` and included property components.
- Static review of loaded page scripts listed above.
- Backend route review for `/property`, `/property/:slug`, `/cart/add`, and `/api/documents/:id/download`.
- Database/schema review for `assets`, `asset_images`, `asset_documents`, `cart_items`, `kyc_records`, `platform_settings`, and `asset_views`.
- Runtime HTTP smoke with local backend on `localhost:8888`.
- Authenticated render smoke using an existing local non-production session.
- Non-destructive CSRF probe against `/cart/add` without a token.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/property` | Authenticated route; no slug can render fallback/default content instead of a clear not-found state. |
| URL | `/property/:slug` | Authenticated property detail route. |
| Template | `frontend/platform/property.html` | Main page template. |
| Component | `frontend/platform/components/property/gallery.html` | Desktop gallery and virtual tour control. |
| Component | `frontend/platform/components/property/documents.html` | Static disabled document list. |
| Component | `frontend/platform/components/property/modals.html` | Lightbox and YouTube modals. |
| Component | `frontend/platform/components/property/faq.html` | FAQ accordion content. |
| Component | `frontend/platform/components/property/contact.html` | Dead "Chat us" CTA. |
| JS | `frontend/platform/static/js/property-detail-cart.js` | Desktop add-to-cart and quick amount controls. |
| JS | `frontend/platform/static/js/property-detail-mobile.js` | Mobile gallery, quick amount, add-to-cart, and mobile helpers. |
| Backend page route | `GET /property`, `GET /property/:slug` | Registered in `backend/src/assets/mod.rs`; rendered by `page_property`. |
| Backend mutation route | `POST /cart/add` | Registered in `backend/src/cart/mod.rs`; requires auth, KYC approval, CSRF, and transaction. |
| Backend document route | `GET /api/documents/:id/download` | Exists, but authorizes only asset owner or admin. |
| Database table | `assets` | Source of property metadata and token availability. |
| Database table | `asset_images` | Source of gallery images. |
| Database table | `asset_documents` | Exists and has seeded records, but is not queried by `/property`. |
| Database table | `cart_items` | Target of successful add-to-cart mutations. |
| Database table | `asset_views` | Fire-and-forget page-view analytics insert. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb marketplace label | `#property-breadcrumbs` | Navigate/contextual path back to marketplace. | No link behavior. | Not needed. | Static only. |
| Desktop gallery images | `.gallery-image[onclick]` | Open lightbox at clicked image. | Inline global `openLightbox(index)`. | Uses server-rendered `asset.image_urls`. | Static review OK; keyboard access weak because clickable divs are not buttons. |
| View all photos | `#gallery-view-all-button` | Open gallery lightbox. | Inline `openLightbox(0)`. | Uses server-rendered images. | Wired. |
| Virtual tour | `.gallery-btn-group`, `.mobile-gallery-btn` | Open YouTube modal when `asset.youtube_video_id` exists. | Calls `openVirtualTour()`/video modal JS. | Uses `assets.video_url`. | Wired when video exists. |
| Lightbox close/prev/next | `#lightbox-modal` buttons | Close/navigate images. | `property-detail.js`. | Not needed. | Wired; no focus trap documented. |
| Financial tabs | `.financial-tab[data-tab]` | Switch property cost vs rental income. | `property-detail.js`. | Server-rendered values only. | Wired. |
| Calculator sliders | `#calc-slider-1`, `#calc-slider-2`, `#calc-slider-3` | Update projected chart and labels. | `property-detail.js`. | Client-only projection. | Wired; projection is UX-only and not trusted for backend. |
| Documents download buttons | `.download-button[disabled]` | Show downloadable diligence documents or truthful empty state. | Disabled only. | `asset_documents` exists but not queried; download route excludes investors. | Broken/incomplete. |
| FAQ rows | `.faq-item-content` | Expand/collapse answers. | `property-detail.js`. | Not needed. | Click wired; keyboard semantics weak. |
| Chat CTA | `.contact-button.chat-button` | Open chat/support/contact flow. | No listener, link, form, or data action found. | Support module exists but no route wiring from this CTA. | Dead UI. |
| Desktop amount input | `#investment-amount-input` | Enter investment amount. | Read by `property-detail-cart.js`. | `/cart/add` parses amount as cents. | Submit blocked by missing CSRF. |
| Desktop quick amount buttons | `.quick-add-btn` | Add preset amount to input. | Wired. | Not needed until submit. | Wired. |
| Desktop Add to cart | `#add-to-cart-main-btn` | Add selected property tokens to cart then redirect to `/cart`. | Posts `fetch('/cart/add')`. | Route exists and is transactional. | Runtime HTTP probe: no-CSRF POST returns 403. Page JS does not send CSRF, so core action is broken. |
| Mobile amount input | `#mobile-investment-amount` | Enter investment amount. | Read by `property-detail-mobile.js`. | `/cart/add`. | Same missing-CSRF issue. |
| Mobile quick amount buttons | `.mobile-quick-btn` | Add preset amount to mobile input. | Wired. | Not needed until submit. | Wired. |
| Mobile Add to cart | `#mobile-add-to-cart-btn` | Add selected property tokens to cart. | Posts `fetch('/cart/add')`. | Route exists and is transactional. | Same missing-CSRF issue. |
| Mobile gallery arrows | `.mobile-gallery-prev`, `.mobile-gallery-next` | Scroll mobile gallery. | Inline `scrollGallery()`. | Server-rendered images. | Wired if slides exist. |
| Mobile gallery dots | `.gallery-dot-clickable` | Jump to mobile slide. | Two handlers bind; one calls undefined `updateGallery()`. | Not needed. | Console error risk on dot click. |

---

## Frontend Findings

### P1 - Add to cart omits required CSRF token - Fixed 2026-04-28

Location:

- Template: `frontend/platform/property.html`
- JS: `frontend/platform/static/js/property-detail-cart.js:104`
- JS: `frontend/platform/static/js/property-detail-mobile.js:190`

Problem:

Both desktop and mobile add-to-cart handlers post `URLSearchParams` to `/cart/add` without an `X-CSRF-Token` header or `csrf_token` body field. The global CSRF middleware validates every POST, including `/cart/add`.

Expected:

The fetch should include the `csrf_token` cookie value as `X-CSRF-Token` or append a matching `csrf_token` field. Non-OK responses should surface a visible error instead of redirecting to the response URL.

Evidence:

Authenticated GET `/property/best-villa-0e7fb42b-1480-4a2c-a278-359d81804e94` returned `200`. A non-destructive authenticated POST to `/cart/add` without CSRF returned `403`. The page scripts do not add CSRF, so the visible "Add to cart" CTA cannot complete.

Recommended fix:

Read `csrf_token` from the cookie, send it in `X-CSRF-Token` for both desktop and mobile fetches, set a loading/disabled state, and handle `!response.ok` with a visible inline error. Add an authenticated E2E that verifies cart row creation and CSRF rejection.

### P1 - Property documents are not backed by real asset documents - Fixed 2026-04-28

Location:

- Template: `frontend/platform/components/property/documents.html:6`
- Backend: `backend/src/assets/routes.rs:109`
- Backend: `backend/src/storage/routes.rs:1326`

Problem:

The property page shows hardcoded document names and disabled download buttons even though the database has `asset_documents` records. The `/property` route does not query documents, and the existing `/api/documents/:id/download` route only authorizes the developer owner or an admin, not authenticated investors viewing diligence material.

Expected:

The page should either render the current asset's real approved/public investor documents with authorized download links, or show a truthful empty state when no investor-visible documents exist.

Evidence:

`psql` showed seeded `asset_documents` rows such as investment expose, appraisal, and proof of title. The component still renders static disabled "Smart Contract", "Token Registration", and "Articles_of_Organization_LLC.pdf" rows.

Recommended fix:

Define which asset document types are investor-visible, query them in `page_property`, render per-asset document rows, and update the download route or add a dedicated investor document route with asset visibility and authorization checks.

### P2 - Contact CTA has no behavior - Fixed 2026-04-28

Location:

- Template: `frontend/platform/components/property/contact.html:15`

Problem:

The "Chat us" button has no link, form action, data action, or JavaScript listener in the loaded property scripts.

Expected:

The CTA should route to support, open an existing support/chat modal, or be removed/disabled with clear copy.

Evidence:

Static search found no property-page listener for `.contact-button` or `.chat-button`.

Recommended fix:

Wire it to the support flow, for example `/support?topic=property&asset=<slug>`, and add a browser smoke for click/navigation.

### P2 - Mobile gallery dot handler calls an undefined function - Fixed 2026-04-28

Location:

- JS: `frontend/platform/static/js/property-detail-mobile.js:98`

Problem:

`initializeDotNavigation()` attaches a click listener that calls `updateGallery()`, but no `updateGallery` function exists in the loaded script. A document-level delegated handler later calls `goToSlide(index)`, but the direct dot handler can still throw a console error.

Expected:

Only one dot handler should exist, and it should call `goToSlide(index)` without console errors.

Evidence:

Static review found `updateGallery()` is undefined. `node --check` passes because the function is only resolved at runtime.

Recommended fix:

Remove the stale direct handler or replace it with `goToSlide(index)`. Add a mobile viewport browser smoke that clicks gallery dots and checks console output.

---

## Backend Findings

### P1 - Core property investment action is protected correctly by CSRF, but the page contract is mismatched

`POST /cart/add` is registered and performs the important server-side checks: authentication, KYC approval, integer-cent amount parsing, asset row `FOR UPDATE`, transactional upsert to `cart_items`, and token availability caps. The frontend contract is wrong because it omits the CSRF token required by `auth::csrf::csrf_middleware`.

### P1 - Investor document access policy is undefined/missing

`GET /api/documents/:id/download` is suitable for private developer/admin access, but it does not support the property page's implied investor diligence flow. This is safer than overexposing documents, but it leaves the visible property documents UI unimplemented.

### P2 - `/property` without slug has no explicit not-found state

The route accepts `/property` with no slug and renders the template with `asset => None`, relying on defaults in the template. This can show generic property content and a default add-to-cart property id instead of a clear 404 or redirect to marketplace.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated property route | `curl http://localhost:8888/property/<slug>` without session | Redirect to login | `303` to `/auth/login` with CSRF cookie | Pass |
| Authenticated property route | `curl` with local `poool_session` for `/property/best-villa-...` | Page HTML renders | `200`, 167,678 bytes, Add to cart/Documents/Chat CTA present | Pass |
| Add-to-cart CSRF probe | Authenticated `POST /cart/add` without CSRF | Reject unsafe request | `403` | Pass for backend protection; fail for page contract |
| JS syntax | `node --check` on loaded property scripts | No syntax errors | No syntax errors | Pass |
| Browser click/mobile/console | Not run | Click controls and inspect console/network | Not run; browser automation not used in this documentation run | Needs coverage |

---

## Security Findings

- P1: Add-to-cart mutation is correctly rejected without CSRF, but the visible page action does not satisfy the route's CSRF contract. This blocks the user flow rather than creating a CSRF bypass.
- P1: Investor-facing document authorization is undefined. Do not simply reuse the developer/admin document route without a deliberate investor visibility policy.
- P2: The page renders `asset.long_description | safe`; current developer edit paths sanitize descriptions before storage, but legacy/imported asset descriptions should remain in scope for stored-XSS regression tests.

---

## Database Findings

- `cart_items` has the expected `UNIQUE (user_id, asset_id)` constraint and integer-cent `token_price_cents`.
- `/cart/add` uses a transaction and locks the selected asset row with `FOR UPDATE`.
- `asset_documents` exists and has local rows, but `/property` does not query it.
- `asset_views` has indexes but no uniqueness constraint for one row per user/asset/day; the handler performs an application-level existence check before insert. This is acceptable for analytics, but concurrent reloads can still duplicate daily view rows.

---

## Missing Tests

- Authenticated property add-to-cart E2E: page click sends CSRF, redirects to cart, and creates/updates exactly one `cart_items` row.
- CSRF negative E2E: `/cart/add` without token returns 403 and the UI surfaces a refresh/retry message.
- KYC gate E2E: non-approved investor is redirected to `/kyc?reason=required` without a cart write.
- Investor document E2E after product policy is defined: render only allowed documents and deny unauthorized private documents.
- Mobile gallery dot browser test with console-error assertion.
- Contact CTA navigation/support-flow browser test.

---

## Recommended Fix Order

1. Fix desktop and mobile add-to-cart CSRF/header handling and visible error states.
2. Define and implement investor-visible property document rendering/download authorization.
3. Wire or remove the "Chat us" CTA.
4. Remove the stale mobile gallery dot handler and add mobile console coverage.
5. Add an explicit 404/empty state for `/property` without a valid slug.

---

## Final Status

`needs_recheck`

Reason: The page renders, but the core Add to cart action is broken by a frontend/backend CSRF contract mismatch, and due-diligence documents/contact controls remain incomplete.
