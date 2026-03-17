---
description: Make the rewards system fully production-ready with real DB values, verified API responses, and a passing test suite
---

# Rewards System – Production-Ready Workflow
// turbo-all

## Overview

This workflow makes the entire rewards system production-ready end-to-end:
- Database tables seeded with real, correct values for the test user
- Backend API (`/api/rewards`, `/api/rewards/tiers`) returning accurate live data
- Frontend `rewards.html` wired correctly to all API fields
- Referral code auto-generated and persisted
- Tier progress computed correctly from `user_tiers` + `tiers` tables
- KYC banner fully wired and operational
- Full test suite (`tests/test_rewards.py`) passing with 0 failures

---

## Step 1 – Verify database migrations are applied

Ensure the rewards schema (migration 004) is applied before seeding.

```bash
psql -d poool -c "\dt tiers" 2>&1
psql -d poool -c "\dt rewards_balances" 2>&1
psql -d poool -c "\dt user_tiers" 2>&1
psql -d poool -c "\dt referral_codes" 2>&1
psql -d poool -c "\dt referral_tracking" 2>&1
```

If any table is missing, apply the migration:

```bash
psql -d poool -f database/004_rewards_schema.sql
```

---

## Step 2 – Seed rewards data for the test user

The main `002_seed_data.sql` does NOT seed rewards-specific rows. Run the dedicated rewards seed script below — create it as `database/002b_seed_rewards.sql` if it doesn't exist:

```sql
-- database/002b_seed_rewards.sql
-- Seeds rewards_balances, user_tiers, referral_codes for test@poool.app

BEGIN;

DO $$
DECLARE
    v_user_id UUID;
    v_tier_id INT;
BEGIN
    SELECT id INTO v_user_id FROM users WHERE email = 'test@poool.app' LIMIT 1;

    IF v_user_id IS NULL THEN
        RAISE EXCEPTION 'User test@poool.app not found – run 002_seed_data.sql first';
    END IF;

    -- Get the "Intro" tier id
    SELECT id INTO v_tier_id FROM tiers WHERE name = 'Intro' LIMIT 1;

    -- Seed rewards_balances (cashback=5000, referrals=3000, promotions=2000 cents)
    INSERT INTO rewards_balances (user_id, cashback, referrals, promotions)
    VALUES (v_user_id, 5000, 3000, 2000)
    ON CONFLICT (user_id) DO UPDATE SET
        cashback    = EXCLUDED.cashback,
        referrals   = EXCLUDED.referrals,
        promotions  = EXCLUDED.promotions,
        updated_at  = NOW();

    -- Seed user_tiers (invested_12m = 250000 cents = $2,500 → "Plus" tier range)
    INSERT INTO user_tiers (user_id, tier_id, invested_12m)
    VALUES (v_user_id, v_tier_id, 250000)
    ON CONFLICT (user_id) DO UPDATE SET
        tier_id      = EXCLUDED.tier_id,
        invested_12m = EXCLUDED.invested_12m,
        updated_at   = NOW();

    -- Seed referral_codes
    INSERT INTO referral_codes (user_id, code)
    VALUES (v_user_id, 'TEST1234')
    ON CONFLICT (user_id) DO NOTHING;

    RAISE NOTICE 'Rewards seed data inserted for test@poool.app ✓';
END $$;

COMMIT;
```

Then apply it:
```bash
psql -d poool -f database/002b_seed_rewards.sql
```

---

## Step 3 – Verify seeded data directly in DB

```bash
psql -d poool -c "
SELECT 'rewards_balances' AS tbl, cashback, referrals, promotions
FROM rewards_balances rb
JOIN users u ON u.id = rb.user_id
WHERE u.email = 'test@poool.app';
"

psql -d poool -c "
SELECT t.name AS tier, ut.invested_12m
FROM user_tiers ut
JOIN tiers t ON t.id = ut.tier_id
JOIN users u ON u.id = ut.user_id
WHERE u.email = 'test@poool.app';
"

psql -d poool -c "
SELECT code FROM referral_codes rc
JOIN users u ON u.id = rc.user_id
WHERE u.email = 'test@poool.app';
"

psql -d poool -c "SELECT * FROM tiers ORDER BY sort_order;"
```

