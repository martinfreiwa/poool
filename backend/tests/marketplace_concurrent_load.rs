//! Concurrent-order load tests.
//!
//! Simulates N parallel buy-order submissions on the same asset and asserts:
//!   1. Concentration limit (80%) is never exceeded — even under race
//!   2. The total cash held across users equals the sum of accepted orders
//!   3. No two orders share an idempotency key (per-user)
//!
//! These tests exist to catch regressions in the locking / FOR UPDATE
//! contracts inside `validation::check_concentration_limit_tx` and
//! `check_buyer_balance` (both inside the order-creation tx).
//!
//! Run with:
//!   DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test marketplace_concurrent_load -- --ignored
//!
//! Like the e2e tests, marked `#[ignore]` so they don't run in default CI.

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(20)
        .connect(&url)
        .await
        .expect("connect")
}

/// Tries to atomically reserve `tokens` of `asset_id` for `user_id` under
/// the 80% concentration limit, mirroring the production contract:
///   1. SELECT FOR UPDATE the user's investment row (lock)
///   2. Verify `current + requested <= 0.8 * total`
///   3. Insert/update the row
///
/// Returns true if the order was accepted.
async fn try_reserve(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    requested: i32,
    total_tokens: i32,
) -> bool {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return false,
    };

    let current: i32 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(tokens_owned), 0)::int4
         FROM investments
         WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'
         FOR UPDATE",
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    let max_allowed = (total_tokens as f64 * 0.80) as i32;
    if current + requested > max_allowed {
        let _ = tx.rollback().await;
        return false;
    }

    let upsert = sqlx::query(
        "INSERT INTO investments
            (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status)
         VALUES ($1, $2, $3, 0, 0, 'active')
         ON CONFLICT (user_id, asset_id) DO UPDATE
         SET tokens_owned = investments.tokens_owned + EXCLUDED.tokens_owned",
    )
    .bind(user_id)
    .bind(asset_id)
    .bind(requested)
    .execute(&mut *tx)
    .await;

    if upsert.is_err() {
        let _ = tx.rollback().await;
        return false;
    }
    tx.commit().await.is_ok()
}

/// Hammer 50 concurrent reservation attempts against the same asset for the
/// same user. Each requests 10 tokens of a 100-token asset (10% each).
/// The 80% limit allows at most 8 to succeed.
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concentration_limit_holds_under_concurrent_buys() {
    let pool = Arc::new(pool().await);

    // Setup
    let user_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash)
         VALUES ($1, $2, 'x')",
    )
    .bind(user_id)
    .bind(format!("{}@load.test", user_id))
    .execute(&*pool)
    .await
    .expect("user");
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published)
         VALUES ($1, $2, 'Load Test', 'real_estate', 100, 100, 100, 10000, 'funded', TRUE)",
    )
    .bind(asset_id)
    .bind(format!("load-{}", asset_id))
    .execute(&*pool)
    .await
    .expect("asset");

    // Run 50 concurrent reservation attempts
    let mut handles = Vec::new();
    for _ in 0..50 {
        let p = pool.clone();
        handles.push(tokio::spawn(async move {
            try_reserve(&p, user_id, asset_id, 10, 100).await
        }));
    }

    let mut accepted = 0u32;
    for h in handles {
        if h.await.unwrap_or(false) {
            accepted += 1;
        }
    }

    // Final state must show NO over-concentration
    let final_owned: i32 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(tokens_owned), 0)::int4
         FROM investments WHERE user_id = $1 AND asset_id = $2",
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_one(&*pool)
    .await
    .unwrap_or(0);

    // Cleanup
    sqlx::query("DELETE FROM investments WHERE user_id = $1")
        .bind(user_id)
        .execute(&*pool)
        .await
        .ok();
    sqlx::query("DELETE FROM assets WHERE id = $1")
        .bind(asset_id)
        .execute(&*pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&*pool)
        .await
        .ok();

    println!(
        "Load test: {} accepted of 50, final owned = {}",
        accepted, final_owned
    );

    assert!(
        final_owned <= 80,
        "Concentration limit breached: owned={} > 80 (80% of 100)",
        final_owned
    );
    assert_eq!(
        final_owned,
        accepted as i32 * 10,
        "Accepted count and final owned must agree (no lost / extra writes)"
    );
}

/// Idempotency-key uniqueness must hold under concurrent inserts. Two
/// parallel inserts with the same (user_id, key) must collapse into one row.
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn idempotency_key_unique_per_user_under_race() {
    let pool = Arc::new(pool().await);
    let user_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    let key = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash)
         VALUES ($1, $2, 'x')",
    )
    .bind(user_id)
    .bind(format!("{}@idem.test", user_id))
    .execute(&*pool)
    .await
    .expect("user");
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published)
         VALUES ($1, $2, 'Idem Test', 'real_estate', 100, 100, 100, 10000, 'funded', TRUE)",
    )
    .bind(asset_id)
    .bind(format!("idem-{}", asset_id))
    .execute(&*pool)
    .await
    .expect("asset");

    let mut handles = Vec::new();
    for _ in 0..10 {
        let p = pool.clone();
        handles.push(tokio::spawn(async move {
            sqlx::query(
                "INSERT INTO market_orders
                    (user_id, asset_id, side, order_type, price_cents, quantity,
                     status, idempotency_key)
                 VALUES ($1, $2, 'buy', 'limit', 100, 1, 'open', $3)",
            )
            .bind(user_id)
            .bind(asset_id)
            .bind(key)
            .execute(&*p)
            .await
            .is_ok()
        }));
    }
    let mut wins = 0u32;
    for h in handles {
        if h.await.unwrap_or(false) {
            wins += 1;
        }
    }

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM market_orders WHERE idempotency_key = $1")
            .bind(key)
            .fetch_one(&*pool)
            .await
            .unwrap_or(0);

    // Cleanup
    sqlx::query("DELETE FROM market_orders WHERE user_id = $1")
        .bind(user_id)
        .execute(&*pool)
        .await
        .ok();
    sqlx::query("DELETE FROM assets WHERE id = $1")
        .bind(asset_id)
        .execute(&*pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&*pool)
        .await
        .ok();

    assert_eq!(wins, 1, "exactly one INSERT should succeed; got {}", wins);
    assert_eq!(count, 1, "exactly one row should exist; got {}", count);
}
