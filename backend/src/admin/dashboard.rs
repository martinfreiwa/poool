use super::extractors::AdminUser;
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};

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
) -> axum::response::Response {
    // Verify the user has admin privileges
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

    // Fetch real stats from database - use unwrap_or(0) for tables that might not exist yet
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let new_users_range: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE created_at > NOW() - $1::interval")
            .bind(interval)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let aum_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(purchase_value_cents), 0)::bigint FROM investments WHERE status NOT IN ('exited', 'cancelled')"
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let deposits_range_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM wallet_transactions WHERE type = 'deposit' AND status = 'completed' AND created_at > NOW() - $1::interval"
    )
    .bind(interval)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let deposits_range_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM wallet_transactions WHERE type = 'deposit' AND status = 'completed' AND created_at > NOW() - $1::interval"
    )
    .bind(interval)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let pending_kyc: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM kyc_records WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let live_assets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE published = true")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let funded_assets: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets WHERE published = true AND tokens_available = 0",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let pending_deposits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deposit_requests WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    // support_tickets table may not exist yet  gracefully handle
    let open_tickets: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tickets WHERE status = 'open'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let rewards_liability_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cashback + referrals + promotions), 0)::bigint FROM rewards_balances",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    // Fetch recent activity from audit logs
    let activity_rows = sqlx::query_as::<_, (String, String, String, Option<String>, String)>(
        "SELECT id::text, action, entity_type, entity_id::text, created_at::text FROM audit_logs ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

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

    // Fetch recent orders
    let order_rows = sqlx::query_as::<_, (String, String, i64, String, String)>(
        "SELECT o.order_number, u.email, o.total_cents, o.status, o.created_at::text 
         FROM orders o JOIN users u ON u.id = o.user_id 
         ORDER BY o.created_at DESC LIMIT 5",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

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

    // Fetch pending deposits
    let deposit_rows = sqlx::query_as::<_, (String, i64, String, String, String)>(
        "SELECT u.email, d.amount_cents, d.provider, d.status, d.created_at::text 
         FROM deposit_requests d JOIN users u ON u.id = d.user_id 
         WHERE d.status = 'pending' 
         ORDER BY d.created_at DESC LIMIT 10",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let deposits_json: Vec<serde_json::Value> = deposit_rows
        .iter()
        .map(|(email, amount, provider, status, created_at)| {
            serde_json::json!({
                "user_email": email,
                "amount_cents": amount,
                "provider": provider,
                "status": status,
                "created_at": created_at
            })
        })
        .collect();

    // Fetch 7-day user trend for sparklines
    let user_trend_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT TO_CHAR(series.day, 'YYYY-MM-DD') as day, COUNT(u.id) as count \
         FROM generate_series(NOW() - INTERVAL '6 days', NOW(), '1 day') series(day) \
         LEFT JOIN users u ON DATE_TRUNC('day', u.created_at) = DATE_TRUNC('day', series.day) \
         GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let user_trend: Vec<i64> = user_trend_rows
        .into_iter()
        .map(|(_, count)| count)
        .collect();

    // Fetch 7-day deposit trend for sparklines
    let deposit_trend_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT TO_CHAR(series.day, 'YYYY-MM-DD') as day, COALESCE(SUM(t.amount_cents), 0)::bigint as count \
         FROM generate_series(NOW() - INTERVAL '6 days', NOW(), '1 day') series(day) \
         LEFT JOIN wallet_transactions t ON DATE_TRUNC('day', t.created_at) = DATE_TRUNC('day', series.day) \
         AND t.type = 'deposit' AND t.status = 'completed' \
         GROUP BY 1 ORDER BY 1"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let deposit_trend: Vec<i64> = deposit_trend_rows
        .into_iter()
        .map(|(_, count)| count)
        .collect();

    Json(serde_json::json!({
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
    }))
    .into_response()
}
