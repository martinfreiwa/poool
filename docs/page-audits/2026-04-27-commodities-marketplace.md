# Page Audit: Commodities Marketplace

Date: 2026-04-27
Fix pass: 2026-04-28
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/commodities-marketplace`
Template: `frontend/platform/commodities-marketplace.html`
JavaScript: `frontend/platform/static/js/commodities-marketplace.js`, `frontend/platform/static/js/marketplace-search.js`, `frontend/platform/static/js/property-card.js`
CSS: `frontend/platform/static/css/marketplace.css`, `frontend/platform/static/css/property-card.css`
Backend Routes: `backend/src/assets/mod.rs`, `backend/src/assets/routes.rs`

---

## Summary

The six findings from the 2026-04-27 audit were fixed and verified on 2026-04-28. The commodities marketplace now uses commodity-specific filters, exposes a reachable More Filters panel on desktop and mobile, renders semantic commodity card links, keeps initial and HTMX-swapped card data attributes aligned, returns safe 5xx states for commodity query failures, and no longer exposes MiniJinja render error details in authenticated HTML responses.

Follow-up verification also fixed a runtime MiniJinja type error in the parent template by using preformatted commodity display fields, added explicit `data-yield` attributes for yield filtering, and restored the mobile More Filters control. Focused desktop/mobile browser E2E, static checks, formatter checks, and isolated backend check passed.

---

## Fixed Issues

| Issue | Severity | Status | Fix |
|------|----------|--------|-----|
| PAGE-ISSUE-0374 | High | Fixed | Replaced property-specific filters with commodity location/type/term filters and added matching card data attributes. |
| PAGE-ISSUE-0375 | Medium | Fixed | Added the More Filters button and applied commodity type filtering alongside price/yield filters. |
| PAGE-ISSUE-0376 | Medium | Fixed | Replaced divergent tab HTML with typed commodity display data and the shared commodity card renderer. |
| PAGE-ISSUE-0377 | Medium | Fixed | Removed click-only card navigation and added semantic `.property-card-link` anchors to commodity titles. |
| PAGE-ISSUE-0378 | High | Fixed | Replaced `unwrap_or_default()` query fallbacks with logged safe 500 page/fragment responses. |
| PAGE-ISSUE-0379 | Medium | Fixed | Logged template failures server-side and returned a generic internal error page. |

---

## Remaining Issues

No functional/security/accessibility issue from this page audit remains open.

---

## Verification

- `node --check frontend/platform/static/js/commodities-marketplace.js`
- `node --check frontend/platform/static/js/marketplace-search.js`
- `node --check frontend/platform/static/js/property-card.js`
- `python3 -m pytest tests/test_commodities_tab_static.py`
- `BASE_URL=http://localhost:8894 python3 -m pytest tests/e2e/test_commodities_marketplace.py -q` passed 2/2 against a current local backend.
- `cargo fmt`
- `cargo fmt --check`
- `CARGO_TARGET_DIR=/tmp/poool-commodities-run CARGO_INCREMENTAL=0 cargo check --quiet`
- `python3 scripts/audit_page_review_tracker.py --write-md` regenerated `docs/PAGE_REVIEW_TRACKER.md`; passed with 0 missing tracker routes.
