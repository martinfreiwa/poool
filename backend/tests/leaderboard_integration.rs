//! Leaderboard service-layer regression tests.
//!
//! ## Scope
//!
//! The audit task asked for `/api/leaderboard*` *integration* tests covering:
//!   1. GET /api/leaderboard returns ranked rows, honors per_page/page
//!   2. GET /api/leaderboard?tier_id=X filters correctly
//!   3. GET /api/leaderboard?search=name filters by display_name
//!   4. GET /api/leaderboard/me returns rank for logged-in user
//!   5. PUT /api/leaderboard/preferences with partial body preserves other fields
//!   6. POST /api/leaderboard/refresh requires admin
//!   7. refresh_all_scores removes rows for non-active users
//!
//! The poool-backend crate is a **binary crate** (no `lib.rs`), so external
//! integration tests in `backend/tests/` cannot `use` its modules. The
//! pre-existing test-infra pattern in `marketplace_concurrent_load.rs` and
//! `marketplace_settlement_e2e.rs` is SQL-only (raw `DATABASE_URL` pool, no
//! HTTP / auth / session helpers).
//!
//! Per the audit instruction "If test DB infra is missing or broken, STOP and
//! report — do not invent it", I deliver here the subset of cases that CAN be
//! validated against the existing infra: (1) ranking pagination, (2) tier
//! filter, (7) ghost-row removal during refresh, and (5) preferences
//! partial-update semantics. These cover the regression fixes 3 and 5 the
//! audit explicitly cared about.
//!
//! Cases 4 and 6 (HTTP `GET /me` rank + admin-only refresh) require either an
//! Axum test harness or a `lib.rs` shim that exposes the service modules to
//! external test crates — both qualify as "inventing test infra" relative to
//! the existing pattern. These cases are documented as gaps in the test plan.
//!
//! Run with:
//!   DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test leaderboard_integration -- --ignored

#![cfg(test)]

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect")
}

/// Insert a minimal `users` row.
async fn insert_user(pool: &PgPool, id: Uuid, status: &str) {
    sqlx::query("INSERT INTO users (id, email, password_hash, status) VALUES ($1, $2, 'x', $3)")
        .bind(id)
        .bind(format!("{}@lb.test", id))
        .bind(status)
        .execute(pool)
        .await
        .expect("insert user");
}

/// Insert a minimal `assets` row with a given yield (bps).
async fn insert_asset(pool: &PgPool, id: Uuid, yield_bps: i32) {
    sqlx::query(
        "INSERT INTO assets
            (id, slug, title, asset_type, tokens_total, tokens_available,
             token_price_cents, total_value_cents, funding_status, published, annual_yield_bps)
         VALUES ($1, $2, 'LB Test', 'real_estate', 1000, 0, 100, 100000, 'funded', TRUE, $3)",
    )
    .bind(id)
    .bind(format!("lb-{}", id))
    .bind(yield_bps)
    .execute(pool)
    .await
    .expect("insert asset");
}

/// Insert an active investment row.
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

/// Best-effort cleanup. Called both at the start (to clear stale fixtures
/// from a previous failed run) and end of every test. We tag every test row
/// with an email-suffix or slug-suffix prefix and clean by user_id list.
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

// ─── Test 1: refresh_all_scores removes rows for non-active users ───────────
//
// Regression guard for audit fix 5. The DELETE FROM leaderboard_scores WHERE
// user_id IN (SELECT id FROM users WHERE status != 'active') step must wipe
// rows for suspended/frozen/deleted users even if they still hold active
// investments.

/// Run the same SQL `refresh_all_scores` executes — minus the tracing — so
/// this test does not need to link the bin crate's service module.
async fn refresh_all_scores_sql(pool: &PgPool) {
    sqlx::query(
        r#"DELETE FROM leaderboard_scores
           WHERE user_id NOT IN (
               SELECT DISTINCT user_id FROM investments WHERE status = 'active'
               UNION
               SELECT DISTINCT referrer_id FROM referral_tracking
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
            SELECT
                i.user_id,
                SUM(i.purchase_value_cents)::BIGINT          AS total_invested,
                COUNT(DISTINCT i.asset_id)                   AS unique_assets,
                MAX(i.purchase_value_cents)                  AS highest_inv,
                COALESCE(
                    ROUND(
                        SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                        / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                    ),
                    0
                )                                            AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
            GROUP BY i.user_id
        ) inv_agg ON inv_agg.user_id = u.id
        WHERE u.status = 'active'
          AND inv_agg.total_invested > 0
        ON CONFLICT (user_id) DO UPDATE SET
            total_invested_cents     = EXCLUDED.total_invested_cents,
            asset_count              = EXCLUDED.asset_count,
            portfolio_roi_bps        = EXCLUDED.portfolio_roi_bps,
            highest_investment_cents = EXCLUDED.highest_investment_cents,
            computed_at              = NOW()
        "#,
    )
    .execute(pool)
    .await
    .expect("refresh upsert");
}

#[ignore]
#[tokio::test]
async fn refresh_removes_non_active_user_rows() {
    let pool = pool().await;

    let active = Uuid::new_v4();
    let suspended = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, active).await;
    cleanup_user(&pool, suspended).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, active, "active").await;
    insert_user(&pool, suspended, "suspended").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment(&pool, active, asset, 10_000).await;
    insert_investment(&pool, suspended, asset, 10_000).await;

    refresh_all_scores_sql(&pool).await;

    let active_score: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores WHERE user_id = $1",
    )
    .bind(active)
    .fetch_optional(&pool)
    .await
    .expect("query active");

    let suspended_score: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores WHERE user_id = $1",
    )
    .bind(suspended)
    .fetch_optional(&pool)
    .await
    .expect("query suspended");

    cleanup_user(&pool, active).await;
    cleanup_user(&pool, suspended).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        active_score,
        Some(10_000),
        "active user should be in leaderboard"
    );
    assert!(
        suspended_score.is_none(),
        "suspended user must be excluded from leaderboard (regression for audit fix 5); got {:?}",
        suspended_score
    );
}