Expected: 5 tiers (Intro, Plus, Pro, Elite, Premium), rewards_balances row, user_tiers row, referral_codes row all present.

---

## Step 4 – Verify backend compiles and routes are registered

Check that the rewards router is mounted in `main.rs`:

```bash
grep -n "rewards" backend/src/main.rs
```

Expected output should show something like `.merge(rewards::router())`. If not, open `backend/src/main.rs` and add:

```rust
.merge(crate::rewards::router())
```

Then confirm it compiles:
```bash
cd backend && cargo check 2>&1 | tail -20
```

---

## Step 5 – Start the backend server

```bash
cd backend && cargo run 2>&1 &
sleep 5
curl -s http://localhost:8888/auth/login | head -5
```

The server must respond (even with an HTML page) before proceeding.

---

## Step 6 – Verify the API endpoints manually

Get an authenticated session token from the DB and hit both API endpoints:

```bash
# Get a valid session token for the test user
SESSION_TOKEN=$(psql -d poool -t -c "
    SELECT s.session_token FROM user_sessions s
    JOIN users u ON u.id = s.user_id
    WHERE u.email = 'test@poool.app'
    AND s.expires_at > NOW()
    ORDER BY s.created_at DESC LIMIT 1;
" | tr -d ' \n')

echo "Session token: $SESSION_TOKEN"

# Test GET /api/rewards
curl -s -b "poool_session=$SESSION_TOKEN" http://localhost:8888/api/rewards | python3 -m json.tool

# Test GET /api/rewards/tiers
curl -s -b "poool_session=$SESSION_TOKEN" http://localhost:8888/api/rewards/tiers | python3 -m json.tool
```

**Expected for `/api/rewards`:**
```json
{
  "total_balance": 10000,
  "cashback": 5000,
  "referrals": 3000,
  "promotions": 2000,
  "tier_name": "Intro",
  "tier_target": "Plus",
  "tier_target_amount": 1000000,
  "invested_12m": 250000,
  "progress_pct": 25,
  "referral_code": "TEST1234",
  "referral_url": "https://app.poool.com/rewards/TEST1234"
}
```

**Expected for `/api/rewards/tiers`:**
- Array of 5 objects with `id`, `name`, `min_invest`, `badge_color`, `sort_order`, `cashback_pct`
- Names: `Intro`, `Plus`, `Pro`, `Elite`, `Premium`

If the totals are wrong, trace the issue in `backend/src/rewards/service.rs` → `get_rewards_overview`.

---

## Step 7 – Verify frontend HTML elements are present

Check that `rewards.html` uses the correct DOM IDs that `rewards.js` expects:

```bash
grep -n "rewards-total-balance\|breakdown-value\|tp-amount\|tp-badge\|tp-progress-fill\|tp-hint\|rewards-referral-input\|rewards-copy-btn" \
  frontend/platform/rewards.html
```

All of the above selectors must be present. Also confirm `rewards.js` is included:

```bash
grep -n "rewards.js" frontend/platform/rewards.html
```

If `rewards.js` is missing, add it to the `<head>` or before `</body>` in `rewards.html`:
```html
<script src="/static/js/rewards.js"></script>
```

---

## Step 8 – Verify KYC banner is wired

The test suite checks for `kyc-banner.js` in `rewards.html`:

```bash
grep -n "kyc-banner.js" frontend/platform/rewards.html
```

If missing, add:
```html
<script src="/static/js/kyc-banner.js"></script>
```

Also verify these DOM IDs exist in `rewards.html`:
```bash
grep -n "rewards-kyc-banner\b\|rewards-kyc-banner-complete-btn\|rewards-kyc-banner-learn-more-btn" \
  frontend/platform/rewards.html
```

