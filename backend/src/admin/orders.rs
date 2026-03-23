use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};

//
//  Admin Orders API
//

/// GET /api/admin/orders/:id — Full order detail (items + invoice + wallet tx).
pub async fn api_admin_order_detail(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    use sqlx::Row;

    let order_uuid: uuid::Uuid = order_id
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid order ID".into()))?;

    // 1. Fetch order header + user info
    let order_row = sqlx::query(
        r#"SELECT o.id::text, o.order_number, o.user_id::text, o.total_cents, o.status,
                  o.payment_method, o.payment_ref_id, o.currency,
                  o.payment_currency, o.fx_rate::text, o.fx_provider,
                  o.proof_of_transfer_url,
                  o.created_at::text, o.completed_at::text,
                  COALESCE(u.email, '') AS user_email,
                  COALESCE(up.first_name, '') AS first_name,
                  COALESCE(up.last_name, '') AS last_name
           FROM orders o
           JOIN users u ON u.id = o.user_id
           LEFT JOIN user_profiles up ON up.user_id = o.user_id
           WHERE o.id = $1"#,
    )
    .bind(order_uuid)
    .fetch_optional(&state.db)
    .await?;

    let order_row = order_row.ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    let first: String = order_row.get("first_name");
    let last: String = order_row.get("last_name");
    let user_name = format!("{} {}", first, last).trim().to_string();
    let user_email: String = order_row.get("user_email");

    let order_json = serde_json::json!({
        "id":                  order_row.get::<String, _>("id"),
        "order_number":        order_row.get::<String, _>("order_number"),
        "user_id":             order_row.get::<String, _>("user_id"),
        "total_cents":         order_row.get::<i64, _>("total_cents"),
        "status":              order_row.get::<String, _>("status"),
        "payment_method":      order_row.get::<Option<String>, _>("payment_method"),
        "payment_ref_id":      order_row.get::<Option<String>, _>("payment_ref_id"),
        "currency":            order_row.get::<String, _>("currency"),
        "payment_currency":    order_row.get::<Option<String>, _>("payment_currency"),
        "fx_rate":             order_row.get::<Option<String>, _>("fx_rate"),
        "fx_provider":         order_row.get::<Option<String>, _>("fx_provider"),
        "proof_of_transfer_url": order_row.get::<Option<String>, _>("proof_of_transfer_url"),
        "created_at":          order_row.get::<String, _>("created_at"),
        "completed_at":        order_row.get::<Option<String>, _>("completed_at"),
        "user_email":          &user_email,
        "user_name":           if user_name.is_empty() { user_email.clone() } else { user_name },
    });

    // 2. Fetch order items (line items)
    let item_rows = sqlx::query(
        r#"SELECT oi.id::text, oi.asset_id::text, oi.tokens_quantity,
                  oi.token_price_cents, oi.subtotal_cents,
                  COALESCE(a.title, 'Unknown Asset') AS asset_title
           FROM order_items oi
           LEFT JOIN assets a ON a.id = oi.asset_id
           WHERE oi.order_id = $1
           ORDER BY oi.id"#,
    )
    .bind(order_uuid)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = item_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":               r.get::<String, _>("id"),
                "asset_id":         r.get::<String, _>("asset_id"),
                "asset_title":      r.get::<String, _>("asset_title"),
                "tokens_quantity":  r.get::<i32, _>("tokens_quantity"),
                "token_price_cents": r.get::<i64, _>("token_price_cents"),
                "subtotal_cents":   r.get::<i64, _>("subtotal_cents"),
            })
        })
        .collect();

    // 3. Fetch invoice (if any)
    let invoice_row = sqlx::query(
        r#"SELECT invoice_number, subtotal_cents, tax_cents, total_cents,
                  currency, pdf_url, status, issued_at::text
           FROM invoices
           WHERE order_id = $1
           LIMIT 1"#,
    )
    .bind(order_uuid)
    .fetch_optional(&state.db)
    .await?;

    let invoice = invoice_row.map(|r| {
        serde_json::json!({
            "invoice_number": r.get::<String, _>("invoice_number"),
            "subtotal_cents": r.get::<i64, _>("subtotal_cents"),
            "tax_cents":      r.get::<i64, _>("tax_cents"),
            "total_cents":    r.get::<i64, _>("total_cents"),
            "currency":       r.get::<String, _>("currency"),
            "pdf_url":        r.get::<Option<String>, _>("pdf_url"),
            "status":         r.get::<String, _>("status"),
            "issued_at":      r.get::<String, _>("issued_at"),
        })
    });

    // 4. Fetch related wallet transactions
    let wallet_tx_rows = sqlx::query(
        r#"SELECT wt.id::text, wt.type, wt.status, wt.amount_cents,
                  wt.description, wt.currency, wt.created_at::text
           FROM wallet_transactions wt
           JOIN wallets w ON w.id = wt.wallet_id
           WHERE wt.related_order_id = $1
           ORDER BY wt.created_at"#,
    )
    .bind(order_uuid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let wallet_txs: Vec<serde_json::Value> = wallet_tx_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":          r.get::<String, _>("id"),
                "type":        r.get::<String, _>("type"),
                "status":      r.get::<String, _>("status"),
                "amount_cents": r.get::<i64, _>("amount_cents"),
                "description": r.get::<Option<String>, _>("description"),
                "currency":    r.get::<String, _>("currency"),
                "created_at":  r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "order": order_json,
        "items": items,
        "invoice": invoice,
        "wallet_transactions": wallet_txs,
    }))
    .into_response())
}

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
