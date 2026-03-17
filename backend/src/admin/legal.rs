use super::extractors::AdminUser;
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

/// Thin wrappers so the router() can reference legal handlers that live in main.rs.
/// These call through to the identically-named functions in the outer scope.
/// TODO: move these to a proper `admin::legal` sub-module in Phase 5.
pub async fn api_admin_legal_get_version_handler(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT key, value FROM platform_settings WHERE key IN ('legal_terms_version', 'legal_privacy_version', 'legal_last_updated') ORDER BY key"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut settings = serde_json::Map::new();
    for row in &rows {
        let k: String = row.get("key");
        let v: String = row.get("value");
        settings.insert(k, serde_json::Value::String(v));
    }

    let total_consents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_consents")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let current_version = settings
        .get("legal_terms_version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0");

    let accepted_current: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM user_consents WHERE terms_version = $1",
    )
    .bind(current_version)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let pending_reaccept: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM users u
           WHERE u.status = 'active'
           AND NOT EXISTS (
               SELECT 1 FROM user_consents uc
               WHERE uc.user_id = u.id AND uc.terms_version = $1
           )"#,
    )
    .bind(current_version)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Json(serde_json::json!({
        "settings": settings,
        "stats": {
            "total_consents": total_consents,
            "accepted_current_version": accepted_current,
            "pending_reacceptance": pending_reaccept,
        }
    }))
}

/// POST /api/admin/legal/version — Update legal document versions.
/// Body: `{ "legal_terms_version": "2.0", "legal_privacy_version": "1.1", "legal_last_updated": "..." }`
pub async fn api_admin_legal_update_version_handler(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let actor_id = Some(_admin.user.id);

    let updatable_keys = [
        "legal_terms_version",
        "legal_privacy_version",
        "legal_last_updated",
    ];
    let mut updated = Vec::new();

    if let Some(obj) = payload.as_object() {
        for key in &updatable_keys {
            if let Some(new_val) = obj.get(*key).and_then(|v| v.as_str()) {
                let old_val: Option<String> =
                    sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = $1")
                        .bind(key)
                        .fetch_optional(&state.db)
                        .await
                        .ok()
                        .flatten();

                let result = sqlx::query(
                    r#"INSERT INTO platform_settings (key, value, value_type, updated_at, updated_by)
                       VALUES ($1, $2, 'string', NOW(), $3)
                       ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW(), updated_by = $3"#
                )
                .bind(key)
                .bind(new_val)
                .bind(actor_id)
                .execute(&state.db)
                .await;

                if result.is_ok() {
                    let _ = sqlx::query(
                        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, previous_state, new_state)
                           VALUES ($1, 'legal_document.updated', 'platform_settings', $2::jsonb, $3::jsonb)"#
                    )
                    .bind(actor_id)
                    .bind(serde_json::json!({"key": key, "version": old_val}).to_string())
                    .bind(serde_json::json!({"key": key, "version": new_val}).to_string())
                    .execute(&state.db)
                    .await;

                    updated.push(key.to_string());
                }
            }
        }
    }

    Json(serde_json::json!({
        "status": "success",
        "updated_keys": updated,
        "message": "Legal version updated. Users who haven't accepted the new version will see a re-acceptance prompt."
    }))
}
