/// Marketplace HTTP routes — thin handlers that delegate to the service layer.
///
/// Each handler is responsible ONLY for:
/// 1. Extracting data from the HTTP request (session, path params, body)
/// 2. Calling the appropriate service/validation function
/// 3. Formatting the HTTP response
///
/// NO business logic lives here.
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use super::{models, service, validation};
use crate::auth::routes::AppState;
use crate::error::AppError;

use sqlx::PgPool;

pub async fn resolve_asset_id(pool: &PgPool, id_or_slug: &str) -> Result<Uuid, AppError> {
    if let Ok(uuid) = Uuid::parse_str(id_or_slug) {
        Ok(uuid)
    } else {
        let asset_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM assets WHERE slug = $1")
            .bind(id_or_slug)
            .fetch_optional(pool)
            .await
            .map_err(AppError::Database)?;
        asset_id.ok_or_else(|| AppError::NotFound("Asset not found".into()))
    }
}

// ═══════════════════════════════════════════════════════════════
// ── PUBLIC READ APIs ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// GET /api/marketplace/:asset_id/orderbook
///
/// Returns the aggregated orderbook snapshot for the given asset.
/// Public — no authentication required.
pub async fn api_orderbook(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let mut snapshot = match state.redis.as_ref() {
        Some(redis) => {
            match super::orderbook::get_orderbook_snapshot(redis, asset_id, None).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(
                        "Redis orderbook unavailable ({}). Falling back to PostgreSQL.",
                        e
                    );
                    super::service::get_orderbook_snapshot_from_db(&state.db, asset_id, None)
                        .await?
                }
            }
        }
        None => super::service::get_orderbook_snapshot_from_db(&state.db, asset_id, None).await?,
    };

    // Fill last_price from trade_history
    let last_price: Option<i64> = sqlx::query_scalar(
        "SELECT price_cents FROM trade_history WHERE asset_id = $1 ORDER BY executed_at DESC LIMIT 1",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    snapshot.last_price_cents = last_price;

    Ok(Json(snapshot))
}

/// GET /api/marketplace/:asset_id/trades
///
/// Returns the most recent trades for an asset (trade tape).
/// Public — no authentication required.
pub async fn api_recent_trades(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let trades = service::get_recent_trades(&state.db, asset_id, 50).await?;
    Ok(Json(trades))
}

/// GET /api/marketplace/:asset_id/ticker
///
/// Returns 24-hour ticker data for an asset.
/// Public — no authentication required.
pub async fn api_ticker(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let ticker = service::get_ticker(&state.db, asset_id).await?;
    Ok(Json(ticker))
}

/// GET /api/marketplace/secondary/assets
///
/// Returns all assets currently available on the secondary market.
/// Public — no authentication required.
pub async fn api_secondary_assets(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let assets = service::get_secondary_assets(&state.db).await?;
    Ok(Json(assets))
}

// ═══════════════════════════════════════════════════════════════
// ── AUTHENTICATED TRADING APIs ────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// POST /api/marketplace/orders
///
/// Submit a new buy or sell order. Requires authentication.
///
/// Flow:
/// 1. Validate input fields
/// 2. Verify KYC status
/// 3. Check rate limits
/// 4. Begin DB transaction
/// 5. Check balance/tokens (FOR UPDATE)
/// 6. Create order in DB
/// 7. Place hold on balance/tokens
/// 8. Insert into Redis orderbook
/// 9. Return order confirmation
pub async fn api_submit_order(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<models::SubmitOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Authenticate
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    // 2. Validate request fields
    validation::validate_order_fields(&body)?;

    // 3. Step-up 2FA for trades — TEMPORARILY DISABLED.
    //    Will re-enable once the step-up flow has more UX polish (inline
    //    modal instead of full-page redirect, "remember device" option,
    //    threshold tunable from admin settings). Withdrawals still gated.
    //
    // To re-enable, uncomment:
    //
    // if body.order_type == "limit" {
    //     let estimated_cents = body
    //         .price_cents
    //         .unwrap_or(0)
    //         .saturating_mul(body.quantity as i64);
    //     crate::auth::step_up::require_step_up_2fa(
    //         &state.db,
    //         state.redis.as_ref(),
    //         user.id,
    //         crate::auth::step_up::FinancialAction::Trade,
    //         estimated_cents,
    //     )
    //     .await?;
    // }

    // 4. Execute order creation (all business logic in service layer)
    let response = service::create_order(&state.db, state.redis.as_ref(), user.id, body).await?;

    Ok(Json(response))
}

#[derive(serde::Deserialize, Default)]
pub struct PageQuery {
    /// ISO-8601 timestamp — return rows strictly older than this.
    pub before: Option<chrono::DateTime<chrono::Utc>>,
    /// 1..=200, default 50.
    pub limit: Option<i64>,
}

