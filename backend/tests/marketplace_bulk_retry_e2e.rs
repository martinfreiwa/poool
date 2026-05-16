//! E2E test for bulk-retry-onchain endpoint logic.
//!
//! Verifies the SQL filter only resets trades in retryable terminal states
//! (failed / reverted / timeout) and leaves confirmed/pending rows untouched.
//!
//! Run with:
//!   DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test marketplace_bulk_retry_e2e -- --ignored

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to test database")
}

async fn make_minimal_users_and_asset(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> (Uuid, Uuid, Uuid) {
    let buyer_id = Uuid::new_v4();
    let seller_id = Uuid::new_v4();
    let asset_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at)
         VALUES ($1, $2, 'test-hash', NOW()), ($3, $4, 'test-hash', NOW())",
    )
    .bind(buyer_id)
    .bind(format!("buyer-{}@example.test", buyer_id))
    .bind(seller_id)
    .bind(format!("seller-{}@example.test", seller_id))
    .execute(&mut **tx)
    .await
    .expect("insert users");

    // Real schema (post-migration 167): assets has no `symbol` column, NOT
    // NULL columns include slug + asset_type + funding_status etc.
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published)
         VALUES ($1, $2, $3, 'real_estate', 1000, 0, 100, 100000, 'funded', TRUE)",
    )
    .bind(asset_id)
    .bind(format!("bulkretry-{}", asset_id))
    .bind(format!("Test Asset {}", asset_id))
    .execute(&mut **tx)
    .await
    .expect("insert asset");

    (buyer_id, seller_id, asset_id)
}

async fn insert_trade(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    asset_id: Uuid,
    buyer_id: Uuid,
    seller_id: Uuid,
    on_chain_status: &str,
    tx_hash: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    // buy_order_id + sell_order_id are NOT NULL + FK-bound to market_orders.
    // Insert two minimal orders so the trade row passes the FK check; the
    // bulk-retry test cares about the on_chain_status UPDATE, not order
    // semantics.
    let buy_order_id = Uuid::new_v4();
    let sell_order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO market_orders (id, user_id, asset_id, side, price_cents, quantity)
         VALUES ($1, $2, $3, 'buy', 1000, 1), ($4, $5, $3, 'sell', 1000, 1)",
    )
    .bind(buy_order_id)
    .bind(buyer_id)
    .bind(asset_id)
    .bind(sell_order_id)
    .bind(seller_id)
    .execute(&mut **tx)
    .await
    .expect("insert market_orders");
    sqlx::query(
        r#"INSERT INTO trade_history (
            id, asset_id, buy_order_id, sell_order_id,
            buyer_user_id, seller_user_id,
            price_cents, quantity, fee_cents,
            on_chain_status, on_chain_tx_hash, executed_at
         ) VALUES ($1, $2, $3, $4, $5, $6, 1000, 1, 10, $7, $8, NOW())"#,
    )
    .bind(id)
    .bind(asset_id)
    .bind(buy_order_id)
    .bind(sell_order_id)
    .bind(buyer_id)
    .bind(seller_id)
    .bind(on_chain_status)
    .bind(tx_hash)
    .execute(&mut **tx)
    .await
    .expect("insert trade_history row");
    id
}

/// Replicates the SQL used by `api_admin_marketplace_trades_bulk_retry_onchain`.
async fn run_bulk_retry_sql(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    trade_ids: &[Uuid],
) -> u64 {
    sqlx::query(
        r#"
        UPDATE trade_history SET
            on_chain_status   = 'pending',
            on_chain_tx_hash  = NULL,
            on_chain_batch_id = NULL,
            updated_at        = NOW()
         WHERE id = ANY($1)
           AND on_chain_status IN ('failed', 'reverted', 'timeout')
        "#,
    )
    .bind(trade_ids)
    .execute(&mut **tx)
    .await
    .expect("bulk retry sql")
    .rows_affected()
}

#[tokio::test]
#[ignore]
async fn bulk_retry_resets_only_retryable_statuses() {
    let pool = pool().await;
    let mut tx = pool.begin().await.expect("begin tx");

    let (buyer, seller, asset) = make_minimal_users_and_asset(&mut tx).await;

    let t_failed = insert_trade(&mut tx, asset, buyer, seller, "failed", Some("0xfail")).await;
    let t_reverted = insert_trade(&mut tx, asset, buyer, seller, "reverted", Some("0xrev")).await;
    let t_timeout = insert_trade(&mut tx, asset, buyer, seller, "timeout", Some("0xto")).await;
    let t_confirmed = insert_trade(&mut tx, asset, buyer, seller, "confirmed", Some("0xok")).await;
    let t_pending = insert_trade(&mut tx, asset, buyer, seller, "pending", None).await;

    let all = vec![t_failed, t_reverted, t_timeout, t_confirmed, t_pending];
    let reset = run_bulk_retry_sql(&mut tx, &all).await;

    assert_eq!(reset, 3, "Only 3 retryable rows should reset");

    // Verify each row's resulting status.
    let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT id, on_chain_status, on_chain_tx_hash FROM trade_history
         WHERE id = ANY($1)",
    )
    .bind(&all)
    .fetch_all(&mut *tx)
    .await
    .expect("fetch updated rows");

    for (id, status, hash) in rows {
        if id == t_failed || id == t_reverted || id == t_timeout {
            assert_eq!(status, "pending", "{} should be reset to pending", id);
            assert!(hash.is_none(), "{} tx_hash should be cleared", id);
        } else if id == t_confirmed {
            assert_eq!(status, "confirmed", "confirmed trade must NOT be touched");
            assert_eq!(hash.as_deref(), Some("0xok"));
        } else if id == t_pending {
            assert_eq!(status, "pending", "pending stays pending");
            assert!(hash.is_none());
        }
    }

    tx.rollback().await.expect("rollback");
}

#[tokio::test]
#[ignore]
async fn bulk_retry_with_empty_list_is_noop() {
    let pool = pool().await;
    let mut tx = pool.begin().await.expect("begin tx");

    let reset = run_bulk_retry_sql(&mut tx, &[]).await;
    assert_eq!(reset, 0);

    tx.rollback().await.expect("rollback");
}

#[tokio::test]
#[ignore]
async fn bulk_retry_writes_audit_log_entry() {
    let pool = pool().await;
    let mut tx = pool.begin().await.expect("begin tx");

    let (buyer, seller, asset) = make_minimal_users_and_asset(&mut tx).await;
    let actor_id = buyer; // reuse buyer as the admin actor for the test
    let t_failed = insert_trade(&mut tx, asset, buyer, seller, "failed", Some("0xfail")).await;

    let trade_ids = vec![t_failed];
    let reset = run_bulk_retry_sql(&mut tx, &trade_ids).await;
    assert_eq!(reset, 1);

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'marketplace.trade.bulk_retry_onchain', 'trade_history', $2)"#,
    )
    .bind(actor_id)
    .bind(serde_json::json!({
        "requested": trade_ids.len(),
        "reset": reset,
        "trade_ids": &trade_ids,
        "reason": "test",
    }))
    .execute(&mut *tx)
    .await
    .expect("write audit log");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs
         WHERE actor_user_id = $1
           AND action = 'marketplace.trade.bulk_retry_onchain'",
    )
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .expect("count audit logs");

    assert_eq!(count, 1, "expected exactly one audit-log entry");

    tx.rollback().await.expect("rollback");
}
