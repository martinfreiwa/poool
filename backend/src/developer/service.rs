/// Developer dashboard service layer — all DB queries and business logic.
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;

/// Format cents as a compact USD string: $138.4k, $1.5M, $250, etc.
fn format_usd_compact(cents: i64) -> String {
    let dollars = cents as f64 / 100.0;
    if dollars.abs() >= 1_000_000.0 {
        format!("${:.1}M", dollars / 1_000_000.0)
    } else if dollars.abs() >= 1_000.0 {
        format!("${:.1}k", dollars / 1_000.0)
    } else {
        format!("${:.0}", dollars)
    }
}

/// Format a percentage for display.
fn format_pct(value: f64) -> String {
    if value.abs() < 0.01 {
        "0%".to_string()
    } else if value.abs() >= 10.0 {
        format!("{:.0}%", value)
    } else {
        format!("{:.1}%", value)
    }
}

/// Determine trend direction.
fn trend(change: f64) -> String {
    if change > 0.0 {
        "up".to_string()
    } else if change < 0.0 {
        "down".to_string()
    } else {
        "neutral".to_string()
    }
}

/// Build a single `DeveloperMetric` helper.
fn make_metric(label: &str, value: String, change_pct: f64) -> DeveloperMetric {
    DeveloperMetric {
        label: label.to_string(),
        value,
        change_pct,
        trend: trend(change_pct),
        change_display: format_pct(change_pct.abs()),
        timeframe: "vs last mth".to_string(),
    }
}