/// GET /api/marketplace/:asset_id/fee-rate
///
/// Returns the resolved taker / maker fee rates (in basis points) and the
/// human-readable percentage strings for an asset. Used by the frontend to
/// display the *actual* fee in the trade-confirm UI instead of a hardcoded
/// 5%. Public — no auth required (fee rates are not sensitive and the same
/// rate applies to every user; promotions are encoded into the rate).
pub async fn api_fee_rate(
    State(state): State<AppState>,
    Path(asset_id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = service::resolve_asset_id(&state.db, &asset_id_or_slug).await?;
    let fees = validation::resolve_fees(&state.db, asset_id).await?;
    Ok(Json(serde_json::json!({
        "asset_id": asset_id,
        "taker_fee_bps": fees.taker_fee_bps,
        "maker_fee_bps": fees.maker_fee_bps,
        "taker_fee_pct": fees.taker_fee_bps as f64 / 100.0,
        "maker_fee_pct": fees.maker_fee_bps as f64 / 100.0,
    })))
}

/// GET /api/marketplace/orders/mine?before=...&limit=...
///
/// Returns the authenticated user's orders, newest first. Cursor pagination
/// — pass the last row's `created_at` as `before` to fetch the next page.
pub async fn api_my_orders(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<PageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let orders = service::get_user_orders(&state.db, user.id, q.before, q.limit).await?;
    Ok(Json(orders))
}

/// GET /api/marketplace/trades/mine?before=...&limit=...
pub async fn api_my_trades(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<PageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let trades = service::get_user_trades_history(&state.db, user.id, q.before, q.limit).await?;
    Ok(Json(trades))
}

#[derive(serde::Deserialize)]
pub struct TaxExportQuery {
    pub year: i32,
    pub format: String,
}

/// GET /api/marketplace/tax-export
///
/// Download tax report (trade history with P&L) for the authenticated user.
pub async fn api_export_tax_report(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<TaxExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    // Export logic - using trade history dump for the requested year
    let trades = sqlx::query!(
        r#"
        SELECT 
            TO_CHAR(t.executed_at, 'YYYY-MM-DD HH24:MI:SS') as "executed_at_str!", 
            a.title as "asset_name!",
            t.price_cents, 
            t.quantity, 
            t.total_cents, 
            t.fee_cents, 
            t.buyer_user_id, 
            t.seller_user_id
        FROM trade_history t
        JOIN assets a ON t.asset_id = a.id
        WHERE (t.buyer_user_id = $1 OR t.seller_user_id = $1)
        AND EXTRACT(YEAR FROM t.executed_at) = $2
        ORDER BY t.executed_at ASC
        "#,
        user.id,
        query.year as f64
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    let mut csv_data =
        String::from("Date,Asset Name,Side,Price,Quantity,Gross Total,Fee,Net Total\n");
    for t in trades {
        let is_buyer = t.buyer_user_id == user.id;
        let side = if is_buyer { "BUY" } else { "SELL" };
        let date_str = t.executed_at_str;

        let price = t.price_cents as f64 / 100.0;
        let gross = t.total_cents.unwrap_or(0) as f64 / 100.0;
        let fee = t.fee_cents as f64 / 100.0;

        // Let's assume the user paid the fee whether buying or selling (simplification matching frontend logic)
        let net = if is_buyer { gross + fee } else { gross - fee };

        // Escape commas in asset names (e.g. "Villa, Bali")
        let safe_asset_name = if t.asset_name.contains(',') {
            format!("\"{}\"", t.asset_name)
        } else {
            t.asset_name
        };

        csv_data.push_str(&format!(
            "{},{},{},${:.2},{},${:.2},${:.2},${:.2}\n",
            date_str, safe_asset_name, side, price, t.quantity, gross, fee, net
        ));
    }

    let is_pdf = query.format.to_lowercase() == "pdf";
    // NOTE: True PDF generation requires an external crate. For now, returning CSV data universally.
    let content_type = if is_pdf { "text/csv" } else { "text/csv" };
    let filename = if is_pdf {
        format!("tax_report_{}.csv", query.year)
    } else {
        format!("tax_report_{}.csv", query.year)
    };

    use axum::http::header;

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        csv_data,
    ))
}

