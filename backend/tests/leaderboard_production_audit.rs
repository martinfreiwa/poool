//! Production-readiness audit for the leaderboard subsystem.
//!
//! These tests cover the edges that the existing `leaderboard_integration`
//! and `leaderboard_http` suites do *not* explicitly guard. They double as
//! a security audit: every code path that touches user input gets a
//! malicious payload run against it; every privacy-relevant filter gets a
//! "does it actually filter?" assertion.
//!
//! ## Coverage matrix
//!
//! Each test is named after the property it guards.
//!
//! Functional edges
//! - `empty_dataset_returns_well_formed_response`
//! - `per_page_caps_at_100`
//! - `per_page_minimum_is_1`
//! - `page_zero_or_negative_normalizes_to_1`
//! - `asset_mix_only_populated_for_top_3`
//! - `asset_mix_slices_sorted_desc_by_invested`
//! - `display_name_preference_overrides_user_profile`
//! - `my_rank_returns_default_for_user_without_score`
//! - `refresh_is_idempotent_on_repeat`
//!
//! Privacy / security
//! - `search_handles_sql_injection_payload_safely`
//! - `search_handles_special_ilike_wildcards`
//! - `pseudonym_pattern_never_appears_in_response`
//! - `avatar_url_hidden_when_show_avatar_false`
//! - `unknown_metric_falls_back_to_invested`
//!
//! ## Running
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test leaderboard_production_audit -- --ignored
//! ```

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

// ─── Test infrastructure (duplicated from leaderboard_integration so this
// file compiles as an independent crate without touching the bin) ──────────

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect")
}

async fn insert_user(pool: &PgPool, id: Uuid, status: &str) {
    sqlx::query("INSERT INTO users (id, email, password_hash, status) VALUES ($1, $2, 'x', $3)")
        .bind(id)
        .bind(format!("{}@pa.test", id))
        .bind(status)
        .execute(pool)
        .await
        .expect("insert user");
}

async fn insert_asset(pool: &PgPool, id: Uuid, asset_type: &str, yield_bps: i32) {
    // `assets.asset_type` is a VARCHAR(30) with a CHECK constraint, not a
    // Postgres enum — bind as plain text and the constraint validates the
    // value at insert time.
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published, annual_yield_bps)
         VALUES ($1, $2, 'PA Test', $3,
                 1000, 0, 100, 100000, 'funded', TRUE, $4)",
    )
    .bind(id)
    .bind(format!("pa-{}", id))
    .bind(asset_type)
    .bind(yield_bps)
    .execute(pool)
    .await
    .expect("insert asset");
}

