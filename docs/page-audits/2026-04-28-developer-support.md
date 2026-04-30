# Page Audit: Developer support

Date: 2026-04-28
Status: fixed, runtime verification blocked
Auditor: ChatGPT/Codex
Page URL: `/developer/support`
Template: `frontend/platform/support.html`
JavaScript: `frontend/platform/static/js/support.js`
CSS: `frontend/platform/static/css/support.css`
Backend Routes: `backend/src/developer/routes.rs`, `backend/src/support/handlers.rs`, `backend/src/support/service.rs`, `backend/src/support/db.rs`

---

## Summary

`/developer/support` reuses the authenticated support page inside the developer shell. The page route is developer-gated, and the main ticket list/create/reply/reopen flows are wired to real backend routes with ownership checks and CSRF headers. Static review found no fake primary action.

Follow-up fixes were applied in this run for the documented implementation issues: support mutation rate limits, truthful attachment failures, removal of sensitive `localStorage` drafts, retryable ticket-list errors, accessible ticket/upload/FAQ controls, file signature validation, transactional reply timestamp updates, and authenticated developer support E2E coverage. The remaining status is verification-only because the local backend cannot start while concurrent cargo/rustc jobs hold the build lock.

---

## Fix Update - 2026-04-28

Fixed:

- PAGE-ISSUE-0516: Added per-user support rate-limit checks for ticket create, reply, and reopen actions.
- PAGE-ISSUE-0517: Attachment handling now validates file signatures, fails when GCS upload is unavailable or fails, uploads before ticket creation, and inserts attachment metadata in the ticket transaction.
- PAGE-ISSUE-0518: Support ticket subject/message drafts are no longer persisted in `localStorage`; legacy drafts are cleared on page load.
- PAGE-ISSUE-0519: Ticket list failures now render a visible retryable error state.
- PAGE-ISSUE-0520: Dynamic ticket expanders now render as buttons with `aria-expanded`/`aria-controls`; upload uses a focusable label control; FAQ toggles update `aria-expanded`.
- PAGE-ISSUE-0521: Removed the dead FAQ category-tab branch from the page JS.
- Additional data integrity cleanup: reply insertion and ticket `updated_at` updates now run in one transaction.

- PAGE-ISSUE-0565: Added authenticated browser/API E2E coverage for `/developer/support` page load, ticket create, reply, reopen, ticket-list error state, rate-limit response, and keyboard expansion.
- Static regression coverage was added in `tests/test_developer_support_static.py` and passes.
- Full `cargo fmt --check` is blocked by unrelated pre-existing formatting diffs outside the support files; touched support Rust files pass `rustfmt --edition 2021 --check`.
- Runtime E2E execution and `cargo check` remain blocked by heavy concurrent local cargo/rustc jobs and no backend listening on `localhost:8888`; rerun once the build queue is clear.

---

## Tested Scope

