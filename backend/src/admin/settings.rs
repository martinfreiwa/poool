use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::common::sanitize;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Settings API
//

/// GET /api/admin/settings  Load platform settings from DB + admin users list
pub async fn api_admin_get_settings(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "platform.manage").await?;
    // 1. Load all platform_settings from DB
    let setting_rows =
        sqlx::query("SELECT key, value, value_type FROM platform_settings ORDER BY key")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let mut settings = serde_json::Map::new();
    for row in &setting_rows {
        let k: String = row.get("key");
        let v: String = row.get("value");
        let vt: String = row.get("value_type");
        let parsed: serde_json::Value = match vt.as_str() {
            "boolean" => serde_json::Value::Bool(v == "true"),
            "number" => serde_json::Value::Number(
                serde_json::Number::from_f64(v.parse::<f64>().unwrap_or(0.0))
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            _ => serde_json::Value::String(v),
        };
        settings.insert(k, parsed);
    }

    // 2. Load admin users (users with admin-level roles)
    let admin_rows = sqlx::query(
        r#"SELECT u.id, u.email, 
                  COALESCE(up.first_name || ' ' || up.last_name, up.first_name, '') AS name,
                  r.name AS role_name, r.description AS role_description,
                  ur.granted_at::text
           FROM user_roles ur
           JOIN users u ON u.id = ur.user_id
           LEFT JOIN user_profiles up ON u.id = up.user_id
           JOIN roles r ON r.id = ur.role_id
           WHERE r.name IN ('admin', 'super_admin', 'compliance', 'support', 'finance')
             AND ur.is_active = TRUE
           ORDER BY ur.granted_at ASC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut admins: Vec<serde_json::Value> = Vec::new();
    for row in &admin_rows {
        let user_id: uuid::Uuid = row.get("id");
        let email: String = row.get("email");
        let name: String = row.get("name");
        let role_name: String = row.get("role_name");
        let _role_desc: Option<String> = row.get("role_description");
        let granted: String = row.get("granted_at");

        // Load permissions for this role
        let perm_rows = sqlx::query(
            "SELECT ap.permission FROM admin_permissions ap JOIN roles r ON r.id = ap.role_id WHERE r.name = $1"
        ).bind(&role_name).fetch_all(&state.db).await.unwrap_or_default();

        let perms: Vec<String> = perm_rows
            .iter()
            .map(|pr| pr.get::<String, _>("permission"))
            .collect();

        let display_name = if name.trim().is_empty() {
            email.split('@').next().unwrap_or("Admin").to_string()
        } else {
            name.clone()
        };

        let role_display = match role_name.as_str() {
            "super_admin" => "Super Admin",
            "admin" => "Admin",
            "compliance" => "Compliance Officer",
            "support" => "Support Agent",
            "finance" => "Finance Manager",
            _ => &role_name,
        };

        admins.push(serde_json::json!({
            "id": user_id,
            "name": display_name,
            "email": email,
            "role": role_name,
            "role_display": role_display,
            "permissions": perms,
            "created_at": &granted[..10.min(granted.len())]
        }));
    }

    Ok(Json(serde_json::json!({
        "settings": serde_json::Value::Object(settings),
        "admins": admins
    }))
    .into_response())
}

/// POST /api/admin/settings  Persist platform settings to DB
pub async fn api_admin_update_settings(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "platform.manage").await?;
    let user_id = admin.user.id;

    // Upsert each submitted setting
    if let Some(settings_obj) = body.as_object() {
        for (key, val) in settings_obj {
            let (str_val, val_type) = match val {
                serde_json::Value::Bool(b) => (b.to_string(), "boolean"),
                serde_json::Value::Number(n) => (n.to_string(), "number"),
                serde_json::Value::String(s) => (sanitize::sanitize_text(s), "string"),
                _ => (val.to_string(), "json"),
            };

            let _ = sqlx::query(
                r#"INSERT INTO platform_settings (key, value, value_type, updated_at, updated_by)
                   VALUES ($1, $2, $3, NOW(), $4)
                   ON CONFLICT (key)
                   DO UPDATE SET value = EXCLUDED.value, value_type = EXCLUDED.value_type, updated_at = NOW(), updated_by = EXCLUDED.updated_by"#
            )
            .bind(key)
            .bind(&str_val)
            .bind(val_type)
            .bind(user_id)
            .execute(&state.db)
            .await;
        }
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'admin.platform_settings_update', 'platform_settings', $2)"#,
    )
    .bind(user_id)
    .bind(&body)
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({"status": "updated"})).into_response())
}

