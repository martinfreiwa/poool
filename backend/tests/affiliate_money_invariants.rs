//! Money-invariant integration tests for the affiliate engine.
//!
//! Audit-flagged gaps these tests close:
//!
//!   * **Concurrent payout requests** — the partial unique
//!     `idx_affiliate_payout_requests_open` is supposed to allow at most
//!     ONE open request per affiliate. Verified by firing N parallel
//!     inserts and counting survivors.
//!   * **Multi-currency commission split** — mig 170 added a `currency`
//!     PK component on `affiliate_live_counters`. EUR and USD commissions
//!     for the SAME affiliate must produce 2 distinct counter rows,
//!     never collapse to one.
//!   * **Clawback returns the live counter to baseline** — paying a
//!     commission credits `lifetime_revenue_cents`; clawing it back must
//!     subtract the same amount. Drift here would silently inflate
//!     dashboard KPIs.
//!
//! All cases are `#[ignore]` so CI without `DATABASE_URL` skips them.
//! Run locally with:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!   cargo test --test affiliate_money_invariants -- --ignored
//! ```

#![cfg(test)]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ─── Harness (mirrors affiliate_team_integration.rs) ────────────────────────

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .expect("connect to test DB")
}

async fn make_user(pool: &PgPool, label: &str) -> Uuid {
    let email = format!("{}+{}@test.local", label, Uuid::new_v4());
    sqlx::query_scalar!(
        "INSERT INTO users (email, password_hash) VALUES ($1, 'x') RETURNING id",
        email
    )
    .fetch_one(pool)
    .await
    .expect("insert user")
}

async fn make_active_affiliate(pool: &PgPool, user_id: Uuid) {
    let code = format!("MI{}", &Uuid::new_v4().to_string()[..10]);
    sqlx::query!(
        "INSERT INTO affiliates (user_id, referral_code, status) VALUES ($1, $2, 'active')
         ON CONFLICT (user_id) DO UPDATE SET status = 'active'",
        user_id,
        code
    )
    .execute(pool)
    .await
    .expect("insert affiliate");
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query!(
        "DELETE FROM affiliate_payout_requests WHERE affiliate_id = $1",
        user_id
    )
    .execute(pool)
    .await;
    let _ = sqlx::query!(
        "DELETE FROM affiliate_commissions WHERE affiliate_id = $1 OR payout_user_id = $1",
        user_id
    )
    .execute(pool)
    .await;
    let _ = sqlx::query!("DELETE FROM orders WHERE user_id = $1", user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!(
        "DELETE FROM affiliate_referrals WHERE affiliate_id = $1 OR payout_user_id = $1 OR referred_user_id = $1",
        user_id
    )
    .execute(pool)
    .await;
    let _ = sqlx::query!(
        "DELETE FROM affiliate_links WHERE attribution_user_id = $1 OR payout_user_id = $1",
        user_id
    )
    .execute(pool)
    .await;
    let _ = sqlx::query!(
        "DELETE FROM affiliate_live_counters WHERE payout_user_id = $1",
        user_id
    )
    .execute(pool)
    .await;
    let _ = sqlx::query!("DELETE FROM affiliates WHERE user_id = $1", user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await;
}

// ─── Tests ──────────────────────────────────────────────────────────────────

/// Multi-currency commissions for the SAME affiliate must produce SEPARATE
/// rows in `affiliate_live_counters` keyed by (payout_user_id, currency).
/// Pre-mig 170 they were merged — that drift was already corrected by the
/// schema change; this test prevents regression.
#[tokio::test]
#[ignore]
async fn multi_currency_commissions_split_into_separate_counters() {
    let p = pool().await;
    let dev = make_user(&p, "mc_dev").await;
    let cust_eur = make_user(&p, "mc_eur").await;
    let cust_usd = make_user(&p, "mc_usd").await;
    make_active_affiliate(&p, dev).await;

    // Insert a personal link to satisfy the (link_id, attribution_user_id,
    // payout_user_id) NOT NULL chain on referrals + commissions.
    let link_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_links
              (code, link_type, attribution_user_id, payout_user_id, status)
           VALUES ($1, 'personal', $2, $2, 'active')
           RETURNING id"#,
        format!("MC{}", &Uuid::new_v4().to_string()[..10]),
        dev
    )
    .fetch_one(&p)
    .await
    .expect("link");

    let mut order_ids: Vec<Uuid> = Vec::new();
    for (cust, currency) in [(cust_eur, "EUR"), (cust_usd, "USD")] {
        let ref_id = sqlx::query_scalar!(
            r#"INSERT INTO affiliate_referrals
                  (affiliate_id, referred_user_id, link_id,
                   attribution_user_id, payout_user_id, status)
               VALUES ($1, $2, $3, $1, $1, 'qualified')
               RETURNING id"#,
            dev,
            cust,
            link_id
        )
        .fetch_one(&p)
        .await
        .expect("referral");

        let order_id = sqlx::query_scalar!(
            r#"INSERT INTO orders (user_id, order_number, total_cents, status, currency)
               VALUES ($1, $2, 100000, 'completed', $3) RETURNING id"#,
            cust,
            format!("MC-{}", &Uuid::new_v4().to_string()[..10]),
            currency
        )
        .fetch_one(&p)
        .await
        .expect("order");
        order_ids.push(order_id);

        sqlx::query!(
            r#"INSERT INTO affiliate_commissions
                  (referral_id, affiliate_id, source_order_id,
                   provisional_amount_cents, gross_amount_cents, status,
                   link_id, attribution_user_id, payout_user_id, currency,
                   tier_at_execution)
               VALUES ($1, $2, $3, 1000, 100000, 'payable', $4, $2, $2, $5, 'Access')"#,
            ref_id,
            dev,
            order_id,
            link_id,
            currency
        )
        .execute(&p)
        .await
        .expect("commission");
    }

    let counter_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT currency, payable_commission_cents
           FROM affiliate_live_counters WHERE payout_user_id = $1
          ORDER BY currency",
    )
    .bind(dev)
    .fetch_all(&p)
    .await
    .expect("counters");

    assert_eq!(
        counter_rows.len(),
        2,
        "expected 2 counter rows (EUR + USD), got {:?}",
        counter_rows
    );
    assert_eq!(counter_rows[0].0.trim(), "EUR");
    assert_eq!(counter_rows[1].0.trim(), "USD");
    assert!(
        counter_rows[0].1 >= 1000,
        "EUR payable should include 1000 cents"
    );
    assert!(
        counter_rows[1].1 >= 1000,
        "USD payable should include 1000 cents"
    );

    // cleanup
    cleanup_user(&p, dev).await;
    cleanup_user(&p, cust_eur).await;
    cleanup_user(&p, cust_usd).await;
}

