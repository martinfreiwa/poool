/// Community Audit Log — immutable logging for all admin actions in the community system.
///
/// Usage:
/// ```text
/// community_audit::log(&c_pool, admin_id, "post.hide", "post", Some(post_id), Some(target_user_id), details).await;
/// ```
use sqlx::PgPool;
use uuid::Uuid;

/// Log a community admin action. Fire-and-forget — errors are logged but don't fail the caller.
pub async fn log(
    pool: &PgPool,
    actor_user_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    target_user_id: Option<Uuid>,
    details: Option<serde_json::Value>,
) {
    let result = sqlx::query(
        r#"INSERT INTO community_audit_logs (actor_user_id, action, entity_type, entity_id, target_user_id, details)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(actor_user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(target_user_id)
    .bind(details.unwrap_or(serde_json::json!({})))
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!(
            "Failed to write community audit log: {} — action={}, entity={}:{:?}",
            e,
            action,
            entity_type,
            entity_id
        );
    }
}
