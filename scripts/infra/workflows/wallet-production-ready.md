---
description: Make the wallet system fully production-ready – real DB values, verified deposits/withdrawals, passing tests
---

# Wallet System – Production Readiness Workflow

This workflow brings the POOOL wallet system to a fully production-ready state: real data from the database, all UI elements correctly wired up, deposit/withdrawal flows verified end-to-end, and the full automated test suite passing.

> **Scope:** `backend/src/wallet/`, `frontend/platform/wallet.html`, `frontend/platform/static/css/wallet*.css`, `database/` schema, and all related test files.

---

## Prerequisites

1. Ensure PostgreSQL is running:
```bash
psql -U postgres -c "\l" | grep poool
```

2. Start the backend server (keep this running in a separate terminal during the whole workflow):
```bash
cd /Users/martin/Projects/poool/backend && cargo run
```
> Backend listens on `http://localhost:8888`

3. Confirm test user exists:
```bash
psql -U postgres -d poool -c "SELECT id, email FROM users WHERE email = 'test@poool.app';"
```
If missing, seed it:
```bash
psql -U postgres -d poool -f /Users/martin/Projects/poool/database/seeds/test_user.sql
```

---

## Phase 1 – Database Schema Audit

### 1.1 Verify table columns are correct

```sql
-- wallets table
SELECT column_name, data_type, column_default, is_nullable
FROM information_schema.columns
WHERE table_name = 'wallets'
ORDER BY ordinal_position;
```

Expected columns: `id` (uuid), `user_id` (uuid), `wallet_type` (text/enum: `cash`|`rewards`), `balance_cents` (bigint, default 0), `currency` (text, default `USD`), `created_at`, `updated_at`.

> **CRITICAL:** If `currency` column is missing, the checkout flow will fail. Run:
> ```sql
> ALTER TABLE wallets ADD COLUMN IF NOT EXISTS currency TEXT NOT NULL DEFAULT 'USD';
> ```

```sql
-- wallet_transactions table
SELECT column_name, data_type, column_default, is_nullable
FROM information_schema.columns
WHERE table_name = 'wallet_transactions'
ORDER BY ordinal_position;
```

Expected columns: `id` (uuid), `wallet_id` (uuid FK), `type` (text: `deposit`|`withdrawal`|`purchase`|`refund`|`fee`), `status` (text: `pending`|`completed`|`failed`), `amount_cents` (bigint), `description` (text, nullable), `external_ref_id` (text, nullable), `related_order_id` (uuid, nullable), `metadata` (jsonb, nullable), `created_at`, `completed_at` (nullable).

### 1.2 Verify constraints and indexes

```sql
-- Check balance constraint (prevents negative balances)
SELECT conname, consrc FROM pg_constraint
WHERE conrelid = 'wallets'::regclass AND contype = 'c';

-- Verify critical indexes exist
SELECT indexname FROM pg_indexes WHERE tablename IN ('wallets', 'wallet_transactions');
```

Required indexes:
- `idx_wallets_user` on `wallets(user_id)`
- `idx_wallet_transactions_wallet` on `wallet_transactions(wallet_id)`
- `idx_wallet_transactions_created` on `wallet_transactions(created_at DESC)`

If any index is missing:
```sql
CREATE INDEX IF NOT EXISTS idx_wallets_user ON wallets(user_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_wallet ON wallet_transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_wallet_transactions_created ON wallet_transactions(created_at DESC);
```

### 1.3 Ensure wallets exist for all active users

The `ensure_wallets()` function auto-creates wallets on first visit, but let's seed all existing users now:

```sql
INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
SELECT u.id, t.wallet_type, 0, 'USD'
FROM users u
CROSS JOIN (VALUES ('cash'), ('rewards')) AS t(wallet_type)
ON CONFLICT DO NOTHING;
```

Verify:
```sql
SELECT u.email, w.wallet_type, w.balance_cents, w.currency
FROM wallets w JOIN users u ON u.id = w.user_id
ORDER BY u.email, w.wallet_type;
```

---

## Phase 2 – Backend: Wallet Routes Audit and Fixes

All wallet backend logic lives in `backend/src/wallet/routes.rs`.

### 2.1 Review `ensure_wallets()` function

Open: `/Users/martin/Projects/poool/backend/src/wallet/routes.rs` (lines 47–59)

**Current issue:** The INSERT does not set a `currency` column. If the schema requires it with no default, the insert will fail.

