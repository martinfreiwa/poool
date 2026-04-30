use super::db;
use super::models::SupportTicketWithReplies;
use crate::common::sanitize;
use crate::AppState;
use anyhow::Result;
use uuid::Uuid;

/// Valid priority values accepted by the system.
const VALID_PRIORITIES: &[&str] = &["low", "normal", "high", "urgent"];
/// Valid category values accepted by the system.
const VALID_CATEGORIES: &[&str] = &[
    "general",
    "account",
    "deposits",
    "investments",
    "kyc",
    "technical",
    "billing",
    "other",
];

/// Maximum attachment size: 5 MB.
const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;

/// Lists tickets for a user with their replies (batch-loaded in a single query).
pub async fn list_tickets(
    state: &AppState,
    user_id: Uuid,
) -> Result<Vec<SupportTicketWithReplies>, anyhow::Error> {
    let tickets = db::list_user_tickets(&state.db, user_id).await?;

    if tickets.is_empty() {
        return Ok(Vec::new());
    }

    // Batch-load all replies in one query instead of N+1
    let ticket_ids: Vec<String> = tickets.iter().map(|t| t.id.clone()).collect();
    let mut replies_map = db::get_replies_for_tickets(&state.db, &ticket_ids).await?;

    let result = tickets
        .into_iter()
        .map(|ticket| {
            let replies = replies_map.remove(&ticket.id).unwrap_or_default();
            SupportTicketWithReplies { ticket, replies }
        })
        .collect();

    Ok(result)
}

/// Validates priority value. Returns sanitized value or error.
fn validate_priority(priority: &str) -> Result<&str, String> {
    let p = priority.trim().to_lowercase();
    if VALID_PRIORITIES.contains(&p.as_str()) {
        Ok(VALID_PRIORITIES
            .iter()
            .find(|&&v| v == p)
            .expect("just validated"))
    } else {
        Err(format!(
            "Invalid priority '{}'. Must be one of: {}",
            priority,
            VALID_PRIORITIES.join(", ")
        ))
    }
}

/// Validates category value. Returns sanitized value or error.
fn validate_category(category: &str) -> Result<&str, String> {
    let c = category.trim().to_lowercase();
    if VALID_CATEGORIES.contains(&c.as_str()) {
        Ok(VALID_CATEGORIES
            .iter()
            .find(|&&v| v == c)
            .expect("just validated"))
    } else {
        Err(format!(
            "Invalid category '{}'. Must be one of: {}",
            category,
            VALID_CATEGORIES.join(", ")
        ))
    }
}

fn attachment_signature_matches(mime: &str, bytes: &[u8]) -> bool {
    match mime {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        _ => false,
    }
}

/// Submits a new support ticket (with context and attachments) and notifies admins.
#[allow(clippy::too_many_arguments)]
pub async fn submit_ticket(
    state: &AppState,
    user_id: Uuid,
    subject: &str,
    message: &str,
    priority: &str,
    category: &str,
    context: &str,
    file_bytes: Option<Vec<u8>>,
    file_name: Option<String>,
    file_type: Option<String>,
) -> Result<(), anyhow::Error> {
    // Validate priority and category before hitting DB
    let priority =
        validate_priority(priority).map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;
    let category =
        validate_category(category).map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;

    // Validate file size server-side
    if let Some(ref bytes) = file_bytes {
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(anyhow::anyhow!(
                "Attachment too large. Maximum size is 5MB, got {}MB.",
                bytes.len() / (1024 * 1024)
            ));
        }
        let mime = file_type
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Attachment content type is required"))?;
        if !attachment_signature_matches(mime, bytes) {
            return Err(anyhow::anyhow!(
                "Attachment file content does not match the declared file type"
            ));
        }
    }

    // 1. Fetch Backend Context (KYC, Balances)
    let user_ctx = db::get_user_context(&state.db, user_id)
        .await
        .unwrap_or(serde_json::json!({}));

    // Combine Client Context
    let combined_context = serde_json::json!({
        "client": serde_json::from_str::<serde_json::Value>(context).unwrap_or(serde_json::json!({})),
        "backend": user_ctx
    });

    // 2. Insert the Ticket (returns ticket_id AND reply_id)
    let sanitized_subject = sanitize::sanitize_text(subject);
    let sanitized_message = sanitize::sanitize_multiline(message);

    let mut uploaded_attachment: Option<(String, String)> = None;
    if let Some(bytes) = file_bytes {
        let mime = file_type
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Attachment content type is required"))?;
        let bucket = state
            .config
            .gcs_bucket
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Attachment upload is not configured"))?;
        let ext = crate::storage::service::extension_for_mime(mime);
        let fname = file_name
            .as_deref()
            .unwrap_or("attachment")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let object_path = format!("support/pending/{}_{}.{}", Uuid::new_v4(), fname, ext);

        let file_url = crate::storage::service::upload_private(bucket, &object_path, bytes, mime)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload support attachment: {}", e);
                anyhow::anyhow!("Attachment upload failed. Please try again.")
            })?;
        uploaded_attachment = Some((file_url, mime.to_string()));
    }

    let attachment_ref = uploaded_attachment
        .as_ref()
        .map(|(file_url, mime)| (file_url.as_str(), mime.as_str()));

    let (_ticket_id, _reply_id) = db::create_ticket_v2(
        &state.db,
        user_id,
        &sanitized_subject,
        &sanitized_message,
        priority,
        category,
        &combined_context,
        attachment_ref,
    )
    .await?;

    let _ = db::notify_admins_of_ticket(&state.db, subject).await;

    // Email admins: fetch their user_ids and send notification
    let user_email = sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let admin_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        r#"SELECT u.id FROM users u
           JOIN user_roles ur ON u.id = ur.user_id
           JOIN roles r ON ur.role_id = r.id
           WHERE r.name IN ('admin', 'super_admin')"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for admin_id in admin_ids {
        let _ = crate::email::trigger_transactional_email(
            &state.db,
            &admin_id,
            "support_ticket_new",
            serde_json::json!({
                "ticket_subject": subject,
                "user_email": user_email,
                "priority": priority,
            }),
        )
        .await;
    }

    Ok(())
}

