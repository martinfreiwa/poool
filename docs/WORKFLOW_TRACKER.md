# Poool Workflow Tracker

Tracks end-to-end workflows for investors, developers, and admins. Each workflow section includes implementation status, last test date, open bugs, and severity.

**Last full audit:** 2026-04-28 (source: `page-review-tracker.yml` — 93 open issues: 38 high, 45 medium, 10 low)

---

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Implemented and working |
| 🚧 | Partially implemented / incomplete |
| ❌ | Not implemented |
| 🔒 | Auth-gated (correct behaviour) |
| ⚠️ | Implemented but has open bugs |
| 🔁 | Fixed, needs runtime recheck |

**Bug severity:** 🔴 High · 🟡 Medium · 🔵 Low

---

## 1. Investor Workflow

---

### 1.1 Registration & Email Verification

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Sign up (email + password) | Investor | `POST /auth/register` | ✅ | |
| 2 | Receive verification email | System | Resend → email link | ⚠️ | No retry on delivery failure |
| 3 | Click verify link → `email_verified = true` | Investor | `GET /auth/verify?token=` | ✅ | |
| 4 | Redirect to marketplace / onboarding | System | `303 → /marketplace` | ✅ | |

#### Open Bugs — Registration

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0460](docs/page-audits/) | 🟡 | Verification email delivery lacks outbox retry worker | open | 2026-04-25 |

---

### 1.2 KYC Submission

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Complete profile (name, address, DOB) | Investor | `/settings/profile` | ✅ | |
| 2 | Upload KYC documents | Investor | `/kyc` → `POST /api/kyc/submit` | ⚠️ | Audit + email side effects can be swallowed |
| 3 | Wait for admin review | Investor | — | ✅ | |
| 4 | Receive approval/rejection email | System | Resend | 🚧 | Rejection email not reliably sent; no outbox |
| 5 | Resubmit after rejection | Investor | `/kyc` | 🚧 | Resubmit flow incomplete |

#### Open Bugs — KYC Submission

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0543](docs/page-audits/) | 🟡 | KYC mutation audit and email side effects are swallowed | partially fixed | 2026-04-25 |
| [PAGE-ISSUE-0550](docs/page-audits/) | 🟡 | KYC email delivery still lacks durable outbox | open | 2026-04-25 |
| [PAGE-ISSUE-0566](docs/page-audits/) | 🔵 | KYC upload can orphan private GCS object after DB failure | open | 2026-04-25 |

---

### 1.3 Deposit & Wallet Funding

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass (full flow tested in browser)

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Open deposit modal | Investor | `/wallet` → "Deposit" button | ✅ | Button correctly opens modal |
| 2 | Enter amount and submit | Investor | `POST /wallet/deposit` | ✅ | Redirects to `/wallet?deposit_created=true&ref=...` with wire instructions |
| 3 | Admin confirms deposit | Admin | `/admin/deposits` → `POST /api/admin/deposits/:id/confirm` | ✅ | 200 OK; atomic TX credited balance |
| 4 | Wallet balance credited | System | `wallets.balance_cents` incremented | ✅ | Balance reflects immediately on reload |
| 5 | Investor notified | System | Resend email | 🚧 | Notification missing after confirmation |

#### Open Bugs — Deposit

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | No deposit confirmation email sent to investor after admin approves | open | — |
| — | 🟡 | No payment proof file upload; investors can only enter text reference | open | — |
| WALLET-FIX-001 | 🟡 | Deposit modal showed misleading "Payment Method" dropdown (backend ignores it) | **fixed 2026-04-30** | 2026-04-30 |

---

### 1.4 Browsing & Investing (Cart + Checkout)

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass (full flow verified end-to-end)

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Browse marketplace | Investor | `/marketplace` | ✅ | Loads; filters present |
| 2 | View asset detail | Investor | `/property/:slug` | ✅ | Share price, quick-add buttons, "Add to cart" all present |
| 3 | Add to cart | Investor | `POST /cart` (form) | ✅ | Redirects to `/cart`; validation now blocks sub-share-price amounts with error message |
| 4 | View cart — qty controls, T&C | Investor | `/cart` | ✅ | +/− work, subtotal updates; order summary line item now updates on qty change |
| 5 | Checkout — bank wire details, proof upload | Investor | `/checkout` | ✅ | USD+IDR toggle, copy buttons, reference ID, validation all work |
| 6 | Confirm payment — order created | Investor | `POST /checkout` → `/payment-in-progress` | ✅ | Order ORD-20260430154702-47bfe9 created; portfolio shows $5,000 |
| 7 | Confirmation email | System | Resend | 🚧 | Not verifiable in local env |

#### Open Bugs — Cart & Checkout

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0387](docs/page-audits/) | 🔴 | Cart HTML uses incomplete manual escaping for asset data (XSS risk) | open | 2026-04-25 |
| [PAGE-ISSUE-0388](docs/page-audits/) | 🔴 | Cart quantity update fails open when availability lock cannot be read | open | 2026-04-25 |
| [PAGE-ISSUE-0389](docs/page-audits/) | 🟡 | Cart item controls lack robust accessible labels | open | 2026-04-25 |
| CART-BUG-001 | 🟡 | Amount below share price silently rounds up to 1 share — no user feedback | **fixed 2026-04-30** | 2026-04-30 |
| CART-BUG-002 | 🟡 | Order Summary line item label stays "1 × $X" after quantity increase | **fixed 2026-04-30** | 2026-04-30 |
| CHECKOUT-BUG-001 | 🟡 | Error message renders as "Payment FailedOrder exceeds..." — missing separator | **fixed 2026-04-30** | 2026-04-30 |
| CHECKOUT-BUG-002 | 🔵 | Proof of transfer is frontend-required but backend-optional — bypassable | open | 2026-04-30 |
| CHECKOUT-BUG-003 | 🔵 | IDR bank account number `0987654321` appears to be placeholder test data | open | 2026-04-30 |
| — | 🟡 | No investment confirmation email after checkout | open | — |

---

### 1.4a Property Amount Quick-Add

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | View property detail | Navigate to `/property/:slug` | — | ✅ | Amount input pre-filled with one share price |
| 2 | Click +USD 500 / +USD 2000 / +USD 5000 | Property detail sidebar | Client-side JS | ✅ | Adds to current input value |
| 3 | Add to cart with custom amount | Click "Add to cart" | `POST /cart` | ✅ | Frontend now validates amount ≥ share price before submitting (CART-BUG-001 fixed) |

---

### 1.4b Cart Reservation Timer

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass (timer observed; expiry not tested)

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Add item to cart | `POST /cart` | — | ✅ | Countdown timer appears: "Reserved for MM:SS" |
| 2 | Timer counts down in cart | Stay on `/cart` | Client-side | ✅ | Timer visible in order summary |
| 3 | Timer counts down in checkout | Navigate to `/checkout` | — | ✅ | Timer resets to ~15 min on checkout page |
| 4 | Timer expires — cart cleared | Wait for expiry | `DELETE /cart/:item` or server-side | 🚧 | Expiry behaviour not tested |

---

### 1.4c Cart T&C + KFS Acknowledgment

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | View cart with item | `/cart` | — | ✅ | Two checkboxes visible |
| 2 | Click "Proceed to Checkout" without T&C | Click button | Client-side | ✅ | First checkbox highlighted red |
| 3 | Check T&C only, proceed | Click button | Client-side | ✅ | Button becomes active but second checkbox still required |
| 4 | Check both → proceed | Click "Proceed to Checkout" | `GET /checkout` | ✅ | Navigates to checkout |

---

### 1.4d Checkout IDR Currency Toggle

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | View checkout | `/checkout` | — | ✅ | USD selected by default; Deutsche Bank details shown |
| 2 | Click "IDR" toggle | Client-side | — | ✅ | Switches to BCA Indonesia details |
| 3 | Click "USD" toggle | Client-side | — | ✅ | Switches back to Deutsche Bank |
| 4 | Submit in either currency | `POST /checkout` | `payment_currency` field sent | ✅ | Backend records currency from form |

---

### 1.4e Payment In Progress Confirmation

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Submit checkout | `POST /checkout` | — | ✅ | Redirects to `/payment-in-progress` |
| 2 | View order details | `/payment-in-progress` | — | ✅ | Order number, amount, payment method, status shown |
| 3 | View 3-step progress tracker | Page | — | ✅ | Order Placed ✅ → Awaiting Payment ⏳ → Payment Activated ○ |
| 4 | Click "View Portfolio" | Button | `/portfolio` | ✅ | Portfolio shows investment |
| 5 | Click "Continue Shopping" | Button | `/marketplace` | 🚧 | Not tested |

