---
description: Implement all rewards page features (balance, tier, referrals, copy link) with comprehensive testing
---

# Rewards Page – Full Implementation Workflow

// turbo-all

> This workflow makes the Rewards page fully functional with real backend API endpoints,
> database persistence, interactive UI, and comprehensive testing.

## Architecture Overview

```
Frontend: frontend/platform/rewards.html
          frontend/platform/static/css/rewards.css
          frontend/platform/static/js/rewards.js (NEW)

Backend:  backend/src/rewards/mod.rs (NEW)
          backend/src/rewards/models.rs (NEW)
          backend/src/rewards/service.rs (NEW)
          backend/src/rewards/routes.rs (NEW)

Database: rewards_balances     (NEW – tracks cashback, referral, promo balances)
          referral_codes       (NEW – unique referral links per user)
          referral_tracking    (NEW – who referred whom, status, payouts)
          user_tiers           (NEW – tier progress and history)

Tests:    tests/test_rewards.py (NEW – standalone rewards test suite)
          tests/test_platform.py (MODIFY – expand existing test_rewards function)
```

**API Pattern**: Page load → JS fetches data from API → populates cards. Actions → JSON response → toast notification.

---

## Phase 1: Database Schema

### Step 1.1 – Create migration file

Create `database/004_rewards_schema.sql`:

```sql
-- ═══════════════════════════════════════════════════════════════════
-- Migration 004: Rewards System Schema
-- ═══════════════════════════════════════════════════════════════════

-- Tier definitions (Intro, Plus, Pro, Elite, Premium)
CREATE TABLE IF NOT EXISTS tiers (
    id          SERIAL PRIMARY KEY,
    name        VARCHAR(32) NOT NULL UNIQUE,
    min_invest  BIGINT NOT NULL DEFAULT 0,   -- cents
    max_invest  BIGINT,                       -- cents, NULL = unlimited
    cashback_pct DECIMAL(5,2) NOT NULL DEFAULT 0,
    badge_color VARCHAR(7) NOT NULL DEFAULT '#D0D5DD',
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed tier data
INSERT INTO tiers (name, min_invest, max_invest, cashback_pct, badge_color, sort_order) VALUES
    ('Intro',   0,        999999,  1.00, '#98FB96', 1),
    ('Plus',    1000000,  4999999, 2.00, '#027A48', 2),
    ('Pro',     5000000,  9999999, 3.00, '#7A5AF8', 3),
    ('Elite',   10000000, 24999999,4.00, '#F79009', 4),
    ('Premium', 25000000, NULL,    5.00, '#0000FF', 5)
ON CONFLICT (name) DO NOTHING;

-- User tier tracking
CREATE TABLE IF NOT EXISTS user_tiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier_id         INT NOT NULL REFERENCES tiers(id),
    invested_12m    BIGINT NOT NULL DEFAULT 0,  -- cents invested in last 12 months
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Rewards balances
CREATE TABLE IF NOT EXISTS rewards_balances (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cashback        BIGINT NOT NULL DEFAULT 0,  -- cents
    referrals       BIGINT NOT NULL DEFAULT 0,  -- cents
    promotions      BIGINT NOT NULL DEFAULT 0,  -- cents
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Referral codes
CREATE TABLE IF NOT EXISTS referral_codes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code            VARCHAR(32) NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Referral tracking
CREATE TABLE IF NOT EXISTS referral_tracking (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_id     UUID NOT NULL REFERENCES users(id),
    referred_id     UUID NOT NULL REFERENCES users(id),
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, qualified, paid
    referrer_reward BIGINT NOT NULL DEFAULT 3000,  -- 30 USD in cents
    referred_reward BIGINT NOT NULL DEFAULT 3000,  -- 30 USD in cents
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    qualified_at    TIMESTAMPTZ,
    UNIQUE(referred_id)
);
```

### Step 1.2 – Run migration

```bash
psql "dbname=poool user=martin host=localhost" -f /Users/martin/Projects/poool/database/004_rewards_schema.sql
```

### Step 1.3 – Seed test user rewards data

