use super::service;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;

/// Hard per-field cap for support attachments. Mirrors the post-upload size
/// guard in `service::MAX_ATTACHMENT_BYTES` (5 MB) so an oversized payload is
/// rejected before the multipart body is buffered into RAM.
const SUPPORT_ATTACHMENT_MAX_BYTES: usize = 5 * 1024 * 1024;

async fn require_support_rate_limit(
    state: &AppState,
    user_id: uuid::Uuid,
    action: &str,
) -> Result<(), axum::response::Response> {
    match state
        .auth_rate_limiter
        .check(&format!("support:{}:{}", action, user_id))
        .await
    {
        Ok(_) => Ok(()),
        Err(retry_after) => {
            tracing::warn!(
                "Rate limit exceeded for support action: action={} user={}",
                action,
                user_id
            );
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Too many support requests. Please wait {} seconds.", retry_after)
                })),
            )
                .into_response())
        }
    }
}

/// GET /api/support/tickets — List the current user's own support tickets with replies.
pub async fn api_support_tickets_list(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    match service::list_tickets(&state, user.id).await {
        Ok(tickets) => Json(serde_json::json!({ "tickets": tickets })).into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to fetch tickets"})),
        )
            .into_response(),
    }
}

use axum::extract::Multipart;

/// POST /api/support/tickets — Submit a new support ticket.
pub async fn api_support_tickets_submit(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            tracing::warn!("Auth failed in support submit");
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response();
        }
    };

    if let Err(resp) = require_support_rate_limit(&state, user.id, "create").await {
        return resp;
    }

    let mut subject = String::new();
    let mut message = String::new();
    let mut priority = "normal".to_string();
    let mut category = "general".to_string();
    let mut context_str = String::new();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut file_type: Option<String> = None;

    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(e) => {
            tracing::warn!("Invalid support multipart payload: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid upload payload"})),
            )
                .into_response();
        }
    } {
        let name = field.name().unwrap_or("").to_string();

        if name == "attachment" {
            file_name = field.file_name().map(|s| s.to_string());
            file_type = field.content_type().map(|s| s.to_string());

            // Validate MIME type server-side
            if let Some(ref mime) = file_type {
                if !["image/png", "image/jpeg", "application/pdf"].contains(&mime.as_str()) {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "Invalid file type. Allowed: JPG, PNG, PDF."})),
                    )
                        .into_response();
                }
            }

            // Chunked read with hard cap (5 MB) — matches MAX_ATTACHMENT_BYTES
            // enforced by the service layer. Prevents `field.bytes()` from
            // buffering the full payload before the size check.
            let mut field = field;
            match crate::storage::upload_helpers::read_field_capped(
                &mut field,
                SUPPORT_ATTACHMENT_MAX_BYTES,
                "attachment",
            )
            .await
            {
                Ok(data) if !data.is_empty() => file_bytes = Some(data),
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Failed reading support attachment bytes");
                    return e.into_response();
                }
            }
        } else if let Ok(text) = field.text().await {
            match name.as_str() {
                "subject" => subject = text,
                "message" => message = text,
                "priority" => priority = text,
                "category" => category = text,
                "context" => context_str = text,
                _ => {}
            }
        }
    }

    if subject.trim().is_empty() || message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Subject and message are required"})),
        )
            .into_response();
    }

    // Validate lengths
    if subject.trim().len() < 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Subject must be at least 5 characters"})),
        )
            .into_response();
    }

    if message.trim().len() < 20 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Message must be at least 20 characters"})),
        )
            .into_response();
    }

    match service::submit_ticket(
        &state,
        user.id,
        &subject,
        &message,
        &priority,
        &category,
        &context_str,
        file_bytes,
        file_name,
        file_type,
    )
    .await
    {
        Ok(_) => {
            Json(serde_json::json!({ "status": "success", "message": "Support ticket created" }))
                .into_response()
        }
        Err(e) => {
            let err_msg = e.to_string();
            // Return validation errors as 400, everything else as 500
            if err_msg.contains("Validation error")
                || err_msg.contains("too large")
                || err_msg.contains("Attachment")
            {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": err_msg})),
                )
                    .into_response()
            } else {
                tracing::error!("Failed to create ticket: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to create support ticket"})),
                )
                    .into_response()
            }
        }
    }
}

/// POST /api/support/tickets/:ticket_id/reply — Add a user reply to their own ticket.
/// Accepts multipart/form-data with fields: `message` (required), `attachment` (optional file).
pub async fn api_support_ticket_reply(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(ticket_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let mut message = String::new();
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut file_type: Option<String> = None;

    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Invalid reply multipart payload: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid request format"})),
            )
                .into_response();
        }
    } {
        let name = field.name().unwrap_or("").to_string();
        if name == "message" {
            message = field.text().await.unwrap_or_default();
        } else if name == "attachment" {
            file_name = field.file_name().map(|s| s.to_string());
            file_type = field.content_type().map(|s| s.to_string());

            if let Some(ref mime) = file_type {
                if !matches!(
                    mime.as_str(),
                    "image/jpeg" | "image/png" | "application/pdf"
                ) {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "Invalid file type. Allowed: JPG, PNG, PDF."})),
                    )
                        .into_response();
                }
            }
            // Chunked read with hard cap — reply path previously had NO size
            // limit at all. Capped at the same 5 MB used by the submit path so
            // a single field cannot buffer the full 25 MB request body.
            let mut field = field;
            match crate::storage::upload_helpers::read_field_capped(
                &mut field,
                SUPPORT_ATTACHMENT_MAX_BYTES,
                "attachment",
            )
            .await
            {
                Ok(data) if !data.is_empty() => file_bytes = Some(data),
                Ok(_) => {}
                Err(e) => return e.into_response(),
            }
        }
    }

    if message.trim().len() < 2 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Message is required (min 2 characters)"})),
        )
            .into_response();
    }

    if let Err(resp) = require_support_rate_limit(&state, user.id, "reply").await {
        return resp;
    }

    match service::reply_to_ticket(
        &state, user.id, &ticket_id, &message, file_bytes, file_name, file_type,
    )
    .await
    {
        Ok(_) => Json(serde_json::json!({ "status": "success", "message": "Reply added" }))
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// PUT /api/support/tickets/:ticket_id/reopen — Reopen a resolved/closed ticket.
pub async fn api_support_ticket_reopen(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(ticket_id): Path<String>,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    if let Err(resp) = require_support_rate_limit(&state, user.id, "reopen").await {
        return resp;
    }

    match service::reopen_ticket(&state, user.id, &ticket_id).await {
        Ok(_) => Json(serde_json::json!({ "status": "success", "message": "Ticket reopened" }))
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// PATCH /api/support/tickets/:ticket_id/csat — Store user satisfaction rating.
pub async fn api_support_ticket_csat(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(ticket_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let rating = match payload.get("rating").and_then(|v| v.as_str()) {
        Some(r) if r == "good" || r == "bad" => r.to_string(),
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "rating must be 'good' or 'bad'"})),
            )
                .into_response()
        }
    };

    // Verify ownership and store rating in ticket metadata
    let result = sqlx::query(
        r#"UPDATE support_tickets
           SET metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{csat}', $1::jsonb, true),
               updated_at = NOW()
           WHERE id = $2::uuid AND user_id = $3"#,
    )
    .bind(serde_json::json!(rating))
    .bind(&ticket_id)
    .bind(user.id)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({"status": "ok"})).into_response(),
        Ok(_) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Ticket not found"})),
        )
            .into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Database error"})),
        )
            .into_response(),
    }
}

/// GET /support — Render the support dashboard page.
pub async fn page_support(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "support.html").await
}
