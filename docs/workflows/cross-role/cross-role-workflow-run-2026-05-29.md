# Cross-Role Workflow Run - 2026-05-29

Environment:
- Local backend: `http://localhost:8888`
- Database: local `poool`
- Browser/E2E harness: Playwright via pytest

Executed:
- `python3 -m pytest tests/e2e/test_user_lifecycle.py::test_full_user_lifecycle_hybrid -q`
- `python3 -m pytest tests/e2e/test_auth_login.py tests/e2e/test_public_property.py tests/e2e/test_user_lifecycle.py -q`

Result:
- Passed after fixes: `1 passed in 16.98s`
- Follow-up combined browser/E2E pass: `26 passed in 52.88s`

Issues found and fixed during this run:
- Mutating API calls in the lifecycle harness needed CSRF headers and correct form encoding for browser-equivalent requests.
- The Developer dashboard emitted early-load JavaScript errors because HTMX listeners were attached to `document.body` before it was guaranteed to exist.
- The Developer Operations submit page could build an invalid URL: `/api/developer/villas/:asset_id/operations/operations`. The workflow now needs to include browser network-failure checks, not only final database/API readback.
- Local cleanup had to respect append-only operation and asset-link guards by temporarily disabling the local-only cleanup triggers around disposable workflow records.
- Authenticated property/cart redirects could render `500` because the templates were called without the shared sidebar context. The lifecycle workflow now checks this redirect path through cart and checkout.
- Public preview property pages could make a failed performance API call with a static slug instead of a UUID, so the public-property branch now checks for failed network requests.

Workflow coverage added:
- Check console errors and failed network requests after loading `/developer/dashboard`.
- Check console errors and failed network requests after loading `/developer/villas/:asset_id/operations/new`.
- Verify the operations page parses both create URLs and edit URLs without treating the route segment `operations` as a log id.
- Verify wallet deposit, cart purchase, checkout, operations submit, and secondary order calls include CSRF headers when exercised through browser-context API requests.
- Verify purchase wallet-balance assertions use the actual wallet transaction amount, including fees, not only the nominal token amount.

Remaining gaps:
- This run covers the hybrid cross-role lifecycle path, not every individual cross-role workflow file.
- Full manual UI interaction for document uploads, dividend execution, tax report download, and every negative case still needs separate browser evidence if release sign-off requires complete visual coverage.
