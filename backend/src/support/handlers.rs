use super::models::AddReplyRequest;
use super::service;
use crate::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;

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

    let mut subject = String::new();
    let mut message = String::new();
    let mut priority = "normal".to_string();
    let mut category = "general".to_string();
    let mut context_str = String::new();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut file_type: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "attachment" {
            file_name = field.file_name().map(|s| s.to_string());
            file_type = field.content_type().map(|s| s.to_string());

            // Validate MIME type server-side
            if let Some(ref mime) = file_type {
                if !["image/png", "image/jpeg", "application/pdf"].contains(&mime.as_str()) {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "Invalid file type. Allowed: JPG, PNG, PDF."})),
                    )
                        .into_response();
                }
            }

            if let Ok(data) = field.bytes().await {
                if !data.is_empty() {
                    file_bytes = Some(data.to_vec());
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
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Subject and message are required"})),
        )
            .into_response();
    }

    // Validate lengths
    if subject.trim().len() < 5 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Subject must be at least 5 characters"})),
        )
            .into_response();
    }

    if message.trim().len() < 20 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
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
            if err_msg.contains("Validation error") || err_msg.contains("too large") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": err_msg})),
                )
                    .into_response()
            } else {
                tracing::error!("Failed to create ticket: {:?}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to create support ticket"})),
                )
                    .into_response()
            }
        }
    }
}

/// POST /api/support/tickets/:ticket_id/reply — Add a user reply to their own ticket.
pub async fn api_support_ticket_reply(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(ticket_id): Path<String>,
    Json(payload): Json<AddReplyRequest>,
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

    if payload.message.trim().len() < 2 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Message is required (min 2 characters)"})),
        )
            .into_response();
    }

    match service::reply_to_ticket(&state, user.id, &ticket_id, &payload.message).await {
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

/// GET /support — Render the support dashboard page.
pub async fn page_support(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "support.html").await
}