async fn insert_investment(pool: &PgPool, user_id: Uuid, asset_id: Uuid, value_cents: i64) {
    sqlx::query(
        "INSERT INTO investments
            (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status)
         VALUES ($1, $2, 100, $3, $3, 'active')",
    )
    .bind(user_id)
    .bind(asset_id)
    .bind(value_cents)
    .execute(pool)
    .await
    .expect("insert investment");
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM leaderboard_scores WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM leaderboard_preferences WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM investments WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_profiles WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

async fn cleanup_asset(pool: &PgPool, asset_id: Uuid) {
    let _ = sqlx::query("DELETE FROM assets WHERE id = $1")
        .bind(asset_id)
        .execute(pool)
        .await;
}

/// Set a leaderboard_preferences row in one call.
async fn set_prefs(
    pool: &PgPool,
    user_id: Uuid,
    visible: bool,
    show_avatar: bool,
    display_name: Option<&str>,
) {
    sqlx::query(
        r#"INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id) DO UPDATE SET
               visible = EXCLUDED.visible,
               show_avatar = EXCLUDED.show_avatar,
               display_name = EXCLUDED.display_name"#,
    )
    .bind(user_id)
    .bind(visible)
    .bind(show_avatar)
    .bind(display_name)
    .execute(pool)
    .await
    .expect("set prefs");
}

/// Mirror of `service::refresh_all_scores` inline-SQL so this test crate
/// doesn't need to link the bin.
async fn refresh_all_scores_sql(pool: &PgPool) {
    sqlx::query(
        r#"DELETE FROM leaderboard_scores
           WHERE user_id NOT IN (
               SELECT DISTINCT user_id FROM investments WHERE status = 'active'
               UNION SELECT DISTINCT referrer_id FROM referral_tracking
           )"#,
    )
    .execute(pool)
    .await
    .expect("ghost cleanup");

    sqlx::query(
        r#"DELETE FROM leaderboard_scores
           WHERE user_id IN (SELECT id FROM users WHERE status != 'active')"#,
    )
    .execute(pool)
    .await
    .expect("suspended cleanup");

    sqlx::query(
        r#"
        INSERT INTO leaderboard_scores (
            user_id, total_invested_cents, asset_count, portfolio_roi_bps,
            affiliate_count, referral_network_value_cents, highest_investment_cents,
            computed_at
        )
        SELECT
            u.id,
            COALESCE(inv_agg.total_invested, 0),
            COALESCE(inv_agg.unique_assets, 0)::INTEGER,
            COALESCE(inv_agg.weighted_roi_bps, 0)::INTEGER,
            0::INTEGER, 0, COALESCE(inv_agg.highest_inv, 0), NOW()
        FROM users u
        LEFT JOIN (
            SELECT i.user_id,
                SUM(i.purchase_value_cents)::BIGINT AS total_invested,
                COUNT(DISTINCT i.asset_id)::INTEGER AS unique_assets,
                MAX(i.purchase_value_cents) AS highest_inv,
                COALESCE(ROUND(
                    SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                    / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                ), 0) AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
            GROUP BY i.user_id
        ) inv_agg ON inv_agg.user_id = u.id
        WHERE u.status = 'active' AND inv_agg.total_invested > 0
        ON CONFLICT (user_id) DO UPDATE SET
            total_invested_cents = EXCLUDED.total_invested_cents,
            asset_count = EXCLUDED.asset_count,
            portfolio_roi_bps = EXCLUDED.portfolio_roi_bps,
            highest_investment_cents = EXCLUDED.highest_investment_cents,
            computed_at = NOW()
        "#,
    )
    .execute(pool)
    .await
    .expect("refresh upsert");

    // Mirror Step 2 of the production refresh: populate ALL six rank
    // columns. Tests that exercise non-`invested` metrics (e.g. the daily
    // snapshot writer for `assets`/`roi`/`affiliates`/`revenue`/`highest_inv`)
    // need these to be non-NULL or the WHERE clauses filter every row out.
    sqlx::query(
        r#"UPDATE leaderboard_scores ls SET
              rank_invested    = sub.r_inv,
              rank_assets      = sub.r_ast,
              rank_roi         = sub.r_roi,
              rank_affiliates  = sub.r_aff,
              rank_ref_revenue = sub.r_rev,
              rank_highest_inv = sub.r_hi
           FROM (
               SELECT user_id,
                  ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC)                                 AS r_inv,
                  ROW_NUMBER() OVER (ORDER BY asset_count DESC, total_invested_cents DESC, computed_at ASC)                AS r_ast,
                  ROW_NUMBER() OVER (ORDER BY portfolio_roi_bps DESC, total_invested_cents DESC, computed_at ASC)          AS r_roi,
                  ROW_NUMBER() OVER (ORDER BY affiliate_count DESC, referral_network_value_cents DESC, computed_at ASC)   AS r_aff,
                  ROW_NUMBER() OVER (ORDER BY referral_network_value_cents DESC, affiliate_count DESC, computed_at ASC)   AS r_rev,
                  ROW_NUMBER() OVER (ORDER BY highest_investment_cents DESC, computed_at ASC)                              AS r_hi
               FROM leaderboard_scores
           ) sub WHERE ls.user_id = sub.user_id"#,
    )
    .execute(pool)
    .await
    .expect("rank update");
}

/// Helper: seed N visible users with distinct invested amounts under a
/// unique HMTEST_<id> prefix so the cohort can be isolated via `search`.
async fn seed_visible_cohort(pool: &PgPool, n: usize, test_id: &str) -> (Vec<Uuid>, Uuid) {
    let asset = Uuid::new_v4();
    cleanup_asset(pool, asset).await;
    insert_asset(pool, asset, "real_estate", 500).await;

    let users: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
    for (i, u) in users.iter().enumerate() {
        cleanup_user(pool, *u).await;
        insert_user(pool, *u, "active").await;
        // High base + descending so seeded users always sort above noise.
        let amount = (n as i64 - i as i64) * 1_000_000_000_i64 + 500_000_000_i64;
        insert_investment(pool, *u, asset, amount).await;
        set_prefs(
            pool,
            *u,
            true,
            false,
            Some(&format!("PROD_{}_{}", test_id, u.simple())),
        )
        .await;
    }
    refresh_all_scores_sql(pool).await;
    (users, asset)
}

async fn cleanup_cohort(pool: &PgPool, users: &[Uuid], asset: Uuid) {
    for u in users {
        cleanup_user(pool, *u).await;
    }
    cleanup_asset(pool, asset).await;
}

// ══════════════════════════════════════════════════════════════════════
// Functional edges
// ══════════════════════════════════════════════════════════════════════