---

### 1.4f Annual Investment Limit Gate

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** partial (limit enforced; UI for seeing own limit not found)

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | User has annual limit set by admin | Admin sets via `POST /api/admin/users/:id/investment-limit` | — | ✅ | Limit stored in `investment_limits` table |
| 2 | User attempts checkout exceeding limit | `POST /checkout` | — | ✅ | Returns inline error with available amount |
| 3 | Error message shown on checkout | Page | — | ✅ | Server HTML error box now rendered directly — heading and body display correctly (CHECKOUT-BUG-001 fixed) |
| 4 | User can see their own remaining limit | Settings or wallet | — | 🚧 | No visible UI for remaining investment capacity |
| 5 | Admin removes limit (sets to 0) | `POST /api/admin/users/:id/investment-limit` | — | ✅ | Checkout proceeds normally |

---

### 1.5 Portfolio & Dividends

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | View portfolio overview | Investor | `/portfolio` | ✅ | Chart, key financials, My Assets table all render |
| 2 | View per-investment detail & milestones | Investor | `/portfolio/:investment_id` | ❌ | Route does not exist — "See Details" navigates to public `/property/:slug` with no investment context or milestone section |
| 3 | Receive dividend payout | System | Admin-triggered → `dividend_payouts` | 🚧 | Backend routes exist (`POST /api/admin/dividends/calculate` + `/process`); UI not tested; "Rent Paid" tx type confirmed visible in wallet |
| 4 | View payout history | Investor | `/wallet` (transactions tab) | ✅ | Dividend payouts appear as "Rent Paid" rows in Transactions table |
| 5 | Request withdrawal | Investor | `/wallet` → `POST /wallet/withdraw` | ⚠️ | Modal opens when payment method on file; withdrawals ≥$100 require 2FA step-up; error codes were not mapped (fixed 2026-04-30) |

#### Open Bugs — Portfolio & Dividends

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0527](docs/page-audits/) | 🟡 | Dividend lifecycle E2E coverage is stale | fixed, needs recheck | 2026-04-25 |
| — | 🟡 | Withdraw fails silently with generic error if no payment method on file | **fixed 2026-04-30** — button now disabled, descriptive toast added | 2026-04-30 |
| — | 🟡 | Per-investment detail/milestone page incomplete | open | — |
| 1.5-BUG-001 | ❌ | `/portfolio/:investment_id` route missing — "See Details" goes to public property page, no investment context | open | 2026-04-30 |
| 1.5-BUG-002 | 🟡 | Wallet "View details" buttons non-functional — JS-rendered rows use class `wallet-transaction-action-btn` but click handler only matches `.ds-btn.ds-btn--ghost`; all clicks silently no-op | **fixed 2026-04-30** | 2026-04-30 |
| 1.5-BUG-003 | 🟡 | `2fa_required`, `withdrawal_cooldown`, `daily_limit_exceeded` error codes missing from both wallet error maps → generic "An error occurred" toast gives investor no actionable guidance | **fixed 2026-04-30** | 2026-04-30 |
| 1.5-BUG-004 | 🔵 | Duplicate error handler scripts in wallet.html (inline) and wallet.js with differing error maps — creates maintenance risk | open | 2026-04-30 |

### 1.5a Withdrawal 2FA Step-up

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** partial pass (redirect confirmed; 2FA UI flow not tested)

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Submit withdrawal ≥ $100 without 2FA active | Investor | `POST /wallet/withdraw` | ✅ | Backend redirects to `/wallet?error=2fa_required` |
| 2 | Error message shown | Investor | `/wallet` | ✅ | Now shows "Two-factor authentication required…" (fixed 2026-04-30) |
| 3 | Investor enables 2FA | Investor | `/settings` → 2FA section | 🚧 | Not tested in this run |
| 4 | Retry withdrawal after 2FA setup | Investor | `POST /wallet/withdraw` | 🚧 | Not tested |

---

## 2. Developer Workflow

---

### 2.1 Onboarding & Role Assignment

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Register account | Developer | `POST /auth/signup` | ✅ | Route is `/auth/signup` not `/auth/register` (404). Email verified via DB in local env. |
| 2 | Request developer role | Developer | `/developer` → redirect to `/developer/application-form` | ⚠️ | Self-service path exists: non-developer navigates to `/developer` → redirected to application form → on first draft submit backend auto-assigns developer role. No explicit "request role" UI. |
| 3 | Admin assigns developer role | Admin | `POST /api/admin/users/:id/roles` | ✅ | Was broken (`developer` excluded from `ASSIGNABLE_ROLES`). Fixed 2026-04-30 — super_admin can now assign developer role via API. Requires `{"roles":["developer"]}` payload; CSRF token required. |
| 4 | Access developer dashboard | Developer | `/developer/dashboard` | ✅ | Dashboard loads with stats cards, chart, activity snapshot. All zeroed for new account as expected. |
| 5 | Dashboard chart/assets fragments auth-gated | System | `/developer/dashboard/fragments/*` | ✅ | chart: 401 (unauth), 303→/marketplace (investor), 200 (developer). assets: same. Bugs PAGE-ISSUE-0481/0482 fixed. |

#### Open Bugs — Developer Onboarding

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0481](docs/page-audits/) | 🟡 | Chart fragment returns HTTP 200 for unauthenticated requests | **fixed 2026-04-30** | 2026-04-25 |
| [PAGE-ISSUE-0482](docs/page-audits/) | 🟡 | Assets fragment returns HTTP 200 for unauthenticated requests | **fixed 2026-04-30** | 2026-04-25 |
| 2.1-BUG-001 | 🔴 | `developer` role not in `ASSIGNABLE_ROLES` — admin API cannot assign developer role | **fixed 2026-04-30** | 2026-04-30 |
| 2.1-BUG-002 | 🟡 | Developer Identity & Developer Links settings visible to non-developer users | **not reproducible** — HTML sends elements with `hidden`; JS gates on `is_developer` from API; false positive from testing with wrong user session | 2026-04-30 |
| 2.1-BUG-003 | 🟡 | Step 1 tracker lists wrong route `/auth/register` (404); correct route is `/auth/signup` | **fixed 2026-04-30** | 2026-04-30 |

#### Discovered Workflow — 2.1a Self-Service Developer Onboarding

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Navigate to `/developer` as investor | Developer | `/developer` → 302 → `/developer/application-form` | ✅ | `require_developer_page` redirects non-developers to application form |
| 2 | View application form | Developer | `/developer/application-form` | ✅ | Accessible via `serve_protected` (login check only, no role check). Full developer sidebar rendered. |
| 3 | Submit first draft | Developer | `POST /api/developer/draft` | ✅ | Backend auto-assigns developer role on first submission if user lacks it |
| 4 | Access developer dashboard | Developer | `/developer/dashboard` | ✅ | Full access granted after role auto-assign |

---

### 2.2 Asset Creation & Submission

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

> **Wizard step order (actual):** add-asset → application-form → document-upload-step3 → property-content → submission-success.
> Tracker previously listed media (step 2) before documents (step 3) — corrected below.
> Financials are embedded on the property-content page, not a separate step.
> Milestones are added post-submission from the asset detail page (not part of the creation wizard).

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Choose asset type | Developer | `/developer/add-asset` | ✅ | Loads ✅; Real Estate pre-selected; 5 Coming Soon cards greyed. Cards keyboard-accessible via `role="radio"` + `aria-checked`/`aria-disabled` (PAGE-ISSUE-0390 fixed 2026-04-30) |
| 2 | Fill property details + financials | Developer | `/developer/application-form` → `POST /api/developer/draft` | ✅ | All PooolDropdown + text inputs accept values; draft saved; redirects to doc-upload |
| 3 | Upload documents (optional) | Developer | `/developer/document-upload-step3` → `POST /api/developer/draft/:id/documents` | ✅ | 6 sections render correctly; file inputs have `aria-label` (PAGE-ISSUE-0483 fixed 2026-04-30); docs are optional — Next Step works without uploads |
| 4 | Add media, descriptions & financials | Developer | `/developer/property-content` → `POST /api/developer/draft/:id/images` | ✅ | Image upload API 200 ✅; image count + remove/set-cover UI works ✅; subtitle validation inconsistency fixed (2.2-BUG-003); `.gif` confirmed accepted by `validate_asset_image_mime` (PAGE-ISSUE-0485 closed — not a bug) |
| 5 | Submit for review | Developer | `POST /api/developer/draft/:id/submit` | ✅ | `developer_projects.status = submitted` confirmed in DB; backend enforces minimum 1 image |
| 6 | Submission success page | Developer | `/developer/submission-success` | ✅ | Renders correctly; Telegram/WhatsApp/Email links real (PAGE-ISSUE-0486 fixed); heading redundancy fixed (2.2-BUG-004 fixed) |