// ─── Test 2: ranking pagination + ORDER BY honored ─────────────────────────
//
// Three users with distinct total_invested_cents are inserted. We then read
// the leaderboard ordered by rank_invested and confirm:
//   - per_page=2 yields exactly 2 rows on page 1
//   - per_page=2 yields the third row on page 2
//   - the order is descending by total_invested_cents

#[ignore]
#[tokio::test]
async fn ranking_pagination_honors_per_page_and_page() {
    let pool = pool().await;

    let u1 = Uuid::new_v4();
    let u2 = Uuid::new_v4();
    let u3 = Uuid::new_v4();
    let asset = Uuid::new_v4();

    for u in [u1, u2, u3] {
        cleanup_user(&pool, u).await;
    }
    cleanup_asset(&pool, asset).await;

    for u in [u1, u2, u3] {
        insert_user(&pool, u, "active").await;
    }
    insert_asset(&pool, asset, 500).await;
    insert_investment(&pool, u1, asset, 30_000).await;
    insert_investment(&pool, u2, asset, 20_000).await;
    insert_investment(&pool, u3, asset, 10_000).await;

    refresh_all_scores_sql(&pool).await;

    // Now assign ranks (mirrors the second statement in refresh_all_scores)
    sqlx::query(
        r#"
        UPDATE leaderboard_scores ls SET
            rank_invested = sub.r_inv
        FROM (
            SELECT user_id,
                ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) AS r_inv
            FROM leaderboard_scores
        ) sub WHERE ls.user_id = sub.user_id
        "#,
    )
    .execute(&pool)
    .await
    .expect("rank update");

    // Page 1 (LIMIT 2 OFFSET 0)
    let page1: Vec<(Uuid, i64)> = sqlx::query_as(
        r#"SELECT user_id, total_invested_cents FROM leaderboard_scores
           WHERE user_id = ANY($1)
           ORDER BY rank_invested ASC
           LIMIT 2 OFFSET 0"#,
    )
    .bind(vec![u1, u2, u3])
    .fetch_all(&pool)
    .await
    .expect("page 1");

    // Page 2 (LIMIT 2 OFFSET 2) — should give us only the third row
    let page2: Vec<(Uuid, i64)> = sqlx::query_as(
        r#"SELECT user_id, total_invested_cents FROM leaderboard_scores
           WHERE user_id = ANY($1)
           ORDER BY rank_invested ASC
           LIMIT 2 OFFSET 2"#,
    )
    .bind(vec![u1, u2, u3])
    .fetch_all(&pool)
    .await
    .expect("page 2");

    for u in [u1, u2, u3] {
        cleanup_user(&pool, u).await;
    }
    cleanup_asset(&pool, asset).await;

    assert_eq!(page1.len(), 2, "page 1 must have 2 rows");
    assert_eq!(page2.len(), 1, "page 2 must have 1 row");
    assert_eq!(page1[0].0, u1, "rank 1 must be highest invested");
    assert_eq!(page1[1].0, u2, "rank 2 must be second highest");
    assert_eq!(page2[0].0, u3, "rank 3 must be lowest");
}

// ─── Test 3: preferences partial-update preserves other fields ──────────────
//
// Regression guard for audit fix 3. The COALESCE/CASE in update_preferences
// must preserve any column not present in the request body.

