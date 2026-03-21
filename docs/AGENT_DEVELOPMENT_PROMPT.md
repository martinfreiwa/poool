# 🛡️ POOOL Agent Development Directive — Zero-Defect Financial Engineering

> **What is this file?**
> This is the **mandatory instruction set** for any AI Agent or Human Developer implementing tasks from `IMPLEMENTATION_ROADMAP.md`. Every agent MUST read this file before writing a single line of code. It is designed to produce code that is **bug-free by construction**, **self-healing under failure**, and **secure against financial exploits**.

---

## 📋 PRE-FLIGHT: Before You Write Code

Before implementing ANY task from the roadmap, execute this mental checklist:

```
1. READ this entire file.
2. READ `AGENTS.md` for stack context.
3. CLAIM your task in `IMPLEMENTATION_ROADMAP.md` (update status + assignee).
4. READ the specific Masterplan section referenced by your task (e.g., §3.1.6).
5. IDENTIFY all tables, columns, and Redis keys your code will touch.
6. IDENTIFY all other modules that interact with your code (caller + callee).
7. DETERMINE the failure modes: what happens if DB is down? Redis is empty? Network drops?
8. ONLY THEN begin implementation.
```

---

## 🏗️ ARCHITECTURAL MANDATES (Non-Negotiable)

### 1. Money Is Always `i64` Cents — No Exceptions

```rust
// ✅ CORRECT — All monetary values in cents as i64
let price_cents: i64 = 10500; // $105.00
let fee_cents: i64 = price_cents * fee_bps / 10_000;
let total_cents: i64 = price_cents * quantity as i64;

// ❌ FORBIDDEN — Never use floats for money
let price: f64 = 105.00; // WILL cause rounding drift
let fee: f64 = price * 0.05; // 5.25000000000000001 — WRONG
```

**Rule:** If you see `f64`, `f32`, `FLOAT`, or `REAL` anywhere near a monetary value, it is a **critical bug**. Use `i64` (Rust), `BIGINT` (PostgreSQL), or `integer` (JavaScript — via `Math.round(parseFloat(x) * 100)`).

**Fee Calculation Pattern (basis points):**
```rust
/// Calculates fee in cents from a total in cents and a fee rate in basis points.
/// 500 BPS = 5.00%. Uses integer math to avoid floating-point errors.
fn calculate_fee_cents(total_cents: i64, fee_bps: i32) -> i64 {
    // Guard: fee can never be negative
    let bps = fee_bps.max(0) as i64;
    // Guard: fee can never exceed the total
    let fee = total_cents.saturating_mul(bps) / 10_000;
    fee.min(total_cents)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_fee_calculation() {
        assert_eq!(calculate_fee_cents(10_000, 500), 500);   // $100.00 * 5% = $5.00
        assert_eq!(calculate_fee_cents(1, 500), 0);           // $0.01 * 5% = $0.00 (rounds down)
        assert_eq!(calculate_fee_cents(10_000, 0), 0);        // 0% fee
        assert_eq!(calculate_fee_cents(10_000, 10_000), 10_000); // 100% fee = total
        assert_eq!(calculate_fee_cents(10_000, -100), 0);     // Negative BPS → 0
    }
}
```

### 2. Every Database Write Is a Transaction

```rust
// ✅ CORRECT — ACID transaction for any multi-table operation
let mut tx = pool.begin().await.map_err(AppError::from)?;

sqlx::query!("UPDATE wallets SET balance_cents = balance_cents - $1 WHERE user_id = $2 AND balance_cents >= $1", amount, user_id)
    .execute(&mut *tx).await?;

sqlx::query!("INSERT INTO wallet_transactions (user_id, amount_cents, type) VALUES ($1, $2, 'withdrawal')", user_id, amount)
    .execute(&mut *tx).await?;

tx.commit().await.map_err(AppError::from)?;

// ❌ FORBIDDEN — Two separate queries without a transaction
pool.execute("UPDATE wallets SET ...").await?; // If THIS succeeds...
pool.execute("INSERT INTO wallet_transactions ...").await?; // ...but THIS fails → INCONSISTENT STATE
```

**Rule:** If your code modifies MORE than one table, or modifies a table AND Redis, it MUST be wrapped in a `sqlx::Transaction`. The only exception is read-only queries.

### 3. `SELECT ... FOR UPDATE` for All Balance Reads Before Writes

