---
description: Make the cart and payment system fully production-ready with real DB values, end-to-end testing, and verification
---

# Cart & Payment System – Production Readiness Workflow

This workflow makes the entire cart → checkout → payment flow production-ready. It covers backend correctness, database integrity, frontend wiring, and full end-to-end test verification.

> **Stack reminder:** Rust/Axum backend, PostgreSQL via SQLx, plain HTML + Vanilla JS frontend, Python test suite at `tests/test_platform.py`.

---

## Phase 0: Pre-flight – Confirm the System is Running

Before making any changes, confirm the baseline state.

// turbo
1. Start the backend (use the `/start-backend` workflow if needed):
   ```bash
   cd /Users/martin/Projects/poool/backend && cargo run 2>&1 | head -30
   ```

2. Check the database is reachable and has data:
   ```bash
   psql -d poool -c "SELECT COUNT(*) FROM assets WHERE published = true AND tokens_available > 0;"
   psql -d poool -c "SELECT COUNT(*) FROM wallets;"
   psql -d poool -c "SELECT wallet_type, currency, balance_cents FROM wallets WHERE user_id = (SELECT id FROM users WHERE email='test@poool.app');"
   ```
   **Expected:** At least 1 published asset with tokens, wallets exist for the test user.

3. Run the existing test suite to capture the baseline failure count:
   ```bash
   cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | tail -30
   ```

---

## Phase 1: Database – Ensure Cart & Payment Schema is Complete

All tables must exist with correct constraints, indexes, and seed data.

1. **Verify all required tables exist:**
   ```bash
   psql -d poool -c "\dt" | grep -E "cart_items|orders|order_items|invoices|deposit_requests|wallet_transactions|investments"
   ```
   **Expected tables:** `cart_items`, `orders`, `order_items`, `invoices`, `deposit_requests`, `wallet_transactions`, `investments`

2. **Verify the `invoice_number_seq` sequence exists** (used in `service.rs` to generate `INV-YYYY-NNNNN`):
   ```bash
   psql -d poool -c "SELECT sequence_name FROM information_schema.sequences WHERE sequence_name = 'invoice_number_seq';"
   ```
   If missing, create it:
   ```bash
   psql -d poool -c "CREATE SEQUENCE IF NOT EXISTS invoice_number_seq START 1;"
   ```

3. **Verify wallets have a `currency` column and UNIQUE constraint** on `(user_id, wallet_type, currency)`:
   ```bash
   psql -d poool -c "\d wallets"
   ```
   The constraint `UNIQUE (user_id, wallet_type, currency)` is critical for upsert logic in `service.rs`.

4. **Ensure the test user has a funded USD cash wallet** so wallet checkout can be tested:
   ```bash
   psql -d poool -c "
     INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
     SELECT id, 'cash', 'USD', 500000
     FROM users WHERE email='test@poool.app'
     ON CONFLICT (user_id, wallet_type, currency)
     DO UPDATE SET balance_cents = GREATEST(wallets.balance_cents, 500000);
   "
   ```
   This gives the test user \$5,000 USD to test wallet payments.

5. **Ensure at least one published asset with enough tokens exists:**
   ```bash
   psql -d poool -c "
     UPDATE assets SET published = true, tokens_available = 1000
     WHERE id = (SELECT id FROM assets ORDER BY created_at LIMIT 1);
   "
   ```

---

## Phase 2: Backend – Verify All Cart & Payment Routes Work

Read the key backend files to understand the current state:
- `backend/src/cart/routes.rs` — `add_to_cart`, `remove_from_cart`, `update_cart_item`, `api_cart`, `page_cart`
- `backend/src/payments/routes.rs` — `checkout_page`, `handle_checkout`, `initiate_deposit`, `payment_webhook`, `list_wallets`, `api_latest_order`
- `backend/src/payments/service.rs` — `execute_checkout`, `confirm_deposit`, `create_deposit_request`

### 2a. Verify all routes are registered in `main.rs`