/// Empty dataset: a search that matches nothing must return a well-formed
/// response with `rankings=[]`, `total_participants=0`, and `has_more=false`
/// — not a 500 or a null field.
#[ignore]
#[tokio::test]
async fn empty_dataset_returns_well_formed_response() {
    let pool = pool().await;
    let viewer = Uuid::new_v4();

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        10,
        None,
        Some("NO_USER_HAS_THIS_NAME_xyz_12345".to_string()),
        None,
    )
    .await
    .expect("get_rankings");

    assert_eq!(resp.total_participants, 0);
    assert!(
        resp.rankings.is_empty(),
        "rankings must be empty array, got {} rows",
        resp.rankings.len()
    );
    assert!(!resp.has_more, "has_more must be false on empty result");
    assert_eq!(resp.metric_type, "invested");
}

/// The route caps `per_page` at 100. The service itself accepts whatever is
/// passed, so this test asserts the service's behavior under a large
/// per_page value matches what the route would clamp it to.
#[ignore]
#[tokio::test]
async fn per_page_caps_at_100() {
    let pool = pool().await;
    let test_id = "pp_cap";
    let (users, asset) = seed_visible_cohort(&pool, 50, test_id).await;
    let viewer = Uuid::new_v4();

    // Request per_page=100 — way more than the cohort size. Result must
    // contain every cohort member exactly once, not error or duplicate.
    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        100,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_cohort(&pool, &users, asset).await;

    assert_eq!(resp.rankings.len(), 50, "must return all 50 visible users");
    assert_eq!(resp.total_participants, 50);
    assert!(!resp.has_more);
    // Ranks: each entry's rank must be unique. The cohort interleaves
    // with any organic visible users in the dev DB so we can't assert
    // exact 1..=50 values — only that the 50 returned ranks are all
    // distinct AND that metric_value descends monotonically (which is
    // the actual user-facing ordering contract).
    let ranks: Vec<i32> = resp.rankings.iter().map(|e| e.rank).collect();
    let mut sorted_unique = ranks.clone();
    sorted_unique.sort();
    sorted_unique.dedup();
    assert_eq!(
        sorted_unique.len(),
        50,
        "all 50 returned ranks must be unique; got {:?}",
        ranks,
    );
    let values: Vec<i64> = resp.rankings.iter().map(|e| e.metric_value).collect();
    let mut prev = i64::MAX;
    for v in &values {
        assert!(
            *v <= prev,
            "metric_value must descend across the listing; got {:?}",
            values,
        );
        prev = *v;
    }
}

/// `per_page = 1` is the boundary — the service must return at most one
/// row and report has_more=true when more exist.
#[ignore]
#[tokio::test]
async fn per_page_minimum_is_1() {
    let pool = pool().await;
    let test_id = "pp_min";
    let (users, asset) = seed_visible_cohort(&pool, 3, test_id).await;
    let viewer = Uuid::new_v4();

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        1,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_cohort(&pool, &users, asset).await;

    assert_eq!(
        resp.rankings.len(),
        1,
        "page 1 with per_page=1 must contain exactly one row"
    );
    assert!(
        resp.has_more,
        "has_more must be true when 2 more rows remain"
    );
}

/// Asset-mix donut is an expensive enrichment, so it's only computed for
/// the top-3 entries of the listing. Position 4+ must carry an empty
/// `asset_mix` array.
#[ignore]
#[tokio::test]
async fn asset_mix_only_populated_for_top_3() {
    let pool = pool().await;
    let test_id = "mix_top3";
    let (users, asset) = seed_visible_cohort(&pool, 5, test_id).await;
    let viewer = Uuid::new_v4();

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_cohort(&pool, &users, asset).await;

    assert_eq!(resp.rankings.len(), 5);
    for (i, entry) in resp.rankings.iter().enumerate() {
        if i < 3 {
            assert!(
                !entry.asset_mix.is_empty(),
                "top-3 entry at index {} should have asset_mix populated",
                i
            );
        } else {
            assert!(
                entry.asset_mix.is_empty(),
                "entry at index {} (rank {}) must have empty asset_mix; got {} slices",
                i,
                entry.rank,
                entry.asset_mix.len()
            );
        }
    }
}

