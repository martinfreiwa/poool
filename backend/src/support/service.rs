use super::db;
use super::models::SupportTicketWithReplies;
use crate::AppState;
use anyhow::Result;
use uuid::Uuid;

/// Lists tickets for a user with their replies.
pub async fn list_tickets(
    state: &AppState,
    user_id: Uuid,
) -> Result<Vec<SupportTicketWithReplies>, anyhow::Error> {
    let tickets = db::list_user_tickets(&state.db, user_id).await?;
    let mut result = Vec::new();

    for ticket in tickets {
        let replies = db::get_ticket_replies(&state.db, &ticket.id).await?;
        result.push(SupportTicketWithReplies { ticket, replies });
    }

    Ok(result)
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
    _file_name: Option<String>,
    file_type: Option<String>,
) -> Result<(), anyhow::Error> {
    // 1. Fetch Backend Context (KYC, Balances)
    let user_ctx = db::get_user_context(&state.db, user_id)
        .await
        .unwrap_or(serde_json::json!({}));

    // Combine Client Context
    let combined_context = serde_json::json!({
        "client": serde_json::from_str::<serde_json::Value>(context).unwrap_or(serde_json::json!({})),
        "backend": user_ctx
    });

    // 2. Insert the Ticket
    let ticket_id = db::create_ticket_v2(
        &state.db,
        user_id,
        subject,
        message,
        priority,
        category,
        &combined_context,
    )
    .await?;

    // 3. Handle File Upload if present
    if let Some(bytes) = file_bytes {
        if let Some(mime) = file_type {
            let ext = crate::storage::service::extension_for_mime(&mime);
            let object_path = format!("support/{}/{}.{}", ticket_id, Uuid::new_v4(), ext);

            let bucket = state.config.gcs_bucket.as_deref().unwrap_or("poool-bucket");
            match crate::storage::service::upload_private(bucket, &object_path, bytes, &mime).await
            {
                Ok(file_url) => {
                    let _ =
                        db::add_ticket_attachment(&state.db, &ticket_id, user_id, &file_url, &mime)
                            .await;
                }
                Err(e) => tracing::error!("Failed to upload support attachment: {}", e),
            }
        }
    }

    let _ = db::notify_admins_of_ticket(&state.db, subject).await;
    Ok(())
}

/// Adds a reply to a ticket after validating ownership and status.
pub async fn reply_to_ticket(
    state: &AppState,
    user_id: Uuid,
    ticket_id: &str,
    message: &str,
) -> Result<(), String> {
    let status = db::check_ticket_ownership(&state.db, ticket_id, user_id)
        .await
        .map_err(|_| "Database error".to_string())?
        .ok_or_else(|| "Ticket not found".to_string())?;

    if status != "open" && status != "in_progress" {
        return Err("Cannot reply to a closed/resolved ticket. Reopen it first.".to_string());
    }

    let author_name = db::get_user_display_name(&state.db, user_id).await;

    db::add_reply(&state.db, ticket_id, user_id, &author_name, message)
        .await
        .map_err(|e| format!("Failed to add reply: {}", e))?;

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