#### Discovered Sub-Workflows

| ID | Title | Description |
|----|-------|-------------|
| 2.2a | Skip-documents path | Developer can proceed through doc-upload step without any file uploads; docs are advisory only |
| 2.2b | Image management on property-content | Uploaded images show "Set Cover" + red-X remove buttons; cover border highlights in green |

#### Open Bugs — Asset Submission

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0390](docs/page-audits/) | 🟡 | Asset type cards have no `tabindex`/`role` — keyboard inaccessible | **fixed 2026-04-30** — all 5 coming-soon cards now have `role="radio" aria-checked="false" aria-disabled="true"` | 2026-04-25 |
| [PAGE-ISSUE-0483](docs/page-audits/) | 🟡 | Document upload file inputs lack `aria-label`/`aria-labelledby` | **fixed 2026-04-30** — all 6 file inputs given descriptive `aria-label` | 2026-04-25 |
| [PAGE-ISSUE-0485](docs/page-audits/) | 🟡 | Media upload accept attr includes `.gif`; backend acceptance unverified | **closed — not a bug** — `validate_asset_image_mime` in `storage/service.rs:355` explicitly accepts `image/gif` | 2026-04-25 |
| [PAGE-ISSUE-0488](docs/page-audits/) | 🟡 | Property image remove buttons have no accessible names | **closed — already fixed** — `setAttribute("aria-label", "Remove image")` at `developer-property-content.js:258` | 2026-04-25 |
| [PAGE-ISSUE-0487](docs/page-audits/) | 🟡 | Developer logo upload UI advertises SVG files that the backend rejects | open | 2026-04-25 |
| 2.2-BUG-003 | 🟡 | Photo subtitle said "8-16 required" but backend minimum is 1 | **fixed 2026-04-30** — JS + HTML updated; 0=red, 1-7=orange warning, 8-16=green, >16=red | 2026-04-30 |
| 2.2-BUG-004 | 🔵 | Success page nav title "Submission Submitted" redundant with card "Submission successful!" | **fixed 2026-04-30** — nav title changed to "Submission Successful" | 2026-04-30 |
| [PAGE-ISSUE-0484](docs/page-audits/) | 🟡 | Document upload hardcoded demo rows | **fixed** (not reproduced 2026-04-30) | 2026-04-25 |
| [PAGE-ISSUE-0486](docs/page-audits/) | 🔵 | Submission success WhatsApp/Telegram links were placeholders | **fixed** (real contact links confirmed 2026-04-30) | 2026-04-25 |

---

### 2.3 Admin Review Cycle

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Admin reviews submitted asset | Admin | `/admin/assets` | ✅ | |
| 2 | Admin requests changes (with notes) | Admin | Asset edit panel | 🚧 | No formal change-request flow; informal notes only |
| 3 | Developer revises and resubmits | Developer | Asset edit → resubmit | 🚧 | No revision/resubmit state machine |
| 4 | Admin publishes asset | Admin | `PUT /api/admin/assets/:id/status` → `published` | ✅ | |
| 5 | Asset live on marketplace | System | `/marketplace` | ✅ | |

#### Open Bugs — Review Cycle

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0391](docs/page-audits/) | 🔴 | Asset detail destructive/publish controls are placeholders that imply success | open | 2026-04-25 |
| — | 🟡 | No formal change-request / revision cycle between admin and developer | open | — |

---

### 2.4 Post-Launch Management

**Last tested:** not tested · **Environment:** — · **Result:** —

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Update milestones | Developer | `/developer/assets/:id/milestones` | ✅ | |
| 2 | Upload new documents | Developer | `/developer/assets/:id/documents` | ✅ | |
| 3 | View investor count & funding % | Developer | `/developer/asset-detail` | ⚠️ | Cap table shows admin links (wrong audience) |
| 4 | Request dividend distribution | Developer | Submit request → admin approves | 🚧 | No request flow; admin initiates only |
| 5 | View fee structure for asset | Developer | Developer dashboard → fees | 🚧 | Not visible in developer dashboard |

#### Open Bugs — Post-Launch

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0392](docs/page-audits/) | 🟡 | Developer cap table renders admin user-detail links (wrong audience) | open | 2026-04-25 |
| — | 🟡 | Developer cannot see their own fee structure | open | — |
| — | 🟡 | No developer-initiated dividend request flow | open | — |

---

## 3. Admin Workflow

---

### 3.1 User Management

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | View all users | Admin | `/admin/users` | ⚠️ | Clean URL returns 404; PII exposed without permission gate |
| 2 | Filter by role | Admin | `#filter-role` (All / Investor / Developer / Admin) | ✅ | |
| 3 | View user detail | Admin | `/admin/users/:id` | ✅ | |
| 4 | Edit roles | Admin | `POST /api/admin/users/:id/roles` | ✅ | |
| 5 | Suspend / reactivate user | Admin | `PUT /api/admin/users/:id/status` | ✅ | |
| 6 | Invalidate all sessions | Admin | `DELETE /api/admin/users/:id/sessions` | ✅ | |
| 7 | Set investment limits | Admin | `POST /api/admin/users/:id/investment-limit` | ✅ | |

#### Open Bugs — User Management

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0291](docs/page-audits/) | 🔴 | User directory exposes PII and status mutation without granular user permissions | open | 2026-04-25 |
| [PAGE-ISSUE-0290](docs/page-audits/) | 🟡 | Clean URL `/admin/users` returns 404 | open | 2026-04-25 |
| [PAGE-ISSUE-0023](docs/page-audits/) | 🟡 | Admin directory staff PII and security posture reads are not audit logged | open | 2026-04-25 |

---

### 3.2 KYC Review

**Last tested:** 2026-04-30 · **Environment:** localhost:8888 · **Result:** pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | View pending KYC submissions | Admin | `/admin/kyc` | ✅ | Loads; all tabs render; `kyc.view` permission gate enforced; all 747 records shown (LIMIT 200 removed, verified 2026-04-30) |
| 2 | Review documents via signed GCS URL | Admin | `GET /api/admin/kyc/:id/documents` | ✅ | Viewer opens; "📁 View Documents" button in modal (only when `has_documents=true`); audit log failure logged via `tracing::error!`; empty-URL shows "not available" |
| 3 | Approve KYC | Admin | `POST /api/admin/kyc/:id/approve` | ✅ | DB transaction + affiliate referral update + audit log all atomic; email fire-and-forget; counter updates live |
| 4 | Reject KYC with reason | Admin | `POST /api/admin/kyc/:id/reject` | ✅ | Backend validates rejection_reason (400 if missing); DB transaction + audit log atomic; email fire-and-forget |
| 5 | Notify user of decision | System | Resend email | 🚧 | Not tested end-to-end (RESEND_API_KEY not configured in dev); DB failure now returns 500 not silent empty 200 |