```rust
// ✅ CORRECT — Lock the row to prevent concurrent modification
let wallet = sqlx::query_as!(Wallet,
    "SELECT * FROM wallets WHERE user_id = $1 AND wallet_type = 'cash' FOR UPDATE",
    user_id
).fetch_one(&mut *tx).await?;

if wallet.balance_cents - wallet.held_balance_cents < required_cents {
    return Err(AppError::InsufficientBalance);
}

// ❌ FORBIDDEN — Reading balance without lock
let wallet = sqlx::query_as!(Wallet,
    "SELECT * FROM wallets WHERE user_id = $1",
    user_id
).fetch_one(pool).await?;
// Another thread could modify the balance between this SELECT and the UPDATE!
```

### 4. Never `unwrap()` in Production Paths

```rust
// ✅ CORRECT — Propagate errors with context
let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
    .fetch_optional(pool).await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or(AppError::NotFound("User not found".into()))?;

// ❌ FORBIDDEN — Panic in production
let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
    .fetch_one(pool).await
    .unwrap(); // PANIC if user doesn't exist → server crash → all users affected
```

**Rule:** Every `.unwrap()` and `.expect()` in a non-test file is a potential server crash. Use `?` with `AppError` propagation, or `.unwrap_or_default()` for non-critical values.

### 5. No `innerHTML` with User-Generated Data (Frontend)

```javascript
// ✅ CORRECT — Use textContent for user data
const el = document.getElementById('username');
el.textContent = user.name; // XSS-safe: HTML entities are escaped

// ✅ CORRECT — DOM construction for complex elements
const row = document.createElement('div');
row.className = 'trade-row';
const priceEl = document.createElement('span');
priceEl.textContent = `$${(trade.price_cents / 100).toFixed(2)}`;
row.appendChild(priceEl);

// ❌ FORBIDDEN — XSS vulnerability
el.innerHTML = `<span>${user.name}</span>`; // If name is "<script>alert('xss')</script>" → XSS
// ❌ FORBIDDEN — Template literal injection
el.innerHTML = `Welcome back, ${userData.displayName}!`; // Same XSS risk
```

**Exception:** `innerHTML` is acceptable ONLY with static, developer-controlled strings (never user input).

---

## 🔄 SELF-HEALING PATTERNS (Make the System Recover Automatically)

### Pattern 1: Redis Orderbook Rebuild on Startup

Every time the server starts, check if Redis is populated. If not, rebuild from PostgreSQL:

```rust
// In main.rs startup sequence
if let Some(redis) = &state.redis {
    let key_count: i64 = redis::cmd("DBSIZE").query_async(&mut *conn).await.unwrap_or(0);
    if key_count == 0 {
        tracing::warn!("⚠️ Redis is empty — rebuilding orderbook from PostgreSQL");
        match orderbook::rebuild_from_postgres(redis, &state.db).await {
            Ok(n) => tracing::info!("✅ Orderbook rebuilt: {} orders loaded", n),
            Err(e) => tracing::error!("🔴 Orderbook rebuild FAILED: {}", e),
        }
    }
}
```

### Pattern 2: Background Redis-Sync Worker (Every 5 Minutes)

Detect and fix orderbook drift between Redis and PostgreSQL:

```rust
pub async fn run_redis_sync_worker(redis: &RedisPool, pool: &PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    loop {
        interval.tick().await;
        match sync_redis_with_postgres(redis, pool).await {
            Ok(fixed) if fixed > 0 => {
                tracing::warn!("🔧 Redis sync: re-inserted {} missing orders", fixed);
                sentry::capture_message(
                    &format!("Redis drift detected: {} orders re-synced", fixed),
                    sentry::Level::Warning,
                );
            }
            Err(e) => tracing::error!("Redis sync failed: {}", e),
            _ => {} // 0 fixed = healthy
        }
    }
}
```

### Pattern 3: Graceful Degradation When Redis Is Down

