/// Immutable audit logging for compliance.
///
/// Every critical action is logged to the audit_logs table.
/// These records are NEVER updated or deleted (regulatory requirement).
use sqlx::PgPool;
use uuid::Uuid;

/// Log an auditable action. This should be called within the same DB transaction
/// as the action being audited, to guarantee atomicity.
pub async fn log(
    pool: &PgPool,
    actor_user_id: Option<Uuid>,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5::inet, $6)
        "#,
    )
    .bind(actor_user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await?;

    Ok(())
}