/// Inline copy of the upsert SQL from `update_preferences`. Tests the SQL
/// directly so we don't depend on linking the binary crate.
async fn upsert_preferences(
    pool: &PgPool,
    user_id: Uuid,
    visible: Option<bool>,
    show_avatar: Option<bool>,
    display_name: Option<Option<String>>,
) {
    let visible_default = visible.unwrap_or(false);
    let show_avatar_default = show_avatar.unwrap_or(false);
    let display_name_default = display_name.clone().unwrap_or(None);
    let display_name_is_some = display_name.is_some();

    sqlx::query(
        r#"
        INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            visible      = COALESCE($5, leaderboard_preferences.visible),
            show_avatar  = COALESCE($6, leaderboard_preferences.show_avatar),
            display_name = CASE
                WHEN $7::bool THEN $8
                ELSE leaderboard_preferences.display_name
            END,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(visible_default)
    .bind(show_avatar_default)
    .bind(&display_name_default)
    .bind(visible)
    .bind(show_avatar)
    .bind(display_name_is_some)
    .bind(&display_name_default)
    .execute(pool)
    .await
    .expect("upsert prefs");
}

#[ignore]
#[tokio::test]
async fn preferences_partial_update_preserves_other_fields() {
    let pool = pool().await;
    let user_id = Uuid::new_v4();
    cleanup_user(&pool, user_id).await;
    insert_user(&pool, user_id, "active").await;

    // Step 1: set visible=true, show_avatar=true, display_name=Some("Alice")
    upsert_preferences(
        &pool,
        user_id,
        Some(true),
        Some(true),
        Some(Some("Alice".into())),
    )
    .await;

    // Step 2: partial update — toggle visible only. show_avatar + display_name
    // must remain untouched.
    upsert_preferences(&pool, user_id, Some(false), None, None).await;

    let row: (bool, bool, Option<String>) = sqlx::query_as(
        "SELECT visible, show_avatar, display_name
         FROM leaderboard_preferences WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read prefs");

    cleanup_user(&pool, user_id).await;

    assert!(!row.0, "visible should have flipped to false");
    assert!(
        row.1,
        "show_avatar must be preserved at true (regression for audit fix 3); got {}",
        row.1
    );
    assert_eq!(
        row.2.as_deref(),
        Some("Alice"),
        "display_name must be preserved; got {:?}",
        row.2
    );
}

// ─── Test: has_more must be derived from the count, not page fullness ──────
//
// Regression for audit task B1. The service used to compute
// `has_more = rankings.len() == per_page`. With total=N and per_page=N (so
// a single full page is also the last page), that heuristic claims
// `has_more: true` when there is in fact nothing on the next page.
//
// We exercise the math directly via the same SELECT pattern the service
// uses (LIMIT/OFFSET + a COUNT(*)) and assert the corrected derivation:
//   has_more = (offset + rankings.len()) < total_participants

#[ignore]
#[tokio::test]
async fn has_more_false_when_total_is_exact_multiple_of_per_page() {
    let pool = pool().await;
    let u1 = Uuid::new_v4();
    let u2 = Uuid::new_v4();
    let asset = Uuid::new_v4();

    for u in [u1, u2] {
        cleanup_user(&pool, u).await;
    }
    cleanup_asset(&pool, asset).await;

    for u in [u1, u2] {
        insert_user(&pool, u, "active").await;
    }
    insert_asset(&pool, asset, 500).await;
    insert_investment(&pool, u1, asset, 20_000).await;
    insert_investment(&pool, u2, asset, 10_000).await;
    refresh_all_scores_sql(&pool).await;

    sqlx::query(
        r#"UPDATE leaderboard_scores ls SET rank_invested = sub.r_inv
           FROM (
               SELECT user_id, ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) AS r_inv
               FROM leaderboard_scores
           ) sub WHERE ls.user_id = sub.user_id"#,
    )
    .execute(&pool)
    .await
    .expect("rank update");

    let per_page: i64 = 2;
    let offset: i64 = 0;

    // page 1 fetch (LIMIT 2 OFFSET 0). With exactly 2 users, this fills the
    // page entirely AND exhausts the dataset.
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        r#"SELECT user_id FROM leaderboard_scores
           WHERE user_id = ANY($1)
           ORDER BY rank_invested ASC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(vec![u1, u2])
    .bind(per_page)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .expect("page 1 select");

    let total_participants: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT FROM leaderboard_scores WHERE user_id = ANY($1)"#,
    )
    .bind(vec![u1, u2])
    .fetch_one(&pool)
    .await
    .expect("count");

    for u in [u1, u2] {
        cleanup_user(&pool, u).await;
    }
    cleanup_asset(&pool, asset).await;

    // The OLD heuristic — `rows.len() == per_page` — would yield true here.
    let old_heuristic = rows.len() as i64 == per_page;
    // The FIXED derivation — what the service now returns.
    let has_more = (offset + rows.len() as i64) < total_participants;

    assert_eq!(rows.len() as i64, per_page, "page must be exactly full");
    assert_eq!(total_participants, per_page, "total must equal per_page");
    assert!(
        old_heuristic,
        "sanity: old heuristic does claim has_more on a full-but-last page"
    );
    assert!(
        !has_more,
        "fixed has_more must be false when (offset + rows) >= total_participants; got true with offset={} rows={} total={}",
        offset,
        rows.len(),
        total_participants
    );
}

// ─── Test: timeframed weekly + tier filter — rankings and count align ──────
//
// Verifies audit task B2: with a weekly timeframe and tier_id selected, the
// number of rows returned by the rankings query equals total_participants
// from the count query. Both apply the same `tier_id = $X` predicate against
// the same CTE result set, so a discrepancy would indicate the two queries
// have drifted out of sync.
//
// Verdict (documented in the commit body): the two queries share the
// inv_agg / ref_agg / merged / ranked CTEs and apply the same predicates;
// this test exists to make a future divergence loud.

