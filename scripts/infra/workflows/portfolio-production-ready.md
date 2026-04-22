---
description: Make the portfolio system fully production-ready – real DB values, live API, dynamic pie chart, end-to-end verification, and passing tests
---

# /portfolio-production-ready

Make the POOOL portfolio system fully production-ready. This covers:
- **Backend**: New `backend/src/payments/` module or a dedicated `portfolio/` module with `GET /api/portfolio` route
- **API endpoints**: `GET /api/portfolio` → returns user's investments, total value, annual limit, dividends
- **Frontend**: `frontend/platform/portfolio.html`, `frontend/platform/static/js/pie-chart.js`
- **Database tables**: `investments`, `investment_limits`, `dividend_payouts`, `asset_financials`
- **Test suite**: `tests/test_platform.py` (`test_portfolio`, `test_portfolio_dashboard`) + new `test_portfolio_api()`

---

## Phase 1 – Diagnose Current State

1. Start the backend server (see `/start-backend` workflow).

2. Run the full test suite and capture results:
   ```bash
   python3 tests/test_platform.py 2>&1 | tee /tmp/portfolio_test_before.txt
   ```
   Focus on failing sections:
   - `PAGE: /portfolio`
   - `DATABASE – User Data Integrity` (investments, investment_limits)
   - `/api/portfolio – Portfolio data API NOT IMPLEMENTED (404)`

3. Manually test the portfolio page in the browser:
   - Navigate to `http://127.0.0.1:8888/portfolio`
   - Verify the page loads (200 OK, not redirect)
   - Check if **My Assets** table shows real data or is empty/hardcoded
   - Check if the pie chart (financials section) shows real asset breakdown or hardcoded percentages
   - Open DevTools → Network → confirm whether `GET /api/portfolio` is called at all

4. Probe the API directly:
   ```bash
   TOKEN=$(psql -h 127.0.0.1 -d poool -t -c "SELECT s.session_token FROM user_sessions s JOIN users u ON u.id = s.user_id WHERE u.email = 'test@poool.app' AND s.expires_at > NOW() ORDER BY s.created_at DESC LIMIT 1;" | tr -d ' \n')

   curl -s -b "poool_session=$TOKEN" http://127.0.0.1:8888/api/portfolio | python3 -m json.tool
   ```
   Expected: JSON with `investments`, `total_value_cents`, `annual_limit`. Actual: likely 404.

5. Check the database for the test user's portfolio:
   ```bash
   psql -h 127.0.0.1 -d poool -c "
   SELECT i.id, a.title, i.tokens_owned, i.purchase_value_cents, i.current_value_cents, i.status
   FROM investments i
   JOIN assets a ON a.id = i.asset_id
   JOIN users u ON u.id = i.user_id
   WHERE u.email = 'test@poool.app';
   "
   ```
   Note if the `investments` table is empty.

---

## Phase 2 – Fix Database Issues

### 2a. Ensure investments rows exist for the test user

If the test user has no investments, seed one from a completed order OR insert directly:
```sql
-- Seed a test investment for test@poool.app (pick any published asset)
INSERT INTO investments (
    user_id, asset_id,
    tokens_owned, purchase_value_cents, current_value_cents,
    total_rental_cents, status, purchased_at
)
SELECT
    u.id,
    a.id,
    10,
    (a.token_price_cents * 10),
    (a.token_price_cents * 10),
    0,
    'active',
    NOW()
FROM users u, assets a
WHERE u.email = 'test@poool.app'
  AND a.published = TRUE
LIMIT 1
ON CONFLICT (user_id, asset_id) DO NOTHING;
```

### 2b. Ensure investment_limits row exists

```sql
INSERT INTO investment_limits (user_id, annual_limit_cents, invested_12m_cents, limit_year)
SELECT id, 25000000, 0, EXTRACT(YEAR FROM NOW())::INTEGER
FROM users
WHERE email = 'test@poool.app'
ON CONFLICT (user_id, limit_year) DO NOTHING;
```

### 2c. Verify tables exist

```sql
SELECT tablename FROM pg_tables
WHERE schemaname = 'public'
  AND tablename IN ('investments', 'investment_limits', 'dividend_payouts', 'asset_financials');
```

If any tables are missing, run the relevant migration in `database/` or apply the schema from `docs/DATABASE_SCHEMA.md`.

### 2d. Verify `updated_at` trigger on investments

