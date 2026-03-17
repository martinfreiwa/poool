use super::models::{SupportTicket, SupportTicketReply};
use sqlx::PgPool;
use uuid::Uuid;

/// Fetches up to 50 tickets for a specific user, ordered by creation date.
pub async fn list_user_tickets(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<SupportTicket>, sqlx::Error> {
    sqlx::query_as::<_, SupportTicket>(
        r#"SELECT id::text, subject, message, priority, status, created_at::text, updated_at::text
           FROM support_tickets WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Fetches all replies for a given ticket ID, ordered chronologically.
pub async fn get_ticket_replies(
    pool: &PgPool,
    ticket_id: &str,
) -> Result<Vec<SupportTicketReply>, sqlx::Error> {
    sqlx::query_as::<_, SupportTicketReply>(
        r#"SELECT r.content as message,
                  (r.author_role IN ('admin', 'agent')) as is_admin,
                  r.author_name,
                  r.created_at::text,
                  COALESCE(
                      (SELECT json_agg(json_build_object('file_url', a.file_url, 'file_type', a.file_type)) 
                       FROM support_ticket_attachments a WHERE a.reply_id = r.id), 
                      '[]'::json
                  ) as attachments_json
           FROM support_ticket_replies r 
           WHERE r.ticket_id = $1::uuid 
           ORDER BY r.created_at ASC"#
    )
    .bind(ticket_id)
    .fetch_all(pool)
    .await
}

/// Checks if a ticket belongs to a user and returns its current status.
pub async fn check_ticket_ownership(
    pool: &PgPool,
    ticket_id: &str,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    use sqlx::Row;
    sqlx::query(r#"SELECT status FROM support_tickets WHERE id = $1::uuid AND user_id = $2"#)
        .bind(ticket_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map(|opt| opt.map(|row| row.get("status")))
}

/// Retrieves the profile display name of a user.
pub async fn get_user_display_name(pool: &PgPool, user_id: Uuid) -> String {
    sqlx::query_scalar(
        "SELECT COALESCE(first_name || ' ' || last_name, 'User') FROM user_profiles WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "User".to_string())
}

/// Inserts a new support ticket into the database.
pub async fn create_ticket(
    pool: &PgPool,
    user_id: Uuid,
    subject: &str,
    message: &str,
    priority: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO support_tickets (user_id, subject, message, priority) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(subject)
    .bind(message)
    .bind(priority)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn get_user_context(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<serde_json::Value, sqlx::Error> {
    use sqlx::Row;
    let kyc_status: String = sqlx::query_scalar(
        "SELECT status FROM kyc_records WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "none".to_string());

    let balances = sqlx::query("SELECT wallet_type, balance_cents FROM wallets WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    let mut wallets_json = serde_json::Map::new();
    for row in balances {
        let w_type: String = row.get("wallet_type");
        let cents: i64 = row.get("balance_cents");
        wallets_json.insert(w_type, serde_json::json!(cents));
    }

    let txs = sqlx::query(
        "SELECT t.type, t.status, t.amount_cents, t.created_at FROM wallet_transactions t 
         JOIN wallets w ON t.wallet_id = w.id 
         WHERE w.user_id = $1 ORDER BY t.created_at DESC LIMIT 3",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut recent_txs = Vec::new();
    for row in txs {
        let t_type: String = row.get("type");
        let status: String = row.get("status");
        let amount: i64 = row.get("amount_cents");
        recent_txs.push(serde_json::json!({
            "type": t_type,
            "status": status,
            "amount_cents": amount
        }));
    }

    Ok(serde_json::json!({
        "kyc_status": kyc_status,
        "balances": wallets_json,
        "recent_transactions": recent_txs
    }))
}

pub async fn create_ticket_v2(
    pool: &PgPool,
    user_id: Uuid,
    subject: &str,
    message: &str,
    priority: &str,
    category: &str,
    metadata: &serde_json::Value,
) -> Result<String, sqlx::Error> {
    let breach_hours = match priority {
        "urgent" => 2,
        "high" => 12,
        _ => 24, // normal, low
    };

    let ticket_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO support_tickets (user_id, subject, message, priority, category, metadata, sla_breach_at) 
           VALUES ($1, $2, $3, $4, $5, $6, NOW() + make_interval(hours => CAST($7 AS INT))) RETURNING id"#
    )
    .bind(user_id)
    .bind(subject)
    .bind(message)
    .bind(priority)
    .bind(category)
    .bind(metadata)
    .bind(breach_hours)
    .fetch_one(pool)
    .await?;

    // Create the initial reply to anchor attachments
    let _ = add_initial_reply(pool, &ticket_id.to_string(), user_id, message).await?;

    Ok(ticket_id.to_string())
}

pub async fn add_initial_reply(
    pool: &PgPool,
    ticket_id: &str,
    author_id: Uuid,
    content: &str,
) -> Result<String, sqlx::Error> {
    let author_name = get_user_display_name(pool, author_id).await;

    let reply_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO support_ticket_replies (ticket_id, author_id, author_name, author_role, type, content)
           VALUES ($1::uuid, $2, $3, 'user', 'initial', $4) RETURNING id"#
    )
    .bind(ticket_id)
    .bind(author_id)
    .bind(&author_name)
    .bind(content)
    .fetch_one(pool)
    .await?;

    Ok(reply_id.to_string())
}

pub async fn add_ticket_attachment(
    pool: &PgPool,
    ticket_id: &str,
    user_id: Uuid,
    file_url: &str,
    file_type: &str,
) -> Result<(), sqlx::Error> {
    let reply_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM support_ticket_replies WHERE ticket_id = $1::uuid AND author_id = $2 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(ticket_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r_id) = reply_id {
        sqlx::query(
            "INSERT INTO support_ticket_attachments (reply_id, file_url, file_type) VALUES ($1, $2, $3)"
        )
        .bind(r_id)
        .bind(file_url)
        .bind(file_type)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Creates a system notification for admins about a new ticket.
pub async fn notify_admins_of_ticket(pool: &PgPool, subject: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO notifications (user_id, title, message, type, action_url)
           SELECT u.id, 'New Support Ticket', $1, 'system', '/admin/support.html'
           FROM users u JOIN user_roles ur ON u.id = ur.user_id
           JOIN roles r ON ur.role_id = r.id
           WHERE r.name IN ('admin', 'super_admin')"#,
    )
    .bind(format!("New support ticket: {}", subject))
    .execute(pool)
    .await
    .map(|_| ())
}

/// Inserts a reply into the database and updates the ticket's `updated_at` timestamp.
pub async fn add_reply(
    pool: &PgPool,
    ticket_id: &str,
    author_id: Uuid,
    author_name: &str,
    content: &str,
) -> Result<(), sqlx::Error> {
    let _ = sqlx::query(
        r#"INSERT INTO support_ticket_replies (ticket_id, author_id, author_name, author_role, type, content)
           VALUES ($1::uuid, $2, $3, 'user', 'reply', $4)"#
    )
    .bind(ticket_id)
    .bind(author_id)
    .bind(author_name)
    .bind(content)
    .execute(pool)
    .await;

    sqlx::query("UPDATE support_tickets SET updated_at = NOW() WHERE id = $1::uuid")
        .bind(ticket_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Updates a ticket's status back to 'open'.
pub async fn reopen_ticket(
    pool: &PgPool,
    ticket_id: &str,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE support_tickets SET status = 'open', updated_at = NOW() WHERE id = $1::uuid AND user_id = $2"
    )
    .bind(ticket_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map(|_| ())
}