**Fix** – Update `ensure_wallets` to include the currency column:
```rust
// In ensure_wallets(), change the INSERT to:
sqlx::query(
    r#"
    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES ($1, 'cash', 0, 'USD'), ($1, 'rewards', 0, 'USD')
    ON CONFLICT DO NOTHING
    "#,
)
```

### 2.2 Verify `handle_deposit()` atomicity

Open: `backend/src/wallet/routes.rs` (lines 61–116)

Check that the full deposit flow uses a database transaction and always writes a `wallet_transactions` row. The function `record_manual_deposit` (lines 121–155) handles this atomically via two sequential queries.

**Known issue:** If `ensure_wallets` creates the wallet but between that call and the UPDATE another request races, the wallet may not exist. Wrap both calls in a single transaction:

Review `record_manual_deposit` and if needed, refactor to a proper `BEGIN`/`COMMIT` pattern:
```rust
// Wrap in a transaction to prevent race conditions:
let mut tx = pool.begin().await?;

// INSERT wallet if missing (inside tx)
sqlx::query("INSERT INTO wallets ... ON CONFLICT DO NOTHING")
    .bind(user_id).execute(&mut *tx).await?;

// UPDATE balance and return wallet_id
let wallet_id: Option<Uuid> = sqlx::query_scalar(
    "UPDATE wallets SET balance_cents = balance_cents + $1 WHERE user_id = $2 AND wallet_type = 'cash' RETURNING id"
).bind(amount_cents).bind(user_id).fetch_optional(&mut *tx).await?;

// INSERT transaction record
if let Some(wid) = wallet_id {
    sqlx::query(
        "INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents) VALUES ($1, 'deposit', 'completed', $2)"
    ).bind(wid).bind(amount_cents).execute(&mut *tx).await?;
}

tx.commit().await?;
```

### 2.3 Verify `handle_withdraw()` – balance check guards

Open: `backend/src/wallet/routes.rs` (lines 157–237)

Confirm the withdrawal guard `if current_balance >= amount_cents` exists and that the transaction is rolled back if the balance is insufficient (it currently does commit an empty transaction).

**Fix:** Add an explicit 402 response or redirect with an error message when insufficient funds:
```rust
if current_balance < amount_cents {
    tx.rollback().await.ok();
    tracing::warn!("Insufficient funds: user {} has {} cents, tried to withdraw {}", user.id, current_balance, amount_cents);
    return Redirect::to("/wallet?error=insufficient_funds").into_response();
}
```

### 2.4 Add a `GET /api/wallet/balance` JSON endpoint

This enables frontend JavaScript to fetch real-time balances without a full page reload (needed for the deposit/withdraw modals to refresh without redirect).

Add to `backend/src/wallet/routes.rs`:
```rust
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct WalletBalanceResponse {
    pub cash_cents: i64,
    pub rewards_cents: i64,
    pub asset_cents: i64,
    pub cash_display: String,
    pub rewards_display: String,
}

pub async fn api_wallet_balance(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response(),
    };

    let _ = ensure_wallets(&state.db, user.id).await;

    let cash_cents: i64 = sqlx::query_scalar(
        "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'cash'"
    ).bind(user.id).fetch_optional(&state.db).await.unwrap_or(Some(0)).unwrap_or(0);

    let rewards_cents: i64 = sqlx::query_scalar(
        "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'rewards'"
    ).bind(user.id).fetch_optional(&state.db).await.unwrap_or(Some(0)).unwrap_or(0);

    let asset_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(current_value_cents), 0) FROM investments WHERE user_id = $1 AND status != 'exited'"
    ).bind(user.id).fetch_optional(&state.db).await.unwrap_or(Some(0)).unwrap_or(0);

    Json(WalletBalanceResponse {
        cash_cents,
        rewards_cents,
        asset_cents,
        cash_display: format_usd(cash_cents),
        rewards_display: format_usd(rewards_cents),
    }).into_response()
}
```

Register in `backend/src/wallet/mod.rs`:
```rust
use axum::routing::get;
// Add to the router:
.route("/api/wallet/balance", get(routes::api_wallet_balance))
```

### 2.5 Add a `GET /api/wallet/transactions` JSON endpoint

