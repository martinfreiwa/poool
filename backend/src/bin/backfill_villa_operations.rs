//! Villa-Returns B3 — legacy `asset_financials` → `villa_operations_log` backfill.
//!
//! For each row in the legacy `asset_financials` table, insert a corresponding
//! `villa_operations_log` row with `status='published'` so the new pipeline has
//! historical context. Idempotent: skips `(asset_id, period_year, period_month)`
//! combinations that already have a non-superseded published row.
//!
//! Usage:
//!     # Dry-run (default — prints diff, no writes):
//!     cargo run --bin backfill-villa-operations
//!
//!     # Execute (writes rows):
//!     cargo run --bin backfill-villa-operations -- --execute
//!
//! Legacy fields map as follows:
//!   asset_financials.rental_income_cents → gross_rental_idr_cents
//!   asset_financials.expenses_cents      → total_opex_idr_cents
//!   asset_financials.net_income_cents    → net_rental_income_idr_cents,
//!                                          distributable_idr_cents (no fee/reserve info)
//!   expense breakdown columns            → 0 (legacy lumps OpEx)
//!   correction_reason                    → 'legacy backfill — breakdown unavailable'
//!
//! NOTE — assumption: legacy `*_cents` columns are already in IDR cents. If a
//! given environment stores them in USD cents, **change the mapping or do not
//! run** — the data would be off by ~15,500×.

use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let execute = env::args().any(|a| a == "--execute");
    let database_url = env::var("DATABASE_URL")
        .or_else(|_| env::var("POOOL_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://localhost/poool".to_string());

    println!(
        "Villa-Returns B3 backfill — mode = {}",
        if execute {
            "EXECUTE (writes durable rows)"
        } else {
            "DRY-RUN (no writes)"
        }
    );
    println!("Connecting to: {}", redact_url(&database_url));

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;

    let candidates: Vec<(
        uuid::Uuid,
        i32,
        i32,
        i64,
        i64,
        i64,
        Option<i32>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        r#"
        SELECT af.asset_id, af.period_year, af.period_month,
               COALESCE(af.rental_income_cents, 0)::BIGINT AS gross,
               COALESCE(af.expenses_cents, 0)::BIGINT AS opex,
               COALESCE(af.net_income_cents, 0)::BIGINT AS net,
               af.occupancy_rate_bps,
               af.created_at
        FROM asset_financials af
        ORDER BY af.asset_id, af.period_year, af.period_month
        "#,
    )
    .fetch_all(&pool)
    .await?;

    println!("\nLegacy `asset_financials` rows: {}", candidates.len());

    let mut planned: u64 = 0;
    let mut skipped_existing: u64 = 0;
    let mut written: u64 = 0;

    for (asset_id, py, pm, gross, opex, net, _occ_bps, created) in &candidates {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM villa_operations_log
                WHERE asset_id = $1 AND period_year = $2 AND period_month = $3
                  AND status = 'published' AND supersedes_id IS NULL
            )
            "#,
        )
        .bind(asset_id)
        .bind(py)
        .bind(pm)
        .fetch_one(&pool)
        .await?;

        if exists {
            skipped_existing += 1;
            continue;
        }

        planned += 1;
        println!(
            "  PLAN: asset={} period={}-{:02} gross={} opex={} net={}",
            asset_id, py, pm, gross, opex, net
        );

        if !execute {
            continue;
        }

        // Real write. Submitted_by + approved_by NULL — legacy has no actor info;
        // 4-eyes CHECK passes because both are NULL.
        sqlx::query(
            r#"
            INSERT INTO villa_operations_log
                (asset_id, period_year, period_month,
                 gross_rental_idr_cents, total_opex_idr_cents,
                 net_rental_income_idr_cents, distributable_idr_cents,
                 status, correction_reason,
                 recorded_at, published_at, fx_rate_idr_to_usd_bps)
            VALUES ($1, $2, $3, $4, $5, $6, $7,
                    'published', 'legacy backfill — breakdown unavailable',
                    $8, $8, 1)
            "#,
        )
        .bind(asset_id)
        .bind(py)
        .bind(pm)
        .bind(gross)
        .bind(opex)
        .bind(net)
        .bind(net) // distributable = net (no fee/reserve info in legacy)
        .bind(created)
        .execute(&pool)
        .await?;

        written += 1;
    }

    println!("\nSummary:");
    println!("  Candidate rows:  {}", candidates.len());
    println!("  Already-present: {}", skipped_existing);
    println!("  Planned writes:  {}", planned);
    if execute {
        println!("  Executed inserts: {}", written);
    } else {
        println!("  Executed inserts: 0  (dry-run — re-run with --execute to commit)");
    }

    Ok(())
}

fn redact_url(url: &str) -> String {
    // Hide password if present.
    if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let after_at = &url[at_pos..];
            return format!("{}***{}", scheme, after_at);
        }
    }
    url.to_string()
}