- Reviewed the developer route gate for `GET /developer/support`.
- Reviewed the shared support template, including form fields, upload control, ticket tabs, ticket list, reply/reopen controls, and FAQ.
- Reviewed `support.js` fetch calls, DOM selectors, CSRF handling, draft persistence, render paths, and error states.
- Reviewed support handlers, service validation, storage behavior, and DB writes.
- Reviewed relevant migrations for `support_tickets`, `support_ticket_replies`, `support_ticket_attachments`, and `notifications`.
- Runtime browser/API test execution was blocked because no backend was listening on `localhost:8888` and local cargo jobs were still waiting on/competing for build locks.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/developer/support` | Developer shell for shared support page |
| Template | `frontend/platform/support.html` | Uses `is_developer` context and developer topbar |
| JS | `frontend/platform/static/js/support.js` | Loads tickets, submits tickets, replies, reopens, FAQ search, retry/error states |
| CSS | `frontend/platform/static/css/support.css` | Shared support layout and responsive styling |
| Backend page route | `GET /developer/support` | `page_developer_support`, developer role gate |
| Backend API route | `GET /api/support/tickets` | Lists current user's tickets |
| Backend API route | `POST /api/support/tickets` | Creates ticket, optional attachment |
| Backend API route | `POST /api/support/tickets/:ticket_id/reply` | Adds current-user reply |
| Backend API route | `PUT /api/support/tickets/:ticket_id/reopen` | Reopens current-user closed/resolved ticket |
| Database table | `support_tickets` | Ticket record, priority, category, SLA metadata |
| Database table | `support_ticket_replies` | Initial message and replies |
| Database table | `support_ticket_attachments` | Attachment metadata |
| Database table | `notifications` | Admin notification on new ticket |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Developer topbar | `components/developer-topbar.html` via `is_developer` | Developer navigation with Support active | Yes | Page route gated | Not runtime-tested |
| Category select | `#ticket-category` | Select supported category | Yes, included in `FormData` and draft | Yes, backend allowlist | Static pass |
| Priority select | `#ticket-priority` | Select SLA priority and update response badge | Yes | Yes, backend allowlist and SLA interval | Static pass |
| Subject input | `#ticket-subject` | Required 5-255 chars, count, FAQ suggestions | Yes | Yes, min length and sanitize | Static pass |
| Message textarea | `#ticket-message` | Required 20-5000 chars and count | Yes | Yes, min length and sanitize | Static pass |
| Attachment upload | `#drop-zone`, `#ticket-attachment` | Click/drag upload JPG, PNG, PDF <= 5MB | Yes | Yes, MIME/size/signature checked and fail-closed | Needs E2E recheck |
| Clear Draft | `#clear-ticket-draft-btn` | Clear form and legacy local draft | Yes | No backend needed | Static pass |
| Submit Ticket | `#support-form` / `#submit-ticket-btn` | Create support ticket and refresh list | Yes | Yes | Needs recheck |
| Ticket filters | `.support-tab[data-filter]` | Filter all/open/resolved | Yes | List API provides status | Static pass |
| Ticket list | `#tickets-list` | Load current user's tickets and render replies | Yes | Yes | Needs E2E recheck |
| Ticket expand | `.ticket-card-header` rendered by JS | Expand/collapse ticket detail | Yes, button + aria state | No backend needed | Needs browser recheck |
| Reply form | `.ticket-reply-form` rendered by JS | POST reply for owned open ticket | Yes | Yes, ownership/status/rate-limit checked | Needs E2E recheck |
| Reopen button | `.ticket-reopen-btn` rendered by JS | Confirm and reopen closed/resolved ticket | Yes | Yes, ownership/status/rate-limit checked | Needs E2E recheck |
| FAQ disclosure | `details#faq` | Open knowledge base | Native details | No backend needed | Static pass |
| FAQ search | `#faq-search` | Filter local FAQ rows | Yes | No backend needed | Static pass |
| FAQ questions | `.faq-question` | Expand/collapse answer | Yes, updates expanded state | No backend needed | Needs browser recheck |

---

## Frontend Findings

### P2 - Sensitive support drafts are stored in localStorage - fixed 2026-04-28

Location:

- JS: `frontend/platform/static/js/support.js:12`, `frontend/platform/static/js/support.js:156-195`

Problem:

The page stores support ticket subject, message, category, and priority in `localStorage` under `poool:support-ticket-draft`. Support tickets can contain personal, financial, KYC, account-security, or funds-at-risk details. The shared production standard says sensitive data should not be stored in browser storage unless explicitly justified.

Expected:

Either avoid persistent local drafts for support tickets, use session-only storage with a short expiry and warning, or persist drafts server-side with auth controls.

Evidence:

`saveDraft()` serializes subject/message into localStorage on every input change.

Recommended fix:

Remove ticket body persistence from localStorage or replace it with expiring server-side drafts. At minimum, do not store message text for high-risk categories/priorities.

Fix applied:

`support.js` now clears the legacy localStorage key on page load and no longer persists ticket subject/message/category/priority.

### P2 - Ticket load failures leave the list stuck in loading state - fixed 2026-04-28

Location:

- Template: `frontend/platform/support.html:148-151`
- JS: `frontend/platform/static/js/support.js:294-308`

Problem:

`loadMyTickets()` updates the UI only for `resp.ok` and redirects only for 401. A 403/500/non-JSON response or a network error leaves `#tickets-list` showing `Loading your tickets...` indefinitely.

Expected:

Render an explicit retryable error state for non-OK responses and catch blocks.

Evidence:

The catch block logs to console/Sentry but does not mutate `#tickets-list`.

Recommended fix:

Add `renderTicketsError(message)` and a retry button. Include specific handling for CSRF/session expiry and generic server failure.

Fix applied:

`loadMyTickets()` now renders a visible retryable error for non-OK and network failures.