```sql
SELECT trigger_name, event_object_table
FROM information_schema.triggers
WHERE trigger_schema = 'public'
  AND event_object_table = 'investments';
```

If missing, add the trigger (follow existing `update_updated_at_column` pattern in migration files).

---

## Phase 3 – Implement `GET /api/portfolio` Backend Route

The portfolio API endpoint is currently missing (`404`). It must be created.

### 3a. Verify backend compiles

```bash
cd backend && cargo build 2>&1 | head -50
```

Fix any existing compilation errors before adding new code.

### 3b. Create the portfolio module (if it doesn't exist)

If there is no `backend/src/portfolio/` directory, create `mod.rs`, `models.rs`, `routes.rs`, and `service.rs` following the pattern of `backend/src/settings/` or `backend/src/rewards/`.

If a portfolio module already partially exists inside `payments/` or another module, extend it there instead.

### 3c. Define the API response models (`models.rs`)

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct InvestmentItem {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub asset_title: String,
    pub asset_slug: String,
    pub tokens_owned: i32,
    pub purchase_value_cents: i64,
    pub current_value_cents: i64,
    pub total_rental_cents: i64,
    pub appreciation_pct_bps: i32,
    pub status: String,
    pub payout_expected_at: Option<String>,
    pub purchased_at: String,
}

#[derive(Serialize)]
pub struct AnnualLimit {
    pub annual_limit_cents: i64,
    pub invested_12m_cents: i64,
    pub available_cents: i64,
    pub limit_year: i32,
}

