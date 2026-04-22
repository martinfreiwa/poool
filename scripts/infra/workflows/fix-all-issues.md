---
description: Fix all identified bugs, logic errors, and UI issues across wallet, cart, and portfolio pages
---

# POOOL Platform — Deep Audit Issues & Fix Workflow

Full audit across frontend, backend logic, database connections, value calculations, and UI rendering.

---

## 🔴 CRITICAL BUGS

### 1. Broken Visa image — file format mismatch
**Location:** `backend/src/wallet/routes.rs` lines 539-543, 710-711
**Problem:** Backend references `/static/images/visa.png` but the actual file is `visa.webp`. The `<img>` tag loads a non-existent file, showing a broken image icon.
**Fix:** Change all references from `visa.png` to `visa.webp` in wallet/routes.rs (lines 540 and 711).

### 2. ~~`ensure_wallets()` inserts a `currency` column that doesn't exist in the DB schema~~
**Status:** ✅ NOT A BUG — The `currency` column **was added** in migration `005_payments_checkout.sql` via `ALTER TABLE wallets ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD'`. The initial schema (001) didn't have it, but the live DB does.

### 3. `is_multiple_of()` is a nightly-only Rust feature
**Location:** `backend/src/wallet/routes.rs` line 39, `backend/src/payments/routes.rs` line 544
**Problem:** `(b.len() - i).is_multiple_of(3)` uses a nightly-only method from `feature(unsigned_is_multiple_of)`. On stable Rust, this won't compile.
**Fix:** Replace with `(b.len() - i) % 3 == 0`.

### 4. `format_usd()` truncates cents — all amounts lose decimal precision
**Location:** `backend/src/wallet/routes.rs` lines 32-45
**Problem:** `format_usd()` does `let dollars = cents / 100;` (integer division, drops remainder). A balance of 15050 cents ($150.50) shows as "USD 150". All deposit/withdrawal amounts in the transaction table are rounded down.
**Fix:** Use fractional formatting: `format!("USD {}.{:02}", cents / 100, (cents % 100).abs())` or use `f64` formatting.

### 5. Transaction amounts show purchase amounts as positive instead of negative
**Location:** `backend/src/wallet/routes.rs` lines 413-426
**Problem:** The display logic checks `if *amount >= 0` for the sign prefix. Purchases are stored as **negative** in `wallet_transactions` (`-amount_cents`). But in the screenshot, "purchase" rows show `- USD 266.80` which is correct. However, deposits also stored with `-amount_cents` as noted in withdrawal handler (line 255) would cause confusion. The key issue: the deposit handler stores positive values (line 164: `$2` = `amount_cents`, positive), but the withdrawal handler stores negative (line 255: `.bind(-amount_cents)`). This inconsistency means:
  - Deposits show as `+ USD 100.00` ✅
  - Withdrawals show as `- USD X.XX` ✅
  - But purchases (from checkout) also need to be negative — verified via `payments/service.rs`

### 6. Wallet page balance replacement uses brittle string matching
**Location:** `backend/src/wallet/routes.rs` lines 596-629
**Problem:** The backend replaces hardcoded strings like `USD 2,732` and `USD 1,700` in the HTML template with real values. If anyone changes the template's placeholder text, the replacement silently fails and users see stale hardcoded values. This is the likely cause of the balance cards issue seen in the screenshot.
**Fix:** Use minijinja template variables instead of string replacement. Pass `cash_balance`, `rewards_balance`, `asset_balance` as template context.

---

## 🟡 MEDIUM ISSUES

### 7. Cart page serves static hardcoded HTML when items exist
**Location:** `backend/src/cart/routes.rs` lines 416-449, 744-793
**Problem:** The backend does `html.find("<!-- Cart Content -->")` and `html.find("<!-- Empty Cart State -->")` markers to splice dynamic content into the template. If these HTML comments are removed, modified, or duplicated during template refactoring, the splice fails silently and the page shows the wrong content (static demo items instead of real data, or vice versa).
**Fix:** Migrate to proper Jinja template rendering with `{% for item in items %}` blocks.

### 8. Cart `page_cart()` queries payment methods twice
**Location:** `backend/src/wallet/routes.rs` lines 499-586 and 680-756
**Problem:** The wallet handler calls `payment_methods::service::list_user_payment_methods()` at line 499 and ALSO runs a second `SELECT * FROM payment_methods` query at line 680. Both results are independently processed and injected into the same HTML placeholders (`<!-- CARDS_PLACEHOLDER -->`, etc.). The second injection at line 781 **overwrites** the first one, making lines 499-591 completely dead code.
**Fix:** Remove the duplicate query and keep only one.