#### Open Bugs — KYC Review

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0160](docs/page-audits/) | 🔴 | KYC routes lack KYC-specific permission gates | **closed — fixed** — all 4 routes now use `require_permission("kyc.view"/"kyc.write")` | 2026-04-25 |
| [PAGE-ISSUE-0161](docs/page-audits/) | 🔴 | KYC document signed URLs are overbroad and best-effort audited | **fixed 2026-04-30** — audit log failure now logged via `tracing::error!`; 1h signed-URL TTL preserved | 2026-04-25 |
| [PAGE-ISSUE-0162](docs/page-audits/) | 🔴 | KYC decisions lack audit logs and atomic side effects | **fixed 2026-04-30** — both approve and reject use DB transactions; audit INSERT inside `&mut *tx` before commit | 2026-04-25 |
| [PAGE-ISSUE-0163](docs/page-audits/) | 🟡 | KYC backend failures render as empty states | **fixed 2026-04-30** — `map_err(ApiError::Database)?` propagates DB errors as HTTP 500; LIMIT 200 removed | 2026-04-25 |
| [PAGE-ISSUE-0164](docs/page-audits/) | 🟡 | Document viewer injects signed URL data with innerHTML | **closed — fixed** — viewer uses `createElement`/`img.src`; `esc()` helper used throughout | 2026-04-25 |
| [PAGE-ISSUE-0165](docs/page-audits/) | 🟡 | KYC modals and sort controls lack keyboard semantics | **closed — fixed** — sort `th` has `tabindex`/`role`/keydown; modal has `aria-modal`/`role="dialog"`/focus management | 2026-04-25 |
| [PAGE-ISSUE-0166](docs/page-audits/) | 🟡 | Rejection reason validation is client-side only | **closed — fixed** — backend returns 400 if `rejection_reason` empty or missing | 2026-04-25 |
| PAGE-ISSUE-0589 | 🟡 | PEP/expiring/all tabs show duplicate status dropdown (native + PooolDropdown wrapper) | **fixed 2026-04-30** — `applyFilters()` now hides/shows PooolDropdown wrapper container, not raw select | 2026-04-30 |
| PAGE-ISSUE-0590 | 🔵 | Document viewer shows broken image when GCS signed URL is empty | **fixed 2026-04-30** — renders "Document file not available (type)" message when URL is empty | 2026-04-30 |
| PAGE-ISSUE-0591 | 🟡 | Review modal has no link to documents — admin approves/rejects without viewing files | **fixed 2026-04-30** — "📁 View Documents" button in modal (visible only when `has_documents=true`); provider external links (↗ SumSub / ↗ Didit) shown when `provider_ref_id` set | 2026-04-30 |
| PAGE-ISSUE-0592 | 🟡 | KYC list hard-capped at 200 records — older entries silently invisible | **fixed 2026-04-30** — LIMIT 200 removed from query; verified 747 records load | 2026-04-30 |
| PAGE-ISSUE-0593 | 🟡 | Tracker documented approve/reject as PUT; actual method is POST | **closed — doc only** — corrected in this tracker | 2026-04-30 |

---

### 3.3 Asset Review & Publishing

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | View assets under review | Admin | `/admin/assets` | ✅ | |
| 2 | Review asset details, docs, financials | Admin | `/admin/assets/:id` | ✅ | |
| 3 | Publish asset | Admin | `PUT /api/admin/assets/:id/status` → `published` | ✅ | |
| 4 | Unpublish / suspend asset | Admin | `PUT /api/admin/assets/:id/status` → `suspended` | ✅ | |
| 5 | Apply custom fee for asset or developer | Admin | `POST /api/admin/fees/asset/:id` | ⚠️ | Fee controls show success without persisting; permission gate missing |

#### Open Bugs — Asset Review

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0205](docs/page-audits/) | 🔴 | Fee management routes do not enforce `marketplace.manage` permission | open | 2026-04-25 |
| [PAGE-ISSUE-0206](docs/page-audits/) | 🔴 | Fee controls show success UI without actually persisting to DB | open | 2026-04-25 |
| [PAGE-ISSUE-0207](docs/page-audits/) | 🔴 | Fee list API masks database failures as empty state | open | 2026-04-25 |
| [PAGE-ISSUE-0208](docs/page-audits/) | 🔴 | Fee resolver ignores the accepted developer fee scope | open | 2026-04-25 |
| [PAGE-ISSUE-0209](docs/page-audits/) | 🟡 | Active fee configuration validation is ambiguous | open | 2026-04-25 |
| [PAGE-ISSUE-0210](docs/page-audits/) | 🟡 | Stored fee data renders through raw HTML (XSS risk) | open | 2026-04-25 |
| [PAGE-ISSUE-0211](docs/page-audits/) | 🟡 | Fee mutations are not audit logged | open | 2026-04-25 |
| [PAGE-ISSUE-0212](docs/page-audits/) | 🟡 | Settlement and minimum fee fields lack backend support | open | 2026-04-25 |

---

### 3.4 Financial Operations (Deposits & Dividends)

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass (deposit path works; dividend path unverified)

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | View pending deposit requests | Admin | `/admin/deposits` | ✅ | |
| 2 | Confirm deposit | Admin | `POST /api/admin/deposits/:id/confirm` | ✅ | ACID atomic ✅ |
| 3 | Reject deposit | Admin | `POST /api/admin/deposits/:id/reject` | ✅ | |
| 4 | Draft dividend payout | Admin (finance role) | `/admin/dividends` → `POST /api/admin/dividends/draft` | 🚧 | Frontend UI missing |
| 5 | Approve dividend payout | Superadmin | `POST /api/admin/dividends/:id/approve` | 🚧 | Frontend UI missing |
| 6 | Execute batch payout to investor wallets | System | Batch wallet credit + `dividend_payouts` rows | 🚧 | E2E coverage stale |
| 7 | View audit log | Admin | `/admin/audit-log` | ✅ | |
| 8 | View marketplace orders | Admin | `/admin/marketplace/orders` | ⚠️ | Multiple open security and mock-data bugs |

#### Open Bugs — Financial Operations

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0527](docs/page-audits/) | 🟡 | Dividend lifecycle E2E coverage is stale | fixed, needs recheck | 2026-04-25 |
| [PAGE-ISSUE-0528](docs/page-audits/) | 🔴 | Marketplace orders APIs lack granular permission checks | needs recheck | 2026-04-25 |
| [PAGE-ISSUE-0529](docs/page-audits/) | 🔴 | Admin order cancel is not locked or audited | needs recheck | 2026-04-25 |
| [PAGE-ISSUE-0530](docs/page-audits/) | 🔴 | Open orders page renders mock financial orders on API failure | needs recheck | 2026-04-25 |
| [PAGE-ISSUE-0531](docs/page-audits/) | 🔴 | Open order rows render backend values through innerHTML (XSS risk) | needs recheck | 2026-04-25 |
| [PAGE-ISSUE-0532](docs/page-audits/) | 🟡 | Marketplace orders pagination is unreachable | needs recheck | 2026-04-25 |
| — | 🔴 | Dividend draft + approve frontend UI not built | open | — |

---

### 3.5 Platform Configuration & RBAC

**Last tested:** 2026-04-25 · **Environment:** staging · **Result:** partial pass

| # | Step | Actor | Page / Endpoint | Status | Notes |
|---|------|-------|-----------------|--------|-------|
| 1 | Manage roles & permissions matrix | Superadmin | `/admin/roles` | ⚠️ | Falls back to demo data instead of showing auth failure |
| 2 | Set platform-default fee | Superadmin | `PUT /api/admin/fees/platform` | ✅ | |
| 3 | Create / edit promotions | Admin | `POST /api/admin/fees/promotions` | ✅ | |
| 4 | View fee audit log | Admin | `GET /api/admin/fees/audit-log` | ✅ | |
| 5 | Platform settings (maintenance, config) | Superadmin | `/admin/settings` | ⚠️ | No granular authorization; any admin can trigger |
| 6 | Support ticket management | Admin (support role) | `/admin/support` | ⚠️ | Missing permission gate + audit log |

#### Open Bugs — Platform Config

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0283](docs/page-audits/) | 🔴 | Platform settings and maintenance actions lack granular authorization | open | 2026-04-25 |
| [PAGE-ISSUE-0284](docs/page-audits/) | 🔴 | Support ticket list and bulk update APIs lack support permissions and audit logs | open | 2026-04-25 |
| [PAGE-ISSUE-0285](docs/page-audits/) | 🔴 | Support ticket detail and reply actions lack support permissions and durable audit | open | 2026-04-25 |
| [PAGE-ISSUE-0286](docs/page-audits/) | 🔴 | System dashboard calls unregistered jobs, webhooks, sessions, and reset routes | open | 2026-04-25 |
| [PAGE-ISSUE-0287](docs/page-audits/) | 🔴 | System maintenance and session operations lack granular authorization and audit | open | 2026-04-25 |
| [PAGE-ISSUE-0282](docs/page-audits/) | 🟡 | Roles page falls back to demo data instead of showing authorization failure | open | 2026-04-25 |

---

## 4. Cross-Role Touchpoints

