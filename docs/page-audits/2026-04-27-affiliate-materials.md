# Page Audit Fix: Affiliate Materials

Date audited: 2026-04-27
Fix pass: 2026-04-28
Status: fixed, needs authenticated browser recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/materials`

## Summary

The previously documented `/affiliate/materials` issues have been fixed in code. The page now has a real page controller, wired material downloads, a served guidelines PDF, a locked non-active affiliate state, custom material upload/status UI, server-side upload type validation, and accessible logo preview text.

Remaining issue: authenticated runtime/browser verification is still needed for the active-affiliate path, non-active locked state, PDF content response, generated downloads, and real upload success/failure with configured storage.

## Files Changed

| Area | Path |
|------|------|
| Template | `frontend/platform/affiliate-materials.html` |
| JavaScript | `frontend/platform/static/js/affiliate-materials.js` |
| Static document | `frontend/platform/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf` |
| Backend routes | `backend/src/rewards/routes.rs` |
| Backend router | `backend/src/rewards/mod.rs` |
| Static regression tests | `tests/admin/test_affiliate_route_contract_static.py` |
| Tracker | `docs/issue-tracking/page-review-tracker.yml`, `docs/issue-tracking/PAGE_REVIEW_TRACKER.md` |
| Coverage | `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` |

## Fixed Issues

| Issue | Severity | Status | Fix |
|-------|----------|--------|-----|
| PAGE-ISSUE-0330: Material download buttons are dead UI | High | Fixed | Added `frontend/platform/static/js/affiliate-materials.js`; buttons now use `data-material-download` and generate/download approved PNG materials. Logo downloads are real SVG links. |
| PAGE-ISSUE-0331: Brand guidelines download has no PDF asset | High | Fixed | Added `frontend/platform/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf` and updated the template link to `/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf`. |
| PAGE-ISSUE-0332: Page not gated to active affiliates | High | Fixed | Template now renders approved materials only when `affiliate_status == 'active'`; non-active users see a locked state with onboarding/dashboard actions. APIs use `require_active_affiliate_user_id`. |
| PAGE-ISSUE-0333: Custom material upload lacks file-type validation | High | Fixed | Backend validates empty/oversize uploads, extension, magic bytes/signature, SVG safety, and declared content type before storage. Storage now uses detected content type instead of `application/octet-stream`. |
| PAGE-ISSUE-0334: Custom upload workflow not exposed | Medium | Fixed | Added upload form, file constraints, live status region, review-status table, `GET /api/affiliate/materials`, and JS load/submit handling. |
| PAGE-ISSUE-0335: Logo previews missing alt text | Low | Fixed | Added meaningful `alt` text for logo previews and social asset previews. |

## Remaining Issues

| Remaining Item | Severity | Status | Notes |
|----------------|----------|--------|-------|
| Authenticated runtime/browser recheck | Medium | Open | Static tests pass, but the active affiliate browser path, non-active locked state, PDF response headers, generated downloads, and upload success/failure need verification with a real session and storage configuration. |
| Full Rust verification | Medium | Open | `cargo test affiliate_material_upload_tests` could not complete because local disk space was exhausted while compiling dependencies in `/tmp`. Re-run after freeing enough local build space. |

## Verification

Passed:

```bash
node --check frontend/platform/static/js/affiliate-materials.js
python3 -m pytest tests/admin/test_affiliate_route_contract_static.py -q
git diff --check -- backend/src/rewards/routes.rs backend/src/rewards/mod.rs frontend/platform/affiliate-materials.html frontend/platform/static/js/affiliate-materials.js frontend/platform/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf tests/admin/test_affiliate_route_contract_static.py docs/page-audits/2026-04-27-affiliate-materials.md
```

Attempted:

```bash
cd backend && cargo test affiliate_material_upload_tests --lib
cd backend && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/poool-affiliate-materials-target RUSTFLAGS='-A missing_docs' cargo test affiliate_material_upload_tests --no-fail-fast
```

Result: `--lib` is not valid for this binary-only package. The isolated non-`--lib` run did not reach application code; Rust compilation failed while building dependencies with `No space left on device`. The temporary target directory was removed after the failed run.