### 9. Cart total calculation doesn't use commas — displays as "USD 4935" not "USD 4,935"
**Location:** `backend/src/cart/routes.rs` line 668
**Problem:** `format!("USD {}", total_cents / 100)` — no thousands separator formatting. The screenshot confirms "USD 4935" without comma.
**Fix:** Add comma formatting like in `format_usd()` (once that's also fixed for cents precision).

### 10. Checkout webhook has no signature verification
**Location:** `backend/src/payments/routes.rs` lines 97-139
**Problem:** The `payment_webhook()` handler has `// TODO: In production, verify webhook signature per provider`. Any attacker can POST to this endpoint and credit any user's wallet.
**Fix:** Implement HMAC signature verification before processing webhooks.

### 11. `add_to_cart` ON CONFLICT accumulates quantity without limit
**Location:** `backend/src/cart/routes.rs` lines 139-153
**Problem:** The upsert `SET tokens_quantity = cart_items.tokens_quantity + $3` keeps adding tokens with no cap. A user could add more tokens than exist (`tokens_available`). The frontend sends unlimited `handleQuantityChange()` clicks.
**Fix:** Add a server-side check: `MIN(cart_items.tokens_quantity + $3, a.tokens_available)`.

### 12. Portfolio page serves static HTML without dynamic data injection
**Location:** `backend/src/portfolio/routes.rs` lines 48-51
**Problem:** `page_portfolio()` calls `serve_protected()` which just renders the template without any dynamic data. The portfolio page shows hardcoded values ($34,508, $46 monthly income, etc.) from the HTML template. The real data is only available via `GET /api/portfolio` (JSON), but the portfolio page doesn't fetch it client-side.
**Fix:** Either inject data server-side (like wallet.ts does) or add a JS `fetch('/api/portfolio')` on page load to populate values.

### 13. Withdrawal handler uses unwrap() on `pool.begin()` — can panic
**Location:** `backend/src/wallet/routes.rs` lines 217, 289
**Problem:** `state.db.begin().await.unwrap()` — if the DB connection pool is exhausted, this panics and crashes the server. The deposit handler correctly uses `match` (line 124-130).
**Fix:** Replace `.unwrap()` with proper `match` error handling.

### 14. No CSRF protection on sensitive form actions
**Location:** Cart remove, wallet deposit/withdraw, checkout
**Problem:** All POST form actions (deposit, withdraw, cart remove, checkout) accept standard form submissions without any CSRF token. An attacker could craft a page that auto-submits a form to `/wallet/withdraw`.
**Fix:** Add a CSRF token to all forms and validate on the server.

---

## 🟢 LOWER PRIORITY ISSUES

### 15. Wallet `currency` column inconsistency between routes
**Location:** `payments/routes.rs` line 496 vs `wallet/routes.rs` line 51
**Problem:** `GET /api/wallets` selects `currency` from wallets table, but wallet DB schema doesn't have this column. This API endpoint would fail with a runtime SQL error.
**Fix:** Part of Issue #2 — either add migration or fix all queries.

### 16. Transaction table limited to 10 rows with no pagination
**Location:** `backend/src/wallet/routes.rs` line 383
**Problem:** `LIMIT 10` — users with more than 10 transactions can never see their history.
**Fix:** Add pagination query params (`?page=2`) and a "Load more" button.

### 17. `format_usd` returns "USD -X" for negative dollar amounts
**Location:** `backend/src/wallet/routes.rs` lines 32-45
**Problem:** For negative cents (e.g., -500), `dollars = -500/100 = -5`, so output is `USD -5` but the sign gets embedded in the comma-formatted string. Combined with the prefix logic in line 426, withdrawal amounts would show `- USD -5.00` (double negative).
**Fix:** Use `cents.abs()` in the formatter and handle sign separately.

### 18. Cart item card links use wrong URL format
**Location:** `backend/src/cart/routes.rs` line 499
**Problem:** Links use `/property?id={slug}` but the route is actually `/property/{slug}` based on standard slug-based routing. This means clicking a cart item may lead to a 404 or wrong page.
**Fix:** Change to `/property/{slug}`.

### 19. Admin order approval/rejection has no RBAC — any logged-in user can try
**Location:** `backend/src/payments/routes.rs` lines 697-734
**Problem:** The handler calls `middleware::is_admin()` which provides basic protection, but the admin middleware check is not applied at the router level with middleware guards. A determined attacker could bypass this.
**Fix:** Apply admin middleware at the router level for all `/api/admin/*` routes.

### 20. Cart page `proof_of_transfer` file upload is silently discarded
**Location:** `backend/src/payments/routes.rs` line 250-252
**Problem:** `let _ = field.bytes().await;` — the "proof of transfer" file uploaded during checkout is read and immediately discarded. Users think they've uploaded proof but it's never stored.
**Fix:** Store the file to disk/cloud storage and link to the order.

### 21. No rate limiting on deposit/withdraw endpoints
**Location:** POST `/wallet/deposit`, POST `/wallet/withdraw`
**Problem:** An attacker could spam deposit/withdraw requests. No rate limiting middleware is applied.
**Fix:** Add rate limiting middleware (e.g., tower-governor).

---

## IMPLEMENTATION ORDER

1. **Issue #2** (currency column) — This is likely causing ALL wallet data to fail silently
2. **Issue #3** (is_multiple_of) — Compile-time failure on stable Rust
3. **Issue #1** (visa.webp) — Simple filename fix
4. **Issue #4** (format_usd cents) — Data display corruption
5. **Issue #6** (template string matching) — Root cause of stale balance cards
6. **Issue #9** (cart total formatting)
7. **Issue #13** (unwrap panics)
8. **Issue #11** (cart unlimited tokens)
9. **Issue #12** (portfolio static data)
10. **Issue #18** (cart link URLs)
11. Remaining security issues (#10, #14, #19, #20, #21)

---

## VERIFICATION

After implementing fixes:
1. Reload `/wallet` — verify balance cards render with real DB values
2. Verify Visa card image appears correctly
3. Make a deposit and verify the amount shows with correct cents (e.g., `USD 100.00` not `USD 100`)
4. Check `/cart` — verify total shows with comma formatting and correct total
5. Verify wallet transaction amounts are accurate and signs are correct
6. Run `cargo build --release` to confirm no nightly-only features are used