| Event | Trigger | Roles affected | Status | Open Bugs |
|-------|---------|----------------|--------|-----------|
| KYC approved | Admin action | Investor unblocked for investing | ⚠️ | 0160, 0161, 0162 |
| KYC rejected + resubmit | Admin action | Investor must resubmit | 🚧 | 0166, resubmit flow missing |
| Deposit confirmed | Admin action | Investor wallet credited | ✅ | Missing email notification |
| Asset published | Admin action | Developer live; investors can browse | ✅ | |
| Asset change-request cycle | Admin ↔ Developer | Both | 🚧 | No formal cycle; 0391 |
| Dividend payout | Admin draft + approve | All investors in asset | 🚧 | Frontend missing; 0527 |
| Role assignment | Admin action | Developer or investor gains access | ✅ | |
| Session revocation | Admin action | User force-logged out | ✅ | |
| Fee applied to asset | Admin action | Investor checkout, developer yield | ⚠️ | 0205–0212 |
| Withdrawal processed | Admin manual | Investor balance reduced | ❌ | Endpoint not built |

---

## 5. Bug Priority Matrix

Bugs grouped by impact tier for sprint planning.

### 🔴 Must Fix Before Launch (High, open)

| ID | Workflow | Title |
|----|----------|-------|
| ~~PAGE-ISSUE-0160~~ | Admin KYC | ~~KYC routes lack KYC-specific permission gates~~ — **closed — fixed** |
| ~~PAGE-ISSUE-0161~~ | Admin KYC | ~~KYC document signed URLs audit log silently swallowed~~ — **fixed 2026-04-30** |
| ~~PAGE-ISSUE-0162~~ | Admin KYC | ~~Approve audit post-commit gap; reject has no tx; both `let _ =`~~ — **fixed 2026-04-30** |
| PAGE-ISSUE-0205 | Admin Fees | Fee routes do not enforce `marketplace.manage` |
| PAGE-ISSUE-0206 | Admin Fees | Fee controls show success without DB persistence |
| PAGE-ISSUE-0207 | Admin Fees | Fee list API masks DB failures as empty state |
| PAGE-ISSUE-0208 | Admin Fees | Fee resolver ignores developer fee scope |
| PAGE-ISSUE-0283 | Admin Config | Platform settings lack granular authorization |
| PAGE-ISSUE-0284 | Admin Support | Support APIs lack permission gates + audit logs |
| PAGE-ISSUE-0285 | Admin Support | Support ticket detail lacks permissions + audit |
| PAGE-ISSUE-0286 | Admin System | System dashboard calls unregistered routes |
| PAGE-ISSUE-0287 | Admin System | System maintenance lacks authorization + audit |
| PAGE-ISSUE-0291 | Admin Users | User directory exposes PII without permission gate |
| PAGE-ISSUE-0387 | Investor Cart | Cart HTML incomplete escaping (XSS risk) |
| PAGE-ISSUE-0388 | Investor Cart | Cart update fails open on availability lock failure |
| PAGE-ISSUE-0391 | Developer Asset | Asset publish/destroy controls are fake success placeholders |
| PAGE-ISSUE-0528 | Admin Orders | Orders APIs lack granular permission checks |
| PAGE-ISSUE-0529 | Admin Orders | Order cancel not locked or audited |
| PAGE-ISSUE-0530 | Admin Orders | Orders page renders mock data on API failure |
| PAGE-ISSUE-0531 | Admin Orders | Order rows rendered via innerHTML (XSS risk) |
| — | Admin Financial | Dividend draft + approve frontend not built |
| WALLET-FIX-002 | Investor Wallet | ~~"Add Bank"/"Add Card" buttons opened withdraw modal~~ **fixed 2026-04-30** |

### 🟡 Fix Before Beta (Medium, open)

| ID | Workflow | Title |
|----|----------|-------|
| ~~PAGE-ISSUE-0163~~ | Admin KYC | ~~KYC list `unwrap_or_default()` — DB fail returns empty 200~~ — **fixed 2026-04-30** |
| ~~PAGE-ISSUE-0164~~ | Admin KYC | ~~Document viewer uses innerHTML for signed URL data~~ — **closed — fixed** |
| ~~PAGE-ISSUE-0165~~ | Admin KYC | ~~KYC modals lack keyboard semantics~~ — **closed — fixed** |
| ~~PAGE-ISSUE-0166~~ | Admin KYC | ~~Rejection reason validated client-side only~~ — **closed — fixed** |
| ~~PAGE-ISSUE-0591~~ | Admin KYC | ~~Review modal has no link to documents — admin approves blind~~ — **fixed 2026-04-30** |
| ~~PAGE-ISSUE-0592~~ | Admin KYC | ~~KYC list hard-capped at 200 records — older entries invisible~~ — **fixed 2026-04-30** |
| PAGE-ISSUE-0209 | Admin Fees | Fee configuration validation is ambiguous |
| PAGE-ISSUE-0210 | Admin Fees | Fee data rendered through raw HTML |
| PAGE-ISSUE-0211 | Admin Fees | Fee mutations not audit logged |
| PAGE-ISSUE-0212 | Admin Fees | Settlement and minimum fee fields lack backend |
| PAGE-ISSUE-0282 | Admin Config | Roles page falls back to demo data |
| PAGE-ISSUE-0290 | Admin Users | `/admin/users` clean URL returns 404 |
| PAGE-ISSUE-0389 | Investor Cart | Cart controls lack accessible labels |
| ~~PAGE-ISSUE-0390~~ | Developer | ~~Asset type selection mouse-only~~ — **fixed 2026-04-30** |
| PAGE-ISSUE-0392 | Developer | Cap table shows admin-audience links |
| PAGE-ISSUE-0460 | Investor Auth | Email verification has no outbox retry |
| PAGE-ISSUE-0481 | Developer | Chart fragment unauthenticated HTTP 200 |
| PAGE-ISSUE-0482 | Developer | Assets fragment unauthenticated HTTP 200 |
| ~~PAGE-ISSUE-0483~~ | Developer | ~~Document upload controls no accessible names~~ — **fixed 2026-04-30** |
| PAGE-ISSUE-0484 | Developer | Document upload shows hardcoded demo rows |
| ~~PAGE-ISSUE-0485~~ | Developer | ~~Media upload copy mismatches backend limits~~ — **closed, not a bug** |
| PAGE-ISSUE-0487 | Developer | Logo upload UI accepts SVG, backend rejects it |
| ~~PAGE-ISSUE-0488~~ | Developer | ~~Image remove buttons have no accessible names~~ — **closed, already fixed** |
| PAGE-ISSUE-0543 | Investor KYC | KYC audit + email side effects swallowed |
| PAGE-ISSUE-0550 | Investor KYC | KYC email delivery lacks durable outbox |
| PAGE-ISSUE-0532 | Admin Orders | Orders pagination unreachable |

### 🔵 Nice to Fix (Low, open)

| ID | Workflow | Title |
|----|----------|-------|
| PAGE-ISSUE-0486 | Developer | Submission success WhatsApp link is a placeholder |
| PAGE-ISSUE-0566 | Investor KYC | KYC upload can orphan GCS object after DB failure |

---

## 6. Investor Dashboard — Extended Workflows

---

### 6.1 Wallet Page

**Page:** `/wallet` · **JS:** `wallet.js`, `wallet-service.js` · **Last tested:** 2026-04-30 · **Result:** partial pass

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | View wallet balance | Load page | `GET /api/wallet/balance` | ✅ | Verified in browser |
| 2 | View transaction history | Scroll transactions tab | `GET /api/wallet/transactions` | ✅ | Verified in browser |
| 3 | Add credit/debit card | Open "Add Card" modal → enter card details → submit | `POST /api/payment-methods/card` | ⚠️ | Stripe Elements now forced `locale:'en'`; end-to-end card save not retested |
| 4 | Add bank account | Open "Add Bank" modal → enter IBAN/details → submit | `POST /api/payment-methods/bank` | ⚠️ | Bug fixed: "Add Bank" button previously opened withdraw modal instead |
| 5 | Request deposit | Click "Deposit" → enter amount → submit | `POST /wallet/deposit` | ✅ | Full flow verified; wire instructions shown; admin confirm credits balance |
| 6 | Request withdrawal | Click "Withdraw" → select payment method → enter amount | `POST /wallet/withdraw` | ⚠️ | Modal exists and submits; button now disabled when no payment method on file |
| 7 | View transaction details | Click "View details" on transaction row | `/transactions#tx-{id}` | ⚠️ | Event delegation wired; row ID navigation unverified |
| 8 | Filter transactions by type/date | Use filter controls | Client-side filter | 🚧 | Filter behaviour unverified |

