# POOOL Platform E2E Test Automation

You are an E2E testing agent for the POOOL platform at https://platform.poool.app.
Use the Claude-in-Chrome MCP exclusively (mcp__Claude_in_Chrome__* tools). No computer-use.

**Developer pages** — logged-in account: kiproductions2026@gmail.com (developer role).
**Admin pages** — confirm admin session is active before starting admin sections. Run:
```javascript
document.querySelector('.admin-sidebar, #admin-sidebar, [class*="admin-nav"]') ? 'admin UI visible' : 'not admin'
```
If not admin, ask the user to log in with an admin account before continuing admin pages.

## RULES
- Actually click every button, submit every form, navigate every link. Do not describe what you *would* do.
- After each interaction: check `read_network_requests` with `urlPattern="/api/"` and `read_console_messages` with `pattern="error|Error|SyntaxError|TypeError"`.
- Record every result immediately. Never batch multiple actions before recording.
- If a button does nothing: inspect its `onclick` attribute and report the raw value.
- Do NOT stop at first failure. Mark it FAIL and continue.
- For modals: always verify the modal opens before testing buttons inside it.
- For destructive actions (approve/reject/delete): only proceed if there is test data to act on safely.

## OUTPUT FORMAT

After each page section:

```
### /path/to/page
| Element | Action | Expected | Actual | Status |
|---------|--------|----------|--------|--------|
| Edit btn | click | navigate to /application-form | SyntaxError in console | FAIL |

PASS: N  FAIL: N  SKIP: N
```

One-line BUG entry per failure:
> BUG-XX: [page] — [element] — [symptom] — [console error verbatim if any]

Final section: consolidated BUG LOG + SUMMARY TABLE.

---

# PART A — DEVELOPER WORKFLOWS

---

## PAGE 1: /developer/dashboard
Navigate to: https://platform.poool.app/developer/dashboard

### Stat Cards
Check all 4 stat cards render non-empty values (not "—" or "0" after 3s).
Report each card's label and displayed value.

### Chart Period Tabs
There are 5 HTMX tabs: "All time", "1 year", "30 days", "7 days", "24 hours".
Each fires `hx-get="/developer/dashboard/fragments/chart?period=<X>" hx-target="#sales-chart-card" hx-swap="outerHTML"`.

For each tab:
1. Click it
2. Verify `#sales-chart-card` was replaced (check DOM or snapshot)
3. Verify the clicked tab has CSS class `active` and no other tab has `active`
4. Check console for errors
5. Check network for the fragment request — report HTTP status

**Known issue to recheck**: "30 days" / "7 days" tabs may not gain `active` class after HTMX swap due to template rendering bug. Report exact state after each click.

### Top Performing Assets Tabs
Same HTMX pattern for `/developer/dashboard/fragments/assets?period=<X>`.
Click each period button and verify the assets list updates.

---

## PAGE 2: /developer/submissions
Navigate to: https://platform.poool.app/developer/submissions

