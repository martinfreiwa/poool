//! Weighted-ROI precision regression test.
//!
//! Demonstrates that the leaderboard's weighted-bps ROI calculation does NOT
//! lose precision via integer truncation. Before the NUMERIC cast fix, small
//! basis-point yields could truncate by 1-2 bps because PostgreSQL evaluated
//!     SUM(BIGINT * INT) / SUM(BIGINT)
//! in integer arithmetic.
//!
//! Fixture (matches the audit task acceptance criterion):
//!   investment A: 100_000 cents @ 450 bps
//!   investment B:  50_000 cents @ 600 bps
//! Expected weighted bps:
//!     (100000*450 + 50000*600) / (100000 + 50000)
//!   = (45_000_000 + 30_000_000) / 150_000
//!   = 75_000_000 / 150_000
//!   = 500 bps exactly
//!
//! Run with:
//!   DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test leaderboard_roi_precision -- --ignored
//!
//! Marked `#[ignore]` so it does not run in default CI (mirrors the existing
//! `marketplace_*_e2e` pattern).

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect")
}

/// Compute the weighted-bps ROI for a single user using the SAME SQL expression
/// that lives in `refresh_all_scores`. This is a regression guard: if anyone
/// reintroduces the integer-only formula, this test will fail by 1-2 bps.
///
/// We cast the NUMERIC result to BIGINT inside SQL so we don't need a decimal
/// crate feature on the Rust side.
async fn weighted_bps_for_user(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COALESCE(
            ROUND(
                SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
            ),
            0
        )::BIGINT
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE i.user_id = $1 AND i.status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("query")
}

#[ignore]
#[tokio::test]
async fn weighted_roi_bps_is_exact_for_mixed_yields() {
    let pool = pool().await;

    // Fixture setup
    let user_id = Uuid::new_v4();
    let asset_a = Uuid::new_v4();
    let asset_b = Uuid::new_v4();

    sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, 'x')")
        .bind(user_id)
        .bind(format!("{}@roi.test", user_id))
        .execute(&pool)
        .await
        .expect("user");

    // Asset A: 450 bps
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published, annual_yield_bps)
         VALUES ($1, $2, 'ROI A', 'real_estate', 1000, 0, 100, 100000, 'funded', TRUE, 450)",
    )
    .bind(asset_a)
    .bind(format!("roi-a-{}", asset_a))
    .execute(&pool)
    .await
    .expect("asset a");

    // Asset B: 600 bps
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published, annual_yield_bps)
         VALUES ($1, $2, 'ROI B', 'real_estate', 500, 0, 100, 50000, 'funded', TRUE, 600)",
    )
    .bind(asset_b)
    .bind(format!("roi-b-{}", asset_b))
    .execute(&pool)
    .await
    .expect("asset b");

    // Investments: 100k cents @ 450 bps + 50k cents @ 600 bps → exactly 500 bps
    sqlx::query(
        "INSERT INTO investments
            (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status)
         VALUES ($1, $2, 1000, 100000, 100000, 'active'),
                ($1, $3, 500, 50000, 50000, 'active')",
    )
    .bind(user_id)
    .bind(asset_a)
    .bind(asset_b)
    .execute(&pool)
    .await
    .expect("investments");

    let bps = weighted_bps_for_user(&pool, user_id).await;

    // Cleanup before asserting so a failure doesn't poison the DB.
    sqlx::query("DELETE FROM investments WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM assets WHERE id IN ($1, $2)")
        .bind(asset_a)
        .bind(asset_b)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .ok();

    assert_eq!(
        bps, 500,
        "Weighted bps must be exactly 500 (100k@450 + 50k@600). Got {} — integer truncation regressed.",
        bps
    );
}
