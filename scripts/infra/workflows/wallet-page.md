---
description: Make the /wallet page fully functional – backend, frontend, and testing
---

## Wallet Page – Full Functionality Workflow

This workflow makes the `/wallet` page at `http://localhost:8888/wallet` fully functional. It covers the backend (Rust), frontend (HTML/CSS/JS), database, and testing.

---

### Prerequisites

1. Ensure the backend server is running:
```bash
cd /Users/martin/Projects/poool/backend && cargo run
```

2. Ensure PostgreSQL is running and the database `poool` exists with schema applied.

3. Ensure you have a test user account registered (e.g. `test@poool.app`).

---

### Phase 1: Verify Database Schema

Check that the `wallets` and `wallet_transactions` tables exist and have the correct schema.

1. Connect to the database and verify the wallet tables:
```sql
-- Verify wallets table
SELECT column_name, data_type FROM information_schema.columns
WHERE table_name = 'wallets' ORDER BY ordinal_position;

-- Verify wallet_transactions table
SELECT column_name, data_type FROM information_schema.columns
WHERE table_name = 'wallet_transactions' ORDER BY ordinal_position;
```

Expected `wallets` columns: `id`, `user_id`, `wallet_type` (cash/rewards), `balance_cents`, `created_at`, `updated_at`.

Expected `wallet_transactions` columns: `id`, `wallet_id`, `type`, `status`, `amount_cents`, `description`, `external_ref_id`, `related_order_id`, `metadata`, `created_at`, `completed_at`.

2. Verify wallets exist for the test user:
```sql
SELECT w.wallet_type, w.balance_cents
FROM wallets w JOIN users u ON u.id = w.user_id
WHERE u.email = 'test@poool.app';
```

If no wallets exist, they will be auto-created on first visit to `/wallet` (via `ensure_wallets()` in `backend/src/wallet/routes.rs`).

---

### Phase 2: Backend – Wallet Routes (Rust)

All wallet routes live in `/Users/martin/Projects/poool/backend/src/wallet/routes.rs`.

**Existing routes** (registered in `main.rs`):
- `GET /wallet` → `page_wallet()` – Renders the wallet page with real data
- `POST /wallet/deposit` → `handle_deposit()` – Deposits funds
- `POST /wallet/withdraw` → `handle_withdraw()` – Withdraws funds

#### 2.1 Verify route functionality

The `page_wallet` handler already:
- ✅ Ensures wallets exist for the user
- ✅ Fetches cash, rewards, and asset balances from DB
- ✅ Fetches last 10 transactions
- ✅ Dynamically generates transaction row HTML
- ✅ Replaces hardcoded balance values in the template
- ✅ Replaces transaction body content

#### 2.2 Add deposit/withdraw modal functionality

The deposit and withdraw buttons in the HTML currently don't have form actions. Wire them up:

1. In `wallet.html`, locate the Deposit button (around line 2784):
   - Element ID: `wallet-balance-card-cash-deposit-btn`
   - Wire `onclick` to open a deposit modal

2. In `wallet.html`, locate the Withdraw button (around line 2787):
   - Element ID: `wallet-balance-card-cash-withdraw-btn`
   - Wire `onclick` to open a withdraw modal

3. Add deposit/withdraw modal HTML at the bottom of wallet.html (before `</body>`):

```html
<!-- Deposit Modal -->
<div id="deposit-modal" class="wallet-modal-overlay" style="display: none;">
  <div class="wallet-modal">
    <div class="wallet-modal__header">
      <h3 class="wallet-modal__title">Deposit Funds</h3>
      <button class="wallet-modal__close" onclick="closeDepositModal()">&times;</button>
    </div>
    <form action="/wallet/deposit" method="POST" class="wallet-modal__form">
      <label class="wallet-modal__label" for="deposit-amount">Amount (USD)</label>
      <input type="number" id="deposit-amount" name="amount" min="1" step="0.01"
             placeholder="Enter amount" class="wallet-modal__input" required>
      <button type="submit" class="wallet-modal__submit">Deposit</button>
    </form>
  </div>
</div>

<!-- Withdraw Modal -->
<div id="withdraw-modal" class="wallet-modal-overlay" style="display: none;">
  <div class="wallet-modal">
    <div class="wallet-modal__header">
      <h3 class="wallet-modal__title">Withdraw Funds</h3>
      <button class="wallet-modal__close" onclick="closeWithdrawModal()">&times;</button>
    </div>
    <form action="/wallet/withdraw" method="POST" class="wallet-modal__form">
      <label class="wallet-modal__label" for="withdraw-amount">Amount (USD)</label>
      <input type="number" id="withdraw-amount" name="amount" min="1" step="0.01"
             placeholder="Enter amount" class="wallet-modal__input" required>
      <button type="submit" class="wallet-modal__submit">Withdraw</button>
    </form>
  </div>
</div>
```