```bash
grep -E "cart|checkout|payment|wallet" /Users/martin/Projects/poool/backend/src/main.rs
```

Ensure these routes exist:
- `GET /cart` → `cart::routes::page_cart`
- `POST /cart/add` → `cart::routes::add_to_cart`
- `POST /cart/remove` → `cart::routes::remove_from_cart`
- `POST /cart/update` → `cart::routes::update_cart_item`
- `GET /api/cart` → `cart::routes::api_cart`
- `GET /checkout` → `payments::routes::checkout_page`
- `POST /checkout` → `payments::routes::handle_checkout`
- `GET /api/wallets` → `payments::routes::list_wallets`
- `POST /api/payments/deposit` → `payments::routes::initiate_deposit`
- `POST /api/webhooks/payments` → `payments::routes::payment_webhook`
- `GET /api/orders/latest` → `payments::routes::api_latest_order`

If any are missing, add them to the router in `main.rs` or the relevant module's `router()` function.

### 2b. Fix known issues

**Issue 1: `checkout.js` sends `currency` but `handle_checkout` reads `payment_currency`**

In `frontend/platform/static/js/checkout.js` line 253, `formData.append("currency", selectedCurrency)` but the Rust handler at `backend/src/payments/routes.rs` reads the field as `payment_currency`. Fix the JS to send the correct field name:
- Open `frontend/platform/static/js/checkout.js`
- Change `formData.append("currency", selectedCurrency)` → `formData.append("payment_currency", selectedCurrency)`

**Issue 2: `checkout.js` redirect logic doesn't match backend HX-Redirect header**

The backend `handle_checkout` sends `HX-Redirect: /payment-success` header, but the frontend tries to read `resp.json()` first. Fix `handleCheckout()` in `checkout.js` to check for the `HX-Redirect` header first:
```javascript
const redirect = resp.headers.get('HX-Redirect');
if (redirect) {
  window.location.href = redirect;
  return;
}
```
Insert this check immediately after `if (resp.ok)` in the `handleCheckout` function.

**Issue 3: Wallet checkout uses hardcoded IDR FX rate**

In `backend/src/payments/service.rs`, the IDR exchange rate is hardcoded as `15_500.0`. This is acceptable for an MVP but add a `TODO` comment so it's clearly marked for future live FX rate integration.

**Issue 4: `approve_order` in `service.rs` doesn't update investments by `order_id`**

Line 637 in `service.rs`:
```rust
UPDATE investments SET status = 'active' WHERE order_id = $1 ...
```
But the `investments` table may not have an `order_id` column (check the schema). If missing, investments are linked via `user_id + asset_id`. Update the query accordingly:
```bash
psql -d poool -c "\d investments"
```
If there's no `order_id` column, the `approve_order` logic silently does nothing for investments. This must be fixed.

### 2c. Build the backend and confirm it compiles

```bash
cd /Users/martin/Projects/poool/backend && cargo build 2>&1 | grep -E "^error|^warning\[" | head -30
```

Fix any compilation errors before proceeding.

---

## Phase 3: Frontend – Wire Up Checkout Page Correctly

Read `frontend/platform/checkout.html` and `frontend/platform/static/js/checkout.js` to verify alignment.

1. **Verify the checkout form posts to `/checkout` with multipart encoding** (required by `handle_checkout` which uses `axum::extract::Multipart`). The checkout JS uses `fetch` with `FormData` — this is correct.

2. **Verify the wallet payment flow shows real balance from `/api/wallets`:**
   - `checkout.js` calls `fetch("/api/wallets")` and reads `wallets.find(w => w.wallet_type === "cash")`.
   - The `list_wallets` backend returns `wallet_type`, `currency`, `balance_cents` — confirm the JS accesses the right field (`w.wallet_type`, not `w.type`).
   - Check the current field name: in `payments/routes.rs` line 462, the JSON uses `"type": wtype` — the JS reads `w.wallet_type`. **This is a bug** — fix `list_wallets` to return `"wallet_type"` instead of `"type"`:
     ```rust
     // Change: "type": wtype
     // To:     "wallet_type": wtype
     ```