/// GET /tax-report?year=2026
///
/// Render the tax report as a branded HTML page (for Print → Save as PDF).
/// Loads company settings, user profile, and trade history.
pub async fn page_tax_report_pdf(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<TaxExportQuery>,
) -> axum::response::Response {
    use axum::response::Redirect;
    use sqlx::Row;

    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };
    let year = query.year;

    // ── Load company settings ────────────────────────────────
    let setting_rows = sqlx::query("SELECT key, value FROM platform_settings WHERE key LIKE 'company_%' OR key IN ('platform_name', 'support_email')")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut settings = std::collections::HashMap::<String, String>::new();
    for row in &setting_rows {
        let k: String = row.get("key");
        let v: String = row.get("value");
        settings.insert(k, v);
    }

    // ── Load user profile ────────────────────────────────────
    let profile = sqlx::query!(
        r#"
        SELECT 
            COALESCE(up.first_name, '') as "first_name!",
            COALESCE(up.last_name, '') as "last_name!",
            COALESCE(up.address_line_1, '') as "address!",
            COALESCE(up.city, '') as "city!",
            COALESCE(up.postal_code, '') as "postal!",
            COALESCE(up.country, '') as "country!",
            COALESCE(up.phone_number, '') as "phone!",
            COALESCE(up.tax_id, '') as "tax_id!"
        FROM user_profiles up
        WHERE up.user_id = $1
        "#,
        user.id
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let user_name = match &profile {
        Some(p) if !p.first_name.is_empty() => format!("{} {}", p.first_name, p.last_name),
        _ => user.email.clone(),
    };

    // ── Load trade history ───────────────────────────────────
    let trades = sqlx::query!(
        r#"
        SELECT 
            TO_CHAR(t.executed_at, 'YYYY-MM-DD') as "date!",
            a.title as "asset_name!",
            t.price_cents,
            t.quantity,
            t.total_cents,
            t.fee_cents,
            t.buyer_user_id,
            t.seller_user_id
        FROM trade_history t
        JOIN assets a ON t.asset_id = a.id
        WHERE (t.buyer_user_id = $1 OR t.seller_user_id = $1)
        AND EXTRACT(YEAR FROM t.executed_at) = $2
        ORDER BY t.executed_at ASC
        "#,
        user.id,
        year as f64
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // ── Compute summary totals ───────────────────────────────
    let mut total_volume_cents: i64 = 0;
    let mut total_fees_cents: i64 = 0;
    let mut total_net_cents: i64 = 0;

    let mut trade_rows: Vec<serde_json::Value> = Vec::new();
    for t in &trades {
        let is_buyer = t.buyer_user_id == user.id;
        let side = if is_buyer { "BUY" } else { "SELL" };
        let gross_cents = t.total_cents.unwrap_or(0);
        let fee_cents = t.fee_cents;
        let net_cents = if is_buyer {
            gross_cents + fee_cents
        } else {
            gross_cents - fee_cents
        };

        total_volume_cents += gross_cents;
        total_fees_cents += fee_cents;
        total_net_cents += net_cents;

        trade_rows.push(serde_json::json!({
            "date": t.date,
            "asset_name": t.asset_name,
            "side": side,
            "price": format!("${:.2}", t.price_cents as f64 / 100.0),
            "quantity": t.quantity,
            "gross": format!("${:.2}", gross_cents as f64 / 100.0),
            "fee": format!("${:.2}", fee_cents as f64 / 100.0),
            "net": format!("${:.2}", net_cents as f64 / 100.0),
        }));
    }

    let now = chrono::Utc::now();
    let doc_number = format!("TAX-{}-{}", year, now.format("%Y%m%d%H%M"));

    let context = serde_json::json!({
        // Company details
        "company_name": settings.get("company_legal_name").unwrap_or(&"PT POOOL Finance Indonesia".to_string()),
        "company_address": settings.get("company_address").unwrap_or(&"".to_string()),
        "company_city": settings.get("company_city").unwrap_or(&"".to_string()),
        "company_postal": settings.get("company_postal").unwrap_or(&"".to_string()),
        "company_country": settings.get("company_country").unwrap_or(&"Indonesia".to_string()),
        "company_npwp": settings.get("company_npwp").unwrap_or(&"".to_string()),
        "company_nib": settings.get("company_nib").unwrap_or(&"".to_string()),
        "company_ojk_license": settings.get("company_ojk_license").unwrap_or(&"".to_string()),
        "company_email": settings.get("support_email").unwrap_or(&"support@poool.finance".to_string()),
        "company_phone": settings.get("company_phone").unwrap_or(&"".to_string()),
        "company_website": settings.get("company_website").unwrap_or(&"https://poool.finance".to_string()),
        // Document metadata
        "doc_title": "Tax Report / Laporan Pajak",
        "doc_number": doc_number,
        "doc_date": now.format("%d %B %Y").to_string(),
        "doc_period": format!("January – December {}", year),
        "generated_at": now.format("%Y-%m-%d %H:%M UTC").to_string(),
        "currency": "USD",
        // User details
        "user_name": user_name,
        "user_email": user.email,
        "user_id": user.id.to_string(),
        "user_address": profile.as_ref().map(|p| p.address.clone()).unwrap_or_default(),
        "user_city": profile.as_ref().map(|p| p.city.clone()).unwrap_or_default(),
        "user_postal": profile.as_ref().map(|p| p.postal.clone()).unwrap_or_default(),
        "user_country": profile.as_ref().map(|p| p.country.clone()).unwrap_or_default(),
        "user_npwp": profile.as_ref().map(|p| p.tax_id.clone()).unwrap_or_default(),
        "user_phone": profile.as_ref().map(|p| p.phone.clone()).unwrap_or_default(),
        // Summary
        "tax_year": year,
        "total_trades": trades.len(),
        "total_volume": format!("${:.2}", total_volume_cents as f64 / 100.0),
        "total_fees": format!("${:.2}", total_fees_cents as f64 / 100.0),
        "total_net": format!("${:.2}", total_net_cents as f64 / 100.0),
        "net_pl": format!("${:.2}", total_net_cents as f64 / 100.0),
        "net_pl_positive": total_net_cents >= 0,
        // Trade data
        "trades": trade_rows,
    });

    crate::common::routes_helper::serve_protected_with_context(
        jar,
        &state,
        "templates/pdf-tax-report.html",
        context,
    )
    .await
}

/// DELETE /api/marketplace/orders/:order_id
///
/// Cancel an open order. Requires authentication. The order must
/// belong to the authenticated user and must be in "open" or "partially_filled" status.
pub async fn api_cancel_order(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    // Rate limit cancels at 20/min per user (masterplan §2.12). Uses a
    // separate Redis bucket from order-create (10/min) so a cancel storm
    // can't starve order placement.
    if let Some(redis) = state.redis.as_ref() {
        if let Err(retry_after) =
            super::orderbook::check_cancel_rate_limit(redis, user.id, 20, 60).await
        {
            return Err(AppError::RateLimited(retry_after));
        }
    }

    service::cancel_order(&state.db, state.redis.as_ref(), user.id, order_id).await?;

    Ok(Json(serde_json::json!({
        "status": "cancelled",
        "order_id": order_id,
        "message": "Order cancelled successfully."
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── CANDLESTICK CHART APIs ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// GET /api/marketplace/:asset_id/candles?interval=1h&from=&to=&limit=
///
/// Returns OHLCV candlestick data for charting.
pub async fn api_candles(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Query(query): Query<super::charts::CandleQuery>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;
    let response = super::charts::get_candles(&state.db, asset_id, query).await?;

    Ok(Json(response))
}

/// GET /api/marketplace/:asset_id/chart-summary
///
/// Returns 24h chart summary (last price, high, low, volume, change).
pub async fn api_chart_summary(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let summary = super::charts::get_chart_summary(&state.db, asset_id).await?;
    Ok(Json(summary))
}

// ═══════════════════════════════════════════════════════════════
// ── P2P/OTC OFFER APIs ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// POST /api/marketplace/p2p/offers
///
/// Create a new P2P offer. Requires authentication.
pub async fn api_create_p2p_offer(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<super::p2p::CreateP2POfferRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let response = super::p2p::create_offer(&state.db, user.id, body).await?;
    Ok(Json(response))
}

/// POST /api/marketplace/p2p/offers/:offer_id/respond
///
/// Respond to a P2P offer (accept, decline, or counter).
pub async fn api_respond_p2p_offer(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(offer_id): Path<Uuid>,
    Json(body): Json<super::p2p::RespondP2POfferRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let response = super::p2p::respond_to_offer(&state.db, user.id, offer_id, body).await?;
    Ok(Json(response))
}

/// DELETE /api/marketplace/p2p/offers/:offer_id
///
/// Cancel a pending P2P offer (maker only).
pub async fn api_cancel_p2p_offer(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(offer_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let response = super::p2p::cancel_offer(&state.db, user.id, offer_id).await?;
    Ok(Json(response))
}

/// GET /api/marketplace/p2p/offers/incoming
///
/// Get incoming offers for the authenticated user.
pub async fn api_incoming_offers(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let offers = super::p2p::get_incoming_offers(&state.db, user.id).await?;
    Ok(Json(offers))
}

/// GET /api/marketplace/p2p/offers/outgoing
///
/// Get outgoing offers for the authenticated user.
pub async fn api_outgoing_offers(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let offers = super::p2p::get_outgoing_offers(&state.db, user.id).await?;
    Ok(Json(offers))
}

/// GET /api/marketplace/:asset_id/p2p
///
/// Get pending P2P offers for a specific asset (public).
pub async fn api_asset_p2p_offers(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let offers = super::p2p::get_asset_offers(&state.db, asset_id).await?;
    Ok(Json(offers))
}