/// Fetch all developer dashboard statistics for a given developer user.
pub async fn fetch_dashboard_stats(pool: &PgPool, developer_id: Uuid) -> DeveloperDashboardStats {
    // ── 1. Total Assets (only approved/live) ─────────────────────
    let total_assets: i64 =
        sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM assets a INNER JOIN developer_projects dp ON dp.asset_id = a.id WHERE a.developer_user_id = $1 AND dp.status IN ('approved', 'live') AND a.deleted_at IS NULL"
        )
            .bind(developer_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    // ── 2. Total Sales (sum of investments for developer's assets) ──
    let total_sales_cents: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(i.purchase_value_cents), 0)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE a.developer_user_id = $1
          AND i.status != 'exited'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 3. Total Investors (distinct users who invested in developer's assets) ──
    let total_investors: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT i.user_id)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE a.developer_user_id = $1
          AND i.status != 'exited'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 4. New Investors (invested in the last 30 days) ─────────────
    let new_investors: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT i.user_id)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE a.developer_user_id = $1
          AND i.purchased_at >= NOW() - INTERVAL '30 days'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 5. Total Views (from asset_views if the table exists, else 0) ──
    // We'll try the query; if the table doesn't exist yet, fallback to 0.
    let total_views: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(view_count), 0)::bigint
        FROM (
            SELECT COUNT(*)::bigint AS view_count
            FROM asset_views av
            JOIN assets a ON a.id = av.asset_id
            WHERE a.developer_user_id = $1
        ) sub
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 6. Avg. Conversion Rate ─────────────────────────────────────
    let avg_conversion_rate = if total_views > 0 {
        (total_investors as f64 / total_views as f64) * 100.0
    } else {
        0.0
    };

    // ── 7. Sold Out Ratio ───────────────────────────────────────────
    let funded_assets: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM assets
        WHERE developer_user_id = $1
          AND tokens_available = 0
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let sold_out_ratio = if total_assets > 0 {
        (funded_assets as f64 / total_assets as f64) * 100.0
    } else {
        0.0
    };

    // ── 8. Avg. Investment Amount ───────────────────────────────────
    let avg_investment_cents: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(AVG(i.purchase_value_cents), 0)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE a.developer_user_id = $1
          AND i.status != 'exited'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 8.5 Period-Over-Period calculations ──────────────────────────────
    let pop_row = sqlx::query!(
        r#"
        WITH current_period AS (
            SELECT 
                COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS sales,
                COUNT(DISTINCT i.user_id)::bigint AS investors,
                COALESCE(AVG(i.purchase_value_cents), 0)::bigint AS avg_investment
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE a.developer_user_id = $1 AND i.status != 'exited'
              AND i.purchased_at >= NOW() - INTERVAL '30 days'
        ),
        prev_period AS (
            SELECT 
                COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS sales,
                COUNT(DISTINCT i.user_id)::bigint AS investors,
                COALESCE(AVG(i.purchase_value_cents), 0)::bigint AS avg_investment
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE a.developer_user_id = $1 AND i.status != 'exited'
              AND i.purchased_at >= NOW() - INTERVAL '60 days'
              AND i.purchased_at < NOW() - INTERVAL '30 days'
        )
        SELECT 
            c.sales AS "current_sales!",
            p.sales AS "prev_sales!",
            c.investors AS "current_investors!",
            p.investors AS "prev_investors!",
            c.avg_investment AS "current_avg_inv!",
            p.avg_investment AS "prev_avg_inv!"
        FROM current_period c CROSS JOIN prev_period p
        "#,
        developer_id
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let (sales_pct, inv_pct, avg_pct) = match pop_row {
        Some(row) => {
            let s_pct = if row.prev_sales > 0 {
                ((row.current_sales - row.prev_sales) as f64 / row.prev_sales as f64) * 100.0
            } else if row.current_sales > 0 {
                100.0
            } else {
                0.0
            };
            let i_pct = if row.prev_investors > 0 {
                ((row.current_investors - row.prev_investors) as f64 / row.prev_investors as f64)
                    * 100.0
            } else if row.current_investors > 0 {
                100.0
            } else {
                0.0
            };
            let a_pct = if row.prev_avg_inv > 0 {
                ((row.current_avg_inv - row.prev_avg_inv) as f64 / row.prev_avg_inv as f64) * 100.0
            } else if row.current_avg_inv > 0 {
                100.0
            } else {
                0.0
            };
            (s_pct, i_pct, a_pct)
        }
        None => (0.0, 0.0, 0.0),
    };

    // ── Build 8 metric cards ─────────────────────────────────────────
    let metrics = vec![
        make_metric("Total Assets", total_assets.to_string(), 0.0),
        make_metric(
            "Total Sales",
            format_usd_compact(total_sales_cents),
            sales_pct,
        ),
        make_metric("Total Investors", total_investors.to_string(), inv_pct),
        make_metric("New Investors", new_investors.to_string(), inv_pct),
        make_metric("Total Views", format!("{}", total_views), 0.0),
        make_metric("Avg. Conversion Rate", format_pct(avg_conversion_rate), 0.0),
        make_metric("Sold Out Ratio", format_pct(sold_out_ratio), 0.0),
        make_metric(
            "Avg. Investment Amount",
            format_usd_compact(avg_investment_cents),
            avg_pct,
        ),
    ];

    // ── Top Assets table ──────────────────────────────────────────────
    let top_assets = fetch_top_assets(pool, developer_id).await;

    // ── Chart percentage (all-time sales change vs prior period) ────────
    let chart_percentage_display = if sales_pct > 0.01 {
        format!("+{}", format_pct(sales_pct))
    } else if sales_pct < -0.01 {
        format!("-{}", format_pct(sales_pct.abs()))
    } else {
        "0%".to_string()
    };
    let chart_trend = trend(sales_pct);

    DeveloperDashboardStats {
        total_assets,
        total_sales_cents,
        total_sales_display: format_usd_compact(total_sales_cents),
        total_investors,
        total_views,
        new_investors,
        avg_conversion_rate,
        sold_out_ratio,
        avg_investment_cents,
        avg_investment_display: format_usd_compact(avg_investment_cents),
        metrics,
        top_assets,
        chart_percentage_display,
        chart_trend,
    }
}