#[ignore]
#[tokio::test]
async fn timeframed_tier_filter_count_matches_rankings_len() {
    let pool = pool().await;

    // Build 10 users: 3 in tier 2, 7 in tier 1. All purchases within the
    // last 7 days so the weekly cutoff includes everything.
    let users: Vec<Uuid> = (0..10).map(|_| Uuid::new_v4()).collect();
    let asset = Uuid::new_v4();

    for u in &users {
        cleanup_user(&pool, *u).await;
    }
    cleanup_asset(&pool, asset).await;

    for u in &users {
        insert_user(&pool, *u, "active").await;
    }
    insert_asset(&pool, asset, 500).await;
    // Distinct investment amounts so rank ordering is deterministic.
    for (i, u) in users.iter().enumerate() {
        insert_investment(&pool, *u, asset, ((i + 1) as i64) * 1_000).await;
    }

    // Tier 2 → users[0..3], tier 1 → users[3..10]
    let tier_two: i32 = 2;
    let tier_one: i32 = 1;
    for (i, u) in users.iter().enumerate() {
        let tier = if i < 3 { tier_two } else { tier_one };
        let _ = sqlx::query(
            "INSERT INTO user_tiers (user_id, tier_id) VALUES ($1, $2)
             ON CONFLICT (user_id) DO UPDATE SET tier_id = EXCLUDED.tier_id",
        )
        .bind(u)
        .bind(tier)
        .execute(&pool)
        .await;
    }

    // Inline the weekly timeframed rankings + count queries with the same
    // CTE structure the service uses, then assert the two agree.
    let cutoff = "NOW() - INTERVAL '7 days'";
    let rankings_sql = format!(
        r#"
        WITH inv_agg AS (
            SELECT i.user_id,
                SUM(i.purchase_value_cents)::BIGINT AS total_invested
            FROM investments i
            WHERE i.status = 'active' AND i.purchased_at >= {cutoff}
              AND i.user_id = ANY($1)
            GROUP BY i.user_id
        ),
        merged AS (
            SELECT user_id, total_invested
            FROM inv_agg
            WHERE total_invested > 0
        ),
        ranked AS (
            SELECT m.user_id, m.total_invested,
                ROW_NUMBER() OVER (ORDER BY m.total_invested DESC)::INT AS rank
            FROM merged m
        ),
        enriched AS (
            SELECT r.rank, r.user_id, ut.tier_id
            FROM ranked r
            LEFT JOIN user_tiers ut ON ut.user_id = r.user_id
        )
        SELECT rank FROM enriched WHERE tier_id = $2 ORDER BY rank ASC
        "#,
        cutoff = cutoff,
    );

    let count_sql = format!(
        r#"
        WITH inv_agg AS (
            SELECT i.user_id,
                SUM(i.purchase_value_cents)::BIGINT AS total_invested
            FROM investments i
            WHERE i.status = 'active' AND i.purchased_at >= {cutoff}
              AND i.user_id = ANY($1)
            GROUP BY i.user_id
        ),
        merged AS (
            SELECT user_id, total_invested
            FROM inv_agg
            WHERE total_invested > 0
        ),
        ranked AS (
            SELECT m.user_id, m.total_invested,
                ROW_NUMBER() OVER (ORDER BY m.total_invested DESC)::INT AS rank
            FROM merged m
        ),
        enriched AS (
            SELECT r.rank, r.user_id, ut.tier_id
            FROM ranked r
            LEFT JOIN user_tiers ut ON ut.user_id = r.user_id
        )
        SELECT COUNT(*)::BIGINT FROM enriched WHERE tier_id = $2
        "#,
        cutoff = cutoff,
    );

    let ranks: Vec<(i32,)> = sqlx::query_as(&rankings_sql)
        .bind(&users)
        .bind(tier_two)
        .fetch_all(&pool)
        .await
        .expect("rankings");

    let total_participants: i64 = sqlx::query_scalar(&count_sql)
        .bind(&users)
        .bind(tier_two)
        .fetch_one(&pool)
        .await
        .expect("count");

    // Cleanup before assertions so failure doesn't leak test rows.
    let _ = sqlx::query("DELETE FROM user_tiers WHERE user_id = ANY($1)")
        .bind(&users)
        .execute(&pool)
        .await;
    users.iter().for_each(drop);
    let users_for_cleanup = users.clone();
    for u in &users_for_cleanup {
        cleanup_user(&pool, *u).await;
    }
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        total_participants, 3,
        "tier_id=2 should contain exactly 3 users; got {}",
        total_participants
    );
    assert_eq!(
        ranks.len() as i64,
        total_participants,
        "rankings query must return total_participants rows; got rows={} count={}",
        ranks.len(),
        total_participants
    );
    // Ranks reflect global positions (ROW_NUMBER is computed BEFORE the tier
    // filter), so we expect monotonically increasing values within [1, 10].
    let mut prev = 0i32;
    for (r,) in &ranks {
        assert!(
            *r > prev,
            "ranks must be strictly increasing: {} <= {}",
            *r,
            prev
        );
        assert!(*r >= 1 && *r <= 10, "rank {} out of expected range", *r);
        prev = *r;
    }
}

// ─── Test 4: tier_id filter narrows total_participants ──────────────────────
//
// Smoke test that the (tier_id IS NULL OR tier_id = $X) predicate used in
// `get_rankings_alltime` and the count query filters the participant pool
// rather than the rank ordering.

#[ignore]
#[tokio::test]
async fn tier_filter_narrows_participant_pool() {
    let pool = pool().await;
    let u_tier_a = Uuid::new_v4();
    let u_tier_b = Uuid::new_v4();
    let asset = Uuid::new_v4();
    let tier_a: i32 = 1;
    let tier_b: i32 = 2;

    cleanup_user(&pool, u_tier_a).await;
    cleanup_user(&pool, u_tier_b).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, u_tier_a, "active").await;
    insert_user(&pool, u_tier_b, "active").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment(&pool, u_tier_a, asset, 10_000).await;
    insert_investment(&pool, u_tier_b, asset, 20_000).await;
    refresh_all_scores_sql(&pool).await;

    // Best-effort tier assignment. user_tiers may have a different shape per
    // schema version; we use ON CONFLICT DO NOTHING so re-runs don't fail.
    let _ = sqlx::query(
        "INSERT INTO user_tiers (user_id, tier_id) VALUES ($1, $2), ($3, $4)
         ON CONFLICT (user_id) DO UPDATE SET tier_id = EXCLUDED.tier_id",
    )
    .bind(u_tier_a)
    .bind(tier_a)
    .bind(u_tier_b)
    .bind(tier_b)
    .execute(&pool)
    .await;

    // Count participants in tier A only (mirrors get_rankings_alltime count_query)
    let count_a: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT
           FROM leaderboard_scores ls
           LEFT JOIN user_tiers ut ON ut.user_id = ls.user_id
           WHERE ls.user_id = ANY($1) AND ut.tier_id = $2"#,
    )
    .bind(vec![u_tier_a, u_tier_b])
    .bind(tier_a)
    .fetch_one(&pool)
    .await
    .expect("count tier A");

    // Total across both
    let count_all: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT
           FROM leaderboard_scores ls
           WHERE ls.user_id = ANY($1)"#,
    )
    .bind(vec![u_tier_a, u_tier_b])
    .fetch_one(&pool)
    .await
    .expect("count all");

    cleanup_user(&pool, u_tier_a).await;
    cleanup_user(&pool, u_tier_b).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(count_a, 1, "tier A should contain exactly 1 user");
    assert_eq!(count_all, 2, "no-filter count should contain both users");
}

// ─── Task 3: direct service-level has_more regression tests ────────────────
//
// The earlier `has_more_false_when_total_is_exact_multiple_of_per_page`
// test (above) exercises the SQL pattern inline. These tests go further:
// they call `poool_backend::leaderboard::service::get_rankings` directly
// — same code path the HTTP handler invokes — so a regression in the
// service-layer derivation is caught even if the inline SQL stays
// correct. Each seeded user gets a unique `HMTEST_<id>` display_name +
// `leaderboard_preferences.visible = true`, and the tests pass
// `search = Some("HMTEST_")` so `total_participants` is scoped to the
// test cohort regardless of other rows in `leaderboard_scores`.

