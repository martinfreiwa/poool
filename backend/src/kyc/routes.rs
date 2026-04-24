use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;

use super::models::{KycInitiateRequest, KycSubmitRequest};
use super::provider::KycProvider;
use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;

async fn require_user_id(
    jar: &CookieJar,
    state: &AppState,
) -> Result<uuid::Uuid, axum::response::Response> {
    match middleware::get_current_user(jar, &state.db).await {
        Some(user) => Ok(user.id),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Not authenticated"})),
        )
            .into_response()),
    }
}

/// GET /api/kyc/status — Return the current user's KYC status.
pub async fn get_status(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_kyc_status(&state.db, user_id).await {
        Ok(status) => Json(status).into_response(),
        Err(e) => {
            tracing::error!("Failed to get KYC status for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to check status"})),
            )
                .into_response()
        }
    }
}

/// POST /api/kyc/submit — Legacy manual submission (backward-compat).
pub async fn submit(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<KycSubmitRequest>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::submit_kyc(&state.db, user_id, payload).await {
        Ok(_) => Json(serde_json::json!({"status": "pending"})).into_response(),
        Err(e) => {
            tracing::error!("Failed to submit KYC for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to submit KYC"})),
            )
                .into_response()
        }
    }
}

/// POST /api/kyc/initiate — Create a KYC session with the active provider.
///
/// Returns a `verification_url` for redirect-based providers (Didit, Sumsub).
/// For manual providers, the URL is empty and the frontend should show its own form.
pub async fn initiate(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<KycInitiateRequest>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let provider = service::build_provider();
    let callback_url = format!("{}/kyc", state.config.base_url);

    match service::initiate_kyc(
        &state.db,
        provider.as_ref(),
        user.id,
        Some(&user.email),
        &callback_url,
        payload.document_type.as_deref(),
    )
    .await
    {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => {
            tracing::error!("Failed to initiate KYC for {}: {}", user.id, e);
            let (status_code, error_msg) = match &e {
                crate::error::AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to initiate KYC".to_string(),
                ),
            };
            (status_code, Json(serde_json::json!({"error": error_msg}))).into_response()
        }
    }
}

/// POST /api/webhooks/kyc/didit — Webhook endpoint for Didit status updates.
///
/// Didit sends POST requests with:
/// - Body: JSON with session_id, status, decision, etc.
/// - Header: X-Signature-Simple (HMAC-SHA256 of "{timestamp}:{session_id}:{status}:{webhook_type}")
///
/// This endpoint is unauthenticated (webhooks come from Didit servers)
/// but validates the HMAC signature.
pub async fn didit_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    let provider = match super::didit::DiditConfig::from_env() {
        Some(cfg) => super::didit::DiditProvider::new(cfg),
        None => {
            tracing::error!("Didit webhook received but provider not configured (API key or webhook secret missing)");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "KYC provider not configured"})),
            )
                .into_response();
        }
    };

    // Try multiple signature headers in order of preference (V2 is recommended)
    let signature = headers
        .get("x-signature-v2")
        .or_else(|| headers.get("x-signature-simple"))
        .or_else(|| headers.get("x-signature"))
        .and_then(|v| v.to_str().ok());

    match provider.process_webhook(&body, signature).await {
        Ok(update) => {
            // Replay guard: the signature alone is a nonce — Didit signs
            // deterministic payloads, so a resent webhook carries the same
            // signature. SETNX the signature with a 10-minute TTL (covers the
            // 5-minute freshness window on both sides). If Redis is absent
            // we degrade open with a warning log.
            if let (Some(sig), Some(redis_pool)) = (signature, state.redis.as_ref()) {
                if let Ok(mut conn) = redis_pool.get().await {
                    let key = format!("kyc_webhook_nonce:{}", sig);
                    let res: Result<Option<String>, _> = redis::cmd("SET")
                        .arg(&key)
                        .arg("1")
                        .arg("NX")
                        .arg("EX")
                        .arg(600)
                        .query_async(&mut *conn)
                        .await;
                    if matches!(res, Ok(None)) {
                        tracing::warn!("Didit webhook replay blocked (nonce already seen)");
                        return (StatusCode::OK, Json(serde_json::json!({"ok": true})))
                            .into_response();
                    }
                }
            } else if state.redis.is_none() {
                tracing::warn!("KYC webhook processed without replay guard (Redis unavailable)");
            }

            let provider_name = provider.name();
            if let Err(e) = service::process_webhook_update(&state.db, update, provider_name).await
            {
                tracing::error!("Failed to process Didit webhook update: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Processing failed"})),
                )
                    .into_response();
            }
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => {
            tracing::error!("Didit webhook processing error: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid webhook"})),
            )
                .into_response()
        }
    }
}

/// GET /api/kyc/provider — Return info about the active KYC provider.
///
/// Used by the frontend to decide whether to show the manual form
/// or redirect to an external verification flow.
pub async fn get_provider_info(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let _user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let provider = service::build_provider();
    Json(serde_json::json!({
        "provider": provider.name(),
        "supports_redirect": provider.name() != "manual",
    }))
    .into_response()
}

/// GET /kyc — Render the identity verification page.
pub async fn page_kyc(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "kyc.html").await
}
