use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;

#[derive(Deserialize)]
pub struct PeriodQuery {
    pub period: Option<String>,
}

/// GET /developer/dashboard/fragments/chart
pub async fn fragment_chart(
    jar: CookieJar,
    Query(query): Query<PeriodQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Html("Unauthorized".to_string()).into_response(),
    };
    let is_developer = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name = 'developer')",
        user.id
    ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);

    if !is_developer {
        return Redirect::to("/marketplace").into_response();
    }

    let period = query.period.unwrap_or_else(|| "all".to_string());

    // In a real implementation this would pass the period parameter
    // For now we just fetch the stats which we will soon enhance
    let stats = service::fetch_dashboard_stats(&state.db, user.id).await;

    match state
        .templates
        .get_template("components/developer-chart.html")
    {
        Ok(t) => {
            // Need to pass the active period to the UI so it can style the active tab
            let ctx = minijinja::context! { stats => stats, active_period => period };
            match t.render(ctx) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template render error: {}", e);
                    Html("Template error".to_string()).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Template missing: {}", e);
            Html("Template missing".to_string()).into_response()
        }
    }
}

/// GET /developer/dashboard/fragments/assets  
pub async fn fragment_assets(
    jar: CookieJar,
    Query(query): Query<PeriodQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Html("Unauthorized".to_string()).into_response(),
    };
    let is_developer = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name = 'developer')",
        user.id
    ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);

    if !is_developer {
        return Redirect::to("/marketplace").into_response();
    }

    let period = query.period.unwrap_or_else(|| "all".to_string());

    let assets = service::fetch_all_assets(&state.db, user.id).await;
    let stats = service::fetch_dashboard_stats(&state.db, user.id).await;

    match state
        .templates
        .get_template("components/developer-assets.html")
    {
        Ok(t) => {
            let ctx = minijinja::context! {
                stats => stats,
                developer_assets => assets,
                active_period => period
            };
            match t.render(ctx) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template render error: {}", e);
                    Html("Template error".to_string()).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Template missing: {}", e);
            Html("Template missing".to_string()).into_response()
        }
    }
}