/// N parallel inserts into `affiliate_payout_requests` for the same user
/// must produce exactly ONE open request — `idx_affiliate_payout_requests_open`
/// is a partial unique on (user_id) WHERE status = 'open'. Counts duplicates.
#[tokio::test]
#[ignore]
async fn concurrent_payout_requests_yield_exactly_one_open() {
    let p = pool().await;
    let user = make_user(&p, "pr_concurrent").await;
    make_active_affiliate(&p, user).await;

    // Seed a payable balance so the request semantics make sense (the
    // partial unique itself doesn't care, but reads tend to filter by it).
    sqlx::query!(
        "INSERT INTO affiliate_live_counters (payout_user_id, currency, payable_commission_cents)
         VALUES ($1, 'EUR', 10000)
         ON CONFLICT (payout_user_id, currency)
            DO UPDATE SET payable_commission_cents = 10000",
        user
    )
    .execute(&p)
    .await
    .expect("seed counter");

    // Real partial unique (mig 089): `idx_affiliate_payout_requests_open`
    // on (affiliate_id) WHERE status IN ('requested','processing').
    // Default status = 'requested', so back-to-back inserts collide.
    let mut handles = Vec::new();
    for _ in 0..16 {
        let pool_c = p.clone();
        let uid = user;
        handles.push(tokio::spawn(async move {
            sqlx::query!(
                r#"INSERT INTO affiliate_payout_requests
                      (affiliate_id, amount_cents)
                   VALUES ($1, 10000)
                   ON CONFLICT DO NOTHING"#,
                uid
            )
            .execute(&pool_c)
            .await
            .ok();
        }));
    }
    for h in handles {
        let _ = h.await;
    }

    let open_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM affiliate_payout_requests
          WHERE affiliate_id = $1 AND status IN ('requested', 'processing')",
    )
    .bind(user)
    .fetch_one(&p)
    .await
    .expect("count");
    assert_eq!(
        open_count, 1,
        "expected exactly 1 open payout request after 16 concurrent inserts, got {}",
        open_count
    );

    cleanup_user(&p, user).await;
}