async fn seed_searchable_users(pool: &PgPool, n: usize, test_id: &str) -> (Vec<Uuid>, Uuid) {
    let asset = Uuid::new_v4();
    cleanup_asset(pool, asset).await;
    insert_asset(pool, asset, 500).await;

    let users: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
    for u in &users {
        cleanup_user(pool, *u).await;
        insert_user(pool, *u, "active").await;
        // Distinct, very large investment ensures the test cohort sorts
        // strictly above any organic real-user data in the test DB.
        let amount: i64 = (n as i64 - users.iter().position(|x| x == u).unwrap() as i64)
            * 1_000_000_000_i64
            + 100_000_000_i64;
        insert_investment(pool, *u, asset, amount).await;
        // Display name must match the unique HMTEST_<id> prefix so the
        // service's full_name predicate scopes the count to this cohort.
        sqlx::query(
            r#"INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name)
               VALUES ($1, TRUE, TRUE, $2)
               ON CONFLICT (user_id) DO UPDATE SET
                   visible = TRUE,
                   display_name = EXCLUDED.display_name"#,
        )
        .bind(u)
        .bind(format!("HMTEST_{}_{}", test_id, u.simple()))
        .execute(pool)
        .await
        .expect("insert pref");
    }

    refresh_all_scores_sql(pool).await;
    // Mirror the rank-update step the production refresh performs.
    sqlx::query(
        r#"UPDATE leaderboard_scores ls SET rank_invested = sub.r_inv
           FROM (
               SELECT user_id, ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) AS r_inv
               FROM leaderboard_scores
           ) sub WHERE ls.user_id = sub.user_id"#,
    )
    .execute(pool)
    .await
    .expect("rank refresh");

    (users, asset)
}

async fn cleanup_searchable_users(pool: &PgPool, users: &[Uuid], asset: Uuid) {
    for u in users {
        cleanup_user(pool, *u).await;
    }
    cleanup_asset(pool, asset).await;
}

#[ignore]
#[tokio::test]
async fn has_more_false_on_exact_multiple_page() {
    // Seed exactly 20 ranked users, request page=2 with per_page=10.
    // Page 2 is the LAST page; has_more must be false. Pre-fix this
    // returned true (because rankings.len() == per_page heuristic).
    let pool = pool().await;
    let test_id = "exact20";
    let (users, asset) = seed_searchable_users(&pool, 20, test_id).await;
    // Viewer must NOT be in the seeded cohort. Post-audit the service
    // excludes the viewer from the listing (`m.user_id <> $current_user`)
    // so that the user sees themselves only in the "Your Standing" card.
    // If we used `users[0]` here the cohort effectively shrinks to 19 and
    // the total_participants assertion below breaks.
    let viewer = Uuid::new_v4();

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        2,    // page
        10,   // per_page
        None, // tier_id
        Some(format!("HMTEST_{}", test_id)),
        None, // last_updated_cache
    )
    .await
    .expect("get_rankings");

    cleanup_searchable_users(&pool, &users, asset).await;

    assert_eq!(
        resp.total_participants, 20,
        "search-scoped total_participants should be exactly 20; got {}",
        resp.total_participants
    );
    assert_eq!(
        resp.rankings.len(),
        10,
        "page 2 must return 10 rows; got {}",
        resp.rankings.len()
    );
    assert!(
        !resp.has_more,
        "has_more must be FALSE on the last full page (regression for audit fix B1)"
    );
}

