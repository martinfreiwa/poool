use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};
use std::collections::HashMap;

/// GET /api/admin/search?q=<query>&limit=<n>
///
/// Unified server-side search across users, assets, orders, deposits, and support tickets.
/// Performs indexed ILIKE queries against the database and returns pre-filtered, categorised results.
/// This replaces the old client-side approach that downloaded ALL data from 4 endpoints per keystroke.
pub async fn api_admin_search(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    let q = params.get("q").map(|s| s.trim()).unwrap_or("");
    if q.len() < 2 {
        return Ok(Json(serde_json::json!({ "results": [] })).into_response());
    }

    let limit: i64 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(15)
        .min(50);

    // Prepare ILIKE pattern
    // Escape any SQL LIKE wildcards in the user query, then wrap in %...%
    let pattern = format!(
        "%{}%",
        q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
    );

    let mut results: Vec<serde_json::Value> = Vec::new();

    // ── 1. Search Users ─────────────────────────────────────────
    let user_rows = sqlx::query_as::<
        _,
        (
            String,         // id
            String,         // email
            Option<String>, // first_name
            Option<String>, // last_name
            String,         // status
            Option<String>, // kyc_status
        ),
    >(
        r#"
        SELECT
            u.id::text,
            u.email,
            p.first_name,
            p.last_name,
            u.status,
            (
                SELECT kr.status FROM kyc_records kr
                WHERE kr.user_id = u.id ORDER BY kr.created_at DESC LIMIT 1
            ) AS kyc_status
        FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        WHERE u.email ILIKE $1
           OR p.first_name ILIKE $1
           OR p.last_name ILIKE $1
           OR u.id::text ILIKE $1
           OR CONCAT(p.first_name, ' ', p.last_name) ILIKE $1
        ORDER BY u.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in &user_rows {
        let display = match (&row.2, &row.3) {
            (Some(f), Some(l)) => format!("{} {}", f, l),
            (Some(f), None) => f.clone(),
            _ => row.1.clone(),
        };
        results.push(serde_json::json!({
            "type": "user",
            "icon": "👤",
            "title": display,
            "subtitle": row.1, // email
            "url": format!("/admin/user-details.html?id={}", row.0),
            "badge": row.5.as_deref().unwrap_or("unknown"),
        }));
    }

    // ── 2. Search Assets ────────────────────────────────────────
    let asset_rows = sqlx::query_as::<
        _,
        (
            String,         // id
            Option<String>, // title
            Option<String>, // slug
            Option<String>, // asset_type
            Option<String>, // funding_status
        ),
    >(
        r#"
        SELECT
            a.id::text,
            a.title,
            a.slug,
            a.asset_type,
            a.funding_status
        FROM assets a
        WHERE a.title ILIKE $1
           OR a.slug ILIKE $1
           OR a.id::text ILIKE $1
           OR a.asset_type ILIKE $1
        ORDER BY a.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in &asset_rows {
        results.push(serde_json::json!({
            "type": "asset",
            "icon": "🏠",
            "title": row.1.as_deref().unwrap_or("Untitled"),
            "subtitle": format!("{} · {}", row.3.as_deref().unwrap_or("asset"), row.4.as_deref().unwrap_or("")),
            "url": format!("/admin/asset-details.html?id={}", row.0),
            "badge": row.4.as_deref().unwrap_or(""),
        }));
    }

    // ── 3. Search Orders ────────────────────────────────────────
    let order_rows = sqlx::query_as::<
        _,
        (
            String,         // id
            Option<String>, // order_number
            Option<String>, // user_email
            Option<i64>,    // total_cents
            String,         // status
        ),
    >(
        r#"
        SELECT
            o.id::text,
            o.order_number,
            u.email AS user_email,
            o.total_cents,
            o.status
        FROM orders o
        LEFT JOIN users u ON o.user_id = u.id
        WHERE o.order_number ILIKE $1
           OR u.email ILIKE $1
           OR o.id::text ILIKE $1
        ORDER BY o.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in &order_rows {
        let title = row.1.clone().unwrap_or_else(|| format!("Order {}", &row.0[..8.min(row.0.len())]));
        let cents = row.3.unwrap_or(0);
        let formatted = format!("${:.2}", (cents.abs() as f64) / 100.0);
        results.push(serde_json::json!({
            "type": "order",
            "icon": "📋",
            "title": title,
            "subtitle": format!("{} · {}", row.2.as_deref().unwrap_or(""), formatted),
            "url": format!("/admin/orders.html?id={}", row.0),
            "badge": row.4,
        }));
    }

    // ── 4. Search Deposits ──────────────────────────────────────
    let deposit_rows = sqlx::query_as::<
        _,
        (
            String,         // id
            Option<String>, // external_ref_id
            Option<String>, // provider_reference
            Option<String>, // user_email
            Option<i64>,    // amount_cents
            Option<String>, // currency
            String,         // status
        ),
    >(
        r#"
        SELECT
            wt.id::text,
            wt.external_ref_id,
            wt.provider_reference,
            u.email AS user_email,
            wt.amount_cents,
            w.currency,
            wt.status
        FROM wallet_transactions wt
        JOIN wallets w ON wt.wallet_id = w.id
        JOIN users u ON w.user_id = u.id
        WHERE wt.type IN ('deposit', 'fiat_deposit')
          AND (
              wt.external_ref_id ILIKE $1
              OR wt.provider_reference ILIKE $1
              OR u.email ILIKE $1
              OR wt.id::text ILIKE $1
          )
        ORDER BY wt.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in &deposit_rows {
        let ref_id = row.1.as_deref().unwrap_or(&row.0[..8.min(row.0.len())]);
        let cents = row.4.unwrap_or(0);
        let formatted = format!("${:.2}", (cents.abs() as f64) / 100.0);
        let currency = row.5.as_deref().unwrap_or("USD");
        results.push(serde_json::json!({
            "type": "deposit",
            "icon": "💰",
            "title": format!("Deposit {}", ref_id),
            "subtitle": format!("{} · {} {}", row.3.as_deref().unwrap_or(""), formatted, currency),
            "url": "/admin/deposits.html",
            "badge": row.6,
        }));
    }

    // ── 5. Search Support Tickets ───────────────────────────────
    let ticket_rows = sqlx::query_as::<
        _,
        (
            String,         // id
            Option<String>, // subject
            Option<String>, // user_email
            String,         // status
            Option<String>, // priority
        ),
    >(
        r#"
        SELECT
            t.id::text,
            t.subject,
            u.email AS user_email,
            t.status,
            t.priority
        FROM support_tickets t
        LEFT JOIN users u ON t.user_id = u.id
        WHERE t.subject ILIKE $1
           OR u.email ILIKE $1
           OR t.id::text ILIKE $1
        ORDER BY t.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in &ticket_rows {
        results.push(serde_json::json!({
            "type": "ticket",
            "icon": "🎫",
            "title": row.1.as_deref().unwrap_or("Untitled Ticket"),
            "subtitle": format!("{} · {}", row.2.as_deref().unwrap_or(""), row.4.as_deref().unwrap_or("normal")),
            "url": format!("/admin/support-ticket.html?id={}", row.0),
            "badge": row.3,
        }));
    }

    // Truncate to limit
    results.truncate(limit as usize);

    Ok(Json(serde_json::json!({ "results": results })).into_response())
}