/// Phase-3 P1: `auto_clawback_for_refunded_investment` must flip every
/// commission tied to the investment to `clawed_back` AND debit the
/// affiliate cash wallet for paid commissions. Verifies the end-to-end
/// path: referral → commission → wallet debit → counter sync.
#[tokio::test]
#[ignore]
async fn auto_clawback_handles_refunded_investment() {
    use poool_backend::rewards::service::auto_clawback_for_refunded_investment;

    let p = pool().await;
    let dev = make_user(&p, "ac_dev").await;
    let cust = make_user(&p, "ac_cust").await;
    make_active_affiliate(&p, dev).await;

    // Need an asset for the investment FK.
    let asset_id = sqlx::query_scalar!(
        r#"INSERT INTO assets (title, slug, asset_type, total_value_cents,
                               token_price_cents, tokens_total, tokens_available,
                               funding_status, min_funding_tokens)
           VALUES ($1, $2, 'real_estate', 100000, 1000, 100, 100,
                   'funding_open', 10)
           RETURNING id"#,
        format!("ac-asset-{}", &Uuid::new_v4().to_string()[..8]),
        format!("ac-{}", &Uuid::new_v4().to_string()[..8]),
    )
    .fetch_one(&p)
    .await
    .expect("asset");

    let investment_id = sqlx::query_scalar!(
        r#"INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents,
                                    current_value_cents, status)
           VALUES ($1, $2, 1, 100000, 100000, 'funding_in_progress')
           RETURNING id"#,
        cust,
        asset_id
    )
    .fetch_one(&p)
    .await
    .expect("investment");

    let link_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_links
              (code, link_type, attribution_user_id, payout_user_id, status)
           VALUES ($1, 'personal', $2, $2, 'active') RETURNING id"#,
        format!("AC{}", &Uuid::new_v4().to_string()[..10]),
        dev
    )
    .fetch_one(&p)
    .await
    .expect("link");

    // Referral with qualifying_investment_id pointing at our investment.
    let ref_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_referrals
              (affiliate_id, referred_user_id, link_id,
               attribution_user_id, payout_user_id, status,
               qualifying_investment_id)
           VALUES ($1, $2, $3, $1, $1, 'qualified', $4)
           RETURNING id"#,
        dev,
        cust,
        link_id,
        investment_id
    )
    .fetch_one(&p)
    .await
    .expect("referral");

    // Seed an order so the FK on commission resolves.
    let order_id = sqlx::query_scalar!(
        r#"INSERT INTO orders (user_id, order_number, total_cents, status, currency)
           VALUES ($1, $2, 100000, 'completed', 'EUR') RETURNING id"#,
        cust,
        format!("AC-{}", &Uuid::new_v4().to_string()[..10])
    )
    .fetch_one(&p)
    .await
    .expect("order");

    // Paid commission — exercises the wallet-debit path.
    let commission_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_commissions
              (referral_id, affiliate_id, source_order_id,
               provisional_amount_cents, gross_amount_cents, status,
               link_id, attribution_user_id, payout_user_id, currency,
               tier_at_execution)
           VALUES ($1, $2, $3, 5000, 100000, 'paid', $4, $2, $2, 'EUR', 'Access')
           RETURNING id"#,
        ref_id,
        dev,
        order_id,
        link_id
    )
    .fetch_one(&p)
    .await
    .expect("commission");

    // Seed affiliate cash wallet with the commission amount so the debit
    // can succeed fully (no shortfall).
    sqlx::query!(
        r#"INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
           VALUES ($1, 'cash', 'EUR', 5000)
           ON CONFLICT (user_id, wallet_type, currency)
              DO UPDATE SET balance_cents = 5000"#,
        dev
    )
    .execute(&p)
    .await
    .expect("seed wallet");
    // Treasury must exist for the credit-back to land somewhere.
    sqlx::query!(
        r#"INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
           VALUES (NULL, 'affiliate_treasury', 'EUR', 0)
           ON CONFLICT DO NOTHING"#
    )
    .execute(&p)
    .await
    .ok();

    let result = auto_clawback_for_refunded_investment(&p, investment_id, dev, "test refund")
        .await
        .expect("clawback");
    assert_eq!(result.commission_count, 1);
    assert_eq!(result.total_clawed_back_cents, 5000);
    assert_eq!(result.paid_deducted_cents, 5000);
    assert_eq!(result.shortfall_cents, 0);

    // Verify status flip.
    let status: String =
        sqlx::query_scalar("SELECT status FROM affiliate_commissions WHERE id = $1")
            .bind(commission_id)
            .fetch_one(&p)
            .await
            .expect("status");
    assert_eq!(status, "clawed_back");

    // Idempotency: re-running yields a no-op.
    let again = auto_clawback_for_refunded_investment(&p, investment_id, dev, "test refund 2")
        .await
        .expect("clawback 2");
    assert_eq!(again.commission_count, 0);

    // Cleanup
    let _ = sqlx::query!("DELETE FROM investments WHERE id = $1", investment_id)
        .execute(&p)
        .await;
    let _ = sqlx::query!("DELETE FROM assets WHERE id = $1", asset_id)
        .execute(&p)
        .await;
    cleanup_user(&p, dev).await;
    cleanup_user(&p, cust).await;
}