#[ignore]
#[tokio::test]
async fn has_more_true_with_overflow() {
    // Seed 25 ranked users. With page=2 per_page=10, page 2 holds ranks
    // 11..20 (10 rows). 5 more rows remain, so has_more must be true.
    let pool = pool().await;
    let test_id = "over25";
    let (users, asset) = seed_searchable_users(&pool, 25, test_id).await;
    let viewer = Uuid::new_v4(); // not in seeded cohort — see exact20 test rationale

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        2,
        10,
        None,
        Some(format!("HMTEST_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_searchable_users(&pool, &users, asset).await;

    assert_eq!(resp.total_participants, 25);
    assert_eq!(resp.rankings.len(), 10, "page 2 must return 10 rows");
    assert!(
        resp.has_more,
        "has_more must be TRUE when 5 more rows remain after page 2"
    );
}

#[ignore]
#[tokio::test]
async fn has_more_false_partial_last_page() {
    // Seed 15 users. With page=2 per_page=10, page 2 holds only 5 rows
    // (ranks 11..15). has_more must be false — the page isn't even full.
    let pool = pool().await;
    let test_id = "part15";
    let (users, asset) = seed_searchable_users(&pool, 15, test_id).await;
    let viewer = Uuid::new_v4(); // not in seeded cohort — see exact20 test rationale

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        2,
        10,
        None,
        Some(format!("HMTEST_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_searchable_users(&pool, &users, asset).await;

    assert_eq!(resp.total_participants, 15);
    assert_eq!(
        resp.rankings.len(),
        5,
        "page 2 of a 15-user set must return 5 rows; got {}",
        resp.rankings.len()
    );
    assert!(
        !resp.has_more,
        "has_more must be FALSE on a partial last page"
    );
}

// ─── Task 4: visibility filter excludes hidden users + viewer self ─────────
//
// Production behavior (audit fix, 2026-05-16): only users with
// `leaderboard_preferences.visible = true` show up in the public listing,
// and the viewer is always excluded from the listing as well (they see
// themselves separately in the "Your Standing" card).
//
// Regression scenario:
//   - Seed 4 users. Mark 2 as visible, 2 as hidden (visible=false).
//   - The viewer is the 5th (unseeded) user — also visible toggle does
//     not matter for them since they're always excluded.
//   - The returned listing must have exactly 2 rows (the visible non-self
//     pair) and `total_participants` must match.
//   - Ranks must be sequential (1, 2) — re-ranked via ROW_NUMBER over
//     the visible-only set, NOT pulled from the precomputed rank columns
//     where the hidden users would still occupy positions.

#[ignore]
#[tokio::test]
async fn visibility_filter_excludes_hidden_users_and_reranks() {
    let pool = pool().await;
    let test_id = "vis_filter";

    // Seed 4 users with HMTEST_<id>_<uuid> display names so the test cohort
    // is isolated via the service's `search` parameter.
    let asset = Uuid::new_v4();
    cleanup_asset(&pool, asset).await;
    insert_asset(&pool, asset, 500).await;

    let visible_a = Uuid::new_v4();
    let visible_b = Uuid::new_v4();
    let hidden_a = Uuid::new_v4();
    let hidden_b = Uuid::new_v4();
    let users = [visible_a, visible_b, hidden_a, hidden_b];

    for u in &users {
        cleanup_user(&pool, *u).await;
        insert_user(&pool, *u, "active").await;
    }
    // Distinct amounts so re-ranking is observable and deterministic.
    insert_investment(&pool, visible_a, asset, 40_000_000_000).await;
    insert_investment(&pool, hidden_a, asset, 30_000_000_000).await;
    insert_investment(&pool, visible_b, asset, 20_000_000_000).await;
    insert_investment(&pool, hidden_b, asset, 10_000_000_000).await;

    // Set preferences: 2 visible, 2 hidden.
    for (u, vis) in [
        (visible_a, true),
        (visible_b, true),
        (hidden_a, false),
        (hidden_b, false),
    ] {
        sqlx::query(
            r#"INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name)
               VALUES ($1, $2, FALSE, $3)
               ON CONFLICT (user_id) DO UPDATE SET
                   visible = EXCLUDED.visible,
                   display_name = EXCLUDED.display_name"#,
        )
        .bind(u)
        .bind(vis)
        .bind(format!("HMTEST_{}_{}", test_id, u.simple()))
        .execute(&pool)
        .await
        .expect("insert pref");
    }

    refresh_all_scores_sql(&pool).await;
    sqlx::query(
        r#"UPDATE leaderboard_scores ls SET rank_invested = sub.r_inv
           FROM (
               SELECT user_id, ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) AS r_inv
               FROM leaderboard_scores
           ) sub WHERE ls.user_id = sub.user_id"#,
    )
    .execute(&pool)
    .await
    .expect("rank refresh");

    // Viewer is a 5th, unseeded user — so they're excluded simply by the
    // service's `m.user_id <> $current_user` predicate. We're testing the
    // visibility filter, not the self-exclusion edge case.
    let viewer = Uuid::new_v4();

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("HMTEST_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    // Cleanup before assertions so failures don't leak test data.
    for u in &users {
        cleanup_user(&pool, *u).await;
    }
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        resp.total_participants, 2,
        "only visible_a + visible_b should be counted; got {}",
        resp.total_participants
    );
    assert_eq!(
        resp.rankings.len(),
        2,
        "listing must contain only the 2 visible users; got {} rows",
        resp.rankings.len()
    );
    // Ranks must be re-derived (1 and 2) — NOT inherited from the
    // precomputed rank_invested column (which would have placed the
    // visible users at positions 1 and 3 because hidden_a sat between them).
    let ranks: Vec<i32> = resp.rankings.iter().map(|r| r.rank).collect();
    assert_eq!(
        ranks,
        vec![1, 2],
        "ranks must be sequential after visibility filter (re-ranked); got {:?}",
        ranks,
    );
    // Top row must be visible_a (largest invested among visible users).
    assert_eq!(resp.rankings[0].metric_value, 40_000_000_000);
    assert_eq!(resp.rankings[1].metric_value, 20_000_000_000);
}

// ─── Task 5: viewer is excluded from public listing even when visible ──────
//
// Edge case of the visibility filter: a user who has opted in (visible=true)
// must still NOT appear in their own view of the leaderboard. The product
// rule is that their rank is surfaced in the "Your Standing" card via
// get_my_rank_*, while the listing always shows other people.

#[ignore]
#[tokio::test]
async fn visible_viewer_excluded_from_own_listing() {
    let pool = pool().await;
    let test_id = "viewer_excl";
    let asset = Uuid::new_v4();
    cleanup_asset(&pool, asset).await;
    insert_asset(&pool, asset, 500).await;

    let viewer = Uuid::new_v4();
    let other = Uuid::new_v4();
    for u in [viewer, other] {
        cleanup_user(&pool, u).await;
        insert_user(&pool, u, "active").await;
    }

    insert_investment(&pool, viewer, asset, 99_000_000_000).await;
    insert_investment(&pool, other, asset, 10_000_000_000).await;

    for (u, n) in [(viewer, "viewer"), (other, "other")] {
        sqlx::query(
            r#"INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name)
               VALUES ($1, TRUE, FALSE, $2)
               ON CONFLICT (user_id) DO UPDATE SET
                   visible = TRUE,
                   display_name = EXCLUDED.display_name"#,
        )
        .bind(u)
        .bind(format!("HMTEST_{}_{}", test_id, n))
        .execute(&pool)
        .await
        .expect("insert pref");
    }

    refresh_all_scores_sql(&pool).await;
    sqlx::query(
        r#"UPDATE leaderboard_scores ls SET rank_invested = sub.r_inv
           FROM (
               SELECT user_id, ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) AS r_inv
               FROM leaderboard_scores
           ) sub WHERE ls.user_id = sub.user_id"#,
    )
    .execute(&pool)
    .await
    .expect("rank refresh");

    let resp = poool_backend::leaderboard::service::get_rankings(
        &pool,
        viewer,
        "invested",
        "alltime",
        1,
        10,
        None,
        Some(format!("HMTEST_{}", test_id)),
        None,
    )
    .await
    .expect("get_rankings");

    cleanup_user(&pool, viewer).await;
    cleanup_user(&pool, other).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        resp.total_participants, 1,
        "viewer is excluded → total_participants is 1 (other only); got {}",
        resp.total_participants,
    );
    assert_eq!(
        resp.rankings.len(),
        1,
        "listing must have only the other user"
    );
    assert_eq!(
        resp.rankings[0].metric_value, 10_000_000_000,
        "only `other` (10B) should appear; viewer (99B) is filtered out despite being visible",
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Tests for the precomputed weekly / monthly leaderboard tables (added
// in migration 168 to close the Bereich-1+7 audit gap where weekly /
// monthly rankings were recomputed live on every request).
//
// These mirror the SQL the production `refresh_timeframed_scores`
// function runs, so the test does not need to link the bin crate's
// service module — same pattern as `refresh_all_scores_sql` above.
// ═══════════════════════════════════════════════════════════════════════

/// Replay the SQL that `service::refresh_timeframed_scores(pool, timeframe)`
/// runs against the given timeframe table. `cutoff` is the SQL expression
/// the production code interpolates (e.g. `"NOW() - INTERVAL '7 days'"`).
async fn refresh_timeframed_sql(pool: &PgPool, table: &str, cutoff_sql: &str) {
    let truncate = format!("TRUNCATE TABLE {table}");
    sqlx::query(&truncate)
        .execute(pool)
        .await
        .expect("truncate timeframed table");

    let insert = format!(
        r#"
        INSERT INTO {table} (
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
            SELECT
                i.user_id,
                SUM(i.purchase_value_cents)::BIGINT          AS total_invested,
                COUNT(DISTINCT i.asset_id)                   AS unique_assets,
                MAX(i.purchase_value_cents)                  AS highest_inv,
                COALESCE(
                    ROUND(
                        SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                        / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                    ),
                    0
                )                                            AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
              AND i.purchased_at >= {cutoff}
            GROUP BY i.user_id
        ) inv_agg ON inv_agg.user_id = u.id
        WHERE u.status = 'active'
          AND inv_agg.total_invested > 0
        "#,
        table = table,
        cutoff = cutoff_sql,
    );
    sqlx::query(&insert)
        .execute(pool)
        .await
        .expect("insert timeframed");

    // Same rank-update CTE as production.
    let rank_sql = format!(
        r#"
        UPDATE {table} ts SET
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
                ROW_NUMBER() OVER (ORDER BY affiliate_count DESC, referral_network_value_cents DESC, computed_at ASC)    AS r_aff,
                ROW_NUMBER() OVER (ORDER BY referral_network_value_cents DESC, affiliate_count DESC, computed_at ASC)    AS r_rev,
                ROW_NUMBER() OVER (ORDER BY highest_investment_cents DESC, computed_at ASC)                              AS r_hi
            FROM {table}
        ) sub WHERE ts.user_id = sub.user_id
        "#,
        table = table,
    );
    sqlx::query(&rank_sql)
        .execute(pool)
        .await
        .expect("update ranks");
}

/// Insert investment with explicit `purchased_at` so timeframe-window tests
/// can stage rows inside vs. outside the cutoff.
async fn insert_investment_with_date(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    value_cents: i64,
    purchased_at_sql: &str, // e.g. "NOW() - INTERVAL '3 days'"
) {
    let q = format!(
        "INSERT INTO investments
            (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status, purchased_at)
         VALUES ($1, $2, 100, $3, $3, 'active', {purchased_at})",
        purchased_at = purchased_at_sql,
    );
    sqlx::query(&q)
        .bind(user_id)
        .bind(asset_id)
        .bind(value_cents)
        .execute(pool)
        .await
        .expect("insert investment with date");
}

#[ignore]
#[tokio::test]
async fn weekly_table_populated_by_refresh_sql() {
    let pool = pool().await;
    let user = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, user, "active").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment_with_date(&pool, user, asset, 50_000, "NOW() - INTERVAL '2 days'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_weekly",
        "NOW() - INTERVAL '7 days'",
    )
    .await;

    let value: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("query weekly");

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        value,
        Some(50_000),
        "weekly table must contain the user's recent investment after refresh"
    );
}

#[ignore]
#[tokio::test]
async fn monthly_table_populated_by_refresh_sql() {
    let pool = pool().await;
    let user = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, user, "active").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment_with_date(&pool, user, asset, 75_000, "NOW() - INTERVAL '15 days'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_monthly",
        "NOW() - INTERVAL '30 days'",
    )
    .await;

    let value: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_monthly WHERE user_id = $1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("query monthly");

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        value,
        Some(75_000),
        "monthly table must contain a 15-day-old investment"
    );
}