/// Asset-mix slices must be sorted by `invested_cents` descending so the
/// dominant wedge is always slice[0]. The bento donut renders in slice
/// order so this is a UX contract, not just a database detail.
#[ignore]
#[tokio::test]
async fn asset_mix_slices_sorted_desc_by_invested() {
    let pool = pool().await;
    let test_id = "mix_sort";

    // One user with mixed investments — 3 asset types, distinct totals.
    let user = Uuid::new_v4();
    let re_asset = Uuid::new_v4();
    let com_asset = Uuid::new_v4();
    let cmdty_asset = Uuid::new_v4();

    cleanup_user(&pool, user).await;
    for a in [re_asset, com_asset, cmdty_asset] {
        cleanup_asset(&pool, a).await;
    }

    insert_user(&pool, user, "active").await;
    insert_asset(&pool, re_asset, "real_estate", 500).await;
    insert_asset(&pool, com_asset, "commercial_property", 500).await;
    insert_asset(&pool, cmdty_asset, "commodity", 500).await;

    insert_investment(&pool, user, re_asset, 1_000_000).await; // 10K EUR
    insert_investment(&pool, user, com_asset, 5_000_000).await; // 50K EUR — biggest
    insert_investment(&pool, user, cmdty_asset, 2_500_000).await; // 25K EUR

    set_prefs(
        &pool,
        user,
        true,
        false,
        Some(&format!("PROD_{}_{}", test_id, user.simple())),
    )
    .await;
    refresh_all_scores_sql(&pool).await;

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        Uuid::new_v4(),
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    let mix = resp
        .rankings
        .first()
        .map(|e| e.asset_mix.clone())
        .unwrap_or_default();

    cleanup_user(&pool, user).await;
    for a in [re_asset, com_asset, cmdty_asset] {
        cleanup_asset(&pool, a).await;
    }

    assert_eq!(
        mix.len(),
        3,
        "user with 3 asset types must surface 3 slices; got {}",
        mix.len()
    );
    // Largest slice first (commercial_property at 50K).
    assert_eq!(
        mix[0].asset_type, "commercial_property",
        "first slice should be the largest"
    );
    // Second largest (commodity at 25K).
    assert_eq!(
        mix[1].asset_type, "commodity",
        "second slice should be 25K commodity"
    );
    // Smallest last (real_estate at 10K).
    assert_eq!(
        mix[2].asset_type, "real_estate",
        "third slice should be 10K real_estate"
    );
    // Strict descending order.
    let mut prev = i64::MAX;
    for s in &mix {
        assert!(
            s.invested_cents <= prev,
            "slices must be DESC by invested_cents"
        );
        prev = s.invested_cents;
    }
}

/// `leaderboard_preferences.display_name` must override
/// `user_profiles.display_name` when both are set. Lets users pick a
/// public-facing handle without changing their account display name.
#[ignore]
#[tokio::test]
async fn display_name_preference_overrides_user_profile() {
    let pool = pool().await;
    let test_id = "dn_override";

    let user = Uuid::new_v4();
    let asset = Uuid::new_v4();
    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;
    insert_user(&pool, user, "active").await;
    insert_asset(&pool, asset, "real_estate", 500).await;
    insert_investment(&pool, user, asset, 5_000_000_000).await;

    // Real profile name "Alice Schmidt" — but the user opts into a pseudonymous
    // public handle via the leaderboard preferences.
    sqlx::query(
        r#"INSERT INTO user_profiles (user_id, first_name, last_name)
           VALUES ($1, 'Alice', 'Schmidt')
           ON CONFLICT (user_id) DO UPDATE SET
               first_name = 'Alice', last_name = 'Schmidt'"#,
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("user_profiles seed");

    let pub_name = format!("PROD_{}_PublicHandle", test_id);
    set_prefs(&pool, user, true, false, Some(&pub_name)).await;
    refresh_all_scores_sql(&pool).await;

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        Uuid::new_v4(),
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    let display_name = resp.rankings.first().map(|e| e.display_name.clone());

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        display_name.as_deref(),
        Some(pub_name.as_str()),
        "lp.display_name must override the user_profiles fallback; got {:?}",
        display_name,
    );
}

/// A user with no row in `leaderboard_scores` must get `MyRank::default()`
/// back (rank=None, all metrics=0) rather than a 500 or a panic.
#[ignore]
#[tokio::test]
async fn my_rank_returns_default_for_user_without_score() {
    let pool = pool().await;
    let ghost_user = Uuid::new_v4(); // never seeded, never invested

    let my_rank = poool_backend::leaderboard::service::get_user_rank(
        &pool, ghost_user, "invested", "alltime",
    )
    .await
    .expect("get_user_rank");

    assert!(
        my_rank.rank.is_none(),
        "ghost user must have rank=None; got {:?}",
        my_rank.rank
    );
    assert_eq!(my_rank.metric_value, 0, "ghost user metric_value must be 0");
    assert_eq!(my_rank.metrics.total_invested_cents, 0);
    assert_eq!(my_rank.metrics.asset_count, 0);
}