3. **Verify payment success page at `/payment-success`:**
   - Check `frontend/platform/payment-success.html` exists and loads the latest order from `/api/orders/latest`.
   - Verify `frontend/platform/static/js/payment-success.js` correctly displays the order details.

4. **Verify payment-in-progress page at `/payment-in-progress`:**
   - For bank transfer orders (status = `pending`), the system should redirect here.
   - Check `frontend/platform/payment-in-progress.html` and `payment-in-progress.js` exist and work.

---

## Phase 4: End-to-End Manual Test (Browser)

Run the backend and test the full user journey:

1. Log in as `test@poool.app`.
2. Navigate to `/marketplace`.
3. Click on a property and add it to cart using the "Add to Cart" button.
4. Navigate to `/cart` — verify the item appears with correct price, title, and image.
5. Change the quantity using `+`/`-` buttons — verify the price updates and persists on refresh.
6. Click "Proceed to Payment" — verify redirect to `/checkout`.
7. On `/checkout`:
   - Verify cart items are displayed (fetched from `/api/cart`).
   - Verify the wallet balance is shown (fetched from `/api/wallets`).
   - Verify the wallet balance shows green if the balance is sufficient, red/amber if not.
8. Select **Wallet payment**, tick the terms checkbox, click "Confirm & Invest".
9. Verify redirect to `/payment-success` with order details.
10. Check the database:
    ```bash
    psql -d poool -c "SELECT order_number, status, total_cents, payment_method FROM orders ORDER BY created_at DESC LIMIT 3;"
    psql -d poool -c "SELECT tokens_owned, status FROM investments ORDER BY created_at DESC LIMIT 3;"
    psql -d poool -c "SELECT invoice_number, status FROM invoices ORDER BY issued_at DESC LIMIT 3;"
    psql -d poool -c "SELECT type, status, amount_cents FROM wallet_transactions ORDER BY created_at DESC LIMIT 5;"
    ```
    **Expected:** 1 order (completed), 1 investment (active), 1 invoice (issued), wallet transaction recorded.

---

## Phase 5: Add Dedicated Cart & Payment Tests to `test_platform.py`

Extend `tests/test_platform.py` with a `test_cart_and_payment_e2e` function that tests the full purchase flow programmatically.

Add the following function to `tests/test_platform.py` (before the `main()` function):