#### Open Bugs — Wallet

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| WALLET-FIX-001 | 🟡 | Deposit modal showed misleading "Payment Method" dropdown (backend ignores it for bank wire) | **fixed 2026-04-30** | 2026-04-30 |
| WALLET-FIX-002 | 🔴 | "Add Bank" and "Add Card" buttons opened withdraw modal (overly broad JS selector) | **fixed 2026-04-30** | 2026-04-30 |
| WALLET-FIX-003 | 🟡 | Withdraw button enabled with no payment method; generic error on submit | **fixed 2026-04-30** — button disabled, descriptive toast added | 2026-04-30 |
| WALLET-FIX-004 | 🟡 | Stripe Elements rendered in browser locale (German); form labels in wrong language | **fixed 2026-04-30** — `locale:'en'` added to Elements init | 2026-04-30 |
| BROKEN-LOGICS wallet | 🟡 | Stripe Elements fallback generates mock `manual_*` card tokens accepted by real endpoint | fixed locally, needs runtime recheck | 2026-03-18 |
| — | 🟡 | "View details" row navigation unverified with real transaction IDs | open | 2026-04-30 |
| — | 🟡 | Bank account add flow not verified end-to-end (submit + persist) | open | — |
| — | 🟡 | Withdrawal 72h new-account cooldown not surfaced in UI — user sees generic error | open | 2026-04-30 |

---

### 6.2 Portfolio Page

**Page:** `/portfolio` · **JS:** `portfolio-service.js` · **Last tested:** not tested · **Result:** —

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load portfolio overview | Navigate to `/portfolio` | `GET /api/portfolio` | ✅ | SSR-injected JSON with fetch fallback |
| 2 | View per-investment card (asset name, token count, value) | Browse cards | Rendered from portfolio response | ✅ | |
| 3 | View investment detail / milestones | Click investment card | `/portfolio/:investment_id` | 🚧 | Milestone detail view incomplete |
| 4 | Cancel pending investment | Click "Cancel" on pending order | `POST /api/portfolio/cancel` | 🚧 | Exists in JS; UI trigger unverified |
| 5 | View total value and return % | Page load | Calculated client-side from portfolio data | 🚧 | Accuracy unverified |

#### Open Bugs — Portfolio

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | Per-investment milestone detail page incomplete | open | — |
| — | 🟡 | Cancel pending investment UI trigger not verified | open | — |
| — | 🟡 | Return % calculation accuracy not tested | open | — |

---

### 6.3 Transactions Page

**Page:** `/transactions` · **JS:** `transactions.js` · **Last tested:** not tested · **Result:** —

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load transaction list | Navigate to `/transactions` | `GET /api/wallet/transactions` | ✅ | |
| 2 | Filter by type (deposit / withdrawal / dividend / investment) | Use type filter | Client-side | 🚧 | Filter accuracy unverified |
| 3 | Filter by date range | Set date range inputs | Client-side | 🚧 | Unverified |
| 4 | Error state shown on API failure | Simulate network error | `#transactions-fetch-error` element shown | ✅ | Element present in DOM |
| 5 | Pagination / load more | Scroll or click next page | `GET /api/wallet/transactions?page=N` | 🚧 | Pagination not verified |

#### Open Bugs — Transactions

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | Transaction filter and pagination not verified end-to-end | open | — |

---

### 6.4 My Trading Page

**Page:** `/my-trading` · **JS:** `my-trading.js` · **Last tested:** not tested · **Result:** —

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load open orders | Navigate to `/my-trading` | `GET /api/marketplace/orders/mine` | ✅ | |
| 2 | Load trade history | Tab switch | `GET /api/marketplace/trades/mine` | ✅ | |
| 3 | Load portfolio | On init | `GET /api/portfolio` | ✅ | |
| 4 | Cancel open order | Click cancel button on order row | `DELETE /api/marketplace/orders/:id` | 🚧 | Error handling unverified |
| 5 | Export tax report (CSV / PDF) | Click export | `GET /api/marketplace/tax-export?year=&format=` | 🚧 | Download verified in JS; backend response not tested |
| 6 | View user profile/limits | On init | `GET /api/me` | ✅ | |

#### Open Bugs — My Trading

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | Order cancel error handling not verified | open | — |
| — | 🟡 | Tax export backend response not tested end-to-end | open | — |

---

### 6.5 Secondary Marketplace

**Page:** `/marketplace-secondary` · **JS:** `marketplace-secondary.js` · **Last tested:** 2026-04-24 · **Result:** partial (buy-interest fixed locally)

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Browse secondary listings | Load page | `GET /api/marketplace/secondary` or SSR | ✅ | |
| 2 | Place sell order | Open sell modal → enter price/qty → submit | `POST /api/marketplace/orders` | 🚧 | Flow not fully verified |
| 3 | Place buy interest | Open buy-interest modal → enter price/qty → submit | `POST /api/marketplace/orders` | 🔁 | Fixed locally (was fake success); needs runtime recheck |
| 4 | View order book for asset | Load asset secondary view | `GET /api/marketplace/orderbook/:asset_id` | 🚧 | Unverified |

#### Open Bugs — Secondary Marketplace

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| PAGE-ISSUE-0006 (BROKEN-LOGICS) | 🟡 | Buy-interest modal was fake success (no backend call) | fixed locally, needs recheck | 2026-04-24 |

---

### 6.6 Support Page

**Page:** `/support` · **JS:** `support.js` · **Last tested:** not tested · **Result:** —

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load open/closed tickets | Navigate to `/support` | `GET /api/support/tickets` | ✅ | |
| 2 | Submit new support ticket | Fill form → attach file → submit | `POST /api/support/tickets` | 🚧 | File attachment validation client-side only |
| 3 | Reply to open ticket | Open ticket → write reply → submit | `POST /api/support/tickets/:id/reply` | 🚧 | Unverified end-to-end |
| 4 | Reopen closed ticket | Click "Reopen" | `POST /api/support/tickets/:id/reopen` | 🚧 | Unverified |
| 5 | Filter/search FAQ | Type in FAQ search | Client-side `filterFAQ()` | ✅ | |
| 6 | Draft auto-saved | Type in ticket form | `localStorage` draft | ✅ | |

#### Open Bugs — Support

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | File attachment validation is client-side only; no server-side type/size check | open | — |
| — | 🟡 | Ticket reply and reopen flows not verified end-to-end | open | — |

---

### 6.7 Rewards / Referrals Page

**Page:** `/rewards` · **JS:** `rewards.js`, `rewards-service.js` · **Last tested:** not tested · **Result:** —

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load rewards overview | Navigate to `/rewards` | `GET /api/rewards` or SSR | ✅ | |
| 2 | Copy referral link | Click copy button | Client-side clipboard | ✅ | |
| 3 | View commission history | Switch to Commissions tab | `GET /api/rewards/commissions` | 🚧 | Lazy-loaded; pagination unverified |
| 4 | Save payout settings | Fill bank/payout details → save | `POST /api/rewards/payout-settings` | 🚧 | Unverified end-to-end |
| 5 | Load payout settings | Payout tab load | `GET /api/rewards/payout-settings` | 🚧 | Unverified |
| 6 | Share via social | Click social share button | `shareSocial()` opens new window | ✅ | |
| 7 | Export campaigns CSV | Click export | Client-side CSV generation | ✅ | |

#### Open Bugs — Rewards

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | Payout settings save/load not verified end-to-end | open | — |
| — | 🟡 | Commission pagination unverified | open | — |

---

### 6.8 Leaderboard Page

**Page:** `/leaderboard` · **JS:** `leaderboard.js` · **Last tested:** 2026-04-28 · **Result:** partial

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load leaderboard rankings | Navigate to `/leaderboard` | `GET /api/leaderboard` or SSR | ✅ | |
| 2 | View own rank and score | Scroll to highlighted row | Returned in leaderboard response | ✅ | |
| 3 | Filter by time period | Tab or dropdown switch | Client-side / query param | 🚧 | Unverified |
| 4 | Visibility affects leaderboard appearance | Toggle leaderboard visibility in settings | `POST /api/settings/leaderboard` | 🚧 | Not end-to-end verified |

#### Open Bugs — Leaderboard

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | Leaderboard visibility setting not tested end-to-end with display | open | — |

---

### 6.9 Community Page