```bash
psql "dbname=poool user=martin host=localhost" -c "
INSERT INTO rewards_balances (user_id, cashback, referrals, promotions)
SELECT id, 130000, 18000, 33000 FROM users WHERE email = 'test@poool.app'
ON CONFLICT (user_id) DO UPDATE SET cashback=130000, referrals=18000, promotions=33000;

INSERT INTO user_tiers (user_id, tier_id, invested_12m)
SELECT u.id, t.id, 1250000
FROM users u, tiers t WHERE u.email = 'test@poool.app' AND t.name = 'Plus'
ON CONFLICT (user_id) DO UPDATE SET tier_id=(SELECT id FROM tiers WHERE name='Plus'), invested_12m=1250000;

INSERT INTO referral_codes (user_id, code)
SELECT id, SUBSTRING(MD5(id::text || NOW()::text) FROM 1 FOR 12) FROM users WHERE email = 'test@poool.app'
ON CONFLICT (user_id) DO NOTHING;
"
```

### Step 1.4 – Verify migration

```bash
psql "dbname=poool user=martin host=localhost" -c "\dt rewards_balances; \dt referral_codes; \dt user_tiers; \dt tiers;"
```

---

## Phase 2: Backend – Rewards Module

### Step 2.1 – Create module structure

Create `backend/src/rewards/mod.rs`:
```rust
pub mod models;
pub mod routes;
pub mod service;
```

### Step 2.2 – Create `backend/src/rewards/models.rs`

| Struct | Fields | Purpose |
|--------|--------|---------|
| `RewardsOverview` | `total_balance, cashback, referrals, promotions, tier_name, tier_target, invested_12m, progress_pct, referral_code, referral_url` | Full rewards data for `GET /api/rewards` |
| `TierInfo` | `id, name, min_invest, badge_color, sort_order, cashback_pct` | Tier definitions for `/api/rewards/tiers` |
| `ApiResponse` | `success: bool, message: String` | Standard JSON response |

### Step 2.3 – Create `backend/src/rewards/service.rs`

```
get_rewards_overview(pool, user_id) → Result<RewardsOverview, AppError>
```
- JOIN: `rewards_balances`, `user_tiers`, `tiers`, `referral_codes`
- Compute `total_balance = cashback + referrals + promotions`
- Compute `progress_pct = (invested_12m / next_tier.min_invest) * 100`
- Build `referral_url = format!("https://app.poool.com/rewards/{}", code)`

```
get_all_tiers(pool) → Result<Vec<TierInfo>, AppError>
```
- `SELECT * FROM tiers ORDER BY sort_order`

### Step 2.4 – Create `backend/src/rewards/routes.rs`

| Method | Route | Handler |
|--------|-------|---------|
| `GET` | `/api/rewards` | `get_rewards_handler` → JSON |
| `GET` | `/api/rewards/tiers` | `get_tiers_handler` → JSON |

### Step 2.5 – Register module in `main.rs`

Add `mod rewards;` at the top. Add routes:
```rust
.route("/api/rewards", get(rewards::routes::get_rewards_handler))
.route("/api/rewards/tiers", get(rewards::routes::get_tiers_handler))
```

### Step 2.6 – Verify backend compiles

```bash
cd /Users/martin/Projects/poool/backend && cargo check 2>&1 | tail -20
```

---

## Phase 3: Frontend – JavaScript (`static/js/rewards.js`)

### Step 3.1 – Create `frontend/platform/static/js/rewards.js`