```rust
// Trading should degrade gracefully, not crash
async fn submit_order(state: &AppState, order: NewOrder) -> Result<Json<Value>, AppError> {
    // 1. Check if trading is enabled (Redis-based kill switch)
    match check_trading_enabled(&state.redis).await {
        Ok(_) => {},
        Err(_) => {
            // Redis down → assume trading is enabled (fail-open for availability)
            // BUT log it as a critical warning
            tracing::error!("🔴 Redis unreachable — cannot check kill-switch, proceeding with caution");
            sentry::capture_message("Redis unreachable during order submission", sentry::Level::Error);
        }
    }

    // 2. Rate limiting (Redis-based)
    if let Err(_) = check_rate_limit(&state.redis, order.user_id).await {
        // Redis down → skip rate limiting but log
        tracing::warn!("Rate limiter unavailable, allowing request");
    }

    // 3. The DB transaction is the real safety net (never skip this)
    let result = execute_order_in_transaction(&state.db, &order).await?;

    // 4. Try to insert into Redis orderbook (best-effort)
    if let Err(e) = insert_into_redis_orderbook(&state.redis, &result).await {
        tracing::error!("Failed to insert order into Redis: {} — will be caught by sync worker", e);
        // The 5-minute sync worker (Pattern 2) will fix this automatically
    }

    Ok(Json(result))
}
```

### Pattern 4: Automatic Order Expiry with Balance Recovery

```rust
pub async fn run_order_expiry_worker(redis: &RedisPool, pool: &PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); // hourly
    loop {
        interval.tick().await;

        let expired = sqlx::query_as!(MarketOrder,
            "SELECT * FROM market_orders WHERE status IN ('open', 'partially_filled') AND expires_at < NOW()"
        ).fetch_all(pool).await.unwrap_or_default();

        for order in expired {
            let mut tx = match pool.begin().await {
                Ok(tx) => tx,
                Err(e) => {
                    tracing::error!("Failed to begin tx for order expiry: {}", e);
                    continue; // Don't crash — try next order
                }
            };

            // 1. Release the held balance/tokens
            if order.side == "buy" {
                let remaining = order.quantity - order.quantity_filled;
                let held_release = order.price_cents * remaining as i64;
                let _ = sqlx::query!(
                    "UPDATE wallets SET held_balance_cents = held_balance_cents - $1 WHERE user_id = $2",
                    held_release, order.user_id
                ).execute(&mut *tx).await;
            } else {
                let remaining = order.quantity - order.quantity_filled;
                let _ = sqlx::query!(
                    "UPDATE investments SET held_tokens = held_tokens - $1 WHERE user_id = $2 AND asset_id = $3",
                    remaining, order.user_id, order.asset_id
                ).execute(&mut *tx).await;
            }

            // 2. Mark as expired
            let _ = sqlx::query!(
                "UPDATE market_orders SET status = 'expired', updated_at = NOW() WHERE id = $1",
                order.id
            ).execute(&mut *tx).await;

            // 3. Remove from Redis orderbook
            let _ = orderbook::remove_order(redis, &order).await;

            if let Err(e) = tx.commit().await {
                tracing::error!("Failed to commit order expiry for {}: {}", order.id, e);
            } else {
                tracing::info!("⏰ Expired order {} — balance released", order.id);
            }
        }
    }
}
```

### Pattern 5: Reconciliation Auto-Alert

```rust
pub async fn run_reconciliation_worker(pool: &PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400)); // daily
    loop {
        interval.tick().await;

        let result = run_full_reconciliation(pool).await;

        match result {
            Ok(report) if report.cash_delta_cents == 0 && report.token_mismatches == 0 => {
                tracing::info!("✅ Daily reconciliation PASSED");
            }
            Ok(report) => {
                let msg = format!(
                    "🔴 RECONCILIATION MISMATCH: cash_delta={} cents, token_mismatches={}, fee_delta={} cents",
                    report.cash_delta_cents, report.token_mismatches, report.fee_delta_cents
                );
                tracing::error!("{}", msg);
                sentry::capture_message(&msg, sentry::Level::Fatal);
                // The admin dashboard reconciliation page will show this to humans
            }
            Err(e) => {
                tracing::error!("Reconciliation worker crashed: {}", e);
                sentry::capture_message(
                    &format!("Reconciliation worker CRASHED: {}", e),
                    sentry::Level::Fatal,
                );
            }
        }

        // Always store the report (even if it fails, store the failure)
        let _ = store_reconciliation_report(pool, &result).await;
    }
}
```

---

## 🧪 TESTING MANDATES (Every Task Must Have Tests)

### Mandate 1: Every Financial Function Gets a Unit Test

For any function that touches money (balances, fees, trades), write at least these test cases:

```
✅ Happy path (normal operation)
✅ Zero amount (should it succeed or fail?)
✅ Negative amount (must always fail)
✅ Exact balance (user has exactly the right amount — boundary test)
✅ Insufficient balance (must fail gracefully, no panic)
✅ Overflow protection (i64::MAX — will the math overflow?)
✅ Concurrent access (two operations on the same wallet simultaneously)
```

### Mandate 2: Every API Endpoint Gets Request Validation Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_order_submission_missing_fields(pool: PgPool) {
        // Missing price → should return 400, not 500
    }

    #[sqlx::test]
    async fn test_order_submission_negative_price(pool: PgPool) {
        // price_cents = -100 → should return 400 "Price must be positive"
    }

    #[sqlx::test]
    async fn test_order_submission_zero_quantity(pool: PgPool) {
        // quantity = 0 → should return 400 "Quantity must be >= 1"
    }

    #[sqlx::test]
    async fn test_order_submission_unauthenticated(pool: PgPool) {
        // No session cookie → 401
    }

    #[sqlx::test]
    async fn test_order_submission_no_kyc(pool: PgPool) {
        // User exists but is_kyc_verified = false → 403
    }

    #[sqlx::test]
    async fn test_double_submission_idempotency(pool: PgPool) {
        // Same idempotency key twice → second request returns same order, no duplicate
    }
}
```

### Mandate 3: Every Settlement Gets a Reconciliation Assertion

After any test that moves money, assert that the system is in balance:

```rust
async fn assert_system_balanced(pool: &PgPool) {
    let total_wallets: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance_cents + held_balance_cents), 0) FROM wallets WHERE wallet_type = 'cash'"
    ).fetch_one(pool).await.unwrap();

    let total_deposits: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM wallet_transactions WHERE type = 'deposit' AND status = 'completed'"
    ).fetch_one(pool).await.unwrap();

    let total_withdrawals: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM wallet_transactions WHERE type = 'withdrawal' AND status = 'completed'"
    ).fetch_one(pool).await.unwrap();

    let total_purchases: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_cents), 0) FROM orders WHERE status = 'completed'"
    ).fetch_one(pool).await.unwrap();

    let expected = total_deposits - total_withdrawals - total_purchases;
    assert_eq!(total_wallets, expected, "SYSTEM IMBALANCE DETECTED: wallets={}, expected={}", total_wallets, expected);
}
```

### Mandate 4: Concurrent Race Condition Tests

```rust
#[sqlx::test]
async fn test_concurrent_buy_orders_same_asset(pool: PgPool) {
    // Setup: Asset with 10 tokens available, 2 users each try to buy 7
    // Expected: Only ONE succeeds, the other gets InsufficientTokens
    // This tests FOR UPDATE locking

    let handles: Vec<_> = (0..2).map(|i| {
        let pool = pool.clone();
        tokio::spawn(async move {
            buy_tokens(&pool, user_ids[i], asset_id, 7).await
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles).await;
    let successes = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
    assert_eq!(successes, 1, "Exactly one buy should succeed with 10 tokens available");

    // Assert: Total tokens sold = exactly 7 (not 14!)
    assert_system_balanced(&pool).await;
}
```

---

## 🔒 SECURITY CHECKLIST (Before Merging ANY Code)

Every agent must verify these before marking a task as `✅ DONE`:

```
□ No unwrap() or expect() in non-test code
□ No innerHTML with user-generated data
□ No f64/f32 for monetary values
□ All multi-table writes use transactions
□ All balance reads use FOR UPDATE
□ All user inputs are validated (length, type, range)
□ All API endpoints check authentication (session cookie)
□ All admin endpoints check authorization (permission guard)
□ Rate limiting is applied to state-changing endpoints
□ Idempotency key is used for financial operations
□ Error messages don't leak internal details to the client
□ SQL queries use parameterized bindings ($1, $2), never string concatenation
□ Audit log entry is created for admin actions
□ Unit tests cover happy path + edge cases + failure cases
□ Reconciliation assertion passes after test
```

---

## 📝 CODE STRUCTURE TEMPLATE

When creating a new module in `backend/src/marketplace/`, follow this structure:

```
marketplace/
├── mod.rs            # Re-exports, route registration
├── models.rs         # Structs with serde + sqlx derives
├── routes.rs         # Axum handler functions (thin — delegate to service)
├── service.rs        # Business logic (testable without HTTP)
├── validation.rs     # Input validation functions
└── tests/            # Integration tests
    ├── mod.rs
    └── test_*.rs
```

**Route Handler Pattern (thin handler, fat service):**

```rust
// routes.rs — THIN handler, only does HTTP concerns
pub async fn submit_order(
    State(state): State<AppState>,
    session: AuthenticatedSession,        // Extracted by middleware
    Json(body): Json<SubmitOrderRequest>,
) -> Result<Json<OrderResponse>, AppError> {
    // 1. Validate input
    validation::validate_order_request(&body)?;

    // 2. Delegate to service (all business logic lives here)
    let order = service::create_order(&state.db, &state.redis, session.user_id, body).await?;

    // 3. Return response
    Ok(Json(OrderResponse::from(order)))
}

// service.rs — FAT service, contains all business logic (testable!)
pub async fn create_order(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: Uuid,
    req: SubmitOrderRequest,
) -> Result<MarketOrder, AppError> {
    // All the real logic here — can be tested without HTTP
}
```

---

## 🏥 ERROR HANDLING PATTERN

```rust
// error.rs — Extend AppError for marketplace
pub enum AppError {
    // ... existing variants ...

    // Marketplace-specific errors
    InsufficientBalance { available_cents: i64, required_cents: i64 },
    InsufficientTokens { available: i32, required: i32 },
    OrderNotFound(Uuid),
    OrderAlreadyCancelled(Uuid),
    TradingDisabled,
    TwoFactorRequired,
    WashTradingBlocked,
    RateLimitExceeded { retry_after_secs: u64 },
    OrderRejected(String), // reason
    ServiceUnavailable(String),
}

// Map to HTTP status codes
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::InsufficientBalance { .. } => (StatusCode::BAD_REQUEST, "Insufficient balance"),
            AppError::TwoFactorRequired => (StatusCode::PRECONDITION_REQUIRED, "2FA verification required"), // 428
            AppError::TradingDisabled => (StatusCode::SERVICE_UNAVAILABLE, "Trading is currently paused"),
            AppError::RateLimitExceeded { retry_after_secs } => {
                // Include Retry-After header
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("Retry-After", retry_after_secs.to_string())],
                    Json(json!({ "error": "Rate limit exceeded" }))
                ).into_response();
            }
            AppError::WashTradingBlocked => (StatusCode::FORBIDDEN, "Self-trading is not allowed"),
            // ... etc
        };

        // NEVER leak internal error details to the client
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## 🧭 DECISION TREE: What To Do When Stuck