```python
def test_cart_and_payment_e2e(session, results: TestResults):
    """End-to-end test: add to cart, view cart, proceed to checkout, wallet checkout."""
    results.section("E2E: Cart → Checkout → Payment Flow")

    import psycopg2
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()

    # 1. Get test user and a valid asset
    cur.execute("SELECT id FROM users WHERE email = %s", (TEST_EMAIL,))
    user_row = cur.fetchone()
    if not user_row:
        results.fail("Test user not found in database")
        return
    user_id = str(user_row[0])

    cur.execute("SELECT id, token_price_cents FROM assets WHERE published = true AND tokens_available > 0 LIMIT 1")
    asset_row = cur.fetchone()
    if not asset_row:
        results.fail("No published assets with available tokens — cannot test cart flow")
        return
    asset_id, token_price_cents = str(asset_row[0]), asset_row[1]

    # 2. Ensure test user has sufficient wallet balance (>= 1 token price)
    cur.execute("""
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES (%s, 'cash', 'USD', %s)
        ON CONFLICT (user_id, wallet_type, currency)
        DO UPDATE SET balance_cents = GREATEST(wallets.balance_cents, %s)
    """, (user_id, token_price_cents * 5, token_price_cents * 5))
    conn.commit()
    results.ok(f"Test wallet funded with ${token_price_cents * 5 / 100:.2f} USD")

    # 3. Clear existing cart items for test user to ensure clean state
    cur.execute("DELETE FROM cart_items WHERE user_id = %s", (user_id,))
    conn.commit()

    # 4. Add item to cart via POST /cart/add
    add_resp = session.post(
        f"{BASE_URL}/cart/add",
        data={
            "property_id": asset_id,
            "investment_amount": str(token_price_cents // 100)  # 1 token worth
        },
        allow_redirects=False,
    )
    if add_resp.status_code in (302, 303):
        results.ok(f"POST /cart/add → redirect {add_resp.headers.get('Location', '?')}")
    elif add_resp.status_code == 200:
        results.ok("POST /cart/add → 200 OK")
    else:
        results.fail("POST /cart/add", f"status={add_resp.status_code}")
        cur.close(); conn.close(); return

    # 5. Verify cart has the item via GET /api/cart
    cart_resp = session.get(f"{BASE_URL}/api/cart")
    if cart_resp.status_code == 200:
        cart_data = cart_resp.json()
        item_count = cart_data.get("count", 0)
        total_cents = cart_data.get("total_cents", 0)
        if item_count > 0:
            results.ok(f"GET /api/cart: {item_count} item(s), total ${total_cents/100:.2f}")
        else:
            results.fail("GET /api/cart: cart is empty after add_to_cart")
            cur.close(); conn.close(); return
    else:
        results.fail("GET /api/cart", f"status={cart_resp.status_code}")
        cur.close(); conn.close(); return

    # 6. Test cart page renders correctly
    cart_page_resp = session.get(f"{BASE_URL}/cart")
    if cart_page_resp.status_code == 200:
        results.ok("GET /cart returns 200 with items")
        if "cart-item-card" in cart_page_resp.text or "cart-page-content" in cart_page_resp.text:
            results.ok("  Cart page shows item cards")
        else:
            results.warn("  Cart page HTML may not contain expected cart item markup")
    else:
        results.fail("GET /cart", f"status={cart_page_resp.status_code}")

    # 7. Test quantity update via POST /cart/update
    cart_item_id = cart_data["items"][0]["id"]
    update_resp = session.post(
        f"{BASE_URL}/cart/update",
        data={"cart_item_id": cart_item_id, "tokens_quantity": "2"},
        headers={"Content-Type": "application/x-www-form-urlencoded"},
    )
    if update_resp.status_code == 200:
        update_data = update_resp.json()
        if update_data.get("success"):
            results.ok("POST /cart/update → quantity updated to 2")
        else:
            results.warn(f"POST /cart/update → unexpected response: {update_data}")
    else:
        results.fail("POST /cart/update", f"status={update_resp.status_code}")

    # 8. Verify the checkout page is accessible (cart is not empty)
    checkout_resp = session.get(f"{BASE_URL}/checkout", allow_redirects=False)
    if checkout_resp.status_code == 200:
        results.ok("GET /checkout returns 200 (cart is populated)")
        if "checkout" in checkout_resp.text.lower() or "payment" in checkout_resp.text.lower():
            results.ok("  Checkout page content detected")
    elif checkout_resp.status_code in (302, 303):
        loc = checkout_resp.headers.get("Location", "?")
        results.fail(f"GET /checkout redirects to {loc} — cart may be empty or route missing")
        cur.close(); conn.close(); return
    else:
        results.fail("GET /checkout", f"status={checkout_resp.status_code}")
        cur.close(); conn.close(); return

    # 9. Wallet balance API returns correct data
    wallets_resp = session.get(f"{BASE_URL}/api/wallets")
    if wallets_resp.status_code == 200:
        wallets_data = wallets_resp.json()
        wallets = wallets_data.get("wallets", [])
        cash_wallet = next((w for w in wallets if w.get("wallet_type") == "cash" and w.get("currency") == "USD"), None)
        if cash_wallet:
            results.ok(f"GET /api/wallets: USD cash balance = ${cash_wallet['balance_cents']/100:.2f}")
        else:
            results.warn("GET /api/wallets: no USD cash wallet found (checkout wallet payment may fail)")
    else:
        results.fail("GET /api/wallets", f"status={wallets_resp.status_code}")

    # 10. Execute the wallet checkout via POST /checkout
    import io
    checkout_post_resp = session.post(
        f"{BASE_URL}/checkout",
        data={
            "payment_method": "wallet",
            "payment_currency": "USD",
        },
        allow_redirects=False,
    )
    if checkout_post_resp.status_code in (200, 302, 303):
        hx_redirect = checkout_post_resp.headers.get("HX-Redirect", "")
        location = checkout_post_resp.headers.get("Location", "")
        if "payment-success" in hx_redirect or "payment-success" in location:
            results.ok("POST /checkout → redirected to /payment-success ✅")
        else:
            results.ok(f"POST /checkout → status={checkout_post_resp.status_code}, redirect={hx_redirect or location}")
    elif checkout_post_resp.status_code == 400:
        error_html = checkout_post_resp.text
        results.fail("POST /checkout returned 400", error_html[:200])
        cur.close(); conn.close(); return
    else:
        results.fail("POST /checkout", f"status={checkout_post_resp.status_code}")
        cur.close(); conn.close(); return

    # 11. Verify order was created in database
    cur.execute("""
        SELECT order_number, status, total_cents, payment_method
        FROM orders WHERE user_id = %s ORDER BY created_at DESC LIMIT 1
    """, (user_id,))
    order = cur.fetchone()
    if order:
        order_num, status, total, method = order
        results.ok(f"Order created: {order_num}, status={status}, total=${total/100:.2f}, method={method}")
        if status == "completed":
            results.ok("  Order status is 'completed' ✅")
        else:
            results.warn(f"  Order status is '{status}' (expected 'completed' for wallet payment)")
    else:
        results.fail("No order found in database after checkout")

    # 12. Verify investment was created
    cur.execute("""
        SELECT tokens_owned, status FROM investments WHERE user_id = %s
        AND asset_id = %s ORDER BY created_at DESC LIMIT 1
    """, (user_id, asset_id))
    investment = cur.fetchone()
    if investment:
        tokens, inv_status = investment
        results.ok(f"Investment created: {tokens} tokens, status={inv_status}")
    else:
        results.warn("No investment record found after checkout")

    # 13. Verify invoice was generated
    cur.execute("""
        SELECT invoice_number, status FROM invoices WHERE user_id = %s
        ORDER BY issued_at DESC LIMIT 1
    """, (user_id,))
    invoice = cur.fetchone()
    if invoice:
        inv_num, inv_status = invoice
        results.ok(f"Invoice generated: {inv_num}, status={inv_status}")
    else:
        results.warn("No invoice found after checkout")

    # 14. Verify wallet transaction was logged
    cur.execute("""
        SELECT type, status, amount_cents FROM wallet_transactions wt
        JOIN wallets w ON wt.wallet_id = w.id
        WHERE w.user_id = %s AND w.currency = 'USD'
        ORDER BY wt.created_at DESC LIMIT 1
    """, (user_id,))
    tx = cur.fetchone()
    if tx:
        tx_type, tx_status, tx_amount = tx
        results.ok(f"Wallet transaction logged: type={tx_type}, status={tx_status}, amount={tx_amount/100:.2f}")
    else:
        results.warn("No wallet transaction found — wallet deduction may not have been logged")

    # 15. Verify cart was cleared after successful checkout
    cur.execute("SELECT COUNT(*) FROM cart_items WHERE user_id = %s", (user_id,))
    remaining = cur.fetchone()[0]
    if remaining == 0:
        results.ok("Cart cleared after successful checkout ✅")
    else:
        results.fail(f"Cart NOT cleared after checkout — {remaining} items remain")

    # 16. Test GET /api/orders/latest
    latest_order_resp = session.get(f"{BASE_URL}/api/orders/latest")
    if latest_order_resp.status_code == 200:
        order_data = latest_order_resp.json()
        results.ok(f"GET /api/orders/latest: order #{order_data.get('order_number')}")
        for field in ["order_number", "total_cents", "payment_currency", "status", "items"]:
            if field in order_data:
                results.ok(f"  Field '{field}' present")
            else:
                results.warn(f"  Field '{field}' MISSING from /api/orders/latest")
    else:
        results.fail("GET /api/orders/latest", f"status={latest_order_resp.status_code}")

    # 17. Test payment-success page 
    success_resp = session.get(f"{BASE_URL}/payment-success")
    if success_resp.status_code == 200:
        results.ok("GET /payment-success returns 200")
    else:
        results.warn(f"GET /payment-success returned {success_resp.status_code}")

    cur.close()
    conn.close()
```

