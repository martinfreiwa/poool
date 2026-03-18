use sqlx::PgPool;
use sqlx::Row;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Background task to monitor SLA breaches
pub async fn monitor_sla_breaches(pool: PgPool) {
    loop {
        // Find tickets that just breached their SLA
        let breached: Vec<sqlx::postgres::PgRow> = match sqlx::query(
            r#"SELECT id, subject, status, priority, sla_breach_at
               FROM support_tickets
               WHERE status IN ('open', 'in_progress')
                 AND sla_breach_at < NOW()
                 AND sla_alert_sent = false
               LIMIT 50"#,
        )
        .fetch_all(&pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("SLA monitor: failed to query breached tickets: {}", e);
                vec![]
            }
        };

        for bg in breached {
            let priority: String = bg.get("priority");
            let subject: String = bg.get("subject");
            let ticket_id: Uuid = bg.get("id");

            // Notify Admins
            let _ = sqlx::query(
                r#"INSERT INTO notifications (user_id, title, message, type, action_url)
                   SELECT u.id, 'SLA Breach: ' || $1, 'Ticket is overdue: ' || $2, 'system', '/admin/support'
                   FROM users u JOIN user_roles ur ON u.id = ur.user_id
                   JOIN roles r ON ur.role_id = r.id
                   WHERE r.name IN ('admin', 'super_admin')"#
            )
            .bind(&priority)
            .bind(&subject)
            .execute(&pool)
            .await;

            // Update as sent
            let _ = sqlx::query("UPDATE support_tickets SET sla_alert_sent = true WHERE id = $1")
                .bind(ticket_id)
                .execute(&pool)
                .await;
        }

        sleep(Duration::from_secs(60)).await;
    }
}