4. Add the modal JavaScript (in `wallet.html` or a new `wallet.js`):

```javascript
function openDepositModal() {
  document.getElementById('deposit-modal').style.display = 'flex';
}
function closeDepositModal() {
  document.getElementById('deposit-modal').style.display = 'none';
}
function openWithdrawModal() {
  document.getElementById('withdraw-modal').style.display = 'flex';
}
function closeWithdrawModal() {
  document.getElementById('withdraw-modal').style.display = 'none';
}
// Close on overlay click
document.querySelectorAll('.wallet-modal-overlay').forEach(overlay => {
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) overlay.style.display = 'none';
  });
});
```

5. Add the modal CSS to `wallet.css` (`/Users/martin/Projects/poool/frontend/platform/static/css/wallet.css`):

```css
/* Wallet Modals */
.wallet-modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}
.wallet-modal {
  background: #fff;
  border-radius: 12px;
  padding: 24px;
  width: 400px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0,0,0,0.15);
}
.wallet-modal__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.wallet-modal__title {
  font-family: 'Inter';
  font-weight: 700;
  font-size: 20px;
  color: #181D27;
}
.wallet-modal__close {
  background: none;
  border: none;
  font-size: 24px;
  cursor: pointer;
  color: #717680;
}
.wallet-modal__form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.wallet-modal__label {
  font-family: 'Inter';
  font-weight: 500;
  font-size: 14px;
  color: #535862;
}
.wallet-modal__input {
  padding: 10px 14px;
  border: 1px solid #D0D5DD;
  border-radius: 8px;
  font-family: 'Inter';
  font-size: 16px;
  outline: none;
}
.wallet-modal__input:focus {
  border-color: #2E2EF9;
  box-shadow: 0 0 0 3px rgba(46, 46, 249, 0.1);
}
.wallet-modal__submit {
  padding: 10px 18px;
  background: #2E2EF9;
  color: #fff;
  border: none;
  border-radius: 8px;
  font-family: 'Inter';
  font-weight: 600;
  font-size: 14px;
  cursor: pointer;
}
.wallet-modal__submit:hover {
  background: #1c1cd4;
}
```

6. Wire the buttons to the modals by adding `onclick` attributes:
   - Deposit button: `onclick="openDepositModal()"`
   - Withdraw button: `onclick="openWithdrawModal()"`

7. Rebuild the backend after making HTML changes:
```bash
cd /Users/martin/Projects/poool/backend && cargo run
```

---

### Phase 3: Verify Wallet Page Functionality

1. **Open** `http://localhost:8888/wallet` in the browser (must be logged in).

2. **Verify the following elements are visible:**
   - [ ] Wallet page title with icon
   - [ ] Cash balance card showing real balance from DB
   - [ ] Rewards balance card showing real balance from DB
   - [ ] Asset balance card showing sum of investments
   - [ ] Deposit button (opens modal)
   - [ ] Withdraw button (opens modal)
   - [ ] Transactions table with header columns: Type, Status, Date, Wallet, Amount, Actions
   - [ ] Transaction rows (either from DB or "No transactions yet")
   - [ ] "See all transactions" footer link
   - [ ] Cards section (with Visa card and "Add new card" button)
   - [ ] Banks section (with "Add new bank" button)
   - [ ] Sidebar navigation with Wallet active
   - [ ] KYC banner at the top

3. **Test Deposit Flow:**
   - Click the "Deposit" button
   - Verify modal opens with amount input
   - Enter amount (e.g., `100`)
   - Click "Deposit"
   - Verify redirect back to `/wallet`
   - Verify cash balance increased by $100
   - Verify new transaction row appears in the transactions table