/// POST /api/admin/settings/admins  Add a new admin user
///
/// Only super_admin may assign admin roles. Role must be in allowlist.
pub async fn api_admin_add_admin(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let email = body.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let role = body.get("role").and_then(|v| v.as_str()).unwrap_or("admin");

    if email.is_empty() {
        return Err(ApiError::BadRequest("Email is required".to_string()));
    }

    if !admin.is_super_admin(&state.db).await {
        return Err(ApiError::Forbidden(
            "Only super_admin may add admins".to_string(),
        ));
    }
    if !crate::admin::extractors::ASSIGNABLE_ROLES.contains(&role) {
        return Err(ApiError::BadRequest("Invalid role".to_string()));
    }

    // Find the user by email
    let user_row: Option<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let user_id = match user_row {
        Some((id,)) => id,
        None => {
            return Err(ApiError::NotFound(
                "User not found with that email".to_string(),
            ));
        }
    };

    // Find the role
    let role_row: Option<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM roles WHERE name = $1")
        .bind(role)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let role_uuid = match role_row {
        Some((id,)) => id,
        None => {
            return Err(ApiError::BadRequest("Invalid role".to_string()));
        }
    };

    let result = sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT (user_id, role_id) DO NOTHING"
    ).bind(user_id).bind(role_uuid).execute(&state.db).await;

    match result {
        Ok(_) => Ok(
            Json(serde_json::json!({"status":"added","user_id": user_id.to_string()}))
                .into_response(),
        ),
        Err(e) => {
            tracing::error!("Failed to add admin: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// DELETE /api/admin/settings/admins/:user_id  Remove admin role from user
///
/// Only super_admin may revoke admin roles. Cannot revoke own roles.
pub async fn api_admin_remove_admin(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    if !admin.is_super_admin(&state.db).await {
        return Err(ApiError::Forbidden(
            "Only super_admin may remove admins".to_string(),
        ));
    }
    let uid = ApiError::parse_uuid(&user_id)?;
    if admin.user.id == uid {
        return Err(ApiError::Forbidden(
            "Admins may not revoke their own roles".to_string(),
        ));
    }
    let result = sqlx::query(
        r#"DELETE FROM user_roles WHERE user_id = $1::uuid
           AND role_id IN (SELECT id FROM roles WHERE name IN ('admin', 'super_admin', 'compliance', 'support', 'finance'))"#
    ).bind(&user_id).execute(&state.db).await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Ok(Json(serde_json::json!({"status":"removed"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound("Admin role not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to remove admin {user_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// PATCH /api/admin/settings/admins/:user_id  Update admin role
///
/// Only super_admin may change admin roles. Cannot self-modify.
pub async fn api_admin_update_admin_role(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let new_role = body.get("role").and_then(|v| v.as_str()).unwrap_or("");
    if new_role.is_empty() {
        return Err(ApiError::BadRequest("Role is required".to_string()));
    }

    if !admin.is_super_admin(&state.db).await {
        return Err(ApiError::Forbidden(
            "Only super_admin may change admin roles".to_string(),
        ));
    }
    if !crate::admin::extractors::ASSIGNABLE_ROLES.contains(&new_role) {
        return Err(ApiError::BadRequest("Invalid role".to_string()));
    }
    let target_uid = ApiError::parse_uuid(&user_id)?;
    if admin.user.id == target_uid {
        return Err(ApiError::Forbidden(
            "Admins may not modify their own role".to_string(),
        ));
    }

    // Find the new role ID
    let role_row: Option<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM roles WHERE name = $1")
        .bind(new_role)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let role_uuid = match role_row {
        Some((id,)) => id,
        None => {
            return Err(ApiError::BadRequest("Invalid role".to_string()));
        }
    };

    let uid = ApiError::parse_uuid(&user_id)?;

    // Remove old admin roles and assign new one
    let _ = sqlx::query(
        r#"DELETE FROM user_roles WHERE user_id = $1
           AND role_id IN (SELECT id FROM roles WHERE name IN ('admin', 'super_admin', 'compliance', 'support', 'finance'))"#
    ).bind(uid).execute(&state.db).await;

    let result = sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT (user_id, role_id) DO NOTHING"
    ).bind(uid).bind(role_uuid).execute(&state.db).await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"updated"})).into_response()),
        Err(e) => {
            tracing::error!("Failed to update admin role: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// GET /api/admin/settings/roles  List available admin roles
pub async fn api_admin_list_roles(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query(
        "SELECT id::text, name, description FROM roles WHERE name IN ('admin', 'super_admin', 'compliance', 'support', 'finance') ORDER BY name"
    ).fetch_all(&state.db).await.unwrap_or_default();

    let roles: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "name": r.get::<String, _>("name"),
                "description": r.get::<Option<String>, _>("description")
            })
        })
        .collect();

    Ok(Json(serde_json::json!({"roles": roles})).into_response())
}

/// POST /api/admin/settings/maintenance  Toggle maintenance mode
pub async fn api_admin_toggle_maintenance(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    if !admin.is_super_admin(&state.db).await {
        return Err(ApiError::Forbidden(
            "Super admin required for maintenance mode".to_string(),
        ));
    }
    let enabled = body
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let user_id = admin.user.id;

    let _ = sqlx::query(
        r#"INSERT INTO platform_settings (key, value, value_type, updated_at, updated_by)
           VALUES ('maintenance_mode', $1, 'boolean', NOW(), $2)
           ON CONFLICT (key)
           DO UPDATE SET value = $1, updated_at = NOW(), updated_by = $2"#,
    )
    .bind(if enabled { "true" } else { "false" })
    .bind(user_id)
    .execute(&state.db)
    .await;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'admin.maintenance_mode_toggle', 'platform_settings', $2)"#,
    )
    .bind(user_id)
    .bind(serde_json::json!({ "enabled": enabled }))
    .execute(&state.db)
    .await;

    Ok(
        Json(serde_json::json!({"status": if enabled { "enabled" } else { "disabled" }}))
            .into_response(),
    )
}

/// POST /api/admin/maintenance/clear-cache  Clear system cache
pub async fn api_admin_clear_cache(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "platform.manage").await?;
    // Since we don't have a Redis instance in this mock, we just return success
    tracing::info!("Admin cleared system cache.");
    Ok(
        Json(serde_json::json!({"status":"success","message":"Cache cleared: 0 entries purged."}))
            .into_response(),
    )
}

/// POST /api/admin/maintenance/rotate-logs  Trigger log rotation
pub async fn api_admin_rotate_logs(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "platform.manage").await?;
    // Manual log rotation trigger (stub)
    tracing::info!("Admin triggered log rotation.");
    Ok(Json(
        serde_json::json!({"status":"success","message":"Log rotation initiated successfully."}),
    )
    .into_response())
}
