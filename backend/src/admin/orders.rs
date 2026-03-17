use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

//
//  Admin Orders API
//

/// GET /api/admin/orders  List all orders with user info + item count.
pub async fn api_admin_orders(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            i64,
            String, // id, order_number, user_id, total, status
            Option<String>,
            Option<String>, // payment_method, payment_ref
            String,
            Option<String>, // created_at, completed_at
            String,         // user_email
            Option<String>,
            Option<String>, // first, last
            i64,            // item_count
        ),
    >(
        r#"SELECT o.id::text, o.order_number, o.user_id::text, o.total_cents, o.status,
                  o.payment_method, o.payment_ref_id,
                  o.created_at::text, o.completed_at::text,
                  COALESCE(u.email, ''),
                  COALESCE(up.first_name, ''), COALESCE(up.last_name, ''),
                  COALESCE((SELECT COUNT(*) FROM order_items oi WHERE oi.order_id = o.id), 0)
           FROM orders o
           JOIN users u ON u.id = o.user_id
           LEFT JOIN user_profiles up ON up.user_id = o.user_id
           ORDER BY o.created_at DESC
           LIMIT 10000"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let orders: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let name = format!(
                "{} {}",
                r.10.clone().unwrap_or_default(),
                r.11.clone().unwrap_or_default()
            )
            .trim()
            .to_string();
            serde_json::json!({
                "id": r.0, "order_number": r.1, "user_id": r.2, "total_cents": r.3,
                "status": r.4, "payment_method": r.5, "payment_ref_id": r.6,
                "created_at": r.7, "completed_at": r.8,
                "user_email": r.9,
                "user_name": if name.is_empty() { r.9.clone() } else { name },
                "item_count": r.12
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "orders": orders })).into_response())
}

//
//  Admin Investments API
//

/// GET /api/admin/investments  List all investments with user/asset info.
pub async fn api_admin_investments(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query_as::<_, (
        String, String, // inv_id, user_id
        i32, i64, i64, i64, // tokens_owned, purchase, current, rental
        String, String, // status, purchased_at
        String, Option<String>, Option<String>, // email, first, last
        String, // asset_title
    )>(
        r#"SELECT i.id::text, i.user_id::text,
                  i.tokens_owned, i.purchase_value_cents, i.current_value_cents, i.total_rental_cents,
                  i.status, i.purchased_at::text,
                  COALESCE(u.email, ''),
                  COALESCE(up.first_name, ''), COALESCE(up.last_name, ''),
                  COALESCE(a.title, 'Unknown Asset')
           FROM investments i
           JOIN users u ON u.id = i.user_id
           LEFT JOIN user_profiles up ON up.user_id = i.user_id
           LEFT JOIN assets a ON a.id = i.asset_id
           ORDER BY i.purchased_at DESC
           LIMIT 10000"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let investments: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let name = format!(
                "{} {}",
                r.9.clone().unwrap_or_default(),
                r.10.clone().unwrap_or_default()
            )
            .trim()
            .to_string();
            serde_json::json!({
                "id": r.0, "user_id": r.1,
                "tokens_owned": r.2, "purchase_value_cents": r.3,
                "current_value_cents": r.4, "total_rental_cents": r.5,
                "status": r.6, "purchased_at": r.7,
                "user_email": r.8,
                "user_name": if name.is_empty() { r.8.clone() } else { name },
                "asset_title": r.11
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "investments": investments })).into_response())
}