Then add the call to `main()` after `test_checkout`:
```python
test_cart_and_payment_e2e(session, results)
```

---

## Phase 6: Run the Full Test Suite

// turbo
1. Run the complete test suite:
   ```bash
   cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | tee /tmp/cart_payment_test_results.txt
   ```

2. Check for E2E failures:
   ```bash
   grep -E "❌|FAIL|E2E" /tmp/cart_payment_test_results.txt
   ```

3. Fix each identified failure in the order given:
   - **Backend build failures** → Fix Rust compilation errors first
   - **DB schema failures** → Apply missing migrations
   - **API response failures** → Fix backend route handlers
   - **Frontend JS failures** → Fix field name mismatches (e.g., `currency` vs `payment_currency`)
   - **E2E flow failures** → Fix the specific step identified in the test output

---

## Phase 7: Deposit Flow Verification

Test the bank deposit flow (creates a deposit request, confirms via webhook):

1. **Create a deposit request:**
   ```bash
   curl -s -b "poool_session=$(psql -d poool -tAc "SELECT session_token FROM user_sessions JOIN users ON users.id=user_sessions.user_id WHERE users.email='test@poool.app' AND expires_at>NOW() LIMIT 1")" \
     -X POST http://127.0.0.1:8888/api/payments/deposit \
     -d "amount=100&currency=USD" | head -200
   ```
   **Expected:** HTML response with deposit reference ID (e.g., `STRIPE-20260309-xxxxxxxx`).

