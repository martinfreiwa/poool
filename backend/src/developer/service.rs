/// Developer dashboard service layer — all DB queries and business logic.
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::models::*;
use crate::storage::service::rewrite_gcs_url;

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
    fetch_dashboard_stats_for_period(pool, developer_id, "all").await
}

/// Fetch all developer dashboard statistics for a given developer user and chart period.
pub async fn fetch_dashboard_stats_for_period(
    pool: &PgPool,
    developer_id: Uuid,
    chart_period: &str,
) -> DeveloperDashboardStats {
    // ── 1. Total Assets (only approved/live) ─────────────────────
    let total_assets: i64 =
        sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM assets a INNER JOIN developer_projects dp ON dp.asset_id = a.id WHERE a.developer_user_id = $1 AND dp.status IN ('approved', 'live') AND a.deleted_at IS NULL"
        )
            .bind(developer_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    // ── 2. Total Sales (sum of investments for developer's approved/live assets) ──
    let total_sales_cents: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(i.purchase_value_cents), 0)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
          AND i.status != 'exited'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 2.5 Total Funding Target / Remaining (approved/live assets) ──
    let total_funding_target_cents: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(a.total_value_cents), 0)::bigint
        FROM assets a
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let amount_remaining_cents = total_funding_target_cents.saturating_sub(total_sales_cents);
    let avg_funding_progress = if total_funding_target_cents > 0 {
        (total_sales_cents as f64 / total_funding_target_cents as f64) * 100.0
    } else {
        0.0
    };

    // ── 3. Total Investors (distinct users who invested in developer's approved/live assets) ──
    let total_investors: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT i.user_id)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
          AND i.status != 'exited'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 4. New Investors (invested in the last 30 days, approved/live only) ─────────────
    let new_investors: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT i.user_id)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
          AND i.purchased_at >= NOW() - INTERVAL '30 days'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 5. Total Views (from asset_views, approved/live assets only) ──
    let total_views: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(view_count), 0)::bigint
        FROM (
            SELECT COUNT(*)::bigint AS view_count
            FROM asset_views av
            JOIN assets a ON a.id = av.asset_id
            INNER JOIN developer_projects dp ON dp.asset_id = a.id
            WHERE a.developer_user_id = $1
              AND dp.status IN ('approved', 'live')
              AND a.deleted_at IS NULL
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

    // ── 6.5 Intent metrics (current cart and started order records) ──
    let add_to_cart_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM cart_items ci
        JOIN assets a ON a.id = ci.asset_id
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let checkout_starts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT o.id)::bigint
        FROM orders o
        JOIN order_items oi ON oi.order_id = o.id
        JOIN assets a ON a.id = oi.asset_id
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // There is currently no investor saved-property table. Keep this explicit
    // so the API contract is ready when saved properties are implemented.
    let saved_properties: i64 = 0;

    // ── 7. Sold Out Ratio (approved/live assets only) ───────────────
    let funded_assets: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM assets a
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
          AND a.tokens_available = 0
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

    // ── 8. Avg. Investment Amount (approved/live assets only) ───────
    let avg_investment_cents: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(AVG(i.purchase_value_cents), 0)::bigint
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
          AND i.status != 'exited'
        "#,
    )
    .bind(developer_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // ── 8.5 Period-Over-Period calculations (approved/live only) ──────
    let pop_row = sqlx::query!(
        r#"
        WITH current_period AS (
            SELECT 
                COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS sales,
                COUNT(DISTINCT i.user_id)::bigint AS investors,
                COALESCE(AVG(i.purchase_value_cents), 0)::bigint AS avg_investment
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            INNER JOIN developer_projects dp ON dp.asset_id = a.id
            WHERE a.developer_user_id = $1 AND i.status != 'exited'
              AND dp.status IN ('approved', 'live')
              AND a.deleted_at IS NULL
              AND i.purchased_at >= NOW() - INTERVAL '30 days'
        ),
        prev_period AS (
            SELECT 
                COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS sales,
                COUNT(DISTINCT i.user_id)::bigint AS investors,
                COALESCE(AVG(i.purchase_value_cents), 0)::bigint AS avg_investment
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            INNER JOIN developer_projects dp ON dp.asset_id = a.id
            WHERE a.developer_user_id = $1 AND i.status != 'exited'
              AND dp.status IN ('approved', 'live')
              AND a.deleted_at IS NULL
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

    // ── Build metric cards ───────────────────────────────────────────
    let metrics = vec![
        make_metric("Total Assets", total_assets.to_string(), 0.0),
        make_metric(
            "Funding Target",
            format_usd_compact(total_funding_target_cents),
            0.0,
        ),
        make_metric(
            "Amount Raised",
            format_usd_compact(total_sales_cents),
            sales_pct,
        ),
        make_metric(
            "Amount Remaining",
            format_usd_compact(amount_remaining_cents),
            0.0,
        ),
        make_metric("Total Investors", total_investors.to_string(), inv_pct),
        make_metric("Total Views", format!("{}", total_views), 0.0),
        make_metric("Checkout Starts", checkout_starts.to_string(), 0.0),
        make_metric("Add to Cart", add_to_cart_count.to_string(), 0.0),
        make_metric("Saved Properties", saved_properties.to_string(), 0.0),
        make_metric("New Investors", new_investors.to_string(), inv_pct),
        make_metric("Avg. Conversion Rate", format_pct(avg_conversion_rate), 0.0),
        make_metric("Sold Out Ratio", format_pct(sold_out_ratio), 0.0),
        make_metric(
            "Avg. Funding Progress",
            format_pct(avg_funding_progress),
            0.0,
        ),
        make_metric(
            "Avg. Investment Amount",
            format_usd_compact(avg_investment_cents),
            avg_pct,
        ),
    ];

    // ── Top Assets table ──────────────────────────────────────────────
    let top_assets = fetch_top_assets(pool, developer_id).await;
    let attention_assets = fetch_attention_assets(pool, developer_id).await;

    // ── Chart percentage (all-time sales change vs prior period) ────────
    let chart_data = fetch_sales_chart_data(pool, developer_id, chart_period).await;

    DeveloperDashboardStats {
        total_assets,
        total_funding_target_cents,
        total_funding_target_display: format_usd_compact(total_funding_target_cents),
        total_sales_cents,
        total_sales_display: format_usd_compact(total_sales_cents),
        amount_remaining_cents,
        amount_remaining_display: format_usd_compact(amount_remaining_cents),
        total_investors,
        total_views,
        checkout_starts,
        add_to_cart_count,
        saved_properties,
        new_investors,
        avg_conversion_rate,
        sold_out_ratio,
        avg_investment_cents,
        avg_investment_display: format_usd_compact(avg_investment_cents),
        metrics,
        top_assets,
        attention_assets,
        chart_percentage_display: chart_data.percentage_display,
        chart_trend: chart_data.trend,
        chart_period_label: chart_data.period_label,
        chart_y_axis_labels: chart_data.y_axis_labels,
        chart_x_axis_labels: chart_data.x_axis_labels,
        chart_line_path: chart_data.line_path,
        chart_area_path: chart_data.area_path,
        chart_has_data: chart_data.has_data,
        chart_current_display: chart_data.current_display,
        chart_subtitle: chart_data.subtitle,
        chart_end_x: chart_data.end_x,
        chart_end_y: chart_data.end_y,
    }
}

struct SalesChartData {
    period_label: String,
    percentage_display: String,
    trend: String,
    y_axis_labels: Vec<String>,
    x_axis_labels: Vec<String>,
    line_path: String,
    area_path: String,
    has_data: bool,
    current_display: String,
    subtitle: String,
    end_x: f64,
    end_y: f64,
}

struct SalesChartPoint {
    label: String,
    value_cents: i64,
}

async fn fetch_sales_chart_data(pool: &PgPool, developer_id: Uuid, period: &str) -> SalesChartData {
    let period = normalize_chart_period(period);
    let (period_label, start_sql, end_sql, step_sql, bucket_sql, label_format) = match period {
        "24h" => (
            "24 hours",
            "date_trunc('hour', NOW()) - INTERVAL '23 hours'",
            "date_trunc('hour', NOW())",
            "INTERVAL '1 hour'",
            "date_trunc('hour', i.purchased_at)",
            "HH24:00",
        ),
        "7d" => (
            "7 days",
            "date_trunc('day', NOW()) - INTERVAL '6 days'",
            "date_trunc('day', NOW())",
            "INTERVAL '1 day'",
            "date_trunc('day', i.purchased_at)",
            "Dy",
        ),
        "30d" => (
            "30 days",
            "date_trunc('day', NOW()) - INTERVAL '29 days'",
            "date_trunc('day', NOW())",
            "INTERVAL '1 day'",
            "date_trunc('day', i.purchased_at)",
            "DD Mon",
        ),
        "1y" => (
            "1 year",
            "date_trunc('month', NOW()) - INTERVAL '11 months'",
            "date_trunc('month', NOW())",
            "INTERVAL '1 month'",
            "date_trunc('month', i.purchased_at)",
            "Mon",
        ),
        _ => (
            "All time",
            r#"
            COALESCE((
                SELECT date_trunc('month', MIN(i.purchased_at))
                FROM investments i
                JOIN assets a ON a.id = i.asset_id
                INNER JOIN developer_projects dp ON dp.asset_id = a.id
                WHERE a.developer_user_id = $1
                  AND dp.status IN ('approved', 'live')
                  AND a.deleted_at IS NULL
                  AND i.status != 'exited'
            ), date_trunc('month', NOW()))
            "#,
            "date_trunc('month', NOW())",
            "INTERVAL '1 month'",
            "date_trunc('month', i.purchased_at)",
            "Mon",
        ),
    };

    let query = format!(
        r#"
        WITH bounds AS (
            SELECT {start_sql} AS start_at,
                   {end_sql} AS end_at
        ),
        series AS (
            SELECT generate_series(
                (SELECT start_at FROM bounds),
                (SELECT end_at FROM bounds),
                {step_sql}
            ) AS bucket
        ),
        bucket_sales AS (
            SELECT {bucket_sql} AS bucket,
                   COALESCE(SUM(i.purchase_value_cents), 0)::bigint AS sales_cents
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            INNER JOIN developer_projects dp ON dp.asset_id = a.id
            WHERE a.developer_user_id = $1
              AND dp.status IN ('approved', 'live')
              AND a.deleted_at IS NULL
              AND i.status != 'exited'
              AND i.purchased_at >= (SELECT start_at FROM bounds)
              AND i.purchased_at < NOW() + {step_sql}
            GROUP BY 1
        )
        SELECT TO_CHAR(s.bucket, '{label_format}') AS label,
               (SUM(COALESCE(bs.sales_cents, 0)) OVER (ORDER BY s.bucket))::bigint AS value_cents
        FROM series s
        LEFT JOIN bucket_sales bs ON bs.bucket = s.bucket
        ORDER BY s.bucket
        "#
    );

    let points = sqlx::query(&query)
        .bind(developer_id)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| SalesChartPoint {
                    label: row.try_get::<String, _>("label").unwrap_or_default(),
                    value_cents: row.try_get::<i64, _>("value_cents").unwrap_or(0),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    build_sales_chart_data(period_label, points)
}

fn normalize_chart_period(period: &str) -> &str {
    match period {
        "1y" | "30d" | "7d" | "24h" => period,
        _ => "all",
    }
}

fn build_sales_chart_data(period_label: &str, points: Vec<SalesChartPoint>) -> SalesChartData {
    // Hide the headline percentage for "All time" — comparing the first bucket
    // of a cumulative series to the last is mathematically tautological (always
    // a huge positive number) and not a useful signal.
    let show_percentage = period_label != "All time";
    let points = if points.is_empty() {
        vec![
            SalesChartPoint {
                label: String::new(),
                value_cents: 0,
            },
            SalesChartPoint {
                label: String::new(),
                value_cents: 0,
            },
        ]
    } else {
        points
    };

    let max_value = points
        .iter()
        .map(|point| point.value_cents)
        .max()
        .unwrap_or(0);
    let axis_max = nice_axis_max_cents(max_value);
    let has_data = max_value > 0;
    let y_axis_labels = (0..=5)
        .rev()
        .map(|step| format_chart_money_label(axis_max * step / 5))
        .collect::<Vec<_>>();
    let x_axis_labels = compact_chart_labels(
        points
            .iter()
            .map(|point| point.label.clone())
            .collect::<Vec<_>>(),
    );
    let line_path = chart_line_path(&points, axis_max);
    let area_path = format!("{line_path} L 1002 240 L 0 240 Z");
    let first = points.first().map(|point| point.value_cents).unwrap_or(0);
    let last = points.last().map(|point| point.value_cents).unwrap_or(0);
    // The series is a windowed cumulative sum, so "first" is whatever sales
    // happened to fall in the opening bucket. If that baseline is too small
    // relative to the headline number, the % becomes uninformative noise
    // (always ~+9999%). Require the baseline to be ≥50% of the final value
    // before a meaningful percent can be reported; otherwise the percentage
    // is dominated by ramp-from-zero rather than real growth.
    let meaningful_baseline = first as f64 >= (last as f64) * 0.50;
    let change_pct = if first > 0 && meaningful_baseline {
        ((last - first) as f64 / first as f64) * 100.0
    } else {
        0.0
    };
    let show_percentage = show_percentage && (first > 0 && meaningful_baseline);

    let percentage_display = if !show_percentage {
        String::new()
    } else if change_pct > 0.01 {
        format!("+{}", format_pct(change_pct))
    } else if change_pct < -0.01 {
        format!("-{}", format_pct(change_pct.abs()))
    } else {
        "0%".to_string()
    };

    let (end_x, end_y) = chart_end_point(&points, axis_max);
    let current_display = format_chart_money_label(last);
    let subtitle = match period_label {
        "All time" => "Cumulative sales since launch".to_string(),
        other => format!("Cumulative sales — last {other}"),
    };

    SalesChartData {
        period_label: period_label.to_string(),
        percentage_display,
        trend: trend(change_pct),
        y_axis_labels,
        x_axis_labels,
        line_path,
        area_path,
        has_data,
        current_display,
        subtitle,
        end_x,
        end_y,
    }
}

fn chart_end_point(points: &[SalesChartPoint], axis_max_cents: i64) -> (f64, f64) {
    let width = 1002.0;
    let plot_height = 210.0;
    let top_padding = 10.0;
    let axis_max = axis_max_cents.max(1) as f64;
    let last = points.last().map(|p| p.value_cents).unwrap_or(0);
    let x = width;
    let y = top_padding + (1.0 - (last.max(0) as f64 / axis_max)) * plot_height;
    (x, y)
}

fn chart_line_path(points: &[SalesChartPoint], axis_max_cents: i64) -> String {
    let width = 1002.0;
    let plot_height = 210.0;
    let top_padding = 10.0;
    let axis_max = axis_max_cents.max(1) as f64;
    let denominator = points.len().saturating_sub(1).max(1) as f64;

    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let x = (index as f64 / denominator) * width;
            let y =
                top_padding + (1.0 - (point.value_cents.max(0) as f64 / axis_max)) * plot_height;
            let command = if index == 0 { "M" } else { "L" };
            format!("{command}{x:.2} {y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn nice_axis_max_cents(max_cents: i64) -> i64 {
    if max_cents <= 0 {
        return 0;
    }

    let dollars = ((max_cents + 99) / 100).max(1) as f64;
    let magnitude = 10_f64.powf(dollars.log10().floor());
    let normalized = dollars / magnitude;
    let nice_normalized = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };

    (nice_normalized * magnitude * 100.0).round() as i64
}

fn format_chart_money_label(cents: i64) -> String {
    if cents <= 0 {
        return "$0".to_string();
    }

    let dollars = cents as f64 / 100.0;
    if dollars >= 1_000_000.0 {
        format!("${:.1}M", dollars / 1_000_000.0)
    } else if dollars >= 1_000.0 {
        format!("${:.0}k", dollars / 1_000.0)
    } else {
        format!("${:.0}", dollars)
    }
}

fn compact_chart_labels(labels: Vec<String>) -> Vec<String> {
    if labels.len() <= 12 {
        return labels;
    }

    let last_index = labels.len() - 1;
    let step = (last_index as f64 / 11.0).ceil() as usize;
    labels
        .into_iter()
        .enumerate()
        .filter_map(|(index, label)| {
            if index == 0 || index == last_index || index % step == 0 {
                Some(label)
            } else {
                None
            }
        })
        .collect()
}

#[derive(sqlx::FromRow)]
struct AssetRow {
    id: Uuid,
    title: String,
    cover_image_url: Option<String>,
    total_sales_cents: i64,
    investor_count: i64,
    views: i64,
    add_to_cart_count: i64,
    checkout_starts: i64,
    tokens_total: i32,
    tokens_available: i32,
    funding_status: String,
    city: Option<String>,
    bedrooms: Option<i32>,
    bathrooms: Option<i32>,
    size_sqm: Option<String>,
    total_value_cents: i64,
    occupancy_rate_bps: Option<i32>,
    country: Option<String>,
    lease_type: Option<String>,
    lease_term_years: Option<i32>,
    capital_appreciation_bps: Option<i32>,
    annual_yield_bps: Option<i32>,
}

/// Fetch top assets for the developer, with sales/views/conversion data.
async fn fetch_top_assets(pool: &PgPool, developer_id: Uuid) -> Vec<DeveloperTopAsset> {
    fetch_assets_for_dashboard(
        pool,
        developer_id,
        None,
        "ORDER BY total_sales_cents DESC, views DESC, a.created_at DESC LIMIT 5",
    )
    .await
}

/// Fetch assets with weak funding momentum or low capital raised.
async fn fetch_attention_assets(pool: &PgPool, developer_id: Uuid) -> Vec<DeveloperTopAsset> {
    fetch_assets_for_dashboard(
        pool,
        developer_id,
        None,
        "ORDER BY CASE WHEN a.tokens_total > 0 THEN ((a.tokens_total - a.tokens_available)::double precision / a.tokens_total::double precision) ELSE 0 END ASC, views DESC, add_to_cart_count DESC, a.created_at DESC LIMIT 5",
    )
    .await
}

/// Shared asset aggregation for dashboard tables.
async fn fetch_assets_for_dashboard(
    pool: &PgPool,
    developer_id: Uuid,
    period: Option<&str>,
    order_clause: &str,
) -> Vec<DeveloperTopAsset> {
    let investment_period_filter = period_filter("i.purchased_at", period);
    let view_period_filter = period_filter("av.viewed_at", period);
    let cart_period_filter = period_filter("ci.created_at", period);
    let checkout_period_filter = period_filter("COALESCE(o.completed_at, o.created_at)", period);

    // Fetch the developer's assets with their aggregated investment data
    let query = format!(
        r#"
        SELECT
            a.id,
            a.title,
            (SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY sort_order LIMIT 1) AS cover_image_url,
            COALESCE((
                SELECT SUM(i.purchase_value_cents)
                FROM investments i
                WHERE i.asset_id = a.id AND i.status != 'exited'
                {investment_period_filter}
            ), 0)::bigint AS total_sales_cents,
            COALESCE((
                SELECT COUNT(DISTINCT i.user_id)
                FROM investments i
                WHERE i.asset_id = a.id AND i.status != 'exited'
                {investment_period_filter}
            ), 0)::bigint AS investor_count,
            COALESCE((
                SELECT COUNT(*)
                FROM asset_views av
                WHERE av.asset_id = a.id
                {view_period_filter}
            ), 0)::bigint AS views,
            COALESCE((
                SELECT COUNT(*)
                FROM cart_items ci
                WHERE ci.asset_id = a.id
                {cart_period_filter}
            ), 0)::bigint AS add_to_cart_count,
            COALESCE((
                SELECT COUNT(DISTINCT oi.order_id)
                FROM order_items oi
                JOIN orders o ON o.id = oi.order_id
                WHERE oi.asset_id = a.id
                {checkout_period_filter}
            ), 0)::bigint AS checkout_starts,
            a.tokens_total,
            a.tokens_available,
            a.funding_status,
            a.location_city as city,
            a.bedrooms,
            a.bathrooms,
            CASE
                WHEN a.building_size_sqm IS NOT NULL THEN CONCAT(ROUND(a.building_size_sqm)::int, ' m²')
                WHEN a.land_size_sqm IS NOT NULL THEN CONCAT(ROUND(a.land_size_sqm)::int, ' m²')
                ELSE NULL
            END AS size_sqm,
            COALESCE(a.total_value_cents, 0) as total_value_cents,
            a.occupancy_rate_bps,
            a.location_country as country,
            a.lease_type,
            a.lease_term_years,
            a.capital_appreciation_bps,
            a.annual_yield_bps
        FROM assets a
        INNER JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND dp.status IN ('approved', 'live')
          AND a.deleted_at IS NULL
        {order_clause}
        "#,
    );

    let rows = sqlx::query_as::<_, AssetRow>(&query)
        .bind(developer_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    rows.into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let funding_pct = if row.tokens_total > 0 {
                ((row.tokens_total - row.tokens_available) as f64 / row.tokens_total as f64) * 100.0
            } else {
                0.0
            };

            let conversion_rate = if row.views > 0 {
                (row.investor_count as f64 / row.views as f64) * 100.0
            } else {
                0.0
            };
            let amount_remaining_cents =
                row.total_value_cents.saturating_sub(row.total_sales_cents);

            DeveloperTopAsset {
                index: idx + 1,
                id: row.id.to_string(),
                title: row.title,
                cover_image_url: rewrite_gcs_url(
                    &row.cover_image_url
                        .unwrap_or_else(|| "/static/images/seed/villa1.webp".to_string()),
                ),
                total_sales_display: format_usd_compact(row.total_sales_cents),
                total_sales_cents: row.total_sales_cents,
                sales_change_pct: 0.0,
                sales_trend: "neutral".to_string(),
                views: row.views,
                add_to_cart_count: row.add_to_cart_count,
                checkout_starts: row.checkout_starts,
                saved_count: 0,
                conversion_rate,
                conversion_display: format_pct(conversion_rate),
                funding_pct,
                funding_display: format_pct(funding_pct),
                status: row.funding_status,
                city: row.city,
                bedrooms: row.bedrooms,
                bathrooms: row.bathrooms,
                size_sqm: row.size_sqm,
                total_value_display: format_usd_compact(row.total_value_cents),
                total_value_cents: row.total_value_cents,
                amount_remaining_display: format_usd_compact(amount_remaining_cents),
                amount_remaining_cents,
                is_rented: row.occupancy_rate_bps.unwrap_or(0) > 0,
                country: row.country,
                lease_type: row.lease_type,
                lease_term_years: row.lease_term_years,
                capital_appreciation_bps: row.capital_appreciation_bps,
                annual_yield_bps: row.annual_yield_bps,
            }
        })
        .collect()
}

/// Fetch all assets for the developer, with sales/views/conversion data.
pub async fn fetch_all_assets(pool: &PgPool, developer_id: Uuid) -> Vec<DeveloperTopAsset> {
    fetch_assets_for_dashboard(
        pool,
        developer_id,
        None,
        "ORDER BY total_sales_cents DESC, views DESC, a.created_at DESC",
    )
    .await
}

/// Fetch all dashboard-table assets for a specific period.
pub async fn fetch_assets_for_period(
    pool: &PgPool,
    developer_id: Uuid,
    period: &str,
) -> Vec<DeveloperTopAsset> {
    fetch_assets_for_dashboard(
        pool,
        developer_id,
        Some(period),
        "ORDER BY total_sales_cents DESC, views DESC, a.created_at DESC",
    )
    .await
}

fn period_filter(column: &str, period: Option<&str>) -> String {
    match period {
        Some("30d") => format!("AND {column} >= NOW() - INTERVAL '30 days'"),
        Some("7d") => format!("AND {column} >= NOW() - INTERVAL '7 days'"),
        Some("24h") => format!("AND {column} >= NOW() - INTERVAL '24 hours'"),
        _ => String::new(),
    }
}
