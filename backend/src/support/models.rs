//! Support domain models and data transfer objects.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a support ticket in the database.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SupportTicket {
    /// The unique identifier for the ticket.
    pub id: String,
    /// The subject line of the ticket.
    pub subject: String,
    /// The original message content.
    pub message: String,
    /// The priority level (e.g., normal, high).
    pub priority: String,
    /// The current status (e.g., open, closed).
    pub status: String,
    /// The ticket category (e.g., general, account, billing).
    pub category: Option<String>,
    /// ISO timestamp of when the ticket was created.
    pub created_at: String,
    /// ISO timestamp of the last update, if any.
    pub updated_at: Option<String>,
    /// JSONB metadata (includes csat field after rating).
    pub metadata: Option<serde_json::Value>,
}

/// Represents a single reply in a support ticket thread.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SupportTicketReply {
    /// The content of the reply message.
    pub message: String,
    /// Whether the author is an administrator/agent.
    pub is_admin: bool,
    /// The displayed name of the author.
    pub author_name: String,
    /// ISO timestamp of when the reply was created.
    pub created_at: String,
    /// JSON array of attachments.
    pub attachments_json: serde_json::Value,
}

/// A composite model containing a ticket and all its replies.
#[derive(Debug, Serialize, Deserialize)]
pub struct SupportTicketWithReplies {
    /// The base ticket information.
    #[serde(flatten)]
    pub ticket: SupportTicket,
    /// The list of replies associated with this ticket.
    pub replies: Vec<SupportTicketReply>,
}

/// Request payload for adding a reply to a ticket.
#[derive(Debug, Deserialize)]
pub struct AddReplyRequest {
    /// The content of the reply message.
    pub message: String,
}
