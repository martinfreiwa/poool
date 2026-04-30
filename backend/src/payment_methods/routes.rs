use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use super::models::{AddBankForm, AttachCardTokenForm, PaymentMethod};
use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;

async fn get_user_id(jar: &CookieJar, state: &AppState) -> Option<Uuid> {
    middleware::get_current_user(jar, &state.db)
        .await
        .map(|user| user.id)
}

/// GET /api/payment-methods
pub async fn list_payment_methods(jar: CookieJar, State(state): State<AppState>) -> Response {
    let user_id = match get_user_id(&jar, &state).await {
        Some(uid) => uid,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    match sqlx::query_as::<_, PaymentMethod>(
        "SELECT * FROM payment_methods WHERE user_id = $1 AND status != 'failed' ORDER BY is_default DESC, created_at DESC"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(methods) => Json(serde_json::json!({ "payment_methods": methods })).into_response(),
        Err(e) => {
            tracing::error!("Failed to list payment methods: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to load payment methods"}))).into_response()
        }
    }
}

/// POST /api/payment-methods/card
pub async fn handle_add_card(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<AttachCardTokenForm>,
) -> Response {
    let user_id = match get_user_id(&jar, &state).await {
        Some(uid) => uid,
        None => {
            return (StatusCode::UNAUTHORIZED, Html("Unauthorized".to_string())).into_response()
        }
    };

    if !form.stripe_payment_method_id.starts_with("pm_") {
        tracing::warn!(
            user_id = %user_id,
            "Rejected card save without a Stripe PaymentMethod token"
        );
        return (
            StatusCode::BAD_REQUEST,
            Html(
                "<div class='error'>Card saving requires a verified Stripe payment method.</div>"
                    .to_string(),
            ),
        )
            .into_response();
    }

    match service::attach_card(&state.db, &user_id, form).await {
        Ok(_) => Html("".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Error saving card for user {}: {}", user_id, e);
            (
                StatusCode::BAD_REQUEST,
                Html("Unable to save card. Please try again.".to_string()),
            )
                .into_response()
        }
    }
}

/// POST /api/payment-methods/bank
pub async fn handle_add_bank(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<AddBankForm>,
) -> Response {
    let user_id = match get_user_id(&jar, &state).await {
        Some(uid) => uid,
        None => {
            return (StatusCode::UNAUTHORIZED, Html("Unauthorized".to_string())).into_response()
        }
    };

    match service::add_bank(&state.db, &user_id, form).await {
        Ok(_) => Html("".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Error saving bank for user {}: {}", user_id, e);
            Html("Unable to save bank account. Please try again.".to_string())
                .into_response()
        }
    }
}

/// DELETE /api/payment-methods/:id
pub async fn delete_payment_method(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let user_id = match get_user_id(&jar, &state).await {
        Some(uid) => uid,
        None => {
            return (StatusCode::UNAUTHORIZED, Html("Unauthorized".to_string())).into_response()
        }
    };

    match service::delete_payment_method(&state.db, &user_id, id).await {
        Ok(_) => Html("<script>window.location.reload();</script>".to_string()).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("Error deleting".to_string()),
        )
            .into_response(),
    }
}

/// POST /api/payment-methods/:id/default
pub async fn set_default_payment_method(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let user_id = match get_user_id(&jar, &state).await {
        Some(uid) => uid,
        None => {
            return (StatusCode::UNAUTHORIZED, Html("Unauthorized".to_string())).into_response()
        }
    };

    match service::set_default_payment_method(&state.db, &user_id, id).await {
        Ok(_) => Html("<script>window.location.reload();</script>".to_string()).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("Error setting default".to_string()),
        )
            .into_response(),
    }
}