/// Fetch top assets for the developer, with sales/views/conversion data.
async fn fetch_top_assets(pool: &PgPool, developer_id: Uuid) -> Vec<DeveloperTopAsset> {
    // Fetch the developer's assets with their aggregated investment data
    let rows = sqlx::query_as::<_, (
        Uuid,          // id
        String,        // title
        Option<String>,// cover_image_url
        i64,           // total_sales_cents
        i64,           // investor_count
        i32,           // tokens_total
        i32,           // tokens_available
        String,        // funding_status
        Option<String>,// city
        Option<i32>,   // bedrooms
        i64,           // total_value_cents
        Option<i32>,   // occupancy_rate_bps
    )>(
        r#"
        SELECT
            a.id,
            a.title,
            (SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY sort_order LIMIT 1) AS cover_image_url,
            COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS total_sales_cents,
            COUNT(DISTINCT i.user_id)::bigint AS investor_count,
            a.tokens_total,
            a.tokens_available,
            a.funding_status,
            a.location_city as city,
            a.bedrooms,
            COALESCE(a.total_value_cents, 0),
            a.occupancy_rate_bps
        FROM assets a
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        LEFT JOIN investments i ON i.asset_id = a.id AND i.status != 'exited'
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
        GROUP BY a.id
        ORDER BY COALESCE(SUM(i.purchase_value_cents), 0) DESC
        LIMIT 10
        "#,
    )
    .bind(developer_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .enumerate()
        .map(
            |(
                idx,
                (
                    id,
                    title,
                    cover_img,
                    sales_cents,
                    investors,
                    tokens_total,
                    tokens_available,
                    status,
                    city,
                    bedrooms,
                    total_value,
                    occ_bps,
                ),
            )| {
                let funding_pct = if tokens_total > 0 {
                    ((tokens_total - tokens_available) as f64 / tokens_total as f64) * 100.0
                } else {
                    0.0
                };

                // Views per asset — try asset_views table, fallback to 0
                // We don't do per-asset view query here to keep it efficient;
                // the JS can lazy-load this via API if needed.
                let views: i64 = 0; // Will be populated by API endpoint

                let conversion_rate = if views > 0 {
                    (investors as f64 / views as f64) * 100.0
                } else {
                    0.0
                };

                DeveloperTopAsset {
                    index: idx + 1,
                    id: id.to_string(),
                    title,
                    cover_image_url: cover_img.unwrap_or_else(|| "/images/villa1.webp".to_string()),
                    total_sales_display: format_usd_compact(sales_cents),
                    total_sales_cents: sales_cents,
                    sales_change_pct: 0.0,
                    sales_trend: "neutral".to_string(),
                    views,
                    conversion_rate,
                    conversion_display: format_pct(conversion_rate),
                    funding_pct,
                    funding_display: format_pct(funding_pct),
                    status,
                    city,
                    bedrooms,
                    total_value_display: format_usd_compact(total_value),
                    is_rented: occ_bps.unwrap_or(0) > 0,
                }
            },
        )
        .collect()
}

/// Fetch all assets for the developer, with sales/views/conversion data.
pub async fn fetch_all_assets(pool: &PgPool, developer_id: Uuid) -> Vec<DeveloperTopAsset> {
    // Fetch the developer's assets with their aggregated investment data
    let rows = sqlx::query_as::<_, (
        Uuid,          // id
        String,        // title
        Option<String>,// cover_image_url
        i64,           // total_sales_cents
        i64,           // investor_count
        i32,           // tokens_total
        i32,           // tokens_available
        String,        // funding_status
        Option<String>,// city
        Option<i32>,   // bedrooms
        i64,           // total_value_cents
        Option<i32>,   // occupancy_rate_bps
    )>(
        r#"
        SELECT
            a.id,
            a.title,
            (SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY sort_order LIMIT 1) AS cover_image_url,
            COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS total_sales_cents,
            COUNT(DISTINCT i.user_id)::bigint AS investor_count,
            a.tokens_total,
            a.tokens_available,
            a.funding_status,
            a.location_city as city,
            a.bedrooms,
            COALESCE(a.total_value_cents, 0),
            a.occupancy_rate_bps
        FROM assets a
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        LEFT JOIN investments i ON i.asset_id = a.id AND i.status != 'exited'
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
        GROUP BY a.id
        ORDER BY a.created_at DESC
        "#,
    )
    .bind(developer_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .enumerate()
        .map(
            |(
                idx,
                (
                    id,
                    title,
                    cover_img,
                    sales_cents,
                    investors,
                    tokens_total,
                    tokens_available,
                    status,
                    city,
                    bedrooms,
                    total_value,
                    occ_bps,
                ),
            )| {
                let funding_pct = if tokens_total > 0 {
                    ((tokens_total - tokens_available) as f64 / tokens_total as f64) * 100.0
                } else {
                    0.0
                };

                let views: i64 = 0;

                let conversion_rate = if views > 0 {
                    (investors as f64 / views as f64) * 100.0
                } else {
                    0.0
                };

                DeveloperTopAsset {
                    index: idx + 1,
                    id: id.to_string(),
                    title,
                    cover_image_url: cover_img.unwrap_or_else(|| "/images/villa1.webp".to_string()),
                    total_sales_display: format_usd_compact(sales_cents),
                    total_sales_cents: sales_cents,
                    sales_change_pct: 0.0,
                    sales_trend: "neutral".to_string(),
                    views,
                    conversion_rate,
                    conversion_display: format_pct(conversion_rate),
                    funding_pct,
                    funding_display: format_pct(funding_pct),
                    status,
                    city,
                    bedrooms,
                    total_value_display: format_usd_compact(total_value),
                    is_rented: occ_bps.unwrap_or(0) > 0,
                }
            },
        )
        .collect()
}