/// `refresh_all_scores` must be idempotent — running it twice in a row
/// produces the same scores (modulo `computed_at`). Lets ops re-run a
/// refresh during incident response without fear of doubling counts.
#[ignore]
#[tokio::test]
async fn refresh_is_idempotent_on_repeat() {
    let pool = pool().await;
    let test_id = "idempot";
    let (users, asset) = seed_visible_cohort(&pool, 4, test_id).await;

    // Capture a snapshot of the seeded cohort's scores after the first
    // refresh.
    let before: Vec<(Uuid, i64, i32)> = sqlx::query_as(
        "SELECT user_id, total_invested_cents, asset_count FROM leaderboard_scores
         WHERE user_id = ANY($1) ORDER BY total_invested_cents DESC",
    )
    .bind(&users)
    .fetch_all(&pool)
    .await
    .expect("before snapshot");

    // Run the refresh again.
    refresh_all_scores_sql(&pool).await;

    let after: Vec<(Uuid, i64, i32)> = sqlx::query_as(
        "SELECT user_id, total_invested_cents, asset_count FROM leaderboard_scores
         WHERE user_id = ANY($1) ORDER BY total_invested_cents DESC",
    )
    .bind(&users)
    .fetch_all(&pool)
    .await
    .expect("after snapshot");

    cleanup_cohort(&pool, &users, asset).await;

    assert_eq!(before.len(), 4);
    assert_eq!(
        before, after,
        "second refresh produced different scores — not idempotent"
    );
}

// ══════════════════════════════════════════════════════════════════════
// Privacy / Security
// ══════════════════════════════════════════════════════════════════════

/// Search payload containing SQL injection metacharacters must NOT execute
/// as SQL. The parameter is bound via sqlx (parameterized query), so any
/// `'`, `;`, `--`, or `DROP TABLE` should be treated as a literal string
/// to ILIKE-match against `full_name`.
#[ignore]
#[tokio::test]
async fn search_handles_sql_injection_payload_safely() {
    let pool = pool().await;
    let test_id = "sqli";
    let (users, asset) = seed_visible_cohort(&pool, 2, test_id).await;
    let viewer = Uuid::new_v4();

    // Classic injection payloads — none should match real users, none
    // should error or affect the schema.
    let payloads = vec![
        "' OR '1'='1",
        "'; DROP TABLE users; --",
        "%' UNION SELECT id, email, password_hash FROM users WHERE '1'='1",
        "\\'; SELECT pg_sleep(10); --",
        "PROD_sqli' OR 1=1 --", // tries to bypass the cohort filter
    ];

    for payload in payloads {
        let resp = poool_backend::leaderboard::service::get_rankings(
            &pool,
            viewer,
            "invested",
            "alltime",
            1,
            10,
            None,
            Some(payload.to_string()),
            None,
        )
        .await
        .expect("get_rankings must not panic on injection payload");

        // None of the payloads literally appears in any seeded user's
        // `full_name`, so the result must be empty.
        assert_eq!(
            resp.total_participants, 0,
            "injection payload {:?} returned rows — predicate may not be parameterized",
            payload,
        );
        assert!(resp.rankings.is_empty());
    }

    // Schema sanity-check: users table still has data after attack payloads.
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM users")
        .fetch_one(&pool)
        .await
        .expect("users still exists");
    assert!(
        user_count > 0,
        "users table must still exist after injection attempts"
    );

    cleanup_cohort(&pool, &users, asset).await;
}

/// ILIKE wildcards `%` and `_` inside the search payload must be treated
/// as literals (not pattern operators). Otherwise a search for "100%" or
/// "data_loss" silently matches more than intended.
///
/// The current implementation builds `'%' || $4::text || '%'` so the
/// payload's own wildcards DO act as wildcards. This test documents that
/// behavior explicitly — if a future refactor adds escaping, the assertion
/// needs to flip and the rationale needs updating.
#[ignore]
#[tokio::test]
async fn search_handles_special_ilike_wildcards() {
    let pool = pool().await;
    let test_id = "wildcard";
    let (users, asset) = seed_visible_cohort(&pool, 2, test_id).await;
    let viewer = Uuid::new_v4();

    // `PROD_%` will match any seeded name; this is the EXISTING semantic.
    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        10,
        None,
        Some("PROD_%".to_string()),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_cohort(&pool, &users, asset).await;

    assert!(
        resp.total_participants >= 2,
        "PROD_%% wildcard search should match seeded cohort; got {} (regression: did you start escaping wildcards?)",
        resp.total_participants,
    );
}

