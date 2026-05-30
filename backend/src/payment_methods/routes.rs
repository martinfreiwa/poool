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

    match service::list_user_payment_methods(&state.db, &user_id, None).await {
        Ok(methods) => Json(serde_json::json!({ "payment_methods": methods })).into_response(),
        Err(e) => {
            tracing::error!("Failed to list payment methods: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load payment methods"})),
            )
                .into_response()
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

    // ── P1-6: Compare typed holder name to the verified KYC profile.
    // A mismatch doesn't block the save (legitimate edge cases: married
    // name, transliteration) but opens a compliance alert so the team
    // can review before the next withdrawal goes out.
    let kyc_name: Option<(String, String)> = sqlx::query_as(
        "SELECT COALESCE(first_name, ''), COALESCE(last_name, '')
           FROM user_profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let name_outcome = if let Some((first, last)) = kyc_name.as_ref() {
        crate::common::name_match::compare(first, last, &form.account_holder_name)
    } else {
        // No profile yet — record a mismatch so onboarding can prompt
        // for KYC before withdrawals.
        crate::common::name_match::MatchOutcome::PotentialMismatch { missing: vec![] }
    };
    let holder_name_for_log = form.account_holder_name.clone();

    match service::add_bank(&state.db, &user_id, form).await {
        Ok(pm) => {
            // File a compliance_alerts row if the names don't match.
            if let crate::common::name_match::MatchOutcome::PotentialMismatch { missing } =
                &name_outcome
            {
                let kyc_display = kyc_name
                    .as_ref()
                    .map(|(f, l)| format!("{} {}", f, l).trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "(no KYC profile)".to_string());
                let summary = format!(
                    "Bank account holder \"{}\" does not match KYC profile \"{}\"",
                    holder_name_for_log, kyc_display
                );
                let details = serde_json::json!({
                    "payment_method_id": pm.id.to_string(),
                    "holder_name": holder_name_for_log,
                    "kyc_name": kyc_display,
                    "missing_tokens": missing,
                });
                let _ = sqlx::query(
                    r#"INSERT INTO compliance_alerts
                            (user_id, kind, severity, summary, details)
                       VALUES ($1, 'manual_review', 'high', $2, $3)"#,
                )
                .bind(user_id)
                .bind(&summary)
                .bind(&details)
                .execute(&state.db)
                .await;
                tracing::warn!(
                    user_id = %user_id,
                    payment_method_id = %pm.id,
                    "Bank-holder name mismatch — compliance alert opened"
                );
            }
            Html("".to_string()).into_response()
        }
        Err(e) => {
            tracing::error!("Error saving bank for user {}: {}", user_id, e);
            Html("Unable to save bank account. Please try again.".to_string()).into_response()
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
