# POOOL — Broken Logics & Full Platform Audit

> **Generated:** 2026-03-18  
> **Methodology:** Static code analysis across backend (Rust/Axum) and frontend (Vanilla JS), following the 5-step audit workflow: Control Flow, State Sync, Financial Logic, Auth/CSRF, Edge Case Stress Test.

---

## 🔴 CRITICAL (Fix Immediately)

### BUG-001: Float-to-Cents Conversion in Wallet Deposit & Withdraw Handlers

- **Severity:** 🔴 Critical (Financial)
- **Location:** `backend/src/wallet/routes.rs` → `handle_deposit()` (L324-330) and `handle_withdraw()` (L361-367)
- **The Logic Flaw:** User input is parsed as `f64` and multiplied by 100 to get cents: `(amount_dollars * 100.0).round() as i64`. Floating-point arithmetic can produce rounding errors — e.g., `19.99 * 100.0` may yield `1998.9999…` which rounds to `1999`, but `0.1 + 0.2 != 0.3` in IEEE754. This violates the business rule: **all monetary values must be BIGINT cents, never floats**.
- **Replication:** Enter `$19.99` in the deposit form. The float representation `19.99` may produce an off-by-one cent error depending on the input.
- **Proposed Fix:** Parse the input as a string, split on `.`, and construct cents from integer parts:
  ```rust
  // Instead of: let amount_cents = (amount_dollars * 100.0).round() as i64;
  let parts: Vec<&str> = amount_clean.split('.').collect();
  let dollars: i64 = parts[0].parse().unwrap_or(0);
  let cents: i64 = parts.get(1).map(|s| {
      let s = format!("{:0<2}", &s[..s.len().min(2)]);
      s.parse().unwrap_or(0)
  }).unwrap_or(0);
  let amount_cents = dollars * 100 + cents;
  ```

---

### BUG-002: Float-to-Cents Conversion in Cart Add Handler

- **Severity:** 🔴 Critical (Financial)
- **Location:** `backend/src/cart/routes.rs` → `add_to_cart()` (L141-143)
- **The Logic Flaw:** Same IEEE754 float issue: `(v * 100.0) as i64` truncates instead of rounding, and is subject to float imprecision. This means the number of tokens calculated from the investment amount could be off by one.
- **Replication:** Enter `$999.99` as investment amount → `(999.99 * 100.0) as i64` may truncate to `99998` instead of `99999`.
- **Proposed Fix:** Same string-based parsing as BUG-001.

---

### BUG-003: Withdrawal Balance Check is Outside Transaction (TOCTOU Race)

- **Severity:** 🔴 Critical (Financial — Double-Spend)
- **Location:** `backend/src/wallet/routes.rs` → `handle_withdraw()` (L370-377)
- **The Logic Flaw:** The balance check at L370 runs as a plain `fetch_optional` against the pool (no transaction, no `FOR UPDATE` lock). The withdrawal insert at L388 also runs outside a transaction. Between the balance check and the insert, another concurrent request could succeed, allowing **two withdrawals that together exceed the balance**.
- **Replication:**
  1. User has $100 in wallet.
  2. Open two browser tabs, both on `/wallet`.
  3. Submit $100 withdrawal in both tabs simultaneously.
  4. Both requests read `balance_cents = 10000`, both pass the `>= amount_cents` check, both insert withdrawal requests.
- **Proposed Fix:** Wrap the entire withdraw flow in a `BEGIN ... FOR UPDATE ... COMMIT` transaction:
  ```rust
  let mut tx = state.db.begin().await?;
  let balance: i64 = sqlx::query_scalar(
      "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'cash' FOR UPDATE"
  ).bind(user.id).fetch_one(&mut *tx).await?;
  // ... check, insert, commit
  tx.commit().await?;
  ```

---

### BUG-004: Hardcoded FX Rate in Checkout