If any are missing, add the KYC banner HTML block (see `frontend/platform/wallet.html` for reference pattern).

---

## Step 9 – Verify static resources are accessible

With the backend running, check that all static files serve correctly:

```bash
for f in \
  "/static/css/rewards.css" \
  "/static/css/main.css" \
  "/static/css/kyc-banner.css" \
  "/static/css/mobile-kyc-banner.css" \
  "/static/js/user-data.js" \
  "/static/js/rewards.js" \
  "/static/js/kyc-banner.js" \
  "/images/star-01.svg"; do
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:8888$f")
  echo "$STATUS  $f"
done
```

All should return `200`. Any `404` must be fixed (file missing, wrong path, or static directory not configured correctly).

---

## Step 10 – Run the full rewards test suite

```bash
cd /Users/martin/Projects/poool
python3 tests/test_rewards.py 2>&1
```

**Target: 0 failures, all critical checks passing.**

The test suite covers:
1. Page structure (HTTP 200, CSS, JS, sidebar, mobile header)
2. Rewards content elements (balance, tier progress, referral card)
3. Tooltips (≥4 tooltip wrappers)
4. `GET /api/rewards` – schema, math verification
5. `GET /api/rewards/tiers` – 5 tiers with correct names
6. Referral system UI (text, checklist, copy button)
7. Navigation & links (active sidebar state, /tier link)
8. Static resources (CSS/JS/SVG accessibility)
9. Database integrity (tiers table, rewards_balances, user_tiers, referral_codes)
10. KYC banner – HTML structure, JS wiring, static files, API endpoints

---

## Step 11 – Fix failures (if any)

Work through each `❌` failure in the test output. Common issues and fixes:

| Failure | Fix |
|---------|-----|
| `tiers table is empty` | Re-run `psql -d poool -f database/004_rewards_schema.sql` |
| `No rewards_balances row` | Re-run `psql -d poool -f database/002b_seed_rewards.sql` |
| `total_balance mismatch` | Check `service.rs` – ensure `total_balance = cashback + referrals + promotions` |
| `rewards.js NOT wired` | Add `<script src="/static/js/rewards.js"></script>` to `rewards.html` |
| `kyc-banner.js NOT wired` | Add `<script src="/static/js/kyc-banner.js"></script>` to `rewards.html` |
| `GET /api/rewards → 404` | Ensure `rewards::router()` is merged in `main.rs` |
| `GET /api/rewards/tiers → 404` | Same as above, check route `.route("/api/rewards/tiers", ...)` in `rewards/mod.rs` |
| Static file `404` | Check `backend/src/main.rs` static directory mount path |
| `Tier 'X' MISSING` | Re-run the migration 004 to re-seed the `tiers` table |

---

## Step 12 – Re-run tests to confirm 0 failures

```bash
python3 tests/test_rewards.py 2>&1
```

The run is complete when the summary shows:
```
✅ Passed:   N
❌ Failed:   0
⚠️  Warnings: N
```

Warnings for optional/advisory checks are acceptable. Zero failures is mandatory.

---

## Step 13 – Final production checklist

Run these checks manually before marking as production-ready:

- [ ] `GET /rewards` returns HTTP 200 when authenticated
- [ ] `GET /rewards` redirects (302/303) when unauthenticated
- [ ] `GET /api/rewards` returns correct JSON with real DB values
- [ ] `GET /api/rewards/tiers` returns array of 5 tiers
- [ ] Referral URL displayed on page uses correct domain (`https://app.poool.com/rewards/...`)
- [ ] Tier progress bar reflects `invested_12m` vs `tier_target_amount` correctly
- [ ] Copy referral link button works (requires HTTPS or localhost with clipboard permissions)
- [ ] KYC banner shows/hides correctly based on `/api/kyc/status` response
- [ ] All 5 tier names visible on `/tier` page
- [ ] No JS console errors on `/rewards` page
- [ ] Mobile layout renders correctly on 375px viewport