2. **Confirm the deposit via webhook:**
   ```bash
   DEPOSIT_REF=$(psql -d poool -tAc "SELECT provider_reference FROM deposit_requests WHERE status='pending' ORDER BY created_at DESC LIMIT 1")
   curl -s -X POST http://127.0.0.1:8888/api/webhooks/payments \
     -H "Content-Type: application/json" \
     -d "{\"provider_reference\":\"$DEPOSIT_REF\",\"status\":\"paid\"}"
   ```
   **Expected:** `{"ok": true, "deposit_id": "..."}`.

3. **Verify wallet was credited:**
   ```bash
   psql -d poool -c "
     SELECT balance_cents FROM wallets
     WHERE user_id = (SELECT id FROM users WHERE email='test@poool.app')
     AND wallet_type = 'cash' AND currency = 'USD';
   "
   ```
   **Expected:** Balance increased by 10000 cents (\$100).

4. **Verify wallet transaction was created:**
   ```bash
   psql -d poool -c "
     SELECT type, status, amount_cents FROM wallet_transactions
     WHERE wallet_id IN (
       SELECT id FROM wallets
       WHERE user_id = (SELECT id FROM users WHERE email='test@poool.app')
       AND wallet_type = 'cash' AND currency = 'USD'
     )
     ORDER BY created_at DESC LIMIT 3;
   "
   ```
   **Expected:** A `deposit` / `completed` row exists.

---

## Phase 8: Admin Order Management Verification

Verify admin can approve/reject bank transfer orders.

1. **Create a bank transfer order** by doing a checkout with `payment_method=bank` (requires an existing cart item):
   ```bash
   psql -d poool -c "
     INSERT INTO cart_items (user_id, asset_id, tokens_quantity, token_price_cents)
     SELECT u.id, a.id, 1, a.token_price_cents FROM users u, assets a
     WHERE u.email='test@poool.app' AND a.published=true AND a.tokens_available>0
     LIMIT 1 ON CONFLICT DO NOTHING;
   "
   ```
   Then POST to `/checkout` with `payment_method=bank` using curl or the test session.

