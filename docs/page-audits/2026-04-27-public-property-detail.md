# Page Audit: Public Property Detail

Date: 2026-04-27
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/p/:slug`
Template: `frontend/platform/property-public.html`
JavaScript: `frontend/platform/static/js/property-detail.js`, `frontend/platform/static/js/property-detail-mobile.js`, `frontend/platform/static/js/property-price-sticky.js`, `frontend/platform/static/js/property-card.js`, `frontend/platform/static/js/marketplace.js`, `frontend/platform/static/js/legal-enhancements.js`, `frontend/platform/static/js/mobile-calculator.js`, `frontend/platform/static/js/mobile-financial.js`, `frontend/platform/static/js/mobile-documents.js`, `frontend/platform/static/js/mobile-faq.js`
CSS: `frontend/platform/static/css/property-detail.css`, `frontend/platform/static/css/marketplace.css`, `frontend/platform/static/css/property-card.css`, `frontend/platform/static/css/property-detail-mobile.css`, mobile property CSS files
Backend Routes: `backend/src/assets/mod.rs`, `backend/src/assets/routes.rs`, `backend/src/assets/public_assets.rs`

---

## Summary

The public property detail page loads without authentication, returns 404 for unknown public slugs, and now prefers canonical published asset records before falling back to clearly labelled landing-page preview data. The previously broken mobile gallery, duplicate mobile amount binding, fake contact/tour controls, placeholder footer/developer controls, and public fee display fallback have been fixed and covered by targeted regression tests.

Fix run: 2026-04-28. Remaining page-specific issues: none open for this audit. Future hardening should add live public document coverage if public documents are exposed on `/p/:slug`.

---

## Tested Scope

- Static review of `frontend/platform/property-public.html`.
- Static review of included property components: gallery, documents, FAQ, reviews, contact, modals.
- Static review of page-linked JavaScript, especially public CTA wiring and mobile gallery/amount logic.
- Backend route review for `/p/:slug` registration and handler.
- Public asset data review in `backend/src/assets/public_assets.rs`.
- Safe runtime HTTP smoke for a valid public slug and an invalid slug.
- Targeted public-property Playwright suite.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/p/:slug` | Public, no auth required |
| Template | `frontend/platform/property-public.html` | Public wrapper around property-detail components |
| Component | `frontend/platform/components/property/gallery.html` | Inline `openLightbox()` handlers |
| Component | `frontend/platform/components/property/documents.html` | Static disabled document rows |
| Component | `frontend/platform/components/property/contact.html` | Dead `Chat us` button |
| Component | `frontend/platform/components/property/faq.html` | Desktop FAQ toggles via `property-detail.js` |
| Component | `frontend/platform/components/property/modals.html` | Gallery and YouTube modal shells |
| JS | `frontend/platform/static/js/property-detail.js` | Gallery, desktop financial tabs, docs modal, FAQ |
| JS | `frontend/platform/static/js/property-detail-mobile.js` | Mobile gallery, quick amount, stale add-to-cart helper |
| Backend page route | `GET /p/:slug` | Registered in `backend/src/assets/mod.rs` |
| Backend handler | `page_property_public` | `backend/src/assets/routes.rs` |
| Data source | `assets`, `asset_images`; fallback `backend/src/assets/public_assets.rs` | Live published property first, labelled preview fallback |
| Database table | `platform_settings` | Reads `platform_fee_percent` as Decimal-derived basis points |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Logo | `.header-logo` | Navigate to landing | Yes, anchor | Static route | Not clicked in runtime; href present |
| Header nav | `.header-nav .nav-link` | Jump to landing sections | Yes, anchors | Static route | Hrefs present |
| Login / signup | `.header-actions a` | Navigate to platform auth | Yes, absolute links | Auth routes exist on platform host | Hrefs present |
| Breadcrumb | `#property-breadcrumbs a` | Return to marketplace section | Yes, anchor | Static route | Covered by E2E |
| Desktop gallery tiles | `.gallery-image` | Open lightbox | Inline `openLightbox()` | No backend needed | E2E covers image load, not click |
| View all photos | `#gallery-view-all-button` | Open lightbox | Inline `openLightbox(0)` | No backend needed | Not directly clicked in E2E |
| Mobile gallery prev/next | `.mobile-gallery-prev`, `.mobile-gallery-next` | Scroll gallery | Inline `scrollGallery()` | No backend needed | Not covered by current E2E |
| Mobile gallery dots | `.mobile-gallery-dot` | Jump to slide | Calls `goToSlide(index)` | No backend needed | Fixed; regression covered |
| Financial tabs | `.financial-tab` | Toggle property cost/rental income | `property-detail.js` | No backend needed | Syntax passes; not directly tested |
| Documents | `.download-button` | Download or truthfully indicate unavailable | Disabled | No document API for public page | Truthful disabled state, but static names |
| Contact CTA | `.contact-button.chat-button` | Open support/contact channel | WhatsApp anchor | External | Fixed |
| Desktop amount input | `#investment-amount-input` | Capture intended amount for signup | Inline JS | Signup receives query param only | E2E sees CTA, not amount transfer |
| Desktop quick amounts | `.quick-add-btn` | Add amount once | Inline JS | No backend needed | Static path looks wired |
| Desktop signup CTA | `#property-price-card .add-to-cart-btn` | Redirect to signup with return/amount | Inline JS | Auth route on platform host | E2E verifies visible text only |
| Mobile amount input | `#mobile-investment-amount` | Capture intended amount for signup | Double-bound quick buttons | Signup receives query param only | Double increment risk |
| Mobile quick amounts | `.mobile-quick-btn` | Add amount once | Public inline script owns binding | No backend needed | Fixed; regression covered |
| Mobile signup CTA | `#mobile-property-price-card .add-to-cart-btn` | Redirect to signup with amount | Inline JS | Auth route on platform host | Not covered by current E2E |
| Mobile virtual tour | `.mobile-gallery-btn` with `openVirtualTour()` | Launch real tour or be hidden | Hidden unless real YouTube media exists | No backend/media | Fixed |
| Footer language buttons | `.lang-option` | Show current language state as non-button text | Static text | No | Fixed |
| Footer WhatsApp link | `.footer-brand-link-green` | Open WhatsApp | Anchor | External | Href present |