### P2 - Dynamic ticket/upload controls are not fully keyboard accessible - fixed 2026-04-28

Location:

- Template: `frontend/platform/support.html:86-101`
- JS: `frontend/platform/static/js/support.js:343-397`, `frontend/platform/static/js/support.js:582-593`

Problem:

The upload drop zone is a clickable `<div>` without button semantics or keyboard handlers. Rendered ticket headers are clickable `<div>` elements with inline `onclick` toggles and no `aria-expanded`. FAQ buttons toggle visibility but do not update `aria-expanded`.

Expected:

Use real buttons or add `role`, `tabindex`, Enter/Space handlers, and expanded-state attributes. The file input should remain reachable with a visible/focusable control.

Evidence:

Ticket expansion is handled by inline `onclick` on `.ticket-card-header`, and the drop zone click listener only listens for pointer clicks.

Recommended fix:

Render ticket expanders as `<button type="button">`, bind delegated click/keydown handlers, and synchronize `aria-expanded`/`aria-controls`. Make the upload trigger a labeled button or label.

Fix applied:

Dynamic ticket expanders now render as buttons with ARIA state, the upload trigger is a focusable label control, and FAQ toggles update `aria-expanded`.

### P3 - FAQ category filter code is dead on this template - fixed 2026-04-28

Location:

- JS: `frontend/platform/static/js/support.js:65-76`, `frontend/platform/static/js/support.js:582-593`

Problem:

`support.js` binds `.faq-cat-tab` buttons and reads `.faq-cat-tab.active`, but `support.html` does not render any `.faq-cat-tab` controls. This is harmless at runtime but adds stale behavior and makes the FAQ filter contract unclear.

Expected:

Either render category tabs or remove the unused branch.

Evidence:

Static selector review found no `.faq-cat-tab` elements in `support.html`.

Recommended fix:

Remove the dead category-tab binding unless category tabs are planned for the support page.

Fix applied:

Removed the unused `.faq-cat-tab` binding and category-filter branch from `support.js`.

---

## Backend Findings

### P1 - Support ticket and reply mutations are not rate-limited - fixed 2026-04-28

Location:

- Backend: `backend/src/support/handlers.rs:38-163`, `backend/src/support/handlers.rs:165-225`

Problem:

Authenticated users can repeatedly create tickets, post replies, and reopen tickets without endpoint-level rate limiting. This can fill `support_tickets` / `support_ticket_replies`, spam admin notifications, and create operational load. The production readiness standard explicitly calls out rate limits for support-sensitive actions.

Expected:

Rate-limit ticket creation, replies, and reopen attempts per user/session/IP, with stronger throttling for urgent/funds-at-risk tickets and attachment uploads.

Evidence:

The handlers perform authentication and validation but do not call an auth/support rate limiter before writing.

Recommended fix:

Apply the existing rate limiter pattern to `support:create`, `support:reply`, and `support:reopen` keys. Return 429 JSON and visible frontend errors.

Fix applied:

`backend/src/support/handlers.rs` now checks the shared rate limiter for `support:create`, `support:reply`, and `support:reopen`, returning HTTP 429 JSON on limit breaches.

### P1 - Attachment upload failures can still return ticket success - fixed 2026-04-28

Location:

- Backend: `backend/src/support/service.rs:141-181`
- JS: `frontend/platform/static/js/support.js:533-559`

Problem:

The backend creates the ticket first, then treats GCS upload failure, missing `GCS_BUCKET_NAME`, and attachment metadata insert failure as log-only events. The API still returns success, and the frontend clears the selected file and shows "Ticket submitted!" This is false success for users who attached evidence for KYC, account-security, billing, or funds-at-risk issues.

Expected:

If a user submits an attachment, either persist it reliably before returning success or return an explicit partial-failure status that the frontend shows without clearing the file.

Evidence:

`upload_private` errors and `add_ticket_attachment` errors are logged but not propagated; missing bucket only logs a warning.

Recommended fix:

Make attachment persistence fail-closed when a file is provided, or return `207`/structured JSON such as `{ ticket_created: true, attachment_saved: false }` and keep the UI state actionable.

Fix applied:

Attachment content signatures are now checked, GCS must be configured when a file is provided, upload failure returns an error, upload happens before ticket creation, and attachment metadata is inserted inside the ticket transaction.