**Page:** `/community` · **JS:** `community-feed.js`, `community-circles.js`, `community-amas.js` · **Last tested:** 2026-04-28 · **Result:** partial

| # | Step | Action | Endpoint | Status | Notes |
|---|------|--------|----------|--------|-------|
| 1 | Load feed | Navigate to `/community` | `GET /community/partials/feed/list` | ⚠️ | Comment counters can be stale after new comments |
| 2 | Create post | Write post → submit | `POST /api/community/posts` | 🚧 | Unverified post-fix |
| 3 | React to post | Click reaction button | `POST /api/community/posts/:id/reactions` | ⚠️ | Reaction buttons ignore current user's existing reaction state |
| 4 | Comment on post | Open post → write → submit | `POST /api/community/posts/:id/comments` | ⚠️ | Can leave stale feed counters |
| 5 | Click trending asset card | Click asset in trending section | Routes to `/assets/:id` | ❌ | Wrong route — `/assets/:id` not registered; should be `/property/:slug` or `/commodity/:slug` |
| 6 | View announcements | Switch announcements tab | `GET /community/partials/announcements` | ✅ | |
| 7 | View AMAs | Switch AMA tab | `GET /community/amas` | 🚧 | Unverified |

#### Open Bugs — Community

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| [PAGE-ISSUE-0499](docs/page-audits/) | 🔴 | Comment creation can leave stale feed counters | needs recheck | 2026-04-28 |
| [PAGE-ISSUE-0555](docs/page-audits/) | 🔴 | Feed reaction button used invalid schema value | needs recheck | 2026-04-28 |
| [PAGE-ISSUE-0500](docs/page-audits/) | 🟡 | Feed partial masks backend failures | needs recheck | 2026-04-28 |
| [PAGE-ISSUE-0501](docs/page-audits/) | 🟡 | Reaction buttons ignore current user's reaction state | needs recheck | 2026-04-28 |
| [PAGE-ISSUE-0502](docs/page-audits/) | 🔵 | Feed engagement controls lack state semantics | needs recheck | 2026-04-28 |
| BROKEN-LOGICS community | 🟡 | Trending asset cards route to `/assets/:id` (not registered) | open | 2026-03-18 |

---

## 7. Investor Settings — Field-by-Field Save Verification

**Page:** `/settings` · **JS:** `settings.js` · **API:** `GET /api/settings`, multiple `POST /api/settings/*`

This section documents every single settings field, what it saves to, and whether saving has been verified in a real browser session.

**Test procedure for each field:**
1. Load `/settings` — verify field populates with current value
2. Change the value
3. Click the section's Save button
4. Reload page — verify value persisted

---

### 7.1 Core Profile (`form-core-profile` → `POST /api/settings/profile`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass

| Field | ID | Type | Required | Save Verified | Notes |
|-------|----|------|----------|---------------|-------|
| First name | `settings-first-name` | text | ✅ | ✅ | Saved and persisted on reload |
| Middle name | `settings-middle-name` | text | — | ✅ | Optional |
| Last name | `settings-last-name` | text | ✅ | ✅ | |
| Gender | `settings-gender` | select | — | ❌ not tested | |
| Email | `settings-email` | email | — | N/A | Readonly; changed via modal |
| Phone number | `settings-phone` | tel | — | ❌ not tested | |
| Avatar / profile photo | `btn-photo-upload` → `settings-avatar-img` | file upload | — | ❌ not tested | `POST /api/upload/avatar` |

**Save button:** "Save Profile" (`btn-save-profile[data-section=core]`)
**Known issues:** None.

---

### 7.2 Address (`form-address` → `POST /api/settings/profile`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass

| Field | ID | Type | Required | Save Verified | Notes |
|-------|----|------|----------|---------------|-------|
| Address line 1 | `settings-address-1` | text | — | ✅ | |
| Address line 2 | `settings-address-2` | text | — | ✅ | Optional |
| City | `settings-city` | text | — | ✅ | |
| State / Province | `settings-state` | text | — | ✅ | |
| Postal / ZIP code | `settings-postal` | text | — | ✅ | |
| Country | `settings-country` | searchable select | — | ✅ | |

**Save button:** "Save Address" (`btn-save-profile[data-section=address]`)
**Known issues:** None.

---

### 7.3 Identity Details (`form-identity` → `POST /api/settings/profile`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass

| Field | ID | Type | Required | Save Verified | Notes |
|-------|----|------|----------|---------------|-------|
| Date of birth | `settings-dob` | date | — | ✅ | max = 2012-12-31; backend validates 18+ age |
| Nationality | `settings-nationality` | searchable select | — | ✅ | ISO 2-letter code stored |
| Tax ID | `settings-tax-id` | text | — | ✅ | Stored encrypted |

**Save button:** "Save Identity Details" (`btn-save-profile[data-section=identity]`)
**Known issues:** None. Backend uses partial-update CASE WHEN logic — missing fields preserved from DB.

---

### 7.4 Preferences & Notifications (`form-preferences` → `POST /api/settings/preferences` + `POST /api/settings/notifications`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass

| Field | ID | Type | Required | Save Verified | Notes |
|-------|----|------|----------|---------------|-------|
| Language | `settings-language` | select | — | ✅ | Default: `en` |
| Timezone | `settings-timezone` | select | — | ✅ | Default: `UTC` |
| Currency display | `settings-currency` | select | — | ✅ | Default: `USD` |
| Email notifications | `settings-notify-email` | toggle switch | — | ❌ not tested | `data-toggle-save="notifications"` — auto-saves on toggle |
| Push notifications | `settings-notify-push` | toggle switch | — | ❌ not tested | Auto-saves on toggle |

**Save button:** "Update Preferences" (form submit) — also inline auto-save on notification toggles
**Known issues:** None.

---

### 7.5 Security (`sec-security` — modal-based actions, no form save)

**Last tested:** 2026-04-30 · **Result:** ✅ partial pass

| Action | Trigger | Endpoint | Status | Notes |
|--------|---------|----------|--------|-------|
| Change password | "Change Password" → `modal-change-password` | `POST /api/settings/password` | ✅ | Modal opens; wrong-password error shown correctly |
| Change phone number | "Change Phone" → `modal-change-phone` | `POST /api/settings/phone` | 🚧 | Not end-to-end verified |
| Enable 2FA (TOTP) | `settings-2fa-action` → `/auth/2fa/setup` | `POST /auth/2fa/setup` | 🔁 | Fixed in code (encrypted secret, rate-limit); needs browser recheck |
| Disable 2FA | `settings-2fa-action` → `modal-disable-2fa` | `POST /api/settings/2fa/disable` | 🔁 | Fixed; needs recheck |
| View linked OAuth (Google) | Page load | `GET /api/settings/oauth` | ✅ | Google shown as connected |
| Link Google OAuth | "Link Google" button | `POST /api/settings/oauth/google/link` | 🚧 | Unverified |
| Unlink OAuth connection | "Unlink" button per connection | `DELETE /api/settings/oauth/:id` | 🚧 | Unverified |
| View email + verified badge | Page load | Rendered from `/api/settings` | ✅ | "Verified" badge shown |

#### Open Bugs — Security Settings

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| 2FA audit | 🟡 | `TOTP_SECRET_ENCRYPTION_KEY` must be set in production; legacy plaintext secrets need migration | needs recheck | 2026-04-27 |
| — | 🟡 | Phone change flow not verified end-to-end | open | — |
| — | 🟡 | OAuth link/unlink not verified | open | — |

---

### 7.6 Leaderboard Profile (`form-leaderboard` → `POST /api/settings/leaderboard`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass

| Field | ID | Type | Required | Save Verified | Notes |
|-------|----|------|----------|---------------|-------|
| Leaderboard visibility | `settings-lb-visible` | toggle switch | — | ✅ | |
| Show avatar on leaderboard | `settings-lb-avatar` | toggle switch | — | ✅ | |
| Display name | `settings-lb-display-name` | text | — | ✅ | Blank = real name |
| Bio | `settings-lb-bio` | textarea | — | ✅ | Max 300 chars |

**Save button:** "Save Leaderboard Settings" (form submit)
**Known issues:** Leaderboard visibility end-to-end display on `/leaderboard` not verified.

---

### 7.7 Social Links (`form-social` → `POST /api/settings/social`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass

| Field | ID | Type | Save Verified | Notes |
|-------|-----|------|---------------|-------|
| X / Twitter | `settings-social-twitter` | url | ✅ | |
| LinkedIn | `settings-social-linkedin` | url | ✅ | |
| Instagram | `settings-social-instagram` | url | ✅ | |
| Telegram | `settings-social-telegram` | url | ✅ | |
| Discord | `settings-social-discord` | text | ✅ | `username#0000` format |
| Personal website | `settings-social-website` | url | ✅ | |

**Save button:** "Save Social Links" (`btn-save-social`)
**Known issues:** URL format validation server-side not stress-tested (invalid URLs not rejected in testing).

---

### 7.8 Developer Identity & Links (`form-developer-profile` / `form-developer-links`)

**Last tested:** 2026-04-30 · **Result:** ✅ pass (access control)

| Action | Trigger | Endpoint | Status | Notes |
|--------|---------|----------|--------|-------|
| Save Developer Profile | "Save Developer Profile" | `POST /api/settings/developer-profile` | ✅ | Correctly blocked: "Developer settings are only available to developer accounts." |
| Save Developer Links | "Save Developer Links" | `POST /api/settings/developer-links` | ✅ | Same block message for non-developer accounts |
| Upload developer logo | `btn-dev-logo-upload` | `POST /api/upload/developer-logo` | 🚧 | Not tested — requires developer account |

**Known issues:** Developer sections not testable with investor test account. Need dedicated developer account for full coverage.

---

### 7.9 Privacy & GDPR (`sec-privacy` / `card-data-privacy`)

**Last tested:** 2026-04-30 · **Result:** ✅ partial pass

| Action | Trigger | Endpoint | Status | Notes |
|--------|---------|----------|--------|-------|
| Request data export | `btn-request-data-export` | `GET /api/settings/export-data` + file download | ✅ | "Data export download started." toast confirmed; file download triggered |
| Delete account | `btn-delete-account` → `modal-delete-account` | `POST /api/settings/delete-account` | 🚧 | Not tested — irreversible; requires staging |

#### Open Bugs — Privacy & GDPR

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| — | 🟡 | Account deletion flow not verified; irreversible action must be tested on staging | open | — |

---

### 7.10 Settings: Save Verification Master Checklist

Use this table during QA to sign off each settings section.

| Section | Form ID | Endpoint | Load | Change | Save | Reload + Verify | Tester | Date | Pass? |
|---------|---------|----------|------|--------|------|-----------------|--------|------|-------|
| Core profile | `form-core-profile` | `POST /api/settings/profile` | ✅ | ✅ | ✅ | ✅ | claude | 2026-04-30 | ✅ |
| Avatar upload | `btn-photo-upload` | `POST /api/upload/avatar` | ❌ | ❌ | ❌ | ❌ | — | — | — |
| Address | `form-address` | `POST /api/settings/profile` | ✅ | ✅ | ✅ | ✅ | claude | 2026-04-30 | ✅ |
| Identity details | `form-identity` | `POST /api/settings/profile` | ✅ | ✅ | ✅ | ✅ | claude | 2026-04-30 | ✅ |
| Language | `form-preferences` | `POST /api/settings/preferences` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Timezone | `form-preferences` | `POST /api/settings/preferences` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Currency display | `form-preferences` | `POST /api/settings/preferences` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Email notifications toggle | `settings-notify-email` | `POST /api/settings/notifications` | ✅ | ❌ | auto | ❌ | — | — | — |
| Push notifications toggle | `settings-notify-push` | `POST /api/settings/notifications` | ✅ | ❌ | auto | ❌ | — | — | — |
| Change password | `form-change-password` | `POST /api/settings/password` | N/A | ✅ | ✅ (error) | N/A | claude | 2026-04-30 | ✅ |
| Change phone | `form-change-phone` | `POST /api/settings/phone` | ❌ | ❌ | ❌ | ❌ | — | — | — |
| Enable 2FA | `settings-2fa-action` | `POST /auth/2fa/setup` | ❌ | ❌ | ❌ | ❌ | — | — | — |
| Disable 2FA | `form-disable-2fa` | `POST /api/settings/2fa/disable` | ❌ | ❌ | ❌ | ❌ | — | — | — |
| Link Google OAuth | OAuth link btn | `POST /api/settings/oauth/google/link` | ✅ | ❌ | ❌ | ❌ | — | — | — |
| Unlink OAuth | Unlink btn | `DELETE /api/settings/oauth/:id` | ✅ | ❌ | ❌ | ❌ | — | — | — |
| Leaderboard visibility | `form-leaderboard` | `POST /api/settings/leaderboard` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Leaderboard display name | `form-leaderboard` | `POST /api/settings/leaderboard` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Leaderboard bio | `form-leaderboard` | `POST /api/settings/leaderboard` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Social links | `form-social` | `POST /api/settings/social` | ✅ | — | ✅ | ❌ | claude | 2026-04-30 | partial |
| Developer profile | `form-developer-profile` | `POST /api/settings/developer-profile` | ✅ | — | ✅ (blocked) | N/A | claude | 2026-04-30 | ✅ |
| Developer links | `form-developer-links` | `POST /api/settings/developer-links` | ✅ | — | ✅ (blocked) | N/A | claude | 2026-04-30 | ✅ |
| Data export | `btn-request-data-export` | `GET /api/settings/export-data` | N/A | N/A | ✅ | N/A | claude | 2026-04-30 | ✅ |
| Delete account | `form-delete-account` | `POST /api/settings/delete-account` | N/A | N/A | ❌ | N/A | — | — | **test on staging only** |

#### Open Bugs — Login / CSRF

| ID | Sev | Title | Status | Since |
|----|-----|-------|--------|-------|
| LOGIN-BUG-001 | 🟡 | Login form CSRF token hidden input synced only once at page load — if CSRF cookie rotates before submit, login fails with "Security check failed." | **fixed** 2026-04-30 — re-sync added to `htmx:beforeRequest` in `login.html` | 2026-04-30 |

---

## 8. Testing Coverage Summary

| Workflow | Last Tested | Result | High Bugs | Medium Bugs | Low Bugs |
|----------|-------------|--------|-----------|-------------|----------|
| Investor — Registration | 2026-04-25 | partial | 0 | 1 | 0 |
| Investor — KYC | 2026-04-25 | partial | 0 | 2 | 1 |
| Investor — Deposit | 2026-04-30 | pass | 0 | 2 | 0 |
| Investor — Cart/Checkout | 2026-04-30 | pass | 2 | 0 | 2 |
| Investor — Portfolio/Dividends | not tested | — | 0 | 2 | 0 |
| Investor — Wallet | 2026-04-30 | partial pass | 0 | 3 | 0 |
| Investor — Portfolio page | not tested | — | 0 | 3 | 0 |
| Investor — Transactions | not tested | — | 0 | 1 | 0 |
| Investor — My Trading | not tested | — | 0 | 2 | 0 |
| Investor — Secondary Marketplace | 2026-04-24 | partial | 0 | 1 | 0 |
| Investor — Support | not tested | — | 0 | 2 | 0 |
| Investor — Rewards | not tested | — | 0 | 2 | 0 |
| Investor — Leaderboard | 2026-04-28 | partial | 0 | 1 | 0 |
| Investor — Community | 2026-04-28 | partial | 2 | 2 | 1 |
| Investor — Settings (all fields) | 2026-04-30 | partial pass | 0 | 1 | 0 |
| Developer — Onboarding | 2026-04-30 | pass | 1 | 1 | 0 |
| Developer — Asset Submission | 2026-04-30 | pass | 0 | 1 | 0 |
| Developer — Review Cycle | 2026-04-25 | partial | 1 | 1 | 0 |
| Developer — Post-Launch | not tested | — | 0 | 3 | 0 |
| Admin — User Management | 2026-04-25 | partial | 1 | 2 | 0 |
| Admin — KYC Review | 2026-04-30 | pass | 0 | 0 | 0 |
| Admin — Asset Review | 2026-04-25 | partial | 4 | 5 | 0 |
| Admin — Financial Ops | 2026-04-25 | partial | 5 | 2 | 0 |
| Admin — Platform Config | 2026-04-25 | partial | 5 | 1 | 0 |

---

*Source files: `docs/remaining-documented-issues-2026-04-28.md`, `docs/BROKEN_LOGICS.md`, `docs/page-review-tracker.yml`*
*Next full re-audit target: schedule after sprint resolving high-severity bugs above.*