```javascript
(function() {
  'use strict';

  // ─── Load Rewards Data ──────────────────────────────────────
  async function loadRewards() {
    try {
      const res = await fetch('/api/rewards', { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      populateBalances(data);
      populateTierProgress(data);
      populateReferral(data);
    } catch (e) {
      console.error('Failed to load rewards:', e);
    }
  }

  function populateBalances(data) {
    const totalEl = document.querySelector('.balance-amount');
    if (totalEl) totalEl.textContent = formatCurrency(data.total_balance);

    const rows = document.querySelectorAll('.breakdown-row');
    const values = [data.cashback, data.referrals, data.promotions];
    rows.forEach((row, i) => {
      const val = row.querySelector('.breakdown-value');
      if (val && values[i] !== undefined) val.textContent = formatCurrency(values[i]);
    });
  }

  function populateTierProgress(data) {
    const amountEl = document.querySelector('.tp-amount');
    if (amountEl) amountEl.textContent = formatCurrency(data.invested_12m);

    const badgeEl = document.querySelector('.tp-badge');
    if (badgeEl) badgeEl.textContent = data.tier_target || 'Premium';

    const fill = document.querySelector('.tp-progress-fill');
    if (fill) fill.style.width = (data.progress_pct || 0) + '%';

    const hint = document.querySelector('.tp-hint');
    if (hint && data.tier_target_amount) {
      hint.innerHTML = 'Invest <strong class="text-blue">' +
        formatCurrency(data.tier_target_amount) + '</strong> to reach ' + data.tier_target;
    }
  }

  function populateReferral(data) {
    const input = document.querySelector('.refer-input-wrapper input');
    if (input && data.referral_url) input.value = data.referral_url;
  }

  // ─── Copy Referral Link ─────────────────────────────────────
  function copyReferralLink() {
    const input = document.querySelector('.refer-input-wrapper input');
    if (!input) return;
    navigator.clipboard.writeText(input.value).then(() => {
      showToast('Referral link copied!', 'success');
    }).catch(() => {
      input.select();
      document.execCommand('copy');
      showToast('Referral link copied!', 'success');
    });
  }

  // ─── Toast ──────────────────────────────────────────────────
  function showToast(message, type) {
    let container = document.getElementById('rewards-toast-container');
    if (!container) {
      container = document.createElement('div');
      container.id = 'rewards-toast-container';
      container.style.cssText = 'position:fixed;top:24px;right:24px;z-index:9999;display:flex;flex-direction:column;gap:8px;';
      document.body.appendChild(container);
    }
    const toast = document.createElement('div');
    toast.style.cssText = 'padding:12px 20px;border-radius:8px;color:#fff;font-size:14px;font-weight:500;box-shadow:0 4px 12px rgba(0,0,0,0.15);transition:opacity 0.3s;';
    toast.style.background = type === 'success' ? '#12B76A' : '#F04438';
    toast.textContent = message;
    container.appendChild(toast);
    setTimeout(() => { toast.style.opacity = '0'; setTimeout(() => toast.remove(), 300); }, 3000);
  }

  function formatCurrency(cents) {
    return 'USD ' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 0 });
  }

  // ─── Init ───────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', function() {
    loadRewards();

    // Wire copy buttons
    document.querySelectorAll('.copy-link-btn, .copy-icon-btn').forEach(btn => {
      btn.addEventListener('click', function(e) {
        e.preventDefault();
        copyReferralLink();
      });
    });
  });
})();
```

### Step 3.2 – Add script tag to `rewards.html`

Add before `</body>`:
```html
<script src="/static/js/rewards.js"></script>
```

---

## Phase 4: Frontend – HTML Fixes (`rewards.html`)

### Step 4.1 – Fix hardcoded values

| Element | Current | Fix |
|---------|---------|-----|
| `.balance-amount` | `USD 357` | Keep as placeholder, JS overwrites from API |
| `.breakdown-value` (×3) | `USD 1,300`, `USD 180`, `USD 330` | JS overwrites from API |
| `.tp-amount` | `USD 12,500` | JS overwrites from API |
| `.refer-input-wrapper input` | `https://app.poool.com/rewards/1792...` | JS populates from API |

### Step 4.2 – Add unique IDs to key elements

| Element | New ID |
|---------|--------|
| Balance amount | `id="rewards-total-balance"` |
| Tier progress card | `id="rewards-tier-card"` |
| Refer and earn card | `id="rewards-refer-card"` |
| Copy link button | `id="rewards-copy-btn"` |
| Referral input | `id="rewards-referral-input"` |

### Step 4.3 – Wire "View current balance" link

Change `href="#"` to `href="/wallet"` (navigate to wallet for full transaction history).

### Step 4.4 – Add toast container

Add before `</body>`:
```html
<div id="rewards-toast-container" style="position:fixed;top:24px;right:24px;z-index:9999;display:flex;flex-direction:column;gap:8px;"></div>
```

---

## Phase 5: Comprehensive Testing

### Step 5.1 – Create standalone rewards test suite

Create `tests/test_rewards.py` with the following test categories:

**Category 1: Page Structure (9 tests)**