---

## Frontend Findings

### P1 - Public property detail can drift from live investment records

Status: fixed 2026-04-28.

Location:

- Backend: `backend/src/assets/routes.rs:336`
- Data: `backend/src/assets/public_assets.rs:1`
- Template: `frontend/platform/property-public.html:396`

Problem:

`/p/:slug` renders hardcoded synthetic property data from `public_assets::lookup()` instead of the canonical `assets`, `asset_images`, documents, funding, and investment records. The template presents investment-like values such as funded percentage, available USD, investor count, fees, projected return, and sold-out state.

Expected:

Public property details should either render canonical live asset records with a safe public projection or clearly be treated as marketing examples without live availability/investment claims.

Evidence:

`page_property_public` only calls `super::public_assets::lookup(&slug)`. `public_assets.rs` explicitly states landing cards are static and not backed by the `assets` table.

Fix:

`page_property_public` now queries published non-commodity `assets` and `asset_images` by slug first. If no live record exists, it falls back to `public_assets::lookup()` with `asset.is_public_preview = true`, a visible preview notice, and softened labels such as “preview interest,” “early interest,” and “sign up for live availability.”

### P2 - Mobile gallery dots call an undefined function before the delegated handler runs

Status: fixed 2026-04-28.

Location:

- JS: `frontend/platform/static/js/property-detail-mobile.js:98`
- Template: `frontend/platform/property-public.html:525`

Problem:

`initializeDotNavigation()` binds every `.mobile-gallery-dot` click to `updateGallery()`, but no `updateGallery` function exists in the loaded public page scripts. A later delegated click handler calls `goToSlide(index)`, but the first handler can throw a `ReferenceError`.

Expected:

Mobile gallery dots should call one implemented function, preferably `goToSlide(index)`, without console errors.

Evidence:

`node --check` passes syntax, but static call graph shows `updateGallery()` is undefined in `property-detail-mobile.js`.

Fix:

`property-detail-mobile.js` now calls `goToSlide(index)` from the direct dot listener and guards empty galleries. A mobile regression clicks a second dot and asserts no critical console errors.

### P2 - Mobile quick amount buttons increment twice

Status: fixed 2026-04-28.

Location:

- Template inline JS: `frontend/platform/property-public.html:882`
- JS: `frontend/platform/static/js/property-detail-mobile.js:109`
- Template controls: `frontend/platform/property-public.html:635`

Problem:

The public template binds `.mobile-quick-btn` through `bindQuickAddButtons()`, and `property-detail-mobile.js` binds the same buttons through `initializeQuickAmounts()`. A mobile click on `+ USD 500` can add USD 1,000 to the intended signup amount.

Expected:

Each amount button should have one owner and add exactly the displayed amount once.

Evidence:

The same selector, `.mobile-quick-btn`, is wired in both files.

Fix:

`property-detail-mobile.js` skips quick-amount binding on `body.property-public-body`, leaving the public inline signup amount handler as the only owner. A mobile regression verifies `+ USD 500` changes `2,000` to `2,500`.

### P2 - Visible contact and virtual-tour controls are not real lead-capture flows

Status: fixed 2026-04-28.

Location:

- Contact component: `frontend/platform/components/property/contact.html:15`
- Mobile virtual tour: `frontend/platform/property-public.html:546`
- Inline function: `frontend/platform/property-public.html:850`