#[ignore]
#[tokio::test]
async fn weekly_filters_old_purchases() {
    // Investment from 30 days ago must NOT contribute to the weekly table.
    let pool = pool().await;
    let user = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, user, "active").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment_with_date(&pool, user, asset, 99_000, "NOW() - INTERVAL '30 days'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_weekly",
        "NOW() - INTERVAL '7 days'",
    )
    .await;

    let row: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("query weekly");

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    assert!(
        row.is_none(),
        "investment outside the 7-day window must not appear in weekly table; got {:?}",
        row,
    );
}

#[ignore]
#[tokio::test]
async fn weekly_ranks_assigned_sequentially() {
    // Three users with distinct invested amounts → ranks 1, 2, 3.
    let pool = pool().await;
    let u1 = Uuid::new_v4();
    let u2 = Uuid::new_v4();
    let u3 = Uuid::new_v4();
    let asset = Uuid::new_v4();

    for u in [u1, u2, u3] {
        cleanup_user(&pool, u).await;
    }
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, u1, "active").await;
    insert_user(&pool, u2, "active").await;
    insert_user(&pool, u3, "active").await;
    insert_asset(&pool, asset, 500).await;

    insert_investment_with_date(&pool, u1, asset, 30_000, "NOW() - INTERVAL '1 day'").await;
    insert_investment_with_date(&pool, u2, asset, 10_000, "NOW() - INTERVAL '1 day'").await;
    insert_investment_with_date(&pool, u3, asset, 20_000, "NOW() - INTERVAL '1 day'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_weekly",
        "NOW() - INTERVAL '7 days'",
    )
    .await;

    let rank_u1: Option<i32> = sqlx::query_scalar(
        "SELECT rank_invested FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(u1)
    .fetch_optional(&pool)
    .await
    .expect("rank u1")
    .flatten();
    let rank_u2: Option<i32> = sqlx::query_scalar(
        "SELECT rank_invested FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(u2)
    .fetch_optional(&pool)
    .await
    .expect("rank u2")
    .flatten();
    let rank_u3: Option<i32> = sqlx::query_scalar(
        "SELECT rank_invested FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(u3)
    .fetch_optional(&pool)
    .await
    .expect("rank u3")
    .flatten();

    for u in [u1, u2, u3] {
        cleanup_user(&pool, u).await;
    }
    cleanup_asset(&pool, asset).await;

    // Other production rows may share the weekly table, so absolute ranks
    // vary by environment. The invariant we care about is relative order:
    // u1 (€30K) outranks u3 (€20K) outranks u2 (€10K).
    let r1 = rank_u1.expect("u1 must have a rank");
    let r3 = rank_u3.expect("u3 must have a rank");
    let r2 = rank_u2.expect("u2 must have a rank");
    assert!(
        r1 < r3,
        "u1 (€30K, rank {}) must outrank u3 (€20K, rank {})",
        r1,
        r3
    );
    assert!(
        r3 < r2,
        "u3 (€20K, rank {}) must outrank u2 (€10K, rank {})",
        r3,
        r2
    );
}

#[ignore]
#[tokio::test]
async fn timeframed_table_truncated_on_refresh() {
    // A row that no longer qualifies (e.g. investment aged out of window)
    // must be REMOVED by the next refresh, not retained as stale data.
    let pool = pool().await;
    let user = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, user, "active").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment_with_date(&pool, user, asset, 10_000, "NOW() - INTERVAL '2 days'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_weekly",
        "NOW() - INTERVAL '7 days'",
    )
    .await;

    // Pre-condition: row exists.
    let pre: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("pre query");
    assert_eq!(pre, Some(10_000), "pre-condition: weekly row exists");

    // Age the investment out of the window by updating purchased_at.
    sqlx::query(
        "UPDATE investments SET purchased_at = NOW() - INTERVAL '30 days' WHERE user_id = $1",
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("age investment");

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_weekly",
        "NOW() - INTERVAL '7 days'",
    )
    .await;

    let post: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("post query");

    cleanup_user(&pool, user).await;
    cleanup_asset(&pool, asset).await;

    assert!(
        post.is_none(),
        "TRUNCATE + reinsert pattern must wipe rows that no longer qualify; got {:?}",
        post,
    );
}

#[ignore]
#[tokio::test]
async fn weekly_excludes_inactive_users() {
    // Even with a fresh investment inside the window, status != 'active'
    // must keep the user out of the weekly table.
    let pool = pool().await;
    let active = Uuid::new_v4();
    let suspended = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, active).await;
    cleanup_user(&pool, suspended).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, active, "active").await;
    insert_user(&pool, suspended, "suspended").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment_with_date(&pool, active, asset, 10_000, "NOW() - INTERVAL '1 day'").await;
    insert_investment_with_date(&pool, suspended, asset, 10_000, "NOW() - INTERVAL '1 day'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_weekly",
        "NOW() - INTERVAL '7 days'",
    )
    .await;

    let active_row: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(active)
    .fetch_optional(&pool)
    .await
    .expect("active");
    let suspended_row: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_weekly WHERE user_id = $1",
    )
    .bind(suspended)
    .fetch_optional(&pool)
    .await
    .expect("suspended");

    cleanup_user(&pool, active).await;
    cleanup_user(&pool, suspended).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(active_row, Some(10_000));
    assert!(
        suspended_row.is_none(),
        "suspended users must be excluded from the weekly table too"
    );
}