Enables paginated transaction history for the frontend:
```rust
#[derive(Serialize, sqlx::FromRow)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub tx_type: String,
    pub status: String,
    pub amount_cents: i64,
    pub wallet_type: String,
    pub created_at: DateTime<Utc>,
}

pub async fn api_wallet_transactions(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response(),
    };

    let txs = sqlx::query_as::<_, (Uuid, String, String, i64, String, DateTime<Utc>)>(r#"
        SELECT t.id, t.type, t.status, t.amount_cents, w.wallet_type, t.created_at
        FROM wallet_transactions t
        JOIN wallets w ON w.id = t.wallet_id
        WHERE w.user_id = $1
        ORDER BY t.created_at DESC
        LIMIT 50
    "#).bind(user.id).fetch_all(&state.db).await.unwrap_or_default();

    let result: Vec<serde_json::Value> = txs.iter().map(|(id, tx_type, status, amount, wallet_type, created_at)| {
        serde_json::json!({
            "id": id,
            "type": tx_type,
            "status": status,
            "amount_cents": amount,
            "wallet_type": wallet_type,
            "created_at": created_at.to_rfc3339(),
        })
    }).collect();

    Json(serde_json::json!({ "transactions": result })).into_response()
}
```

Register:
```rust
.route("/api/wallet/transactions", get(routes::api_wallet_transactions))
```

### 2.6 Build and verify the backend compiles

// turbo
```bash
cd /Users/martin/Projects/poool/backend && cargo build 2>&1 | tail -20
```

---

## Phase 3 – Frontend: HTML Layout Fixes

The wallet HTML template is at: `frontend/platform/wallet.html` (4831 lines)

### 3.1 Verify required element IDs exist

The test suite checks for these IDs — confirm they exist and contain real server-injected data:

| Element ID | What it should contain |
|---|---|
| `wallet-balance-card-cash-amount` | Real cash balance from DB |
| `wallet-balance-card-rewards-amount` | Real rewards balance from DB |
| `wallet-balance-card-assets-amount` | Sum of `investments.current_value_cents` |
| `wallet-transactions-body` | Server-rendered transaction rows |
| `deposit-btn` | Button that opens the deposit modal |
| `withdraw-btn` | Button that opens the withdraw modal |
| `transactions-table` | The transactions table wrapper |
| `wallet-balance-card` | The main balance section card |

Search the wallet.html for these IDs:
```bash
grep -n "id=\"deposit-btn\"\|id=\"withdraw-btn\"\|id=\"transactions-table\"\|id=\"wallet-balance-card\"" \
  /Users/martin/Projects/poool/frontend/platform/wallet.html
```

**If any ID is missing**, add it to the appropriate element. For example:
- Find the deposit button (search for `wallet-balance-card-cash-deposit-btn`) and add `id="deposit-btn"` as an alias/additional ID on its nearest parent section, OR rename the button's ID.
- For the transactions table, find `<table` or `<div class="table` wrapping the transactions and add `id="transactions-table"`.
- For `wallet-balance-card`, add it to the outermost wrapper `<div>` of the balance card section.

### 3.2 Verify deposit and withdraw forms are wired to backend

Check that the forms POST to the correct endpoints:
```bash
grep -n "action=" /Users/martin/Projects/poool/frontend/platform/wallet.html | grep -i "wallet"
```

Required: both `/wallet/deposit` and `/wallet/withdraw` must appear as form `action` attributes, OR as JavaScript `fetch`/`hx-post` targets.

If using the modal pattern from `wallet-page.md`, the modals should look like:
```html
<!-- Deposit Modal -->
<div id="deposit-modal" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); z-index:9999; align-items:center; justify-content:center;">
  <div style="background:#fff; border-radius:16px; padding:32px; width:440px; max-width:90vw;">
    <h3>Deposit Funds</h3>
    <form id="deposit-form" action="/wallet/deposit" method="POST">
      <label>Amount (USD)</label>
      <input type="number" name="amount" id="deposit-amount" min="1" step="0.01" required>
      <select name="payment_method_id" id="deposit-payment-method">
        <!-- PAYMENT_METHODS_OPTIONS -->
      </select>
      <button type="submit">Confirm Deposit</button>
    </form>
  </div>
</div>

<!-- Withdraw Modal -->
<div id="withdraw-modal" style="display:none; position:fixed; inset:0; background:rgba(0,0,0,0.5); z-index:9999; align-items:center; justify-content:center;">
  <div style="background:#fff; border-radius:16px; padding:32px; width:440px; max-width:90vw;">
    <h3>Withdraw Funds</h3>
    <form id="withdraw-form" action="/wallet/withdraw" method="POST">
      <label>Amount (USD)</label>
      <input type="number" name="amount" id="withdraw-amount" min="1" step="0.01" required>
      <select name="payment_method_id" id="withdraw-payment-method">
        <!-- PAYMENT_METHODS_OPTIONS -->
      </select>
      <button type="submit">Confirm Withdrawal</button>
    </form>
  </div>
</div>
```