/// Adds a reply to a ticket after validating ownership and status.
/// Optionally uploads a file attachment to GCS and links it to the reply.
pub async fn reply_to_ticket(
    state: &AppState,
    user_id: Uuid,
    ticket_id: &str,
    message: &str,
    file_bytes: Option<Vec<u8>>,
    file_name: Option<String>,
    file_type: Option<String>,
) -> Result<(), String> {
    let status = db::check_ticket_ownership(&state.db, ticket_id, user_id)
        .await
        .map_err(|_| "Database error".to_string())?
        .ok_or_else(|| "Ticket not found".to_string())?;

    if status != "open" && status != "in_progress" && status != "waiting_on_customer" {
        return Err("Cannot reply to a closed/resolved ticket. Reopen it first.".to_string());
    }

    let author_name = db::get_user_display_name(&state.db, user_id).await;
    let sanitized_message = sanitize::sanitize_multiline(message);

    let reply_id = db::add_reply(
        &state.db,
        ticket_id,
        user_id,
        &author_name,
        &sanitized_message,
    )
    .await
    .map_err(|e| format!("Failed to add reply: {}", e))?;

    // Upload attachment if provided
    if let Some(bytes) = file_bytes {
        let mime = file_type.as_deref().unwrap_or("application/octet-stream");
        if let Some(bucket) = state.config.gcs_bucket.as_deref() {
            let ext = crate::storage::service::extension_for_mime(mime);
            let fname = file_name
                .as_deref()
                .unwrap_or("attachment")
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
                .collect::<String>();
            let object_path = format!("support/replies/{}_{}.{}", Uuid::new_v4(), fname, ext);
            match crate::storage::service::upload_private(bucket, &object_path, bytes, mime).await {
                Ok(file_url) => {
                    if let Err(e) = db::add_reply_attachment(&state.db, reply_id, &file_url, mime).await {
                        tracing::warn!("Failed to save reply attachment record: {}", e);
                    }
                }
                Err(e) => tracing::warn!("Failed to upload reply attachment: {}", e),
            }
        }
    }

    Ok(())
}

/// Reopens a closed/resolved ticket.
pub async fn reopen_ticket(state: &AppState, user_id: Uuid, ticket_id: &str) -> Result<(), String> {
    let status = db::check_ticket_ownership(&state.db, ticket_id, user_id)
        .await
        .map_err(|_| "Database error".to_string())?
        .ok_or_else(|| "Ticket not found".to_string())?;

    if status != "resolved" && status != "closed" {
        return Err("Ticket is not resolved/closed".to_string());
    }

    db::reopen_ticket(&state.db, ticket_id, user_id)
        .await
        .map_err(|e| format!("Failed to reopen ticket: {}", e))?;

    Ok(())
}