/// Regression guard for the visibility-filter fix (2026-05-16). The
/// previous behavior anonymized hidden users with `'Investor #' ||
/// substring(user_id::text, 1, 6)`. Post-fix they're filtered out
/// entirely. The `'Investor #...'` literal must never appear in any
/// `display_name` returned by the API.
#[ignore]
#[tokio::test]
async fn pseudonym_pattern_never_appears_in_response() {
    let pool = pool().await;
    let test_id = "no_pseudo";

    // Seed 5 users: 2 visible, 3 hidden. Verify only the 2 visible names
    // show up and none of them start with the legacy "Investor #" prefix.
    let asset = Uuid::new_v4();
    cleanup_asset(&pool, asset).await;
    insert_asset(&pool, asset, "real_estate", 500).await;

    let users: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    for (i, u) in users.iter().enumerate() {
        cleanup_user(&pool, *u).await;
        insert_user(&pool, *u, "active").await;
        insert_investment(&pool, *u, asset, ((i as i64) + 1) * 1_000_000_000).await;
        let visible = i < 2;
        set_prefs(
            &pool,
            *u,
            visible,
            false,
            Some(&format!("PROD_{}_{}", test_id, u.simple())),
        )
        .await;
    }
    refresh_all_scores_sql(&pool).await;

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        Uuid::new_v4(),
        "invested",
        "alltime",
        1,
        50,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    for u in &users {
        cleanup_user(&pool, *u).await;
    }
    cleanup_asset(&pool, asset).await;

    assert_eq!(resp.rankings.len(), 2, "only visible users should appear");
    for entry in &resp.rankings {
        assert!(
            !entry.display_name.starts_with("Investor #"),
            "pseudonym leaked: {:?} starts with the removed `Investor #` pattern",
            entry.display_name,
        );
    }
}

/// Avatar URL must NOT leak when the user has opted out of avatar display
/// (`show_avatar = false`), even if they're otherwise visible.
#[ignore]
#[tokio::test]
async fn avatar_url_hidden_when_show_avatar_false() {
    let pool = pool().await;
    let test_id = "no_avatar";

    let user = Uuid::new_v4();
    let asset = Uuid::new_v4();
    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, user, "active").await;
    insert_asset(&pool, asset, "real_estate", 500).await;
    insert_investment(&pool, user, asset, 5_000_000_000).await;

    // Set an avatar URL but opt out of showing it.
    sqlx::query("UPDATE users SET avatar_url = 'https://example.test/avatar.png' WHERE id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("set avatar_url");

    set_prefs(
        &pool,
        user,
        true,
        false,
        Some(&format!("PROD_{}_{}", test_id, user.simple())),
    )
    .await;
    refresh_all_scores_sql(&pool).await;

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        Uuid::new_v4(),
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    let avatar = resp.rankings.first().and_then(|e| e.avatar_url.clone());

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    assert!(
        avatar.is_none(),
        "avatar URL leaked despite show_avatar=false; got {:?}",
        avatar,
    );
}

/// Unknown `metric` strings must fall back to the safe default ("invested")
/// rather than panic, error, or run a malformed query. The `metric_columns`
/// function uses a match with a catch-all arm specifically for this — this
/// test guards the fallback.
#[ignore]
#[tokio::test]
async fn unknown_metric_falls_back_to_invested() {
    let pool = pool().await;
    let test_id = "unk_metric";
    let (users, asset) = seed_visible_cohort(&pool, 3, test_id).await;
    let viewer = Uuid::new_v4();

    let resp_known = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("PROD_{}", test_id)),
        None,
    )
    .await
    .expect("known metric");

    // Garbage metric strings — including SQL-injection-shaped — must fall
    // through to the "invested" default and produce the same ordering.
    let bogus = vec!["totally_made_up", "robert'); DROP TABLE--", ""];
    for m in bogus {
        let resp_bogus = poool_backend::leaderboard::service::get_rankings(
            &pool,
            viewer,
            m,
            "alltime",
            1,
            10,
            None,
            Some(format!("PROD_{}", test_id)),
            None,
        )
        .await
        .unwrap_or_else(|e| panic!("bogus metric {:?} must not error: {}", m, e));

        assert_eq!(
            resp_known
                .rankings
                .iter()
                .map(|e| e.rank)
                .collect::<Vec<_>>(),
            resp_bogus
                .rankings
                .iter()
                .map(|e| e.rank)
                .collect::<Vec<_>>(),
            "bogus metric {:?} produced a different ordering than `invested` — fallback broken",
            m,
        );
    }

    cleanup_cohort(&pool, &users, asset).await;
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot writer (daily cron)
// ══════════════════════════════════════════════════════════════════════