### 3.3 Wire modal open/close buttons

Deposit button should have: `onclick="document.getElementById('deposit-modal').style.display='flex'"`
Withdraw button should have: `onclick="document.getElementById('withdraw-modal').style.display='flex'"`

Close buttons inside modals: `onclick="this.closest('[id$=-modal]').style.display='none'"`

Add close-on-overlay-click JS (if not already present):
```javascript
document.querySelectorAll('#deposit-modal,#withdraw-modal').forEach(m => {
  m.addEventListener('click', e => { if (e.target === m) m.style.display = 'none'; });
});
```

### 3.4 Verify balance replacement strings match the HTML

The backend (`wallet/routes.rs` lines 499–532) replaces these exact strings:
- `USD 2,732` → real cash balance
- `USD 1,700` → real rewards balance
- `USD 0` → real asset balance

Search the HTML to confirm these exact placeholder strings exist:
```bash
grep -n "USD 2,732\|USD 1,700" /Users/martin/Projects/poool/frontend/platform/wallet.html
```

If the values have been changed (e.g. someone edited the HTML to show different demo data), **update the backend replacement strings** in `routes.rs` to match whatever is actually in the HTML.

### 3.5 Verify transaction body placeholder

The backend searches for:
```
<div id="wallet-transactions-body" class="table__body">
```

And replaces its inner content. Confirm this exact string exists in `wallet.html`:
```bash
grep -n "wallet-transactions-body" /Users/martin/Projects/poool/frontend/platform/wallet.html
```

### 3.6 Add error state display for insufficient funds

If the backend redirects to `/wallet?error=insufficient_funds`, show a toast/banner to the user. Add this JS near the bottom of `wallet.html`:
```javascript
(function() {
  const params = new URLSearchParams(window.location.search);
  const error = params.get('error');
  if (error === 'insufficient_funds') {
    const banner = document.createElement('div');
    banner.style.cssText = 'position:fixed;top:20px;right:20px;background:#F04438;color:#fff;padding:12px 20px;border-radius:8px;z-index:99999;font-family:Inter,sans-serif;font-size:14px;font-weight:500;';
    banner.textContent = 'Insufficient funds. Please deposit before withdrawing.';
    document.body.appendChild(banner);
    setTimeout(() => banner.remove(), 5000);
    // Clean URL
    history.replaceState({}, '', '/wallet');
  }
})();
```

---

## Phase 4 – CSS & Static Assets Verification

### 4.1 Confirm all CSS files exist and are served

// turbo
```bash
for f in wallet.css mobile-wallet.css bem/table-wallet.css; do
  curl -s -o /dev/null -w "%{http_code} $f\n" http://localhost:8888/static/css/$f
done
```

Expected: all return `200`.

### 4.2 Verify wallet modal CSS exists in `wallet.css`

```bash
grep -n "wallet-modal" /Users/martin/Projects/poool/frontend/platform/static/css/wallet.css | head -5
```

If the modal styles are missing, add them to `wallet.css`:
```css
/* ── Wallet Modals ───────────────────────────────────────────── */
.wallet-modal-overlay { position:fixed; inset:0; background:rgba(0,0,0,.55); display:flex; align-items:center; justify-content:center; z-index:10000; }
.wallet-modal { background:#fff; border-radius:16px; padding:32px; width:440px; max-width:90vw; box-shadow:0 20px 60px rgba(0,0,0,.18); }
.wallet-modal h3 { font-family:'Inter',sans-serif; font-size:20px; font-weight:700; color:#181D27; margin:0 0 20px; }
.wallet-modal label { font-family:'Inter',sans-serif; font-size:14px; font-weight:500; color:#535862; display:block; margin-bottom:6px; }
.wallet-modal input, .wallet-modal select { width:100%; padding:10px 14px; border:1px solid #D0D5DD; border-radius:8px; font-size:15px; font-family:'Inter',sans-serif; box-sizing:border-box; margin-bottom:16px; }
.wallet-modal input:focus, .wallet-modal select:focus { outline:none; border-color:#2E2EF9; box-shadow:0 0 0 3px rgba(46,46,249,.1); }
.wallet-modal .submit-btn { width:100%; padding:12px; background:#2E2EF9; color:#fff; border:none; border-radius:8px; font-size:15px; font-weight:600; cursor:pointer; font-family:'Inter',sans-serif; }
.wallet-modal .submit-btn:hover { background:#1c1cd4; }
```

