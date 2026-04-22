---
description: Comprehensive End-to-End (E2E) testing framework detailing strict business logic, edge cases, UI states, and database invariants for the entire POOOL platform.
---

# 🚀 POOOL E2E Testing Master Strategy & Logic Guide

This document defines the **exakte Logik und Struktur** that every End-to-End test must follow. It is the blueprint for writing our automated Pytest/Playwright test suite. Instead of just checking if a page loads, these tests verify the entire lifecycle of data: from User Input -> Frontend UI -> Backend API -> Database State -> Return to UI.

**Core Testing Principles:**
1. **End-to-End Traceability:** Every UI action must be verified in the database.
2. **Atomic Integrity:** Financial transactions (deposits, purchases) must not leave partial states if they fail.
3. **Negative Testing:** Testing what happens on invalid inputs (e.g., negative purchase amounts) is as important as testing the happy path.

---

## 1. System-Wide Database Invariants (Core Rules)
*Every E2E test suite tear-down must verify these rules are never broken.*

- **The Ledger Rule:** `SUM(amount) FROM wallet_transactions WHERE wallet_id = X` **MUST EXACTLY EQUAL** `balance FROM wallets WHERE id = X`.
- **Zero Negative Wealth:** A user's `wallet_balance` must never be `< 0`.
- **Orphan Prevention:** No `cart_items` may exist for an `asset_id` that is deleted or inactive.
- **Investment Consistency:** The sum of `tokens_purchased` for an asset across all users must never exceed `total_tokens` available for that asset.

---

## 2. Module: Authentication & Identity

### 2.1. Registration (`/register`)
- **UI State & Logic:**
  - **Happy Path:** User enters valid email, secure password (>8 chars), selects Terms & Conditions. Submit button shows loading spinner. Forwarded to `/marketplace`.
  - **Edge Cases:** Invalid email format (UI blocks). Weak password. Duplicate email (API returns 409 Conflict, UI shows "Email already in use").
- **API Assertions:** POST `/api/auth/register` must return `201 Created` or `302 Found`.
- **Database Assertions:**
  - `users` table: 1 row added. `password_hash` is not plain text.
  - `user_profiles` table: 1 row added linked to `user_id`.
  - `wallets` table: 1 row created for `user_id` with `balance = 0`.

### 2.2. Login & Session Management (`/login`)
- **UI State & Logic:**
  - Login form requires valid credentials. HTMX handles the error state (red text injection) if 401 Unauthorized is returned.
- **API & Background:**
  - POST `/api/auth/login`. Returns `Set-Cookie: poool_session=...; HttpOnly; Secure`.
- **Database Assertions:**
  - `sessions` table (or Redis): Session token is generated and valid.

### 2.3. KYC Verification State (`/kyc`)
- **Logic Matrix:**
  - *Not Started:* User sees "Complete KYC" banner. Checkout is BLOCKED. 
  - *Pending:* Banner turns blue ("Under Review"). Checkout remains BLOCKED.
  - *Approved:* Banner disappears. Checkout is UNLOCKED.
- **Test Flow:** Simulate KYC provider webhook calling our backend -> Verify user state transitions from `pending` to `approved` -> Refresh Cart page -> Verify Checkout button is now clickable.

---

## 3. Module: Investor Financial Core (The Engine)

### 3.1. Wallet & Ledger Management (`/wallet`)
- **Deposit Logic (Fiat via Bank Transfer):**
  - **Action:** User requests €500 deposit. 
  - **Test:** Verify UI shows instructions. (In a real scenario, this is manual, but we abstract a mock webhook).
  - **DB Check:** `wallet_transactions` gets row `type='deposit'`, `status='pending'`.
- **Withdrawal Logic:**
  - **Action:** User has €1000 balance, requests €1500 withdrawal.
  - **Assertion:** Backend blocks transaction (400 Bad Request). UI shows "Insufficient Funds".
  - **Action:** User requests €500 withdrawal. 
  - **Assertion:** Wallet balance drops to €500 immediately (`Pending` deduction). 

### 3.2. Cart System Logic
- **Addition & Constraint Logic:**
  - **Action:** Add 5 tokens of Asset A to cart.
  - **DB Check:** `cart_items` table gets 1 row.
  - **Edge Case Test:** Attempt to add 0 or negative tokens via API manipulation. Server MUST reject.
  - **Edge Case Test:** Attempt to add more tokens than the asset currently has available. Server MUST reject.