2. **Check the pending order exists:**
   ```bash
   psql -d poool -c "SELECT id, order_number, status FROM orders WHERE status='pending' ORDER BY created_at DESC LIMIT 3;"
   ```

3. **Approve the order via admin API:**
   ```bash
   ORDER_ID=$(psql -d poool -tAc "SELECT id FROM orders WHERE status='pending' ORDER BY created_at DESC LIMIT 1")
   ADMIN_SESSION=$(psql -d poool -tAc "SELECT session_token FROM user_sessions JOIN users ON users.id=user_sessions.user_id WHERE users.email='admin@poool.finance' AND expires_at>NOW() LIMIT 1")
   curl -s -X POST "http://127.0.0.1:8888/api/admin/orders/$ORDER_ID/approve" \
     -b "poool_session=$ADMIN_SESSION"
   ```
   **Expected:** `{"success": true, "message": "Order approved successfully"}`.

4. **Verify the order and investment were updated:**
   ```bash
   psql -d poool -c "SELECT status FROM orders WHERE id='$ORDER_ID';"
   ```
   **Expected:** status = `completed`.

---

## Phase 9: Final Test Run & Pass Criteria

Run the full test suite one final time and confirm:

// turbo
1. Run tests:
   ```bash
   cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | tee /tmp/final_cart_payment_results.txt
   ```

2. Check summary:
   ```bash
   tail -20 /tmp/final_cart_payment_results.txt
   ```

### Pass Criteria

The cart and payment system is considered **production ready** when all of the following pass:

| Test | Expected Result |
|------|----------------|
| `GET /api/cart` | 200, `{"items":[], "count":0, "total_cents":0}` |
| `POST /cart/add` | 302 redirect to `/cart`, item in DB |
| `POST /cart/update` | 200, `{"success":true, "tokens_quantity":N}` |
| `POST /cart/remove` | 302 redirect to `/cart`, item removed from DB |
| `GET /cart` | 200, populates real items from DB |
| `GET /checkout` | 200 when cart has items, 302 to `/cart` when empty |
| `GET /api/wallets` | 200, contains `wallet_type`, `currency`, `balance_cents`, `balance_display` |
| `POST /checkout` (wallet) | `HX-Redirect: /payment-success`, order created, cart cleared, invoice generated |
| `POST /checkout` (bank) | Order created with status=`pending`, redirect to `/payment-in-progress` |
| `POST /api/payments/deposit` | Deposit request created, instructions returned |
| `POST /api/webhooks/payments` | Deposit confirmed, wallet credited, transaction logged |
| `GET /api/orders/latest` | 200 with order details and items |
| `GET /payment-success` | 200, shows order summary |
| DB: `orders` table | Completed order exists with correct `total_cents`, `payment_method` |
| DB: `investments` table | Investment row with `tokens_owned`, `status=active` |
| DB: `invoices` table | Invoice with `INV-YYYY-NNNNN` number, `status=issued` |
| DB: `wallet_transactions` | Debit transaction logged for wallet purchases |
| E2E: Cart clear after checkout | `cart_items` table empty for user after successful checkout |

---

## Troubleshooting

### "No USD wallet found. Please deposit funds first."
The test user's wallet doesn't exist or has insufficient funds. Run the SQL in Phase 1, Step 4 to seed the wallet.

### "cargo build" fails with compilation errors
The most common cause is a schema mismatch (e.g., `investments.order_id` column doesn't exist). Check `\d investments` in psql and update the `approve_order` service function to match.

### `GET /api/wallets` returns `"type"` instead of `"wallet_type"`
Fix `backend/src/payments/routes.rs` in `list_wallets()` — change `"type": wtype` to `"wallet_type": wtype` in the JSON construction.

### Checkout POST returns 400 "Insufficient balance"
The wallet has insufficient funds. Run Phase 1 Step 4 again, or use a smaller investment amount by reducing the asset's `token_price_cents`.

### Invoice generation fails with "Invoice seq failed"
The `invoice_number_seq` sequence doesn't exist. Run Phase 1 Step 2 to create it.