---

## Phase 5 – End-to-End Functional Testing

### 5.1 Run wallet-specific automated tests

// turbo
```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | grep -A 3 "wallet\|Wallet\|WALLET"
```

### 5.2 Run the deposit functional test in isolation

```bash
cd /Users/martin/Projects/poool && python3 -c "
import requests, sys
sys.path.insert(0, 'tests')
from test_platform import get_session, BASE_URL

s = get_session()
r = s.post(f'{BASE_URL}/wallet/deposit', data={'amount': '50'}, allow_redirects=False)
print('Deposit status:', r.status_code)
assert r.status_code in (302, 303, 200), f'Expected redirect, got {r.status_code}'
print('PASS: deposit returns redirect')
"
```

### 5.3 Verify database state after each operation

After a deposit:
```sql
SELECT 
  u.email,
  w.wallet_type,
  w.balance_cents,
  w.balance_cents / 100.0 AS balance_usd
FROM wallets w
JOIN users u ON u.id = w.user_id
WHERE u.email = 'test@poool.app';
```

Check transaction record:
```sql
SELECT 
  t.id,
  t.type,
  t.status,
  t.amount_cents,
  t.amount_cents / 100.0 AS amount_usd,
  t.created_at
FROM wallet_transactions t
JOIN wallets w ON w.id = t.wallet_id
JOIN users u ON u.id = w.user_id
WHERE u.email = 'test@poool.app'
ORDER BY t.created_at DESC
LIMIT 5;
```

After a withdrawal:
```sql
-- Verify balance decreased and a withdrawal record exists
SELECT t.type, t.status, t.amount_cents
FROM wallet_transactions t
JOIN wallets w ON w.id = t.wallet_id
JOIN users u ON u.id = w.user_id
WHERE u.email = 'test@poool.app' AND t.type = 'withdrawal'
ORDER BY t.created_at DESC LIMIT 3;
```

### 5.4 Test withdrawal guard (insufficient funds)

Attempt to withdraw more than the current balance — the balance should NOT go negative, and the user should be redirected with an error:
```bash
cd /Users/martin/Projects/poool && python3 -c "
import requests, psycopg2
from tests.test_platform import get_session, BASE_URL, DB_DSN, TEST_EMAIL

s = get_session()

# First get current balance
conn = psycopg2.connect(DB_DSN)
cur = conn.cursor()
cur.execute(\"SELECT balance_cents FROM wallets JOIN users ON users.id = wallets.user_id WHERE users.email = %s AND wallet_type = 'cash'\", (TEST_EMAIL,))
bal = cur.fetchone()[0]
print(f'Current balance: {bal/100:.2f} USD')

# Try to withdraw 10x more than balance
overdraft = (bal + 1_000_000) / 100
r = s.post(f'{BASE_URL}/wallet/withdraw', data={'amount': str(overdraft)}, allow_redirects=False)
print(f'Overdraft attempt status: {r.status_code}')

# Verify balance unchanged
cur.execute(\"SELECT balance_cents FROM wallets JOIN users ON users.id = wallets.user_id WHERE users.email = %s AND wallet_type = 'cash'\", (TEST_EMAIL,))
new_bal = cur.fetchone()[0]
print(f'Balance after overdraft attempt: {new_bal/100:.2f} USD')
assert new_bal == bal, f'Balance changed! {bal} -> {new_bal}'
print('PASS: balance unchanged after overdraft attempt')
cur.close()
conn.close()
"
```

### 5.5 Test the new JSON API endpoints

```bash
cd /Users/martin/Projects/poool && python3 -c "
import requests, json
from tests.test_platform import get_session, BASE_URL

s = get_session()

# Test balance endpoint
r = s.get(f'{BASE_URL}/api/wallet/balance')
print('Balance API status:', r.status_code)
assert r.status_code == 200, f'Expected 200, got {r.status_code}'
data = r.json()
print('Balance data:', json.dumps(data, indent=2))
assert 'cash_cents' in data
assert 'rewards_cents' in data
print('PASS: balance API returns correct structure')

# Test transactions endpoint
r2 = s.get(f'{BASE_URL}/api/wallet/transactions')
print('Transactions API status:', r2.status_code)
assert r2.status_code == 200
data2 = r2.json()
print(f'Transactions count: {len(data2.get(\"transactions\", []))}')
print('PASS: transactions API working')
"
```