**CRITICAL: Hard-refresh first** (navigate to the URL, don't use cached page).

After load, run:
```javascript
escapeAttr.toString()
```
Report the full function body. It MUST contain `.replace(/"/g, "&quot;")`.
If it doesn't, the fix is not deployed — report immediately and skip button tests.

### Stat Filter Cards
Click each status card (All / Draft / Under Review / etc.).
Verify the table row count changes and matches the card number.

### Search
Type a known draft title in the search box. Verify rows filter.
Clear input. Verify all rows return.

### Sort
Click each column header that has a sort arrow. Verify rows reorder.
Click same header again. Verify direction reverses.

### Edit Button (pencil icon)
On a Draft row, click the pencil icon (title="Resume editing").
Expected: navigation to /developer/application-form.

If it fails:
```javascript
document.querySelector('button[title="Resume editing"]').outerHTML
```
Report the full outerHTML. Then:
```javascript
document.querySelector('button[title="Resume editing"]').getAttribute('onclick')
```
Report verbatim.

### Duplicate Button
Click the duplicate icon on a Draft row.
Check network for POST `/api/developer/draft/:id/duplicate`.
Report HTTP status and response body.
Verify a new row appears.

### Delete Button
Click the delete icon on a Draft row.
Verify a confirmation dialog/modal appears.
Cancel it. Verify row is NOT deleted.
Click delete again, confirm.
Check network for DELETE `/api/developer/draft/:id`.
Report HTTP status. Verify row disappears.

### Bulk Operations
Check 2 row checkboxes. Verify bulk action bar appears.
Click "Deselect all". Verify bar hides.

---

## PAGE 3: /developer/add-asset
Navigate to: https://platform.poool.app/developer/add-asset

### Asset Type Cards
Click "Real Estate" card. Verify `selected` class added, selected-indicator SVG visible.
Click "Commercial Property" (coming-soon). Verify nothing happens (pointer-events: none).

### Next Step Button
With Real Estate selected, click "Next Step" button.
Check network for POST `/api/developer/draft`.
Report HTTP status and `{"id": "..."}` in response.
Verify navigation to `/developer/application-form?id=<uuid>`.

---

## PAGE 4: /developer/application-form
Navigate to: https://platform.poool.app/developer/application-form?id=<use the draft ID from page 3 or an existing draft>

### Form State Check
Run:
```javascript
['property-title','property-city','property-country','total-value','token-price',
 'tokens-total','bedrooms','bathrooms','year-built']
.map(id => {
  const el = document.getElementById(id);
  return {id, value: el?.value, placeholder: el?.placeholder, type: el?.type};
})
```
Report. Distinguish `value` (actual data) from `placeholder` (ghost text only).

### Dropdown
Click the Property Type dropdown. Verify options appear. Select one.
Run: `document.getElementById('property-type')?.value` — report the value.

### Number Inputs
Enter `500000` in Total Property Value field.
Verify the field accepts it without clearing.

### Save & Exit
Click "Save & Exit". Check network for PUT `/api/developer/draft/:id`.
Report HTTP status. Verify navigation to `/developer/submissions`.

### Next Step
(Re-navigate to application-form with same draft ID.)
Fill all required fields. Click "Next Step".
Check network for PUT `/api/developer/draft/:id`.
Report HTTP status. Verify navigation to `/developer/document-upload-step3?id=<id>`.

---

## PAGE 5: /developer/document-upload-step3
Navigate to: https://platform.poool.app/developer/document-upload-step3?id=<same draft>

### Upload Zones
Verify 6 document sections are visible. List their labels.

### File Upload
Upload a small PDF (create one with: `echo "%PDF-1.4" > /tmp/test.pdf`).
In one zone, trigger a file upload.
Check network for POST `/api/developer/draft/:id/documents`.
Report HTTP status and response body (truncated to 200 chars).

### Delete Uploaded File
After upload, click the X/delete button on the uploaded file.
Check network for DELETE `/api/developer/draft/:id/documents/:doc_id`.
Report HTTP status. Verify file disappears.

### Next Step Navigation
Click "Next Step". Verify navigation to `/developer/property-content?id=<id>`.

---

## PAGE 6: /developer/property-content
Navigate to: https://platform.poool.app/developer/property-content?id=<same draft>

### Image Upload
Upload a JPEG image via the file input.
Check network for POST `/api/developer/draft/:id/images`.
Report the EXACT HTTP status and response body verbatim.
If HTTP 500: report `{"error":"..."}` message exactly.

### Text Fields
Fill: Short Description, Description, Location Description.
Verify each accepts text without clearing.

### Submit & Tokenize (validation test)
With 0 images uploaded, click "Submit & Tokenize".
Verify client-side validation error appears (no network request fired).
Report the exact error message shown.

### Submit & Tokenize (with image)
If image upload succeeded in previous step:
Fill all required fields and click "Submit & Tokenize".
Check network for POST `/api/developer/draft/:id/submit`.
Report HTTP status.

---

## PAGE 7: /developer/assets
Navigate to: https://platform.poool.app/developer/assets

### Tab Switching
Click "Funded" tab. Verify funded cards show, available cards hidden.
Click "Available" tab. Verify vice versa.
Run after each:
```javascript
Array.from(document.querySelectorAll('.dev-asset-card')).map(c => ({
  id: c.dataset.assetId,
  display: c.style.display,
  status: c.dataset.status
}))
```

### Asset Card Click
Click a card (not a button within it). Verify navigation to `/developer/asset-detail?id=<uuid>`.

---

## PAGE 8: /developer/asset-detail
Navigate to: https://platform.poool.app/developer/asset-detail?id=<a real asset ID from page 7>

### Tab Switching
Click each tab: Overview, Media, Documents, Financials, Milestones, Cap Table, Orders.
For each: verify the correct panel has class `active`, others do not.

### Refresh Button
Click `#btn-refresh`. Check network for GET `/api/developer/assets/:id`.
Report HTTP status.

### Toggle Featured
Click `#toggle-featured`. Check network for any PUT/PATCH.
Report HTTP status and whether the toggle state changed in UI.

### Edit Mode
Click `#btn-edit-mode`. Verify form fields become editable.
Change one text field. Click `#btn-cancel-edit`. Verify the change was discarded.
Click `#btn-edit-mode` again. Change field. Click `#btn-save-changes`.
Check network for PUT `/api/developer/assets/:id`.
Report HTTP status.

---

## PAGE 9: /developer/settings
Navigate to: https://platform.poool.app/developer/settings

### Scroll Spy
Click each nav link in the settings sidebar (Core Profile, Address, Security, etc.).
After each click:
```javascript
document.querySelector('.settings-nav__link.is-active')?.textContent
```
Verify the active link matches the clicked one and the URL hash updates.

### Core Profile Save
Change the display name field. Click "Save Profile".
Check network for POST `/api/settings/profile`. Report HTTP status.

### Address Save
Change a field in the address section. Click "Save Address".
Check network for POST `/api/settings/profile`. Report HTTP status.

### Social Links Save
Change a social link field. Click "Save Social Links".
Check network for POST `/api/settings/social`. Report HTTP status.

### Developer Profile Save
Change a field in Developer Identity. Click "Save Developer Profile".
Check network for POST `/api/settings/developer/profile`. Report HTTP status.

### Developer Links Save
Change a developer link. Click "Save Developer Links".
Check network for POST `/api/settings/developer/links`. Report HTTP status.

### Password Change — Open Modal
Click `#btn-change-password`. Verify `#modal-change-password` gets class `is-open`.

### Password Change — Mismatch Validation
With modal open, fill mismatched passwords:
```javascript
document.getElementById('modal-password-new').value = 'NewPass123!';
document.getElementById('modal-password-confirm').value = 'WrongConfirm!';
document.getElementById('form-change-password').dispatchEvent(new Event('submit', {bubbles:true}));
```
Verify `#modal-password-error` shows "New passwords do not match." and no POST fired.

### Developer Logo Upload
Upload a small PNG to `#settings-dev-logo-input`.
Check network for POST `/api/upload/developer-logo`. Report HTTP status.
Known: returns 400 for accounts without explicit developer role flag — report exact error body.

---

## PAGE 10: /developer/support
Navigate to: https://platform.poool.app/developer/support

### Page Load
Verify no console errors on load. Report page title visible.

### Category Dropdown
Run: `Array.from(document.getElementById('ticket-category').options).map(o => o.value)`
Expect 8 options (general, account, deposits, investments, kyc, technical, billing, other).

### Priority Dropdown + Response Time
Change `#ticket-priority` to "urgent". Verify `#expected-response-time` updates to "< 1 hour".
Change back to "normal". Verify "~4 hours".

### Subject Counter
Type 20 chars in `#ticket-subject`. Verify `#ticket-subject-count` shows "20".

### Message Hint
Type 5 chars in `#ticket-message`. Verify `#ticket-message-hint` shows "15 more characters needed."

### Clear Draft
Click `#clear-ticket-draft-btn`. Verify subject, message fields empty and category resets.

### Submit Validation (empty)
Click `#submit-ticket-btn` with empty fields. Verify browser HTML5 validation fires. No POST sent.

### Submit Valid Ticket
Fill all fields (category, priority, subject ≥1 char, message ≥20 chars). Click submit.
Check network for POST `/api/support/tickets` 200.
Verify GET `/api/support/tickets` refresh fires after submit.
Verify ticket count in "All" tab increments.

### Ticket Tabs
Click "Open" tab — verify active class moves, list shows open tickets.
Click "Resolved" tab — verify active class moves, empty state or resolved list shown.
Click "All" tab — verify returns to full list.

### Ticket Card Expand
Click a `.ticket-card-header`. Verify `detail-<uuid>` panel gets class `open`.
Click again. Verify class removed.

### FAQ Search
Type "KYC" in `#faq-search`. Verify fewer than 12 `.faq-item` elements remain visible.
Clear. Verify all 12 visible.

### FAQ Accordion
Click first `.faq-question`. Verify parent `.faq-item` gets class `open` and answer has `offsetHeight > 0`.
Click same question again. Verify class `open` removed.

---

# PART B — ADMIN WORKFLOWS

---

## PAGE 11: /admin/ (Dashboard)
Navigate to: https://platform.poool.app/admin/

### KPI Cards
Check network for `GET /api/admin/stats/overview` — report HTTP status.
Verify stat cards render non-zero values. Report each label + value.

### Date Range Selector
Find the range `<select>` (likely `#range-selector`).
Change to each option: 7d, 30d, 90d, 1y.
For each: check network for `GET /api/admin/stats/overview?range=<X>` — report HTTP status.

### Activity Feed
Verify recent activity list renders rows (not empty/error state).

### Recent Orders Table
Verify rows exist. Report first order's status badge text.

### Pending Deposits Table
Verify rows exist or empty state shown cleanly.

### System Health Check
Check network for `GET /api/admin/system` — report HTTP status.

---

## PAGE 12: /admin/kyc
Navigate to: https://platform.poool.app/admin/kyc

### Stats Row
Verify stat cards (Pending / In Review / Approved / Rejected) render numbers.

### Tabs (Queue / All Records)
Click "Queue" tab — verify active class updates, table shows queue rows.
Click "All Records" tab — verify table shows all rows.

### Search / Filter
Type a partial email in the search input. Verify rows filter (debounced).
Clear input. Verify all rows return.
Change status filter dropdown. Verify rows filter by that status.

### Sort Headers
Click 2+ sortable column headers. Verify rows reorder each time.
Click the same header again. Verify direction reverses (asc → desc).

### Open Review Modal
On a pending or in_review row, click "Review".
Verify `#kyc-modal` becomes visible. Verify user info and document list render inside.

### View Documents
Inside the modal, click "View Documents" (if visible).
Check network for `GET /api/admin/kyc/:id/documents` — report HTTP status.

### Approve KYC
With review modal open on a pending record, click "Approve".
Check network for `POST /api/admin/kyc/:id/approve` — report HTTP status.
Verify modal closes and row status updates.

### Reject KYC
Open review modal on another pending record. Click "Reject".
Verify a rejection reason field appears. Enter a reason. Confirm.
Check network for `POST /api/admin/kyc/:id/reject` — report HTTP status.

### Pagination
Click `#queue-next-page` / `#all-next-page`. Verify page increments and rows change.
Click prev-page. Verify return.

---

## PAGE 13: /admin/users
Navigate to: https://platform.poool.app/admin/users

### Table Load
Check network for `GET /api/admin/users` — report HTTP status.
Verify table renders rows with name, email, role badge, KYC badge, status.

### Search
Type a partial name or email in `#search-input`. Verify rows filter live.
Clear. Verify all rows return.

### Filter Dropdowns
Change KYC status filter. Verify rows update.
Change role filter. Verify rows update.

### Sort
Click 2+ column headers. Verify rows reorder. Click again — direction reverses.

### Toggle User Status
Find an active user row. Click its status toggle.
Check network for `POST /api/admin/users/:id/status` — report HTTP status and new status value.
Toggle back to restore.

### Export CSV
Click Export CSV button. Verify a download triggers (network request or file download).

### Pagination
Next / prev page — verify page state changes and rows differ.

---

## PAGE 14: /admin/user-details
Navigate to: https://platform.poool.app/admin/user-details

Get a user ID from the users table first:
```javascript
document.querySelector('[data-user-id]')?.dataset.userId ||
document.querySelector('tr[data-id]')?.dataset.id
```
Then navigate to: https://platform.poool.app/admin/user-details?id=<userId>

### Profile Load
Check network for `GET /api/admin/users/:id` — report HTTP status.
Verify name, email, KYC status, role badges all render.

### Edit Profile
Change the display name field. Click Save.
Check network for `PUT /api/admin/users/:id/profile` — report HTTP status.

### Role Management
In the roles section, add or remove a role.
Check network for `PUT /api/admin/users/:id/roles` — report HTTP status.

### Balance Adjustment
Find the balance adjustment input. Enter a small value. Submit.
Check network for `POST /api/admin/users/:id/balance` — report HTTP status.

### Investment Limit
Change the investment limit field. Save.
Check network for `PUT /api/admin/users/:id/investment-limit` — report HTTP status.

### Active Sessions
Verify active sessions list renders. Report session count shown.

### Order Approve / Reject
If the user has a pending order in their order list:
Click Approve — check network for `POST /api/admin/orders/:id/approve`.
Click Reject on another — check network for `POST /api/admin/orders/:id/reject`.
Report HTTP statuses.

---

## PAGE 15: /admin/developer-submissions
Navigate to: https://platform.poool.app/admin/developer-submissions

### Table Load
Check network for `GET /api/admin/developer-projects` — report HTTP status.
Verify table renders rows.

### Search + Filter
Type a project title fragment. Verify rows filter.
Change status dropdown (All / Pending / In Review / Approved / Rejected / Revision). Verify rows filter.

### Sort
Click sortable column headers. Verify reorder. Click again — direction reverses.

### Review Modal
Click Review on a pending row.
Verify modal opens with project details.

### Decision Actions
Inside the modal, test each decision button:
- "In Review" → `POST /api/admin/developer-projects/:id/review` body `{"action":"in_review"}`
- "Request Revision" → enter reason → `POST /api/admin/developer-projects/:id/review` body `{"action":"request_revision"}`
- "Approve" → `POST /api/admin/developer-projects/:id/review` body `{"action":"approve"}`
- "Reject" → enter reason → `POST /api/admin/developer-projects/:id/review` body `{"action":"reject"}`
Report HTTP status for each.

### Pagination
Next / prev — verify rows change.

---

## PAGE 16: /admin/developer-submission-review
Navigate to: https://platform.poool.app/admin/developer-submission-review?id=<a developer project ID>

Get a project ID from the submissions table row's `data-id` or review button `onclick`.

### Page Load
Check network for `GET /api/admin/developer-projects/:id` — report HTTP status.
Verify asset info panel, developer card, checklist, notes, images, documents all render.

### Checklist Persistence
Check 2–3 checklist boxes.
Check network for `PUT /api/admin/developer-projects/:id/checklist` — report HTTP status.
Reload the page. Verify the same boxes remain checked.

### Notes
Type a note in the notes textarea. Wait for autosave (or click save if button exists).
Check network for `POST /api/admin/developer-projects/:id/notes` — report HTTP status.

### Image Management
If images are present:
- Click "Set Cover" on a non-cover image — check network for image update endpoint.
- Click Delete on an image — verify confirm dialog — confirm — check network for `DELETE /api/admin/assets/:id/images/:imgId`.

### Documents
Verify document list renders with download links.
Click a document download link — check network for `GET /api/documents/:id/download`.

### Decision Buttons
Run:
```javascript
['btn-approve','btn-reject','btn-request-revision','btn-in-review','btn-tokenize']
  .map(id => ({id, exists: !!document.getElementById(id)}))
```
Report which exist. Click each present button and report the resulting API call + HTTP status.

---

## PAGE 17: /admin/assets
Navigate to: https://platform.poool.app/admin/assets

### Table Load
Check network for `GET /api/admin/assets` — report HTTP status.
Verify asset rows render with title, status, funding progress.

### Search / Filter
Type a partial title. Verify rows filter.
Change status filter. Verify rows update.

### Toggle Featured
Click the featured star/toggle on an asset row.
Check network for `POST /api/admin/assets/:id/toggle-featured` — report HTTP status.
Verify the toggle state flips in UI.

### Asset Detail Navigation
Click an asset row or detail icon.
Verify navigation to `/admin/asset-details?id=<uuid>`.

---

## PAGE 18: /admin/asset-details
Navigate to: https://platform.poool.app/admin/asset-details?id=<asset ID from page 17>

### Data Load
Check network for `GET /api/admin/assets/:id/detail` — report HTTP status.
Verify overview panel, financials, documents, images, orders, cap table all render.

### Publication Toggle
Click Publish or Unpublish button.
Check network for `PUT /api/admin/assets/:id/publication` — report HTTP status.
Verify button label flips (Publish ↔ Unpublish).

### Featured Toggle
Click the featured toggle.
Check network for `POST /api/admin/assets/:id/toggle-featured` — report HTTP status.

### Funding Status
Check network for `GET /api/admin/assets/:id/funding-status` — report HTTP status.
Verify funding percentage and progress bar render.

---

## PAGE 19: /admin/orders
Navigate to: https://platform.poool.app/admin/orders

### Table Load
Check network for `GET /api/admin/orders` — report HTTP status.
Verify order rows render.

### Filter by Status
Change status filter (All / Pending / Approved / Rejected / Cancelled).
Verify rows filter per status.

### Sort
Click amount, date, and status headers. Verify rows reorder.

### Order Detail
Click a row or detail icon.
Check network for `GET /api/admin/orders/:id` — report HTTP status.
Verify order detail renders (user, asset, amount, status, timestamps).

### Approve Order
On a pending order detail, click Approve.
Check network for `POST /api/admin/orders/:id/approve` — report HTTP status.
Verify status updates in UI.

### Reject Order
On another pending order, click Reject.
Check network for `POST /api/admin/orders/:id/reject` — report HTTP status.

### Investments Tab
Click Investments tab.
Check network for `GET /api/admin/investments` — report HTTP status.
Verify investment rows render.

---

## PAGE 20: /admin/deposits
Navigate to: https://platform.poool.app/admin/deposits

### Table Load
Check network for `GET /api/admin/deposits` — report HTTP status.
Verify deposit rows render.

### Tabs
Click Disputes tab — check network for `GET /api/admin/disputes/` — report HTTP status.
Click back to Deposits.

### Filter
Change status filter (All / Pending / Confirmed / Cancelled). Verify rows update each time.

### Confirm Deposit
Open a pending deposit. Verify confirmation modal opens.
Click Confirm — check network for `POST /api/admin/deposits/:id/confirm` — report HTTP status.

### Cancel Deposit
Open another pending deposit. Click Cancel.
Check network for `POST /api/admin/deposits/:id/cancel` — report HTTP status.

### Extend Deposit
Find Extend option on a pending deposit.
Check network for `POST /api/admin/deposits/:id/extend` — report HTTP status.

### Dispute Detail
Click a dispute row.
Check network for `GET /api/admin/disputes/:id/evidence` — report HTTP status.
Verify evidence items render.
Change the dispute status.
Check network for `PUT /api/admin/disputes/:id/status` — report HTTP status.

---

## PAGE 21: /admin/approvals
Navigate to: https://platform.poool.app/admin/approvals

### Table Load
Check network for `GET /api/admin/approvals` — report HTTP status.
Verify approval rows render.

### Approve
Click Approve on a pending row.
Check network for `POST /api/admin/approvals/:id/approve` — report HTTP status.

### Reject
Click Reject on another pending row.
Check network for `POST /api/admin/approvals/:id/reject` — report HTTP status.

---

## PAGE 22: /admin/dividends
Navigate to: https://platform.poool.app/admin/dividends

### Load
Check network for `GET /api/admin/dividends/distributions` — report HTTP status.
Verify distribution rows render.

### Calculate
Select an asset from the asset dropdown. Click Calculate.
Check network for `POST /api/admin/dividends/calculate` — report HTTP status.
Verify calculated amount renders.

### Process
Click Process on a calculated result.
Check network for `POST /api/admin/dividends/process` — report HTTP status.

### Approve Distribution
Click Approve on a pending distribution.
Check network for `POST /api/admin/dividends/distributions/:id/approve` — report HTTP status.

### Execute Distribution
Click Execute on an approved distribution.
Check network for `POST /api/admin/dividends/distributions/:id/execute` — report HTTP status.

### Cancel Distribution
Click Cancel on a distribution.
Check network for `POST /api/admin/dividends/distributions/:id/cancel` — report HTTP status.

---

## PAGE 23: /admin/rewards
Navigate to: https://platform.poool.app/admin/rewards

### Load
Check network for `GET /api/admin/rewards` — report HTTP status.
Verify reward tiers table renders.

### Edit Tier
Click a tier row to edit. Change a threshold or benefit value. Save.
Check network for `PUT /api/admin/rewards/tiers/:name` — report HTTP status.

### Balance Adjustment
Find a user in the rewards list. Click Adjust Balance.
Enter a small amount. Confirm.
Check network for `POST /api/admin/rewards/balances/:userId/adjust` — report HTTP status.

### Affiliate Payouts
Check network for `GET /api/admin/rewards/affiliates/payouts/pending` — report HTTP status.
Verify pending payout rows render.
Click Approve on one — check network for `POST /api/admin/affiliates/:userId/approve`.

---

## PAGE 24: /admin/notifications
Navigate to: https://platform.poool.app/admin/notifications

### Load
Check network for `GET /api/admin/notifications` — report HTTP status.
Verify notification list renders.

### Broadcast
Fill title, message, and target audience in the broadcast form. Click Send.
Check network for `POST /api/admin/notifications/broadcast` — report HTTP status.
Verify success state shown.

---

## PAGE 25: /admin/support
Navigate to: https://platform.poool.app/admin/support

### Load
Check network for `GET /api/admin/support` — report HTTP status.
Verify ticket list renders.

### Filter / Search
Change status filter (All / Open / Resolved). Verify rows filter.
Type in search. Verify rows filter by subject or user.

### Bulk Actions
Select 2+ ticket checkboxes. Verify bulk action bar appears.
Trigger a bulk action. Check network for `POST /api/admin/support/bulk` — report HTTP status.

### Open Ticket Detail
Click a ticket row. Verify navigation to `/admin/support-ticket?id=<uuid>`.

---

## PAGE 26: /admin/support-ticket
Navigate to: https://platform.poool.app/admin/support-ticket?id=<ticket ID from page 25>

### Load
Verify ticket subject, category, priority, and user info all render.

### Reply
Type a reply in the reply textarea. Click Send.
Check network for the reply endpoint — report URL + HTTP status.
Verify reply appears in the thread.

### Change Status
Find the status dropdown. Change to Resolved.
Check network for the status update endpoint — report URL + HTTP status.
Verify status badge updates.

---

## PAGE 27: /admin/reports
Navigate to: https://platform.poool.app/admin/reports

### Generate Each Report
Select and generate each report type. Check network for each endpoint — report HTTP status:
- Financial Summary → `GET /api/admin/reports/financial-summary`
- User Growth → `GET /api/admin/reports/user-growth`
- Investment Summary → `GET /api/admin/reports/investment-summary`
- Order Summary → `GET /api/admin/reports/order-summary`
- KYC Status → `GET /api/admin/reports/kyc-status`
- AML Compliance → `GET /api/admin/reports/aml-compliance`
- Asset Performance → `GET /api/admin/reports/asset-performance`
- Tax P&L → `GET /api/admin/reports/tax-pl`
- Tax Withholding → `GET /api/admin/reports/tax-withholding`
- Wallet Transactions → `GET /api/admin/reports/wallet-transactions`
- Rewards Liability → `GET /api/admin/reports/rewards-liability`
- Referral Effectiveness → `GET /api/admin/reports/referral-effectiveness`
- Support Summary → `GET /api/admin/reports/support-summary`

For each: verify data table or chart renders (not empty/error state).

### Export
Click Export CSV or Export PDF if present. Verify download fires.

---

## PAGE 28: /admin/audit-logs
Navigate to: https://platform.poool.app/admin/audit-logs

### Load
Check network for `GET /api/admin/audit-logs` — report HTTP status.
Verify log rows render with timestamp, action, user, IP.

### Filter
Change action type filter. Change date range. Verify rows update.

### Search
Type a user email. Verify rows filter.

### Pagination
Next / prev page — verify rows change.

---

## PAGE 29: /admin/blockchain-contracts
Navigate to: https://platform.poool.app/admin/blockchain-contracts

### Load
Check network for `GET /api/admin/blockchain/treasury` — report HTTP status.
Verify contract list renders.

### Contract Detail
Click a contract row. Verify navigation to `/admin/blockchain-contract-detail?address=<address>`.
Check network for `GET /api/admin/blockchain/contracts/:address/detail` — report HTTP status.
Verify contract metadata, state, and token info render.

### Pause / Unpause
Click Pause — check network for `POST /api/admin/blockchain/contracts/:address/pause` — report HTTP status.
Click Unpause — check network for `POST /api/admin/blockchain/contracts/:address/unpause` — report HTTP status.
Verify contract state label updates after each.

---

## PAGE 30: /admin/blockchain-sync
Navigate to: https://platform.poool.app/admin/blockchain-sync

### Load
Check network for `GET /api/admin/blockchain/sync` — report HTTP status.
Verify sync status panel and recent sync log render.

### Force KYC Sync
Find the user ID input. Enter a test user ID. Click Force Sync.
Check network for `POST /api/admin/blockchain/force-kyc-sync/:userId` — report HTTP status.

---

## PAGE 31: /admin/asset-change-requests
Navigate to: https://platform.poool.app/admin/asset-change-requests

### Load
Check network for `GET /api/admin/change-requests` — report HTTP status.
Verify change request rows render.

### View Diff
Click a row. Check network for `GET /api/admin/change-requests/:id` — report HTTP status.
Verify diff view renders showing old vs. new field values.

### Approve
Click Approve. Check network for `POST /api/admin/change-requests/:id/approve` — report HTTP status.

### Reject
On another row, click Reject.
Check network for `POST /api/admin/change-requests/:id/reject` — report HTTP status.

---

## PAGE 32: /admin/email-marketing
Navigate to: https://platform.poool.app/admin/email-marketing

### Load
Check network for `GET /api/admin/emails/templates` and `GET /api/admin/emails/campaigns` — report HTTP statuses.

### Template Edit
Select a template. Click Edit. Modify content in the editor. Click Save.
Check network for `PUT /api/admin/emails/templates/:id` — report HTTP status.

### Campaign List
Switch to Campaigns tab. Verify campaign rows render with status badges.

---

## PAGE 33: /admin/dividends (pending settlements)
Navigate to: https://platform.poool.app/admin/pending-settlements

### Load
Verify pending settlement rows render.
Report any API calls made on load and their HTTP statuses.

---

## PAGE 34: /admin/roles
Navigate to: https://platform.poool.app/admin/roles

### Load
Check network for `GET /api/admin/roles` and `GET /api/admin/roles/permissions` — report HTTP statuses.
Verify role list and permission matrix render.

### Permission Toggle
Toggle a permission checkbox.
Check network for any PUT/POST to `/api/admin/roles/*` — report HTTP status.

---

## PAGE 35: /admin/system
Navigate to: https://platform.poool.app/admin/system

### Load
Check network for `GET /api/admin/system` — report HTTP status.
Verify system health metrics render (DB status, queue depths, service statuses).

### Settings Save
Navigate to `/admin/settings`.
Change one platform config value. Click Save.
Check network for any `POST /api/admin/settings/*` — report URL + HTTP status.

---

## FINAL OUTPUT

Print:

### BUG LOG
```
BUG-01: [page] — [element] — [symptom]
BUG-02: ...
```

### SUMMARY TABLE
| Page | Elements Tested | PASS | FAIL | SKIP |
|------|----------------|------|------|------|
| /developer/dashboard | N | N | N | N |
| ... | | | | |
| **TOTAL** | **N** | **N** | **N** | **N** |

### API HEALTH
List every endpoint that returned a non-200 status — include exact status code and error body (truncated to 200 chars).

### DEPLOYMENT STATUS
- `escapeAttr` function in production: [paste toString result]
- Submissions fix deployed: YES / NO