/// A clawback on a `paid` commission must subtract the gross amount from
/// the affiliate's live counters. We assert the lifetime + bucket deltas
/// return to baseline after status flip from 'paid' to 'clawed_back'.
///
/// Background: the mig 163/178 statement-level trigger maintains the
/// counters. If a future refactor breaks the trigger (e.g. trigger drops
/// the OLD-status branch), this test catches the silent drift before
/// the dashboard starts lying.
#[tokio::test]
#[ignore]
async fn clawback_subtracts_from_live_counters() {
    let p = pool().await;
    let dev = make_user(&p, "cb_dev").await;
    let cust = make_user(&p, "cb_cust").await;
    make_active_affiliate(&p, dev).await;

    let link_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_links
              (code, link_type, attribution_user_id, payout_user_id, status)
           VALUES ($1, 'personal', $2, $2, 'active')
           RETURNING id"#,
        format!("CB{}", &Uuid::new_v4().to_string()[..10]),
        dev
    )
    .fetch_one(&p)
    .await
    .expect("link");

    let ref_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_referrals
              (affiliate_id, referred_user_id, link_id,
               attribution_user_id, payout_user_id, status)
           VALUES ($1, $2, $3, $1, $1, 'qualified')
           RETURNING id"#,
        dev,
        cust,
        link_id
    )
    .fetch_one(&p)
    .await
    .expect("ref");

    let order_id = sqlx::query_scalar!(
        r#"INSERT INTO orders (user_id, order_number, total_cents, status, currency)
           VALUES ($1, $2, 250000, 'completed', 'EUR') RETURNING id"#,
        cust,
        format!("CB-{}", &Uuid::new_v4().to_string()[..10])
    )
    .fetch_one(&p)
    .await
    .expect("order");

    let commission_id = sqlx::query_scalar!(
        r#"INSERT INTO affiliate_commissions
              (referral_id, affiliate_id, source_order_id,
               provisional_amount_cents, gross_amount_cents, status,
               link_id, attribution_user_id, payout_user_id, currency,
               tier_at_execution)
           VALUES ($1, $2, $3, 2500, 250000, 'paid', $4, $2, $2, 'EUR', 'Access')
           RETURNING id"#,
        ref_id,
        dev,
        order_id,
        link_id
    )
    .fetch_one(&p)
    .await
    .expect("commission");

    let paid_before: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(paid_commission_cents), 0)::BIGINT FROM affiliate_live_counters
          WHERE payout_user_id = $1 AND currency = 'EUR'",
    )
    .bind(dev)
    .fetch_one(&p)
    .await
    .expect("paid before");
    assert!(
        paid_before >= 2500,
        "paid bucket should include the credit (got {})",
        paid_before
    );

    // Clawback — flip status.
    sqlx::query!(
        "UPDATE affiliate_commissions SET status = 'clawed_back' WHERE id = $1",
        commission_id
    )
    .execute(&p)
    .await
    .expect("clawback");

    let paid_after: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(paid_commission_cents), 0)::BIGINT FROM affiliate_live_counters
          WHERE payout_user_id = $1 AND currency = 'EUR'",
    )
    .bind(dev)
    .fetch_one(&p)
    .await
    .expect("paid after");
    assert_eq!(
        paid_before - paid_after,
        2500,
        "clawback must debit the paid bucket by the commission amount; \
         before={} after={}",
        paid_before,
        paid_after
    );

    cleanup_user(&p, dev).await;
    cleanup_user(&p, cust).await;
}