---

## Phase 6 – Full Test Suite Run

### 6.1 Run the complete test suite and check wallet sections pass

// turbo
```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | grep -E "Passed|Failed|Warning|wallet|Wallet|deposit|Deposit|withdraw|Withdraw|transaction"
```

### 6.2 Run the full suite and check the summary

```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | tail -30
```

**Expected wallet test results:**
- ✅ `GET /wallet returns 200`
- ✅ `Element #wallet-balance-card-cash-amount found`
- ✅ `Element #wallet-balance-card-rewards-amount found`
- ✅ `Element #wallet-balance-card-assets-amount found`
- ✅ `Element #wallet-transactions-body found`
- ✅ `CSS 'wallet.css' loaded`
- ✅ `Balance display present (USD format)`
- ✅ `Deposit form action present`
- ✅ `Withdraw form action present`
- ✅ `Transactions section present`
- ✅ `POST /wallet/deposit redirects (status=302)`
- ✅ `Cash balance after deposit: N.NN USD`
- ✅ `Transaction recorded: deposit completed N.NN USD`

**Wallet dashboard test:**
- ✅ `Found #wallet-balance-card`
- ✅ `Found #deposit-btn`
- ✅ `Found #withdraw-btn`
- ✅ `Found #transactions-table`

---

## Phase 7 – Admin Wallet Verification

### 7.1 Verify admin can see user wallet data

```bash
curl -s -b cookies.txt "http://localhost:8888/api/admin/treasury" | python3 -m json.tool | head -30
```

Check that the admin treasury view shows real wallet transaction data (not empty).

### 7.2 Verify admin can view individual user wallet balances

Navigate to: `http://localhost:8888/admin/users.html` → click any user → their wallet balance should show in the user details panel.

---

## Phase 8 – Regression Check (Checkout Integration)

The checkout flow deducts funds from the user's wallet. Verify this still works after our changes:

### 8.1 Ensure wallets are created with currency set

Since checkout uses `wallet.currency`, verify:
```sql
SELECT user_id, wallet_type, currency FROM wallets WHERE currency IS NULL OR currency = '';
```

If any rows are missing currency:
```sql
UPDATE wallets SET currency = 'USD' WHERE currency IS NULL OR currency = '';
```

### 8.2 Run checkout test

```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | grep -A 5 "checkout\|Checkout"
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Balance shows hardcoded `USD 2,732` | HTML string replacement failed | Verify exact string `USD 2,732` exists in `wallet.html`. Check `page_wallet()` replacement logic. |
| `No transaction record found after deposit` | `wallet_transactions` INSERT failing | Check backend logs: `cargo run 2>&1 \| grep -i "deposit\|transaction\|error"` |
| `#deposit-btn` not found in tests | ID missing from HTML | Add `id="deposit-btn"` to the deposit button or its wrapper |
| `#wallet-balance-card` not found | ID missing from HTML | Add `id="wallet-balance-card"` to the balance card container div |
| Withdrawal does not reduce balance | `payment_method_id` required for withdraw | Either add a payment method or allow withdrawal without one (manual bank transfer) |
| `/api/wallet/balance` returns 404 | Route not registered | Add `.route("/api/wallet/balance", get(routes::api_wallet_balance))` to `wallet/mod.rs` |
| Balance goes negative | Missing `CHECK` constraint | `ALTER TABLE wallets ADD CONSTRAINT chk_balance_non_negative CHECK (balance_cents >= 0);` |
| `currency` column missing causing checkout 500 | Schema out of date | `ALTER TABLE wallets ADD COLUMN IF NOT EXISTS currency TEXT NOT NULL DEFAULT 'USD';` |

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `backend/src/wallet/routes.rs` | All wallet handlers: page, deposit, withdraw, balance API |
| `backend/src/wallet/mod.rs` | Route registration |
| `backend/src/main.rs` | Top-level router (wallet router nested here) |
| `frontend/platform/wallet.html` | Wallet page template with placeholder strings |
| `frontend/platform/static/css/wallet.css` | Desktop wallet styles |
| `frontend/platform/static/css/mobile-wallet.css` | Mobile wallet styles |
| `frontend/platform/static/css/bem/table-wallet.css` | Wallet transactions table BEM styles |
| `database/001_initial_schema.sql` | DB schema – wallets & wallet_transactions definitions |
| `tests/test_platform.py` | Automated test suite (wallet: lines 844–948, 1244–1261) |