4. **Test Withdraw Flow:**
   - Click the "Withdraw" button
   - Verify modal opens with amount input
   - Enter amount (e.g., `50`)
   - Click "Withdraw"
   - Verify redirect back to `/wallet`
   - Verify cash balance decreased by $50
   - Verify new withdrawal transaction appears

5. **Test Edge Cases:**
   - Try withdrawing more than available balance → balance should NOT go negative
   - Try depositing $0 → should be ignored
   - Try withdrawing $0 → should be ignored

---

### Phase 4: Run Automated Tests

1. Run the wallet-specific tests:
// turbo
```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py 2>&1 | grep -A 5 "wallet\|Wallet\|WALLET"
```

2. Run the full test suite:
```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py
```

3. **Expected test results for wallet:**
   - ✅ `GET /wallet` returns 200 (when authenticated)
   - ✅ `GET /wallet` redirects unauthenticated users
   - ✅ Element `#wallet-balance-card-cash-amount` found
   - ✅ Element `#wallet-balance-card-rewards-amount` found
   - ✅ Element `#wallet-balance-card-assets-amount` found
   - ✅ Element `#wallet-transactions-body` found
   - ✅ CSS `wallet.css` loaded
   - ✅ Balance display present (USD format)
   - ✅ Deposit form action present (`/wallet/deposit`)
   - ✅ Withdraw form action present (`/wallet/withdraw`)
   - ✅ Transactions section present
   - ✅ `POST /wallet/deposit` redirects (302/303)
   - ✅ Cash balance after deposit reflects correct amount
   - ✅ Transaction record created in DB after deposit

4. Verify database state after tests:
```sql
-- Check wallet balances
SELECT u.email, w.wallet_type, w.balance_cents
FROM wallets w JOIN users u ON u.id = w.user_id;

-- Check recent transactions
SELECT t.type, t.status, t.amount_cents, t.created_at
FROM wallet_transactions t
ORDER BY t.created_at DESC LIMIT 10;
```

---

### Phase 5: Mobile Responsiveness

1. Verify the mobile wallet view works correctly:
   - Open browser DevTools → toggle mobile view
   - Check that mobile-specific wallet components render:
     - Mobile wallet title with icon
     - Mobile cash balance card with Deposit/Withdraw buttons
     - Mobile rewards balance card with star icon
     - Mobile transactions section with scrollable table
   - Mobile KYC banner visible at top
   - Mobile burger menu navigates correctly

2. Verify mobile CSS is loaded:
   - `mobile-wallet.css` should be included in `<head>`

---

### Key Files Reference

| File | Purpose |
|------|---------|
| `backend/src/wallet/mod.rs` | Wallet module declaration |
| `backend/src/wallet/routes.rs` | Wallet route handlers (page, deposit, withdraw) |
| `backend/src/main.rs` | Route registration (lines 74-76) |
| `frontend/platform/wallet.html` | Wallet page template (4136 lines) |
| `frontend/platform/static/css/wallet.css` | Desktop wallet styles |
| `frontend/platform/static/css/mobile-wallet.css` | Mobile wallet styles |
| `frontend/platform/static/css/bem/table-wallet.css` | Wallet table BEM styles |
| `database/001_initial_schema.sql` | DB schema (wallets at line 107, transactions at line 123) |
| `tests/test_platform.py` | Test suite (wallet tests at line 831) |

---

### Troubleshooting

- **Page shows "Page not found"**: Check that `wallet.html` path is correct relative to where `cargo run` is executed. The backend reads from `../frontend/platform/wallet.html`.
- **Balances always show hardcoded values**: The backend does string replacement on the HTML. Ensure the exact strings `USD 2,732` and `USD 1,700` exist in `wallet.html` for the replacement to work.
- **Transactions not showing**: Check the query in `page_wallet()` joins `wallet_transactions` with `wallets` filtered by `user_id`.
- **Deposit/withdraw not working**: Check form `action` attributes point to `/wallet/deposit` and `/wallet/withdraw`. Verify `POST` method.
- **Balance goes negative**: The `wallets` table has `CHECK (balance_cents >= 0)` constraint. The withdraw handler also checks balance before subtracting.