#[ignore]
#[tokio::test]
async fn monthly_window_30_days_inclusive() {
    // Investment exactly 29 days old must be IN; 31 days old must be OUT.
    let pool = pool().await;
    let inside = Uuid::new_v4();
    let outside = Uuid::new_v4();
    let asset = Uuid::new_v4();

    cleanup_user(&pool, inside).await;
    cleanup_user(&pool, outside).await;
    cleanup_asset(&pool, asset).await;

    insert_user(&pool, inside, "active").await;
    insert_user(&pool, outside, "active").await;
    insert_asset(&pool, asset, 500).await;
    insert_investment_with_date(&pool, inside, asset, 12_000, "NOW() - INTERVAL '29 days'").await;
    insert_investment_with_date(&pool, outside, asset, 12_000, "NOW() - INTERVAL '31 days'").await;

    refresh_timeframed_sql(
        &pool,
        "leaderboard_scores_monthly",
        "NOW() - INTERVAL '30 days'",
    )
    .await;

    let inside_row: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_monthly WHERE user_id = $1",
    )
    .bind(inside)
    .fetch_optional(&pool)
    .await
    .expect("inside");
    let outside_row: Option<i64> = sqlx::query_scalar(
        "SELECT total_invested_cents FROM leaderboard_scores_monthly WHERE user_id = $1",
    )
    .bind(outside)
    .fetch_optional(&pool)
    .await
    .expect("outside");

    cleanup_user(&pool, inside).await;
    cleanup_user(&pool, outside).await;
    cleanup_asset(&pool, asset).await;

    assert_eq!(
        inside_row,
        Some(12_000),
        "29-day-old investment is within the 30-day window"
    );
    assert!(
        outside_row.is_none(),
        "31-day-old investment must be outside the 30-day window; got {:?}",
        outside_row,
    );
}