### P2 - Reply insert and ticket timestamp update are not atomic - fixed 2026-04-28

Location:

- Backend: `backend/src/support/db.rs:288-312`

Problem:

Adding a reply inserts into `support_ticket_replies` and then separately updates `support_tickets.updated_at`. If the second query fails, the reply exists but ticket ordering/SLA recency can drift.

Expected:

Wrap reply insert and ticket update in one transaction.

Evidence:

`add_reply()` executes two independent queries against the pool, unlike `create_ticket_v2()` which correctly uses a transaction.

Recommended fix:

Use `pool.begin()`, execute both writes on the transaction, and commit only after both succeed.

Fix applied:

`db::add_reply()` now wraps the reply insert and `support_tickets.updated_at` update in a single SQL transaction.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route map | Reviewed `backend/src/developer/mod.rs`, `backend/src/developer/routes.rs`, `backend/src/support/mod.rs` | `/developer/support` and `/api/support/*` exist | Routes exist | Pass |
| JS syntax | `node --check frontend/platform/static/js/support.js` | Syntax passes | Exit 0 | Pass |
| Static regression tests | `python3 -m pytest tests/test_developer_support_static.py -q` | Support fix guards pass | 5 passed | Pass |
| Touched Rust formatting | `cd backend && rustfmt --edition 2021 --check src/support/handlers.rs src/support/service.rs src/support/db.rs` | Touched support Rust files are formatted | Exit 0 | Pass |
| Targeted Rust tests | `cd backend && cargo test support --quiet` | Support-related tests pass or no failures | First run passed before fixes; post-fix run was stopped after hanging behind concurrent cargo jobs | Blocked |
| Runtime page curl | `curl -I http://localhost:8888/developer/support` | Redirect/login or page response | Connection refused; no local backend | Blocked |
| Runtime API curl | `curl -i http://localhost:8888/api/support/tickets` | 401 JSON or ticket list | Connection refused; no local backend | Blocked |
| Authenticated browser test | `python3 -m pytest tests/e2e/test_developer_support.py -q` | UI and DB state verified | Added, syntax-checked; runtime not run because backend could not start | Blocked |

---

## Security Findings

- Fixed: Support create/reply/reopen endpoints now use the shared rate limiter.
- Fixed: Ticket drafts are no longer persisted in `localStorage`.
- Fixed: Attachment upload now checks file signatures for PNG, JPEG, and PDF in addition to MIME and size.
- Ownership checks are present for list/reply/reopen paths; list queries are scoped to `user_id`, and reply/reopen check `ticket_id` plus `user_id`.
- CSRF headers are sent by the frontend and global CSRF middleware protects mutating API routes.
- Remaining verification: authenticated browser/API E2E should be run against a live backend to verify 429 rendering, attachment failure rendering, and no sensitive draft storage after real interactions.

---

## Database Findings

- `support_tickets`, `support_ticket_replies`, and `support_ticket_attachments` exist through migrations.
- Fixed: Ticket creation, initial reply, and optional attachment metadata are transactional.
- Fixed: Reply insertion and ticket `updated_at` update are transactional.
- Remaining: admin notification delivery remains best-effort and should be covered by a future support-ops reliability decision if support queue delivery must be fail-closed.

---

## Missing Tests

- Runtime execution of `tests/e2e/test_developer_support.py` once the backend can start.
- Additional API tests for rate limiting on `POST /api/support/tickets/:id/reply` and `PUT /api/support/tickets/:id/reopen`.
- Attachment tests for GCS disabled, GCS upload failure, metadata insert failure, oversize file, MIME mismatch, signature mismatch, and successful attachment display.
- Frontend tests or Playwright assertions for list API 500/error states and retry UI.
- Accessibility tests for keyboard expansion of ticket cards, upload control, FAQ state, and focus order.

---

## Recommended Follow-Up

1. Run authenticated browser/API E2E for create, reply, reopen, attachment success/failure, 429 rendering, keyboard behavior, and mobile layout.
2. Add committed regression tests for rate limits, attachment signature validation, GCS failure handling, ticket-list error state, and transactional reply writes.
3. Decide whether admin support notifications should remain best-effort or be made fail-closed/durable through a support queue.

---

## Final Status

`needs_recheck`

Reason: The documented implementation issues were fixed, but authenticated browser/API E2E and the full cargo gate still need recheck in a quiet local build/runtime environment.
