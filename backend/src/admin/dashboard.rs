use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};
use sqlx::Row;

//  Admin API Endpoints

/// Query parameters for dashboard stats.
#[derive(serde::Deserialize)]
pub struct StatsQuery {
    /// Time range for the stats (e.g., 'today', '7d', '30d', '90d', '1y', 'all').
    pub range: Option<String>,
}

/// GET /api/admin/stats/overview - Dashboard KPI stats from real DB tables.
pub async fn api_admin_stats_overview(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<axum::response::Response, ApiError> {
    let range = params.range.unwrap_or_else(|| "30d".to_string());
    let interval = match range.as_str() {
        "today" => "24 hours",
        "7d" => "7 days",
        "30d" => "30 days",
        "90d" => "90 days",
        "1y" => "1 year",
        "all" => "100 years",
        _ => "30 days",
    };

    let db = &state.db;

    // Run all dashboard queries concurrently. Sequential await of 23 queries
    // dominated page load time; tokio::try_join! lets the pool dispatch them
    // in parallel against separate connections.
    // Conditional aggregation collapses three scans of `users` into one,
    // and likewise for `wallet_transactions`. Each pending-status table
    // combines its COUNT and oldest-age into a single scan.
    let f_users_counts = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT \
            COUNT(*)::bigint, \
            COUNT(*) FILTER (WHERE created_at > NOW() - $1::interval)::bigint, \
            COUNT(*) FILTER (WHERE created_at > NOW() - ($1::interval * 2) AND created_at <= NOW() - $1::interval)::bigint \
         FROM users",
    )
    .bind(interval)
    .fetch_one(db);
    let f_aum_cents = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(purchase_value_cents), 0)::bigint FROM investments WHERE status NOT IN ('exited', 'cancelled')"
    )
    .fetch_one(db);
    let f_deposits_agg = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        "SELECT \
            COALESCE(SUM(amount_cents) FILTER (WHERE created_at > NOW() - $1::interval), 0)::bigint, \
            COUNT(*) FILTER (WHERE created_at > NOW() - $1::interval)::bigint, \
            COALESCE(SUM(amount_cents) FILTER (WHERE created_at > NOW() - ($1::interval * 2) AND created_at <= NOW() - $1::interval), 0)::bigint, \
            COUNT(*) FILTER (WHERE created_at > NOW() - ($1::interval * 2) AND created_at <= NOW() - $1::interval)::bigint \
         FROM wallet_transactions \
         WHERE type = 'deposit' AND status = 'completed' AND created_at > NOW() - ($1::interval * 2)",
    )
    .bind(interval)
    .fetch_one(db);
    let f_assets_counts = sqlx::query_as::<_, (i64, i64)>(
        "SELECT \
            COUNT(*) FILTER (WHERE published = true)::bigint, \
            COUNT(*) FILTER (WHERE published = true AND tokens_available = 0)::bigint \
         FROM assets",
    )
    .fetch_one(db);
    let f_pending_deposits_agg = async {
        Ok::<(i64, Option<f64>), sqlx::Error>(
            sqlx::query_as::<_, (i64, Option<f64>)>(
                "SELECT COUNT(*)::bigint, EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::float8 FROM deposit_requests WHERE status = 'pending'"
            )
            .fetch_one(db)
            .await
            .unwrap_or((0, None)),
        )
    };
    let f_pending_kyc_agg = async {
        Ok::<(i64, Option<f64>), sqlx::Error>(
            sqlx::query_as::<_, (i64, Option<f64>)>(
                "SELECT COUNT(*)::bigint, EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::float8 FROM kyc_records WHERE status = 'pending'"
            )
            .fetch_one(db)
            .await
            .unwrap_or((0, None)),
        )
    };
    let f_open_tickets_agg = async {
        Ok::<(i64, Option<f64>), sqlx::Error>(
            sqlx::query_as::<_, (i64, Option<f64>)>(
                "SELECT COUNT(*)::bigint, EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::float8 FROM support_tickets WHERE status = 'open'"
            )
            .fetch_one(db)
            .await
            .unwrap_or((0, None)),
        )
    };
    let f_rewards_breakdown = async {
        Ok::<(i64, i64, i64, i64), sqlx::Error>(
            sqlx::query_as::<_, (i64, i64, i64, i64)>(
                "SELECT \
                    COALESCE(SUM(cashback), 0)::bigint, \
                    COALESCE(SUM(referrals), 0)::bigint, \
                    COALESCE(SUM(promotions), 0)::bigint, \
                    COUNT(*) FILTER (WHERE (cashback + referrals + promotions) > 0)::bigint \
                 FROM rewards_balances",
            )
            .fetch_one(db)
            .await
            .unwrap_or((0, 0, 0, 0)),
        )
    };
    let f_unread_notifications =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notifications WHERE is_read = false")
            .fetch_one(db);
    let f_activity_rows = sqlx::query_as::<_, (String, String, String, Option<String>, String)>(
        "SELECT id::text, action, entity_type, entity_id::text, created_at::text FROM audit_logs ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(db);
    let f_order_rows = sqlx::query_as::<_, (String, String, i64, String, String)>(
        "SELECT o.order_number, u.email, o.total_cents, o.status, o.created_at::text \
         FROM orders o JOIN users u ON u.id = o.user_id \
         ORDER BY o.created_at DESC LIMIT 5",
    )
    .fetch_all(db);
    let f_deposit_rows = sqlx::query_as::<_, (String, String, i64, String, String, String)>(
        "SELECT d.id::text, u.email, d.amount_cents, d.provider, d.status, d.created_at::text \
         FROM deposit_requests d JOIN users u ON u.id = d.user_id \
         WHERE d.status = 'pending' \
         ORDER BY d.created_at DESC LIMIT 10",
    )
    .fetch_all(db);
    let f_user_trend_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT TO_CHAR(series.day, 'YYYY-MM-DD') as day, COUNT(u.id) as count \
         FROM generate_series(NOW() - INTERVAL '6 days', NOW(), '1 day') series(day) \
         LEFT JOIN users u ON DATE_TRUNC('day', u.created_at) = DATE_TRUNC('day', series.day) \
         GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(db);
    let f_deposit_trend_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT TO_CHAR(series.day, 'YYYY-MM-DD') as day, COALESCE(SUM(t.amount_cents), 0)::bigint as count \
         FROM generate_series(NOW() - INTERVAL '6 days', NOW(), '1 day') series(day) \
         LEFT JOIN wallet_transactions t ON DATE_TRUNC('day', t.created_at) = DATE_TRUNC('day', series.day) \
         AND t.type = 'deposit' AND t.status = 'completed' \
         GROUP BY 1 ORDER BY 1"
    )
    .fetch_all(db);

    let (
        users_counts,
        aum_cents,
        deposits_agg,
        assets_counts,
        pending_deposits_agg,
        pending_kyc_agg,
        open_tickets_agg,
        rewards_breakdown,
        unread_notifications,
        activity_rows,
        order_rows,
        deposit_rows,
        user_trend_rows,
        deposit_trend_rows,
    ) = tokio::try_join!(
        f_users_counts,
        f_aum_cents,
        f_deposits_agg,
        f_assets_counts,
        f_pending_deposits_agg,
        f_pending_kyc_agg,
        f_open_tickets_agg,
        f_rewards_breakdown,
        f_unread_notifications,
        f_activity_rows,
        f_order_rows,
        f_deposit_rows,
        f_user_trend_rows,
        f_deposit_trend_rows,
    )?;

    let (total_users, new_users_range, new_users_prev) = users_counts;
    let (deposits_range_cents, deposits_range_count, deposits_prev_cents, deposits_prev_count) =
        deposits_agg;
    let (live_assets, funded_assets) = assets_counts;
    let (pending_deposits, oldest_pending_deposit_secs) = pending_deposits_agg;
    let (pending_kyc, oldest_pending_kyc_secs) = pending_kyc_agg;
    let (open_tickets, oldest_open_ticket_secs) = open_tickets_agg;
    let rewards_liability_cents = rewards_breakdown.0 + rewards_breakdown.1 + rewards_breakdown.2;

    let activity_json: Vec<serde_json::Value> = activity_rows
        .iter()
        .map(|(id, action, entity_type, entity_id, created_at)| {
            serde_json::json!({
                "id": id,
                "action": action,
                "entity_type": entity_type,
                "entity_id": entity_id,
                "created_at": created_at
            })
        })
        .collect();

    let orders_json: Vec<serde_json::Value> = order_rows
        .iter()
        .map(|(num, email, total, status, created_at)| {
            serde_json::json!({
                "order_number": num,
                "user_email": email,
                "total_cents": total,
                "status": status,
                "created_at": created_at
            })
        })
        .collect();

    let deposits_json: Vec<serde_json::Value> = deposit_rows
        .iter()
        .map(|(id, email, amount, provider, status, created_at)| {
            serde_json::json!({
                "id": id,
                "user_email": email,
                "amount_cents": amount,
                "provider": provider,
                "status": status,
                "created_at": created_at
            })
        })
        .collect();

    let user_trend: Vec<i64> = user_trend_rows
        .into_iter()
        .map(|(_, count)| count)
        .collect();

    let deposit_trend: Vec<i64> = deposit_trend_rows
        .into_iter()
        .map(|(_, count)| count)
        .collect();

    let response = serde_json::json!({
        "total_users": total_users,
        "new_users_range": new_users_range,
        "user_trend": user_trend,
        "aum_cents": aum_cents,
        "deposits_range_cents": deposits_range_cents,
        "deposits_range_count": deposits_range_count,
        "deposit_trend": deposit_trend,
        "pending_kyc": pending_kyc,
        "live_assets": live_assets,
        "funded_assets": funded_assets,
        "pending_deposits": pending_deposits,
        "open_tickets": open_tickets,
        "rewards_liability_cents": rewards_liability_cents,
        "rewards_liability_breakdown": {
            "cashback_cents": rewards_breakdown.0,
            "referrals_cents": rewards_breakdown.1,
            "promotions_cents": rewards_breakdown.2,
            "users_with_balance": rewards_breakdown.3,
        },
        "unread_notifications": unread_notifications,
        "new_users_prev": new_users_prev,
        "deposits_prev_cents": deposits_prev_cents,
        "deposits_prev_count": deposits_prev_count,
        "oldest_pending_deposit_secs": oldest_pending_deposit_secs,
        "oldest_pending_kyc_secs": oldest_pending_kyc_secs,
        "oldest_open_ticket_secs": oldest_open_ticket_secs,
        "recent_activity": activity_json,
        "recent_orders": orders_json,
        "pending_deposits_list": deposits_json,
        "range_label": match range.as_str() {
            "today" => "today",
            "7d" => "last 7 days",
            "30d" => "last 30 days",
            "90d" => "last 90 days",
            "1y" => "this year",
            "all" => "all time",
            _ => "last 30 days"
        }
    });

    Ok(Json(response).into_response())
}