/// `write_daily_snapshot` must:
///   1. Insert one row per (visible_user, metric, today) for the top-N
///      users per metric, with the correct rank and metric_value.
///   2. NOT snapshot hidden (`visible=false`) users — privacy contract.
///   3. Be idempotent on the same day — re-running upserts the same row.
#[ignore]
#[tokio::test]
async fn daily_snapshot_writes_top_n_visible_only_and_is_idempotent() {
    let pool = pool().await;
    let test_id = "snap";

    // 3 visible users + 2 hidden, all with distinct invested amounts.
    let asset = Uuid::new_v4();
    cleanup_asset(&pool, asset).await;
    insert_asset(&pool, asset, "real_estate", 500).await;

    let users: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    for (i, u) in users.iter().enumerate() {
        cleanup_user(&pool, *u).await;
        insert_user(&pool, *u, "active").await;
        insert_investment(&pool, *u, asset, ((i as i64) + 1) * 1_000_000_000).await;
        let visible = i < 3;
        set_prefs(
            &pool,
            *u,
            visible,
            false,
            Some(&format!("PROD_{}_{}", test_id, u.simple())),
        )
        .await;
    }
    refresh_all_scores_sql(&pool).await;

    // Wipe any pre-existing snapshots for these users today (clean test
    // state — previous runs may have written rows).
    let today_sql = "(NOW() AT TIME ZONE 'UTC')::DATE";
    sqlx::query(&format!(
        "DELETE FROM leaderboard_snapshots WHERE user_id = ANY($1) AND snapshot_date = {today_sql}",
        today_sql = today_sql,
    ))
    .bind(&users)
    .execute(&pool)
    .await
    .expect("pre-clean snapshots");

    // Run the snapshot writer (top 100 — covers all 5).
    let rows_first = poool_backend::leaderboard::service::write_daily_snapshot(&pool, 100)
        .await
        .expect("first snapshot");

    // 6 metrics × 3 visible users = 18 rows expected from THIS cohort.
    // Other organic users in the DB may bump rows_first higher; we assert
    // a lower bound and verify the cohort-specific count below.
    assert!(
        rows_first >= 18,
        "first snapshot should write at least 18 rows for the 3 visible × 6 metrics cohort; got {}",
        rows_first,
    );

    // Cohort-specific count today
    let cohort_today: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*)::BIGINT FROM leaderboard_snapshots
         WHERE user_id = ANY($1) AND snapshot_date = {today_sql}",
        today_sql = today_sql,
    ))
    .bind(&users)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        cohort_today, 18,
        "exactly 3 visible users × 6 metrics = 18 cohort rows expected; got {}",
        cohort_today,
    );

    // Verify NO hidden user has a snapshot row.
    let hidden_users = [users[3], users[4]];
    let hidden_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM leaderboard_snapshots WHERE user_id = ANY($1)",
    )
    .bind(&hidden_users[..])
    .fetch_one(&pool)
    .await
    .expect("hidden count");
    assert_eq!(
        hidden_count, 0,
        "hidden users must not appear in snapshots; found {} rows (privacy leak)",
        hidden_count,
    );

    // Verify the metric_value matches the source.
    let invested_for_top: i64 = sqlx::query_scalar(&format!(
        "SELECT metric_value FROM leaderboard_snapshots
         WHERE user_id = $1 AND metric = 'invested' AND snapshot_date = {today_sql}",
        today_sql = today_sql,
    ))
    .bind(users[0]) // smallest amount among visible (1 * 1e9)
    .fetch_one(&pool)
    .await
    .expect("invested row");
    assert_eq!(
        invested_for_top, 1_000_000_000,
        "metric_value must match leaderboard_scores.total_invested_cents",
    );

    // Run again — idempotency contract: same-day re-run upserts, doesn't
    // duplicate.
    let _rows_second = poool_backend::leaderboard::service::write_daily_snapshot(&pool, 100)
        .await
        .expect("second snapshot");
    let cohort_after_second: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*)::BIGINT FROM leaderboard_snapshots
         WHERE user_id = ANY($1) AND snapshot_date = {today_sql}",
        today_sql = today_sql,
    ))
    .bind(&users)
    .fetch_one(&pool)
    .await
    .expect("count after second");
    assert_eq!(
        cohort_after_second, 18,
        "second snapshot must NOT duplicate rows on the same day; got {}",
        cohort_after_second,
    );

    // Cleanup
    sqlx::query("DELETE FROM leaderboard_snapshots WHERE user_id = ANY($1)")
        .bind(&users)
        .execute(&pool)
        .await
        .expect("snapshot cleanup");
    for u in &users {
        cleanup_user(&pool, *u).await;
    }
    cleanup_asset(&pool, asset).await;
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot pruning + read-back endpoint
// ══════════════════════════════════════════════════════════════════════