Problem:

The “Chat us” button has no link, form submit, or event listener. The mobile “Virtual tour” button always shows a temporary “Virtual tour coming soon!” toast even though public property specs have no `video_url`/tour media.

Expected:

Public lead-capture controls should open a real support channel, WhatsApp link, contact form, or be hidden/disabled with truthful copy.

Evidence:

The button is plain `<button>` with no handler; `openVirtualTour()` only creates a toast.

Fix:

The shared contact CTA is now a WhatsApp link with `target="_blank"` and `rel="noopener noreferrer"`. The mobile virtual-tour button only renders when `asset.youtube_video_id` is present and `openVirtualTour()` delegates to the real video modal function.

### P3 - Footer language buttons and developer social icons are dead controls

Status: fixed 2026-04-28.

Location:

- Footer language buttons: `frontend/platform/property-public.html:670`
- Developer social icons: `frontend/platform/property-public.html:369`

Problem:

The footer renders `ID`/`EN` as buttons with no language-switch behavior, and developer social icons use `href="#"`. These look interactive but do not perform useful actions.

Expected:

Dead controls should be anchors to real destinations, disabled/non-button text, or removed.

Evidence:

No JS binds `.lang-btn`, and the social anchors point to `#`.

Fix:

Footer language controls were converted to non-button text. Placeholder developer social anchors were removed, leaving only the real developer website link with safe external-link attributes.

---

## Backend Findings

### P2 - Public fee display silently falls back on DB errors and uses floats near fee display

Status: fixed 2026-04-28.

Location:

- Backend: `backend/src/assets/routes.rs:349`
- Data transform: `backend/src/assets/public_assets.rs:302`

Problem:

The public handler reads `platform_settings.platform_fee_percent`, parses it as `f64`, and silently falls back to 5.0 on query or parse failure. The public data transformer also uses `f64` for funded percentages, returns, and price per square meter. This is display-only, but it is fee/return-facing copy on an investment page and can mask configuration problems.

Expected:

Fee display should use the same decimal/basis-point source of truth as production financial code, and configuration read failures should be visible to operators.

Evidence:

The handler uses `.ok().flatten().and_then(|v| v.parse::<f64>().ok()).unwrap_or(5.0)`.

Fix:

The public route now parses `platform_fee_percent` through `Decimal` into integer basis points, logs malformed settings, returns a safe 500 on database read errors, and recalculates display fees through integer basis-point arithmetic.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Valid public slug HTTP | `curl -I http://localhost:8888/p/sunset-luxury-villa` | 200 HTML | 200 HTML | Pass |
| Invalid public slug HTTP | `curl -I http://localhost:8888/p/not-a-real-slug` | 404 | 404 | Pass |
| Render smoke | `curl` valid page and grep title/CTA/docs/contact | Public content appears | Title, CTA, docs, contact, virtual tour present | Pass |
| JS syntax | `node --check` on page-linked JS | No syntax errors | No syntax errors | Pass |
| Public property E2E | `python3 -m pytest tests/e2e/test_public_property.py -q` | Existing public smoke passes | 14 passed | Pass |

---

## Security Findings

- No authentication is required, and that matches the public page contract.
- No state-changing backend action is exposed by the public CTA; public “Sign up to invest” links to signup rather than adding to cart.
- Live DB descriptions are HTML-escaped before being wrapped in paragraphs for the existing `asset.long_description | safe` rendering path.
- CSP, frame, content-type, referrer, and permissions-policy headers were present in HTTP smoke responses.

---

## Database Findings

- The page now prefers published rows from `assets` and `asset_images`; static public specs are only a labelled fallback.
- The route also reads `platform_settings.platform_fee_percent` for fee display and treats DB read errors as safe 500s.
- No database writes are performed.

---

## Missing Tests

- Public CTA test that verifies the signup redirect preserves both `returnTo` and a single intended amount.
- Live DB-backed public asset tests for funding state, sold-out state, public documents, and stale/missing asset handling.
- Authenticated `/property/:slug` should get separate regression coverage for its remaining add-to-cart/document/contact issues.

---

## Recommended Fix Order

All findings from this audit have been addressed locally. Remaining work is follow-up coverage, not an open public-page defect:

1. Add public live-asset fixtures that exercise DB-backed `/p/:slug` rows and public documents if documents become visible.
2. Keep the separate authenticated property-detail audit open until `/property/:slug` add-to-cart/document/contact issues are fixed.

---

## Final Status

`fixed`

Reason: Public `/p/:slug` now renders live published assets first, labels static fallback data as preview content, removes fake/dead controls, fixes the mobile JS defects, and uses Decimal/basis-point fee display handling. Targeted mobile regressions were added for the fixed interactions.