| Test | Expected |
|------|----------|
| `GET /rewards` → 200 | Page loads for authenticated user |
| `GET /rewards` unauthenticated → 302 to login | Auth required |
| Page title is `Rewards - POOOL` | Correct `<title>` |
| Page includes `rewards.css` | Stylesheet linked |
| Page includes `rewards.js` | JS file linked |
| Page includes `user-data.js` | User data JS present |
| Sidebar is present | `.sidebar` or `#sidebar-navigation` |
| KYC banner is present | `#rewards-kyc-banner` |
| Mobile header present | `#mobile-header` |

**Category 2: Rewards Content Elements (12 tests)**

| Test | Expected |
|------|----------|
| Page title "Rewards" with trophy icon | `.rewards-title` contains "Rewards" |
| Balance label "Rewards balance" present | `.balance-label` text |
| Balance amount element exists | `.balance-amount` present |
| "View current balance" link exists | `.view-balance-link` present |
| Cashback breakdown row exists | Text "Cashback" in `.breakdown-label` |
| Referrals breakdown row exists | Text "Referrals" in `.breakdown-label` |
| Promotions breakdown row exists | Text "Promotions" in `.breakdown-label` |
| Tier progress card exists | `.tier-progress-card` present |
| Tier progress bar exists | `.tp-progress-bar` present |
| Refer and earn card exists | `.refer-earn-card` present |
| Referral input field exists | `.refer-input-wrapper input` present |
| Copy link button exists | `.copy-link-btn` present |

**Category 3: Tooltips (4 tests)**

| Test | Expected |
|------|----------|
| Rewards balance tooltip exists | `.tooltip-wrapper` near balance label |
| Cashback tooltip text present | "investments on POOOL" in tooltip |
| Referrals tooltip text present | "friends you invited" in tooltip |
| Promotions tooltip text present | "special offers" in tooltip |

**Category 4: API – GET /api/rewards (8 tests)**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Authenticated → 200 JSON | Returns rewards object |
| Response has `total_balance` | Integer (cents) |
| Response has `cashback` | Integer (cents) |
| Response has `referrals` | Integer (cents) |
| Response has `promotions` | Integer (cents) |
| Response has `referral_url` or `referral_code` | String |
| `total_balance == cashback + referrals + promotions` | Math check |

**Category 5: API – GET /api/rewards/tiers (4 tests)**

| Test | Expected |
|------|----------|
| Unauthenticated → 401 | Auth required |
| Authenticated → 200 JSON array | Returns tier list |
| 5 tiers returned | Intro, Plus, Pro, Elite, Premium |
| Tiers sorted by `sort_order` | Ascending |

**Category 6: Referral System (3 tests)**

| Test | Expected |
|------|----------|
| Referral checklist has 2 items | "Friends get USD 30" and "You get USD 30" |
| Share your link label exists | "Share your link" text |
| Referral URL format valid | Starts with `https://` |

**Category 7: Navigation & Links (4 tests)**

| Test | Expected |
|------|----------|
| Sidebar "Rewards" nav item is active | `.sidebar__nav-item--active` |
| Tier card links to `/tier` | `href="/tier"` |
| "View current balance" links somewhere | `href` is not `#` |
| KYC "Complete KYC" button present | `.kyc-banner-btn-primary` |

**Category 8: CSS & Static Resources (4 tests)**

| Test | Expected |
|------|----------|
| `rewards.css` accessible at `/static/css/rewards.css` | 200 |
| `rewards.js` accessible at `/static/js/rewards.js` | 200 |
| POOOL logo image accessible | `/images/Logo Pool.svg` → 200 |
| Star icon image accessible | `/images/star-01.svg` → 200 |

**Category 9: Database Integrity (4 tests)**

| Test | Expected |
|------|----------|
| `rewards_balances` row exists for test user | Row found |
| `user_tiers` row exists for test user | Row found |
| `referral_codes` row exists for test user | Row found |
| `tiers` table has 5 rows | Intro through Premium |

### Step 5.2 – Update `tests/test_platform.py`

Replace the existing `test_rewards` function (lines 960-978) with expanded version:

```python
def test_rewards(session, results: TestResults):
    """Test /rewards page – COMPREHENSIVE."""
    results.section("PAGE: /rewards")

    html = test_page(session, results, "/rewards", "Rewards",
        expected_styles=["rewards.css"],
        expected_scripts=["rewards.js"],
        expected_elements=[
            "rewards-main",
            "rewards-kyc-banner",
            "rewards-body",
        ],
    )

    if html:
        r = session.get(f"{BASE_URL}/rewards")

        # Content sections
        for text in ["Rewards balance", "Cashback", "Referrals", "Promotions"]:
            if text in r.text:
                results.ok(f"  '{text}' label found")
            else:
                results.fail(f"  '{text}' label MISSING")

        # Tier progress
        if "tier-progress-card" in r.text:
            results.ok("  Tier progress card present")
        else:
            results.fail("  Tier progress card MISSING")

        if "tp-progress-bar" in r.text:
            results.ok("  Progress bar present")
        else:
            results.fail("  Progress bar MISSING")

        # Refer and earn
        if "refer-earn-card" in r.text:
            results.ok("  Refer & earn card present")
        else:
            results.fail("  Refer & earn card MISSING")

        if "Copy link" in r.text:
            results.ok("  Copy link button present")
        else:
            results.fail("  Copy link button MISSING")

        # Tooltips
        tooltip_count = r.text.count("tooltip-wrapper")
        if tooltip_count >= 4:
            results.ok(f"  {tooltip_count} tooltip wrappers found")
        else:
            results.warn(f"  Only {tooltip_count} tooltip wrappers (expected ≥4)")

        # Referral checklist
        if "Friends get USD 30" in r.text:
            results.ok("  Referral friend reward text present")
        else:
            results.warn("  Referral friend reward text missing")

        if "You get USD 30" in r.text:
            results.ok("  Referral self reward text present")
        else:
            results.warn("  Referral self reward text missing")

    # API Tests
    results.section("API: GET /api/rewards")

    r_unauth = requests.get(f"{BASE_URL}/api/rewards")
    if r_unauth.status_code in [401, 302]:
        results.ok("GET /api/rewards rejects unauthenticated")
    else:
        results.warn(f"GET /api/rewards unauthenticated: got {r_unauth.status_code}")

    r_auth = session.get(f"{BASE_URL}/api/rewards")
    if r_auth.status_code == 200:
        results.ok("GET /api/rewards returns 200")
        try:
            data = r_auth.json()
            for field in ["total_balance", "cashback", "referrals", "promotions"]:
                if field in data:
                    results.ok(f"  Field '{field}' present: {data[field]}")
                else:
                    results.fail(f"  Field '{field}' MISSING")
        except Exception:
            results.warn("  Response is not JSON (API may not be implemented yet)")
    elif r_auth.status_code == 404:
        results.warn("GET /api/rewards returns 404 – API not implemented yet")
    else:
        results.fail(f"GET /api/rewards: status={r_auth.status_code}")
```

### Step 5.3 – Run standalone rewards tests

```bash
cd /Users/martin/Projects/poool && python3 tests/test_rewards.py
```

### Step 5.4 – Run full platform test suite

```bash
cd /Users/martin/Projects/poool && python3 tests/test_platform.py
```

### Step 5.5 – Manual browser verification

1. Navigate to `http://localhost:8888/rewards`
2. **Balance Card**:
   - [ ] Total balance shows correct sum of sub-balances
   - [ ] Cashback, Referrals, Promotions values populated from API
   - [ ] All 4 tooltip icons show tooltips on hover
   - [ ] "View current balance" link navigates to `/wallet`
3. **Tier Progress Card**:
   - [ ] Shows correct invested amount from API
   - [ ] Progress bar width reflects actual progress
   - [ ] Tier badge shows correct name (Intro/Plus/Pro/Elite/Premium)
   - [ ] Card links to `/tier` detail page
4. **Refer & Earn Card**:
   - [ ] Referral URL populated from API
   - [ ] Copy button copies URL to clipboard
   - [ ] Toast notification appears on copy
   - [ ] Two checklist items visible
5. **Layout**:
   - [ ] Sidebar shows "Rewards" as active nav item
   - [ ] KYC banner shows at top
   - [ ] Mobile responsive at 768px breakpoint

---

## Phase 6: Restart Backend

After implementing backend changes:

```bash
# Kill the current backend process
pkill -f "cargo run" || true

# Rebuild and restart
cd /Users/martin/Projects/poool/backend && cargo run
```

Wait for "listening on 0.0.0.0:8888" before running tests.