- **Severity:** 🔴 Critical (Financial)
- **Location:** `backend/src/payments/service.rs` → `execute_checkout()` (L338-344)
- **The Logic Flaw:** The USD→IDR exchange rate is hardcoded as `15_500.0`. This means Indonesian users are charged at a stale rate that could be significantly off from market rates. A 5% deviation on a $10,000 order = $500 loss (to either the platform or the user).
- **Replication:** Any IDR checkout uses the static rate, regardless of actual market conditions.
- **Proposed Fix:** Integrate an FX rate API (OpenExchangeRates, or the PSP's conversion API) and cache rates with a 15-minute TTL. Add a `fx_rate_fetched_at` timestamp to orders for audit.

---

### BUG-005: Float in API Transaction Response

- **Severity:** 🟡 High (Data Integrity)
- **Location:** `backend/src/wallet/routes.rs` → `api_wallet_transactions()` (L630)
- **The Logic Flaw:** `amount_usd: *amount as f64 / 100.0` converts cents to a float for the JSON API response. While this is display-only, downstream consumers parsing this field for calculations would introduce float errors. The API should return `amount_cents` (integer) as the source of truth and `amount_display` (formatted string) for UI.
- **Proposed Fix:** Remove or deprecate the `amount_usd` float field; use `amount_cents` + `amount_display` instead.

---

### BUG-006: Payment Webhook Secret Defaults to "dev_secret" in Production

- **Severity:** 🔴 Critical (Security)
- **Location:** `backend/src/payments/routes.rs` → `payment_webhook()` (L106-107)
- **The Logic Flaw:** `std::env::var("PAYMENT_WEBHOOK_SECRET").unwrap_or_else(|_ | "dev_secret".to_string())` — if the env var is not set in production, ANY attacker who sends `{"signature": "dev_secret", "status": "paid", "provider_reference": "..."}` can credit any deposit, stealing funds.
- **Replication:** `curl -X POST https://poool.app/api/webhooks/payments -H 'Content-Type: application/json' -d '{"signature":"dev_secret","status":"paid","provider_reference":"STRIPE-..."}'`
- **Proposed Fix:** Panic on startup if `PAYMENT_WEBHOOK_SECRET` is not set in production (or use a length/entropy check). Never default to a predictable string.

---

## 🟡 HIGH (Fix Before Launch)

### BUG-007: Cart Token Price Uses Stale Snapshot

- **Severity:** 🟡 High (Financial)
- **Location:** `backend/src/cart/routes.rs` → `add_to_cart()` (L190-194)
- **The Logic Flaw:** The `token_price_cents` stored in `cart_items` is snapshotted at add-to-cart time. If the asset's price changes between adding to cart and checkout, the user pays the old price. In `execute_checkout()` (service.rs L320), the total is recalculated using `a.token_price_cents` (the live asset price), but the cart still shows the old price, creating a confusing UX discrepancy.
- **Replication:** Admin changes asset token price. User who had the asset in cart sees the old price on `/cart` but gets charged the new price at checkout.
- **Proposed Fix:** Either (a) always use the live asset price at checkout (current behavior — good) but update the cart display to show live prices, or (b) lock the price at cart time and honor it at checkout (requires business decision).

---

### BUG-008: Order Number is Not Unique Under Concurrency

- **Severity:** 🟡 High (Data Integrity)
- **Location:** `backend/src/payments/service.rs` → `execute_checkout()` (L408)
- **The Logic Flaw:** `order_number = format!("ORD-{}", Utc::now().format("%Y%m%d%H%M%S"))`. Two concurrent checkouts within the same second produce identical order numbers. If the `orders.order_number` column has a `UNIQUE` constraint, one transaction fails. If it doesn't, you get duplicate order numbers.
- **Proposed Fix:** Append a random suffix or use a DB sequence: `format!("ORD-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &Uuid::new_v4().to_string()[..6])`.

---

### BUG-009: No Negative Balance Guard on Wallets Table

- **Severity:** 🟡 High (Financial)
- **Location:** `backend/src/payments/service.rs` (L387), `backend/src/admin/approvals.rs` (L429)
- **The Logic Flaw:** The `UPDATE wallets SET balance_cents = balance_cents - $1` statement has no `CHECK (balance_cents >= 0)` constraint at the database level. While the application checks balance before deduction, if there's ever a code path that skips the check (e.g., admin balance adjustment with a negative amount larger than balance), the wallet can go negative.
- **Proposed Fix:** Add a PostgreSQL CHECK constraint: `ALTER TABLE wallets ADD CONSTRAINT balance_non_negative CHECK (balance_cents >= 0);`

---

### BUG-010: CSRF Bypass via Form Action Query String

- **Severity:** 🟡 High (Security)
- **Location:** `backend/src/auth/csrf.rs` (L66-76)
- **The Logic Flaw:** The CSRF middleware accepts tokens via query string (`?csrf_token=...`). This means CSRF tokens can appear in server access logs, browser history, and referrer headers — weakening the security model. Additionally, HTML forms using `method="POST" action="/cart/remove"` (like in `cart/routes.rs` L653) don't include a CSRF token at all — they rely on neither the header nor the query string approach.
- **Replication:** Submit the cart remove form → CSRF validation fails → item not removed (or was recently fixed by removing CSRF for form posts?).
- **Proposed Fix:** Inject a hidden `<input name="csrf_token">` field into all server-rendered forms, or check the form body for the token (not just query string).

---

## 🟠 MEDIUM (Degraded UX / Data Sync Issues)

### BUG-011: Developer Application Form — Stale draft_asset_id in localStorage

- **Severity:** 🟠 Medium (Workflow Break)
- **Location:** `frontend/platform/static/js/developer-application-form.js` (L83, L133, L149, L171, L390, L406, L422)
- **The Logic Flaw:** The application form persists `draft_asset_id` in `localStorage`. If a user submits an application, then returns to the form later, the stale ID causes a `PUT` to a non-existent or already-submitted draft, resulting in "Unexpected Error". The cleanup at L133 (`localStorage.removeItem`) only fires on specific error paths, not on successful submission.
- **Replication:**
  1. Start a new application, save draft → `draft_asset_id` stored.
  2. Submit the application successfully.
  3. Return to `/developer/apply` → form loads with stale `draft_asset_id`.
  4. Try to save → PUT fails → "Unexpected Error".
- **Proposed Fix:** Clear `draft_asset_id` from localStorage on successful submission (not just on error). Add a startup check: `fetch /api/developer/assets/{id}` → if 404, clear localStorage and start fresh.

---

### BUG-012: IDR Conversion Inconsistency Between Cart and Payments

- **Severity:** 🟠 Medium (Financial Display)
- **Location:** `backend/src/cart/routes.rs` → `format_idr()` (L96-109) vs `backend/src/payments/service.rs` → `format_idr()` (L978-994)
- **The Logic Flaw:** Two different `format_idr()` functions exist with different logic. The cart version uses float math (`(cents as f64 / 100.0) * idr_conversion_rate`) with a rate of `15,700`, while the payments service uses straight integer formatting with no conversion (treating cents as IDR units). Different rates (15,500 vs 15,700) are used in different places.
- **Proposed Fix:** Centralize the IDR conversion rate and formatting logic into a single shared utility module.

---

## 📑 PER-PAGE LOGIC STATUS

### 🟢 INVESTOR PAGES

| Page | Route | Logic Status | Known Issues |
|:---|:---|:---|:---|
| **Marketplace** | `/marketplace` | ✅ OK | — |
| **Property Detail** | `/property/:id` | ✅ OK | — |
| **Wallet** | `/wallet` | ✅ OK | — |
| **Portfolio** | `/portfolio` | ✅ OK | — |
| **Cart** | `/cart` | ✅ OK | — |
| **Checkout** | `/checkout` | ✅ OK | — |
| **Payment Progress** | `/payment-in-progress` | ✅ OK | Polling and redirect logic is functional |
| **Payment Success** | `/payment-success` | ✅ OK | Visual-only page |
| **Rewards** | `/rewards` | ✅ OK | — |
| **Leaderboard** | `/leaderboard` | ✅ OK | Weighted composite ranking (invest 40%, referral 25%, tier 20%, diversity 15%) |
| **Community** | `/community` | ⚪ Static | Placeholder page |
| **Settings** | `/settings` | ✅ OK | Recently fixed |
| **Transactions** | `/transactions` | ✅ OK | — |
| **Support** | `/support` | ✅ OK | Ticket creation and viewing functional |
| **KYC** | `/kyc` | ✅ OK | Webhook HMAC verified; `get_session_result` bug fixed; extracted data auto-populates profile |
| **Commodities** | `/commodities-marketplace` | ✅ OK | Same asset pipeline as marketplace; filtering by `asset_type = 'commodity'` |

### 🟡 DEVELOPER PAGES

| Page | Route | Logic Status | Known Issues |
|:---|:---|:---|:---|
| **Dashboard** | `/developer/dashboard` | ✅ OK | — |
| **Assets List** | `/developer/assets` | ✅ OK | — |
| **Asset Detail** | `/developer/asset-detail` | ✅ OK | — |
| **Application Form** | `/developer/apply` | ✅ OK | — |
| **Document Upload** | `/developer/documents` | ✅ OK | Ownership check, MIME validation, 20MB limit, GCS + local fallback |
| **Submissions** | `/developer/submissions` | ✅ OK | — |
| **Add Asset** | `/developer/add-asset` | ✅ OK | — |
| **Submission Success** | `/developer/submission-success` | ✅ OK | — |

### 🔴 ADMIN PAGES

| Page | Route | Logic Status | Known Issues |
|:---|:---|:---|:---|
| **Dashboard** | `/admin` | ✅ OK | — |
| **Users** | `/admin/users` | ✅ OK | — |
| **User Details** | `/admin/user-details` | ✅ OK | — |
| **Assets List** | `/admin/assets` | ✅ OK | — |
| **Asset Details** | `/admin/asset-details` | ✅ OK | — |
| **Asset Tokenize** | `/admin/asset-tokenize` | ❌ Not Implemented | Blockchain logic missing |
| **Change Requests** | `/admin/asset-change-requests` | ✅ OK | Revision workflow enhanced and tested |
| **Dev Submissions** | `/admin/developer-submissions` | ✅ OK | — |
| **Submission Review** | `/admin/developer-submission-review` | ✅ OK | — |
| **Orders** | `/admin/orders` | ✅ OK | — |
| **Deposits** | `/admin/deposits` | ✅ OK | — |
| **Pending Settlements** | `/admin/pending-settlements` | ⚪ Static | Hardcoded HTML mockup; no backend API |
| **Treasury** | `/admin/treasury` | ✅ OK | Real SQL aggregations; TX limit reduced to 500 |
| **KYC** | `/admin/kyc` | ✅ OK | — |
| **Approvals** | `/admin/approvals` | ✅ Fixed | Four-eyes workflow now executes business logic correctly |
| **Dividends** | `/admin/dividends` | ✅ OK | Fixed-point `u128` math implemented |
| **Rewards** | `/admin/rewards` | ✅ OK | Referral payout race fixed; audit trail added |
| **Reports** | `/admin/reports` | ✅ OK | Fixed column name mismatches; CSV/JSON export functional |
| **Support** | `/admin/support` | ✅ OK | — |
| **Support Ticket** | `/admin/support-ticket` | ✅ OK | — |
| **Notifications** | `/admin/notifications` | ✅ OK | List + broadcast functional; AdminUser extractor used |
| **Audit Logs** | `/admin/audit-logs` | ✅ OK | — |
| **Email Marketing** | `/admin/email-marketing` | ✅ OK | Database aggregations replace hardcoded stats |
| **System** | `/admin/system` | ✅ OK | Buttons linked to mock endpoints |
| **Settings** | `/admin/settings` | ✅ OK | — |
| **Admins** | `/admin/admins` | ✅ OK | — |
| **Roles** | `/admin/roles` | ✅ OK | — |
| **Storage** | `/admin/storage` | ✅ OK | Real SQL aggregations with GCS cost estimates |
| **Blockchain Treasury** | `/admin/blockchain-treasury` | ❌ Not Implemented | — |

---

## 📋 FIX STATUS

| Priority | Bug ID | Summary | Status |
|:---|:---|:---|:---|
| 1 | BUG-006 | Webhook secret defaults to `"dev_secret"` | ✅ **FIXED** — Now rejects all webhooks if env var is unset |
| 2 | BUG-003 | Withdrawal TOCTOU race condition | ✅ **FIXED** — Wrapped in TX with `FOR UPDATE` lock |
| 3 | BUG-001 | Float→cents in deposit/withdraw handlers | ✅ **FIXED** — String-based `parse_dollars_to_cents()` |
| 4 | BUG-002 | Float→cents in cart add handler | ✅ **FIXED** — String-based parsing |
| 5 | BUG-009 | Add `CHECK (balance_cents >= 0)` to wallets | ✅ **FIXED** — Migration `044_wallet_balance_constraint.sql` |
| 6 | BUG-008 | Order number collision under concurrency | ✅ **FIXED** — Added UUID suffix to order numbers |
| 7 | BUG-004 | Hardcoded FX rate | ✅ **FIXED** — Integrated `open.er-api.com` with robust 1-hour atomic timestamp cache |
| 8 | BUG-010 | CSRF in server-rendered HTML forms | ✅ **FIXED** — Form body parsed in middleware & handled via JS injection |
| 9 | BUG-011 | Stale `draft_asset_id` | ✅ **FIXED** — Cleared on successful submission |
| 10 | BUG-005 | Float in API response | ✅ **FIXED** — Replaced with `amount_display` string |
| 11 | BUG-007 | Cart stale price display | ✅ **FIXED** — Cart query now explicitly binds to live asset token price |
| 12 | BUG-012 | IDR format inconsistency | ✅ **FIXED** — Unified rate to 15,500 |

**12 of 12 bugs fixed.** All critical logics audit objectives achieved.

---

## 🔵 SECURITY AUDIT (Round 2) — 2026-03-18

The following bugs were found during a deep security and logic audit of the admin, auth, payments, cart, and wallet modules.

| Priority | Bug ID | Summary | Severity | Status |
|:---|:---|:---|:---|:---|
| 1 | BUG-A01 | Admin withdrawal approve/reject endpoints missing `AdminUser` extractor — any authenticated user could approve/reject withdrawals | 🔴 Critical | ✅ **FIXED** — Added `_admin: AdminUser` to all 3 handlers |
| 2 | BUG-A02 | `dividend.process` not in four-eyes `valid_actions` whitelist — approval validator rejected legitimate dividend processing requests | 🔴 Critical | ✅ **FIXED** — Added to whitelist in `approvals.rs` |
| 3 | BUG-A03 | Float arithmetic (`as f64 / 100.0`) in wallet API `amount_display` field | 🟡 High | ✅ **FIXED** — Integer-only formatting |
| 4 | BUG-A04 | Cart `add_to_cart` TOCTOU race — `tokens_available` read without `FOR UPDATE` lock | 🟡 High | ✅ **FIXED** — Wrapped in transaction with `FOR UPDATE` |
| 5 | BUG-A05 | FX rate cache using `Ordering::Relaxed` — potential torn reads between rate and timestamp | 🟡 High | ✅ **FIXED** — Upgraded to `Ordering::SeqCst` |
| 6 | BUG-A06 | Deposit requests accept unlimited amounts (no max validation) | 🟡 High | ✅ **FIXED** — Added `MAX_DEPOSIT_CENTS` on both entry points |
| 7 | BUG-A07 | `approve_order`/`reject_order` audit log recorded customer as actor instead of admin | 🟠 Medium | ✅ **FIXED** — Added `admin_user_id` parameter |
| 8 | BUG-A08 | Two `format_idr` functions with different logic: cart (float+comma) vs payments (integer+dot) | 🟠 Medium | ✅ **FIXED** — Cart uses integer math + dot separators; created shared `common/currency.rs` |
| 9 | BUG-A09 | Withdrawal rejection fetched refund amount *after* writing rejection status | 🟠 Medium | ✅ **FIXED** — Single `FOR UPDATE` query fetches status+amount atomically |
| 10 | BUG-A10 | `api_admin_disputes_status_update` used manual `is_admin()` instead of `AdminUser` extractor, no input validation, no audit logging | 🟡 High | ✅ **FIXED** — Uses `AdminUser`, validates status values, logs to audit_logs |
| 11 | BUG-A11 | Tax report generation queries all-time data ignoring `fiscal_year` parameter | 🟠 Medium | ✅ **FIXED** — Added date range filter |
| 12 | BUG-A12 | Role update audit log used target user's ID as `actor_user_id` instead of admin's | 🟠 Medium | ✅ **FIXED** — Uses `_admin.user.id` |
| 13 | BUG-A13 | CSRF coverage audit: confirmed global `fetch()` interceptors exist in `head.html` (investor pages) and `admin-permission-guard.js` (admin pages) — all 30 admin and all investor pages are covered | ✅ Verified | ✅ **VERIFIED** — No action needed; interceptors auto-inject `X-CSRF-Token` |
| 14 | BUG-A14 | Float arithmetic in payments routes (checkout page wallet display, deposit success) | 🟠 Medium | ✅ **FIXED** — Integer-only formatting |
| 15 | BUG-A15 | `update_cart_item` had same TOCTOU race as `add_to_cart` | 🟡 High | ✅ **FIXED** — User applied transactional fix |

**15 of 15 security audit bugs fixed.**

---

## 🛡 Phase 3: Hardening (2026-03-18)

| # | Item | Status |
|:--|:-----|:-------|
| H01 | Password reset tokens stored as SHA-256 hashes | ✅ **Already implemented** — `config::hash_token()` uses SHA-256 |
| H02 | Webhook secret enforcement | ✅ **Already implemented** — rejects all webhooks if `PAYMENT_WEBHOOK_SECRET` is unset |
| H03 | Rate limiting on auth endpoints (login, signup, forgot-password) | ✅ **IMPLEMENTED** — 10 req/15min per IP via in-memory rate limiter with Retry-After header |
| H04 | Expired session cleanup worker | ✅ **IMPLEMENTED** — Background task purges expired sessions every 6 hours |
| H05 | Expired password reset token cleanup | ✅ **IMPLEMENTED** — Purges used/expired tokens every 6 hours |
| H06 | Expired email verification token cleanup | ✅ **IMPLEMENTED** — Purges expired tokens every 6 hours |
| H07 | Rate limiter memory cleanup | ✅ **IMPLEMENTED** — Background task cleans stale entries every 10 minutes |

---

## 🛠 New Shared Modules Created

| Module | Purpose |
|:---|:---|
| `backend/src/common/currency.rs` | Centralized currency formatting (format_usd, format_idr, format_amount_display) with unit tests |
| `backend/src/auth/rate_limit.rs` | In-memory per-IP rate limiter with sliding window, integrated into login/signup/forgot-password |

---

## 🔍 Phase 4: Deep Module Sweep (2026-03-18)

| # | Item | Severity | Status |
|:--|:-----|:---------|:-------|
| S01 | FX conversion in `payments/service.rs` used float arithmetic for financial calculation | 🟡 High | ✅ **FIXED** — Integer math: `(total_cents / 100) * rate_i64` |
| S02 | Remaining `as f64 / 100.0` formatting in payments Sentry breadcrumbs | 🟢 Low | ✅ **FIXED** — Replaced with `common::currency::format_usd()` |
| S03 | `kyc/didit.rs` unclosed delimiter (missing `}` for impl block) broke compilation | 🟠 Medium | ✅ **FIXED** |
| S04 | Wallet module audit | ✅ Clean | `parse_dollars_to_cents` integer-only, `handle_withdraw` uses FOR UPDATE lock |
| S05 | Cart module audit | ✅ Clean | Already transactional from earlier fix |
| S06 | Developer module float usage | ✅ Clean | Display-only percentages, acceptable |
| S07 | Assets module float usage | ✅ Clean | Display-only yield/appreciation, acceptable |
| S08 | `kyc/didit.rs` `get_session_result` called `process_webhook` with `None` signature — always fails when `DIDIT_WEBHOOK_SECRET` is set | 🟡 High | ✅ **FIXED** — Extracted `parse_didit_body()` shared by both code paths |
| S09 | 13 compiler dead-code warnings | 🟢 Low | ✅ **FIXED** — Wired 4 change-request routes; `#[allow(dead_code)]` on utilities |
| S10 | Duplicate `api_kyc_submit` in `main.rs` — superseded by `kyc` module's own router | 🟢 Low | ✅ **FIXED** — Removed by user |
| S11 | Alpine.js loaded with floating version `3.x.x` — vulnerable to supply chain risk | 🟢 Low | ✅ **FIXED** — Pinned to `3.14.9` |
| S12 | `rewards-liability` report returned all-time data ignoring date filters | 🟠 Medium | ✅ **FIXED** — Added `WHERE` clause binding `date_from`/`date_to` |
---

## 🛡 Phase 5: Production Hardening (2026-03-18)

| # | Item | Status |
|:--|:-----|:-------|
| P01 | **Redis-backed rate limiter** — shared across Cloud Run instances via sorted sets | ✅ **IMPLEMENTED** — Auto-selects Redis when `REDIS_URL` is set, falls back to in-memory. Fails open if Redis is unavailable. |
| P02 | **XSS sanitization** — `common/sanitize.rs` with `strip_tags`, `sanitize_text`, `sanitize_multiline`, `sanitize_url` | ✅ **IMPLEMENTED** — Integrated into developer draft create & update endpoints. 7 unit tests. |
| P03 | **Comprehensive E2E test suite** — `tests/test_e2e.py` with 12 test categories | ✅ **IMPLEMENTED** — Auth, wallet, cart, marketplace, settings, rewards, XSS, rate limiting, concurrency, admin, DB integrity, portfolio. |

---

## 🛠 New Shared Modules Created

| Module | Purpose |
|:---|:---|
| `backend/src/common/currency.rs` | Centralized currency formatting (format_usd, format_idr, format_amount_display) with unit tests |
| `backend/src/common/sanitize.rs` | HTML tag stripping, text/URL sanitization for XSS prevention, 7 unit tests |
| `backend/src/auth/rate_limit.rs` | Trait-based rate limiter with in-memory + Redis backends, 3 unit tests |
| `tests/test_e2e.py` | Comprehensive E2E test suite (12 categories, ~40 test cases) |
| `tests/test_security_audit.py` | Security audit test suite (8 test categories) |

---

*Last Updated: 2026-03-18 02:10 ICT*