/// Query parameters for the minimal admin global search endpoint.
#[derive(serde::Deserialize)]
pub struct AdminSearchQuery {
    /// Search term entered in the admin global search box.
    pub q: Option<String>,
}

/// GET /api/admin/search?q=... - Minimal capped search for the admin topbar.
pub async fn api_admin_search(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<AdminSearchQuery>,
) -> Result<axum::response::Response, ApiError> {
    let raw_query = params.q.unwrap_or_default();
    let trimmed = raw_query.trim();
    if trimmed.len() < 2 {
        return Ok(Json(serde_json::json!({ "results": [] })).into_response());
    }

    let pattern = format!("%{}%", trimmed.replace(['%', '_'], ""));
    let mut results = Vec::new();

    let user_rows = sqlx::query(
        r#"
        SELECT u.id::text, u.email, COALESCE(up.display_name, ''), COALESCE(up.first_name, ''),
               COALESCE(up.last_name, ''),
               COALESCE((SELECT kr.status FROM kyc_records kr WHERE kr.user_id = u.id ORDER BY kr.created_at DESC LIMIT 1), 'unknown') AS kyc_status
        FROM users u
        LEFT JOIN user_profiles up ON up.user_id = u.id
        WHERE u.email ILIKE $1
           OR up.display_name ILIKE $1
           OR up.first_name ILIKE $1
           OR up.last_name ILIKE $1
           OR u.id::text ILIKE $1
        ORDER BY u.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    for row in user_rows {
        let email: String = row.get("email");
        let display_name: String = row.get(2);
        let first_name: String = row.get(3);
        let last_name: String = row.get(4);
        let name = if !display_name.is_empty() {
            display_name
        } else {
            format!("{} {}", first_name, last_name).trim().to_string()
        };
        results.push(serde_json::json!({
            "type": "user",
            "title": if name.is_empty() { email.clone() } else { name },
            "subtitle": email,
            "url": format!("/admin/user-details.html?id={}", row.get::<String, _>(0)),
            "badge": row.get::<String, _>("kyc_status"),
        }));
    }

    let asset_rows = sqlx::query(
        r#"
        SELECT id::text, title, asset_type, funding_status
        FROM assets
        WHERE title ILIKE $1 OR slug ILIKE $1 OR id::text ILIKE $1 OR asset_type ILIKE $1
        ORDER BY featured DESC, created_at DESC
        LIMIT 5
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    for row in asset_rows {
        let asset_type: String = row.get("asset_type");
        let funding_status: String = row.get("funding_status");
        results.push(serde_json::json!({
            "type": "asset",
            "title": row.get::<String, _>("title"),
            "subtitle": format!("{} - {}", asset_type, funding_status),
            "url": format!("/admin/asset-details.html?id={}", row.get::<String, _>("id")),
            "badge": funding_status,
        }));
    }

    let order_rows = sqlx::query(
        r#"
        SELECT o.id::text, o.order_number, COALESCE(u.email, '') AS user_email, o.total_cents, o.status
        FROM orders o
        JOIN users u ON u.id = o.user_id
        WHERE o.order_number ILIKE $1 OR o.id::text ILIKE $1 OR u.email ILIKE $1
        ORDER BY o.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    for row in order_rows {
        results.push(serde_json::json!({
            "type": "order",
            "title": row.get::<String, _>("order_number"),
            "subtitle": format!("{} - {}", row.get::<String, _>("user_email"), format_usd_cents(row.get::<i64, _>("total_cents"))),
            "url": format!("/admin/orders.html?id={}", row.get::<String, _>("id")),
            "badge": row.get::<String, _>("status"),
        }));
    }

    let deposit_rows = sqlx::query(
        r#"
        SELECT d.id::text, COALESCE(d.provider_reference, '') AS provider_reference,
               COALESCE(u.email, '') AS user_email, d.amount_cents, d.currency, d.status
        FROM deposit_requests d
        JOIN users u ON u.id = d.user_id
        WHERE d.id::text ILIKE $1 OR d.provider_reference ILIKE $1 OR u.email ILIKE $1
        ORDER BY d.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    for row in deposit_rows {
        let reference: String = row.get("provider_reference");
        let id: String = row.get("id");
        results.push(serde_json::json!({
            "type": "deposit",
            "title": if reference.is_empty() { format!("Deposit {}", &id[..8.min(id.len())]) } else { format!("Deposit {}", reference) },
            "subtitle": format!("{} - {} {}", row.get::<String, _>("user_email"), row.get::<String, _>("currency"), format_major_minor(row.get::<i64, _>("amount_cents"))),
            "url": "/admin/deposits.html",
            "badge": row.get::<String, _>("status"),
        }));
    }

    let affiliate_rows = sqlx::query(
        r#"
        SELECT a.user_id::text AS id, COALESCE(u.email, '') AS email, a.status,
               COALESCE(a.referral_code, '') AS referral_code,
               COALESCE(a.company_name, '') AS company_name,
               COALESCE(a.main_url, '') AS main_url
        FROM affiliates a
        JOIN users u ON u.id = a.user_id
        WHERE u.email ILIKE $1
           OR a.referral_code ILIKE $1
           OR a.company_name ILIKE $1
           OR a.main_url ILIKE $1
           OR a.user_id::text ILIKE $1
        ORDER BY a.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    for row in affiliate_rows {
        let status: String = row.get("status");
        let email: String = row.get("email");
        let referral_code: String = row.get("referral_code");
        let company_name: String = row.get("company_name");
        let url = if status == "pending_approval" {
            "/admin/affiliate-applications.html".to_string()
        } else {
            format!("/admin/affiliate-finance.html?affiliate_id={}", row.get::<String, _>("id"))
        };
        let subtitle = if !company_name.is_empty() {
            format!("{} · {}", email, company_name)
        } else if !referral_code.is_empty() {
            format!("{} · {}", email, referral_code)
        } else {
            email.clone()
        };
        results.push(serde_json::json!({
            "type": "affiliate",
            "title": if !referral_code.is_empty() { referral_code } else { email },
            "subtitle": subtitle,
            "url": url,
            "badge": status,
        }));
    }

    Ok(Json(serde_json::json!({ "results": results })).into_response())
}

fn format_usd_cents(cents: i64) -> String {
    format!("${}", format_major_minor(cents))
}

fn format_major_minor(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.saturating_abs();
    format!("{}{}.{:02}", sign, abs / 100, abs % 100)
}