#[derive(Serialize)]
pub struct PortfolioResponse {
    pub investments: Vec<InvestmentItem>,
    pub total_value_cents: i64,
    pub total_purchase_cents: i64,
    pub total_rental_cents: i64,
    pub investment_count: usize,
    pub annual_limit: Option<AnnualLimit>,
}
```

### 3d. Implement the service query (`service.rs`)

```rust
pub async fn get_portfolio(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<PortfolioResponse, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            i.id,
            i.asset_id,
            a.title AS asset_title,
            a.slug  AS asset_slug,
            i.tokens_owned,
            i.purchase_value_cents,
            i.current_value_cents,
            i.total_rental_cents,
            COALESCE(i.appreciation_pct_bps, 0) AS appreciation_pct_bps,
            i.status,
            i.payout_expected_at,
            i.purchased_at
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE i.user_id = $1
        ORDER BY i.purchased_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let annual_limit = sqlx::query!(
        r#"
        SELECT annual_limit_cents, invested_12m_cents, available_cents, limit_year
        FROM investment_limits
        WHERE user_id = $1 AND limit_year = EXTRACT(YEAR FROM NOW())::INTEGER
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    // Build response
    let total_value_cents: i64 = rows.iter().map(|r| r.current_value_cents).sum();
    let total_purchase_cents: i64 = rows.iter().map(|r| r.purchase_value_cents).sum();
    let total_rental_cents: i64 = rows.iter().map(|r| r.total_rental_cents).sum();
    let investment_count = rows.len();

    let investments = rows.into_iter().map(|r| InvestmentItem {
        id: r.id,
        asset_id: r.asset_id,
        asset_title: r.asset_title,
        asset_slug: r.asset_slug,
        tokens_owned: r.tokens_owned,
        purchase_value_cents: r.purchase_value_cents,
        current_value_cents: r.current_value_cents,
        total_rental_cents: r.total_rental_cents,
        appreciation_pct_bps: r.appreciation_pct_bps,
        status: r.status,
        payout_expected_at: r.payout_expected_at.map(|t| t.to_rfc3339()),
        purchased_at: r.purchased_at.to_rfc3339(),
    }).collect();

    Ok(PortfolioResponse {
        investments,
        total_value_cents,
        total_purchase_cents,
        total_rental_cents,
        investment_count,
        annual_limit: annual_limit.map(|l| AnnualLimit {
            annual_limit_cents: l.annual_limit_cents,
            invested_12m_cents: l.invested_12m_cents,
            available_cents: l.available_cents,
            limit_year: l.limit_year,
        }),
    })
}
```

### 3e. Add the route handler (`routes.rs`)

```rust
pub async fn get_portfolio_handler(
    State(state): State<AppState>,
    Extension(session): Extension<UserSession>,
) -> Result<Json<PortfolioResponse>, AppError> {
    let portfolio = service::get_portfolio(&state.db, session.user_id).await?;
    Ok(Json(portfolio))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/portfolio", get(get_portfolio_handler))
}
```

### 3f. Register the router in `main.rs`

Add the portfolio router to the app in `main.rs`:
```rust
// In the router/app builder:
.merge(portfolio::routes::router())
```

Also add `mod portfolio;` to the module declarations in `main.rs` if using a new module.

### 3g. Verify route is registered

```bash
cd backend && cargo build 2>&1 | tail -20
# Then restart and test:
curl -s -b "poool_session=$TOKEN" http://127.0.0.1:8888/api/portfolio | python3 -m json.tool
```

Expected response structure:
```json
{
  "investments": [
    {
      "id": "...",
      "asset_title": "Neo Agro",
      "tokens_owned": 10,
      "current_value_cents": 50000,
      "status": "active"
    }
  ],
  "total_value_cents": 50000,
  "total_purchase_cents": 50000,
  "total_rental_cents": 0,
  "investment_count": 1,
  "annual_limit": {
    "annual_limit_cents": 25000000,
    "invested_12m_cents": 0,
    "available_cents": 25000000,
    "limit_year": 2026
  }
}
```

---

## Phase 4 – Connect Frontend to Real Data

### 4a. Check if `portfolio.html` calls `/api/portfolio`

Search `portfolio.html` for any `fetch('/api/portfolio')` or htmx calls. If absent, add a `<script>` that fetches portfolio data on `DOMContentLoaded` and populates the relevant elements.

### 4b. Update the portfolio value card

The portfolio page has a `#portfolio-value-card` section. Ensure it is populated from the API response:

```javascript
// In portfolio.html or a new portfolio.js:
async function loadPortfolio() {
  try {
    const resp = await fetch('/api/portfolio');
    if (!resp.ok) return;
    const data = await resp.json();

    // Total portfolio value
    const totalEl = document.getElementById('portfolio-total-value');
    if (totalEl) totalEl.textContent = '$' + (data.total_value_cents / 100).toFixed(2).replace(/\B(?=(\d{3})+(?!\d))/g, ',');

    // Annual investment limit
    if (data.annual_limit) {
      const limitEl = document.getElementById('annual-limit-available');
      if (limitEl) limitEl.textContent = '$' + (data.annual_limit.available_cents / 100).toFixed(2).replace(/\B(?=(\d{3})+(?!\d))/g, ',');

      const limitBar = document.getElementById('annual-limit-progress');
      if (limitBar && data.annual_limit.annual_limit_cents > 0) {
        const pct = (data.annual_limit.invested_12m_cents / data.annual_limit.annual_limit_cents) * 100;
        limitBar.style.width = pct.toFixed(1) + '%';
      }
    }

    // Assets table
    populateAssetsTable(data.investments);

    // Pie chart
    updatePieChart(data.investments);
  } catch (err) {
    console.error('Portfolio load error:', err);
  }
}

function populateAssetsTable(investments) {
  const tbody = document.getElementById('portfolio-assets-table-body');
  if (!tbody) return;
  if (investments.length === 0) {
    tbody.innerHTML = '<tr><td colspan="5">No investments yet. Browse the <a href="/marketplace">marketplace</a>.</td></tr>';
    return;
  }
  tbody.innerHTML = investments.map(inv => `
    <tr>
      <td>${inv.asset_title}</td>
      <td>$${(inv.current_value_cents / 100).toFixed(2)}</td>
      <td>$${(inv.total_rental_cents / 100).toFixed(2)}</td>
      <td>${inv.tokens_owned} tokens</td>
      <td><span class="status-badge status-${inv.status}">${inv.status}</span></td>
    </tr>
  `).join('');
}

function updatePieChart(investments) {
  if (!investments.length || !window.financialsPieChart) return;
  const totalCents = investments.reduce((s, i) => s + i.current_value_cents, 0);
  if (totalCents === 0) return;

  const colors = ['#98FB96', '#0000FF', '#FF6B6B', '#FFD700', '#9B59B6'];
  const chartData = investments.map((inv, idx) => ({
    label: inv.asset_title,
    percentage: Math.round((inv.current_value_cents / totalCents) * 100),
    color: colors[idx % colors.length],
  }));

  window.financialsPieChart.updateData(chartData);
  if (window.mobileFinancialsPieChart) {
    window.mobileFinancialsPieChart.updateData(chartData);
  }
}

document.addEventListener('DOMContentLoaded', loadPortfolio);
```

### 4c. Verify required element IDs exist in `portfolio.html`

The test suite checks for these IDs – confirm they exist in the HTML:
- `portfolio-main`
- `portfolio-header`
- `portfolio-page-title`
- `portfolio-value-section`
- `portfolio-value-card`
- `portfolio-assets-table`
- `portfolio-chart-section`
- `financials-pie-chart-dynamic` (desktop pie chart container)
- `mobile-financials-pie-chart` (mobile pie chart container)
- `mobile-portfolio-wrapper`

If any are missing, add them to the appropriate sections of `portfolio.html`.

### 4d. Verify `pie-chart.js` is linked in `portfolio.html`

The test suite checks for `pie-chart.js`. Verify:
```html
<script src="/static/js/pie-chart.js"></script>
```
is present in `portfolio.html`.

### 4e. Ensure `portfolio.css` and `mobile-portfolio.css` are linked

Both are checked by the test suite. Confirm these lines exist in `portfolio.html`:
```html
<link rel="stylesheet" href="/static/css/portfolio.css" />
<link rel="stylesheet" href="/static/css/mobile-portfolio.css" />
```

---

## Phase 5 – Ensure Checkout Creates Investments

When a user completes checkout, an `investments` row should be created (or updated). Verify this is happening in `backend/src/payments/service.rs` or `routes.rs`:

1. Find the `checkout` / `complete_order` handler.
2. Confirm there is an `INSERT INTO investments` (or `ON CONFLICT DO UPDATE`) after the order is marked complete.
3. If missing, add the investment creation logic:
   ```rust
   // After order is marked 'completed':
   sqlx::query!(
       r#"
       INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status)
       SELECT $1, oi.asset_id, oi.tokens_quantity,
              oi.subtotal_cents, oi.subtotal_cents, 'active'
       FROM order_items oi
       WHERE oi.order_id = $2
       ON CONFLICT (user_id, asset_id)
       DO UPDATE SET
           tokens_owned = investments.tokens_owned + EXCLUDED.tokens_owned,
           purchase_value_cents = investments.purchase_value_cents + EXCLUDED.purchase_value_cents,
           current_value_cents = investments.current_value_cents + EXCLUDED.current_value_cents,
           updated_at = NOW()
       "#,
       user_id,
       order_id
   )
   .execute(pool)
   .await?;
   ```

4. Also ensure `investment_limits.invested_12m_cents` is updated after each purchase:
   ```rust
   sqlx::query!(
       r#"
       INSERT INTO investment_limits (user_id, annual_limit_cents, invested_12m_cents, limit_year)
       VALUES ($1, 25000000, $2, EXTRACT(YEAR FROM NOW())::INTEGER)
       ON CONFLICT (user_id, limit_year)
       DO UPDATE SET
           invested_12m_cents = investment_limits.invested_12m_cents + $2,
           updated_at = NOW()
       "#,
       user_id,
       total_investment_cents
   )
   .execute(pool)
   .await?;
   ```

---

## Phase 6 – Enhance the Test Suite

Add dedicated portfolio API tests to `test_platform.py`. Add a new `test_portfolio_api()` function, called from `main()` after `test_portfolio(session, results)`:

```python
def test_portfolio_api(session, results: TestResults):
    """Test the /api/portfolio endpoint."""
    results.section("API – /api/portfolio (Portfolio System)")

    # Unauthenticated request
    r = requests.get(f"{BASE_URL}/api/portfolio", timeout=REQUEST_TIMEOUT)
    if r.status_code == 401:
        results.ok("GET /api/portfolio returns 401 when unauthenticated")
    else:
        results.fail("GET /api/portfolio unauth", f"expected 401, got {r.status_code}")

    # Authenticated request
    r = session.get(f"{BASE_URL}/api/portfolio", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /api/portfolio returns 200")
        try:
            data = r.json()
            # Top-level fields
            for field in ["investments", "total_value_cents", "total_purchase_cents",
                          "total_rental_cents", "investment_count"]:
                if field in data:
                    results.ok(f"  /api/portfolio contains '{field}': {data.get(field)}")
                else:
                    results.fail(f"  /api/portfolio MISSING field '{field}'")

            # Annual limit
            if "annual_limit" in data and data["annual_limit"]:
                al = data["annual_limit"]
                results.ok(f"  Annual limit: ${al.get('annual_limit_cents', 0)/100:,.2f}")
                if al.get("available_cents", -1) >= 0:
                    results.ok(f"  Available: ${al['available_cents']/100:,.2f}")
                else:
                    results.warn("  annual_limit.available_cents missing or negative")
            else:
                results.warn("  No annual_limit data in portfolio response")

            # Investments array
            investments = data.get("investments", [])
            if isinstance(investments, list):
                results.ok(f"  {len(investments)} investment(s) returned")
                for inv in investments[:2]:  # Check first 2
                    for field in ["id", "asset_title", "tokens_owned",
                                  "current_value_cents", "status"]:
                        if field in inv:
                            results.ok(f"    Investment.{field}: {inv[field]}")
                        else:
                            results.warn(f"    Investment missing '{field}'")
            else:
                results.fail("  'investments' is not a list")

        except Exception as e:
            results.fail("/api/portfolio JSON parse error", str(e))
    elif r.status_code == 404:
        results.fail("GET /api/portfolio", "404 – endpoint NOT IMPLEMENTED")
    else:
        results.fail("GET /api/portfolio", f"status={r.status_code}")

    # DB verification
    results.section("DATABASE – Portfolio Integrity")
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()

        # investments table
        cur.execute("""
            SELECT COUNT(*) FROM investments i
            JOIN users u ON u.id = i.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        count = cur.fetchone()[0]
        if count > 0:
            results.ok(f"investments: {count} row(s) for test user")
        else:
            results.warn("investments: 0 rows for test user (portfolio will be empty)")

        # investment_limits table
        cur.execute("""
            SELECT annual_limit_cents, invested_12m_cents
            FROM investment_limits
            WHERE user_id = (SELECT id FROM users WHERE email = %s)
        """, (TEST_EMAIL,))
        lim = cur.fetchone()
        if lim:
            results.ok(f"investment_limits: limit={lim[0]/100:.2f}, invested={lim[1]/100:.2f}")
        else:
            results.warn("investment_limits: no row for test user")

        cur.close()
        conn.close()
    except Exception as e:
        results.fail("Portfolio DB check", str(e))
```

Then call it in `main()`:
```python
test_portfolio_api(session, results)
```

---

## Phase 7 – Run and Verify All Tests Pass

1. Restart the backend:
   ```bash
   cd backend && cargo run 2>&1 &
   ```

2. Run the full test suite:
   ```bash
   python3 tests/test_platform.py 2>&1 | tee /tmp/portfolio_test_after.txt
   ```

3. Verify all of the following pass (no ❌):
   - `PAGE: /portfolio` → 200 OK, portfolio.css loaded, pie-chart.js loaded, portfolio content found
   - `API – /api/portfolio (Portfolio System)` → All required fields present, 401 on unauth
   - `DATABASE – Portfolio Integrity` → investments and investment_limits rows present

4. Compare before and after:
   ```bash
   diff /tmp/portfolio_test_before.txt /tmp/portfolio_test_after.txt
   ```
   Confirm the number of ❌ failures has decreased to 0 for portfolio-related sections.

5. Manual browser verification:
   - Open `http://127.0.0.1:8888/portfolio`
   - Confirm real investment data is shown in the **My Assets** table
   - Confirm the pie chart shows real asset percentages (not hardcoded `NEO AGRO 55% / POOOL AGRO 45%`)
   - Confirm the **Annual investment limit** progress bar reflects real DB values
   - Confirm unauthenticated access to `/portfolio` redirects to login

---

## Checklist Summary

- [ ] Backend compiles without errors
- [ ] `GET /api/portfolio` route registered and returning 200 with correct JSON
- [ ] `investments` table has data for test user
- [ ] `investment_limits` table has row for test user
- [ ] Checkout flow creates `investments` row on order completion
- [ ] `investment_limits.invested_12m_cents` updated on purchase
- [ ] `/api/portfolio` returns `401` when unauthenticated
- [ ] `portfolio.html` contains all required IDs (`portfolio-value-card`, `portfolio-assets-table`, etc.)
- [ ] `pie-chart.js` included and updated from real `/api/portfolio` data
- [ ] `portfolio.css` and `mobile-portfolio.css` linked in `portfolio.html`
- [ ] `portfolio.html` JavaScript calls `/api/portfolio` and populates the page
- [ ] `test_portfolio_api()` added to test suite
- [ ] Full test suite passes with 0 failures related to portfolio