```
Is this a financial operation (moves money or tokens)?
├── YES → Use transaction, FOR UPDATE, i64, test reconciliation
│         └── Does it touch Redis too? → Update Redis AFTER DB commit (not inside TX)
└── NO  → Standard error handling is sufficient

Is this user-facing output?
├── YES → Use textContent (never innerHTML), format cents as dollars client-side
└── NO  → Log with tracing::info/warn/error, include relevant IDs

Is this an admin action?
├── YES → Check permission, create audit log entry, confirm dialog on destructive actions
└── NO  → Standard auth check (session cookie)

Does this need to survive a server restart?
├── YES → Store in PostgreSQL (source of truth)
└── NO  → Store in Redis with appropriate TTL

Can this fail silently?
├── YES (non-critical) → Log warning, continue execution, self-heal later
└── NO (financial)     → Return error to user, rollback transaction, alert Sentry
```

---

## 📊 COMPLETION CRITERIA (When Is a Task REALLY Done?)

A task is `✅ DONE` when ALL of the following are true:

1. **Code compiles** — `cargo check` passes with zero warnings
2. **Linting passes** — `cargo clippy` has no warnings  
3. **Formatting** — `cargo fmt` has been run
4. **Unit tests pass** — All `#[test]` and `#[sqlx::test]` functions pass
5. **Edge cases tested** — Zero, negative, overflow, concurrent, unauthorized
6. **Reconciliation holds** — After test, system balances are correct
7. **No unwrap in prod** — Verified by grep: `rg 'unwrap\(\)' src/ --glob '!*test*'`
8. **Security checklist passed** — All items above are checked
9. **Roadmap updated** — Task marked `✅ DONE`, E2E column updated
10. **Agent log updated** — Check-out entry in Live Agent Logs

---

*This directive is versioned. Last updated: 2026-03-21. Source truth: `docs/MASTERPLAN.md`.*