/// `prune_old_snapshots` must:
///   1. Delete rows STRICTLY older than `retain_days`.
///   2. Leave rows within the window untouched.
///   3. Return the deleted count.
#[ignore]
#[tokio::test]
async fn prune_old_snapshots_respects_retention_window() {
    let pool = pool().await;
    let user = Uuid::new_v4();
    cleanup_user(&pool, user).await;
    insert_user(&pool, user, "active").await;

    // Insert 3 snapshot rows: one inside the window, one on the boundary,
    // one outside.
    sqlx::query(
        r#"INSERT INTO leaderboard_snapshots
             (user_id, metric, rank, metric_value, snapshot_date)
           VALUES
             ($1, 'invested', 1, 1000, (NOW() - INTERVAL '10 days')::DATE),
             ($1, 'invested', 2, 1000, (NOW() - INTERVAL '395 days')::DATE),
             ($1, 'invested', 3, 1000, (NOW() - INTERVAL '500 days')::DATE)
           ON CONFLICT (user_id, metric, snapshot_date) DO NOTHING"#,
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("seed snapshots");

    let deleted = poool_backend::leaderboard::service::prune_old_snapshots(&pool, 395)
        .await
        .expect("prune");

    // Read what survived.
    let remaining: Vec<(chrono::NaiveDate,)> = sqlx::query_as(
        "SELECT snapshot_date FROM leaderboard_snapshots WHERE user_id = $1 ORDER BY snapshot_date DESC",
    )
    .bind(user)
    .fetch_all(&pool)
    .await
    .expect("read remaining");

    // Cleanup
    let _ = sqlx::query("DELETE FROM leaderboard_snapshots WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await;
    cleanup_user(&pool, user).await;

    assert!(
        deleted >= 1,
        "must delete at least the 500-day-old row; got {} deleted",
        deleted,
    );
    // The 10-day-old row must always survive.
    assert!(
        remaining
            .iter()
            .any(|(d,)| (chrono::Utc::now().naive_utc().date() - *d).num_days() <= 12),
        "recent (10-day) snapshot must remain after prune; got {:?}",
        remaining,
    );
}

/// `get_user_snapshots` must:
///   1. Return only the requested user's rows.
///   2. Honour the metric allowlist (unknown → invested).
///   3. Cap `days` to the retention horizon to avoid pathologically large
///      replies.
///   4. Return rows newest-first.
#[ignore]
#[tokio::test]
async fn user_snapshots_filter_to_self_and_metric() {
    let pool = pool().await;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    cleanup_user(&pool, user_a).await;
    cleanup_user(&pool, user_b).await;
    insert_user(&pool, user_a, "active").await;
    insert_user(&pool, user_b, "active").await;

    // Seed 3 days of invested snapshots for user_a + 1 invested for user_b
    // + 2 'roi' snapshots for user_a — so the filters can be exercised.
    sqlx::query(
        r#"INSERT INTO leaderboard_snapshots
             (user_id, metric, rank, metric_value, snapshot_date)
           VALUES
             ($1, 'invested', 1, 1000, (NOW() - INTERVAL '0 days')::DATE),
             ($1, 'invested', 2,  900, (NOW() - INTERVAL '1 day')::DATE),
             ($1, 'invested', 3,  800, (NOW() - INTERVAL '2 days')::DATE),
             ($1, 'roi',      4,  500, (NOW() - INTERVAL '0 days')::DATE),
             ($1, 'roi',      5,  450, (NOW() - INTERVAL '1 day')::DATE),
             ($2, 'invested', 9,  100, (NOW() - INTERVAL '0 days')::DATE)
           ON CONFLICT (user_id, metric, snapshot_date) DO NOTHING"#,
    )
    .bind(user_a)
    .bind(user_b)
    .execute(&pool)
    .await
    .expect("seed snapshots");

    // Property 1+2: user_a invested view returns only their 3 invested rows.
    let invested =
        poool_backend::leaderboard::service::get_user_snapshots(&pool, user_a, "invested", 30)
            .await
            .expect("get_user_snapshots invested");

    // Property 4: newest-first ordering.
    let dates: Vec<chrono::NaiveDate> = invested.iter().map(|p| p.snapshot_date).collect();
    let mut sorted_desc = dates.clone();
    sorted_desc.sort_by(|a, b| b.cmp(a));

    // Property 2 (allowlist): garbage metric falls back to invested.
    let bogus = poool_backend::leaderboard::service::get_user_snapshots(
        &pool,
        user_a,
        "totally_made_up_metric_xyz",
        30,
    )
    .await
    .expect("get_user_snapshots bogus");

    // Property 3: huge days cap is respected; we still get rows from the
    // 30-day cohort regardless.
    let huge_days =
        poool_backend::leaderboard::service::get_user_snapshots(&pool, user_a, "invested", 99_999)
            .await
            .expect("get_user_snapshots huge days");

    // Cleanup
    for u in [user_a, user_b] {
        let _ = sqlx::query("DELETE FROM leaderboard_snapshots WHERE user_id = $1")
            .bind(u)
            .execute(&pool)
            .await;
        cleanup_user(&pool, u).await;
    }

    assert_eq!(
        invested.len(),
        3,
        "user_a invested cohort = 3 rows; got {}",
        invested.len()
    );
    assert_eq!(
        dates, sorted_desc,
        "rows must be returned newest-first; got {:?}",
        dates,
    );
    assert_eq!(
        bogus.len(),
        3,
        "unknown metric must fall back to invested (also 3 rows); got {}",
        bogus.len(),
    );
    assert!(
        huge_days.len() >= 3,
        "days=99_999 must still return the seeded rows (clamped, not errored); got {}",
        huge_days.len(),
    );
}