- **Calculation Logic:**
  - Cart UI must dynamically sum `(Token Price * Quantity) + POOOL Fees (if any) = Total Value`.
  - Changing token quantity updates the Total Value instantly.

### 3.3. The Atomic Checkout Flow (CRITICAL)
- **State Prerequisites:** User must be logged in, KYC Approved, Cart > 0 items, Wallet Balance >= Cart Total.
- **Execution Logic (Happy Path):**
  1. User clicks "Confirm Purchase".
  2. **DB Transaction Starts.**
  3. Deduct total cost from `wallets`.
  4. Log `wallet_transactions` row as `investment`.
  5. Delete items from `cart_items`.
  6. Create ownership records in `investments`.
  7. Increment `funding_progress` on the `assets` table.
  8. **DB Transaction Commits.**
- **Failure Logic (The ACID Test):**
  - Simulate a database failure exactly at step 6 (e.g., throwing a mock error).
  - **Assertion:** The transaction must ROLLBACK entirely. The user's wallet balance must NOT be deducted. The cart items must REMAIN in the cart.

---

## 4. Module: The Marketplace & Assets

### 4.1. Browsing & Filtering (`/marketplace`)
- **Logic:**
  - HTMX data-attribute filtering. Checking the "Real Estate" filter must append `?category=real_estate` to the URL.
  - The UI must dynamically swap out the grid ensuring ONLY Real Estate assets exist.
  - **Assertion:** API `GET /api/marketplace/assets?category=real_estate` payload matches UI render precisely.

### 4.2. Asset Property Data Integrity
- **Logic:**
  - Asset Detail Page must correctly calculate dynamic metrics:
    - `Available Tokens = Total Tokens - Sold Tokens`
    - `Funding Progress % = (Sold Tokens / Total Tokens) * 100`
  - If `Funding Progress == 100%`, the "Add to Cart" button MUST be replaced with "Fully Funded / Sold Out".

---

## 5. Module: Developer Ecosystem

### 5.1. Asset Application Pipeline (`/developer/add-asset`)
- **Step-by-Step State Conservation:**
  - **Step 1 (Basic Info):** Submit valid title and description. DB saves as `status = draft`.
  - **Step 2 (Financials):** Submit target amount. Must reject negative numbers.
  - **Step 3 (Documents):** Upload PDF. 
  - **Final Submission:** Changes status from `draft` to `pending_review`.
- **Security Check:** 
  - Developer A tries to edit Developer B's draft by manipulating the URL `UUID`. Server MUST return `403 Forbidden`.

### 5.2. Dashboard Display
- **Logic:**
  - `My Assets` page segregates assets by status (`Draft`, `In Review`, `Funding`, `Funded`).
  - Progress bars must strictly map to the exact `funding_progress` recorded in the database.

---

## 6. Module: Administration & Control

### 6.1. Platform Authority (`/admin`)
- **Security Logic:**
  - Attempt to access `/admin/*` endpoints with a standard User session.
  - **Assertion:** All requests blocked. No data leakage.
- **User Suspension Test:**
  - Admin clicks "Suspend User".
  - **Assertion:** `users.is_suspended` becomes `true`.
  - **Assertion:** The suspended user's active session is immediately revoked (they are forcibly logged out on their next request).

### 6.2. KYC Administration 
- **Approval Logic:**
  - Admin reviews pending KYC doc. Clicks "Approve".
  - **Assertion:** `kyc_records` updated. The user immediately gains access to Checkout capability on the frontend without needing to log out and log back in (system dynamically checks status).

---

## 7. Test Execution Guidelines

When writing the Python code for these tests in `/tests`:

1. **Isolation:** Every test must start with a clean state. Use a test database that drops and rebuilds schemas or use SQL transaction rollbacks after every test function.
2. **Selectors:** Always use `data-testid="..."` or robust HTML `id` components in Playwright (e.g., `page.click("#-banner-complete-btn")`) instead of brittle CSS classes.
3. **Wait States:** Do not use `time.sleep()`. Rely on Playwright's auto-wait for network requests returning `200 OK` or UI elements becoming `visible`.
