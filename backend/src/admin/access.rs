//! Role-Based Access Control (RBAC) and admin invitation handlers.
use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::{IntoResponse, Response},
};

/// GET /api/admin/admins - List all admin users with their roles and permissions.
pub async fn api_admin_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "admins.manage").await?;

    let users = sqlx::query!(
        r#"
        SELECT
            u.id,
            u.email,
            u.status,
            u.created_at,
            (
                SELECT array_agg(r.name)
                FROM user_roles ur
                JOIN roles r ON r.id = ur.role_id
                WHERE ur.user_id = u.id
            ) as "roles",
            (
                SELECT MAX(created_at)
                FROM user_sessions
                WHERE user_id = u.id
            ) as last_active,
            (
                SELECT ip_address::text
                FROM user_sessions
                WHERE user_id = u.id
                ORDER BY created_at DESC
                LIMIT 1
            ) as last_ip,
            (
                SELECT COUNT(*)::int
                FROM user_sessions
                WHERE user_id = u.id
                  AND expires_at > NOW()
            ) as "session_count",
            COALESCE(us.totp_enabled, FALSE) as "totp_enabled!: bool",
            up.first_name,
            up.last_name
        FROM users u
        LEFT JOIN user_settings us ON us.user_id = u.id
        LEFT JOIN user_profiles up ON up.user_id = u.id
        WHERE EXISTS (
            SELECT 1 FROM user_roles ur
            JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = u.id
            AND r.name NOT IN ('investor', 'developer')
        )
        ORDER BY u.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await;

    let rows = users.map_err(|e| {
        tracing::error!("Failed to fetch admins: {}", e);
        ApiError::Database(e)
    })?;
    let admins: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "email": r.email,
                "first_name": r.first_name,
                "last_name": r.last_name,
                "roles": r.roles.unwrap_or_default(),
                "status": r.status,
                "totp_enabled": r.totp_enabled,
                "last_active": r.last_active,
                "last_ip": r.last_ip,
                "session_count": r.session_count.unwrap_or(0),
                "created_at": r.created_at,
            })
        })
        .collect();
    Ok(Json(admins).into_response())
}

/// GET /api/admin/roles - List all admin roles and their associated permissions.
pub async fn api_roles_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "roles.edit").await?;

    let roles = sqlx::query!(
        r#"
        SELECT 
            r.id, 
            r.name, 
            r.description,
            (
                SELECT array_agg(permission)
                FROM admin_permissions
                WHERE role_id = r.id
            ) as "permissions"
        FROM roles r
        WHERE r.name NOT IN ('investor', 'developer')
        ORDER BY r.name ASC
        "#
    )
    .fetch_all(&state.db)
    .await;

    let rows = roles.map_err(|e| {
        tracing::error!("Failed to fetch roles: {}", e);
        ApiError::Database(e)
    })?;
    let roles_data: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "description": r.description,
                "permissions": r.permissions.unwrap_or_default(),
            })
        })
        .collect();
    Ok(Json(roles_data).into_response())
}

/// GET /api/admin/permissions - Get a static list of all available system permissions.
pub async fn api_permissions_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "roles.edit").await?;

    let permissions = vec![
        "users.view",
        "users.edit",
        "users.delete",
        "kyc.read",
        "kyc.write",
        "kyc.override",
        "treasury.read",
        "treasury.write",
        "financials.payout.draft",
        "financials.payout.approve",
        "assets.view",
        "assets.create",
        "assets.edit",
        "assets.publish",
        "submissions.review",
        "submissions.approve",
        "support.read",
        "support.write",
        "support.manage",
        "audit.read",
        "reports.generate",
        "settings.view",
        "settings.edit",
        "blog.view",
        "blog.edit",
        "blog.publish",
        "blog.archive",
        "blog.import",
        "blog.manage",
        "community.view",
        "community.manage",
        "blockchain.manage",
        "blockchain.tokenize",
        "emails.view",
        "emails.edit",
        "emails.send",
        "admins.manage",
        "roles.edit",
        "pii.view",
        "all",
    ];

    Ok(Json(permissions).into_response())
}

/// Payload for inviting a new admin user.
#[derive(serde::Deserialize)]
pub struct InviteAdminPayload {
    /// Email of the user to invite.
    pub email: String,
    /// Role name to assign to the new user.
    pub role: String,
}

/// POST /api/admin/admins/invite - Invite a new admin user.
pub async fn api_admin_invite(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<InviteAdminPayload>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "admins.manage").await?;

    // Allowlist: reject free-text role strings that would escalate to super_admin.
    if !crate::admin::extractors::ASSIGNABLE_ROLES.contains(&payload.role.as_str()) {
        return Err(ApiError::BadRequest("Invalid role".to_string()));
    }

    // Elevated roles (admin, super_admin) require super_admin to grant.
    if crate::admin::extractors::ELEVATED_ROLES.contains(&payload.role.as_str())
        && !admin.is_super_admin(&state.db).await
    {
        return Err(ApiError::Forbidden(
            "Only super_admin may grant admin/super_admin roles".to_string(),
        ));
    }

    // Check if role exists
    let role_id = sqlx::query_scalar!("SELECT id FROM roles WHERE name = $1", payload.role)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::BadRequest("Invalid role".to_string()))?;

    let user = &admin.user;

    let token = uuid::Uuid::new_v4().to_string();
    let token_hash = crate::config::hash_token(&token);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    sqlx::query!(
        "INSERT INTO admin_invitations (email, role_id, invited_by, token_hash, expires_at) VALUES ($1, $2, $3, $4, $5)",
        payload.email,
        role_id,
        user.id,
        token_hash,
        expires_at
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create admin invitation: {}", e);
        ApiError::Database(e)
    })?;

    // Log the invitation
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'admin.invite', 'admin_invitation', $2)"#,
    )
    .bind(user.id)
    .bind(serde_json::json!({"email": payload.email, "role": payload.role}))
    .execute(&state.db)
    .await;

    let subject = "You have been invited to be a POOOL Admin";
    let body = format!(
        r#"
        <h2>Admin Invitation</h2>
        <p>You have been invited to join the POOOL admin dashboard as: <strong>{}</strong>.</p>
        <p>Please click the link below to accept the invitation:</p>
        <p><a href="{}/auth/admin/accept-invite?token={}">Accept Invitation</a></p>
        "#,
        payload.role, state.config.base_url, token
    );

    let _ = crate::common::email::send_email(&payload.email, subject, &body).await;

    tracing::info!(
        "Admin invitation sent to {} with token {}",
        payload.email,
        token
    );
    Ok(
        Json(serde_json::json!({"status": "success", "message": "Invitation created"}))
            .into_response(),
    )
}

/// Represents a role and its new list of permissions.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct RolePermissionUpdate {
    /// Unique ID of the role.
    pub id: uuid::Uuid,
    /// List of permission names to assign.
    pub permissions: Vec<String>,
}

/// Payload for bulk updating role permissions.
#[derive(serde::Deserialize)]
pub struct BulkRoleUpdatePayload {
    /// List of role updates.
    pub roles: Vec<RolePermissionUpdate>,
}

/// POST /api/admin/roles/permissions - Bulk update role permissions.
pub async fn api_roles_update_permissions(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<BulkRoleUpdatePayload>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "roles.edit").await?;

    let user = &admin.user;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("TX begin: {}", e)))?;

    let audit_data = serde_json::to_value(&payload.roles).unwrap_or_default();

    let mut affected_role_ids: Vec<uuid::Uuid> = Vec::with_capacity(payload.roles.len());

    for role_update in payload.roles {
        affected_role_ids.push(role_update.id);

        sqlx::query!(
            "DELETE FROM admin_permissions WHERE role_id = $1",
            role_update.id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to clear permissions for role {}: {}",
                role_update.id,
                e
            );
            ApiError::Database(e)
        })?;

        for perm in role_update.permissions {
            sqlx::query!(
                "INSERT INTO admin_permissions (role_id, permission) VALUES ($1, $2)",
                role_update.id,
                perm
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to insert permission {} for role {}: {}",
                    perm,
                    role_update.id,
                    e
                );
                ApiError::Database(e)
            })?;
        }
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'roles.bulk_update', 'permissions', $2)"#,
    )
    .bind(user.id)
    .bind(audit_data)
    .execute(&mut *tx)
    .await;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Commit: {}", e)))?;

    // Invalidate sessions for every user holding an affected role so their
    // cached permission set is refreshed from the DB on next request.
    let affected = sqlx::query(
        r#"DELETE FROM user_sessions
            WHERE user_id IN (
                SELECT user_id FROM user_roles
                WHERE role_id = ANY($1) AND is_active = TRUE
            )"#,
    )
    .bind(&affected_role_ids)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to invalidate sessions after role-perm change on {:?}: {}",
            affected_role_ids,
            e
        );
        ApiError::Database(e)
    })?;
    tracing::info!(
        affected_roles = ?affected_role_ids,
        sessions_invalidated = affected.rows_affected(),
        "Invalidated user sessions following role-permission update"
    );

    Ok(Json(serde_json::json!({"status": "success"})).into_response())
}

/// Payload for creating a new role.
#[derive(serde::Deserialize)]
pub struct CreateRolePayload {
    /// Name of the new role (snake_case).
    pub name: String,
    /// Optional human-readable description of the role.
    pub description: Option<String>,
    /// Optional initial permissions to assign.
    pub permissions: Option<Vec<String>>,
}

/// POST /api/admin/roles - Create a new admin role.
pub async fn api_roles_create(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "roles.edit").await?;

    let user = &admin.user;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("TX begin: {}", e)))?;

    let role_id = sqlx::query_scalar!(
        "INSERT INTO roles (name, description) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING RETURNING id",
        payload.name,
        payload.description.unwrap_or_default()
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create role: {}", e);
        ApiError::Database(e)
    })?
    .ok_or_else(|| ApiError::Conflict("A role with this name already exists".to_string()))?;

    if let Some(perms) = payload.permissions {
        for perm in &perms {
            let _ = sqlx::query!(
                "INSERT INTO admin_permissions (role_id, permission) VALUES ($1, $2)",
                role_id,
                perm
            )
            .execute(&mut *tx)
            .await;
        }
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'role.create', 'role', $2)"#,
    )
    .bind(user.id)
    .bind(serde_json::json!({"name": payload.name, "role_id": role_id}))
    .execute(&mut *tx)
    .await;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Commit: {}", e)))?;

    Ok(Json(serde_json::json!({"status": "success", "role_id": role_id})).into_response())
}

/// GET /api/admin/admins/invitations - List all pending admin invitations.
pub async fn api_admin_invitations_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "admins.manage").await?;

    let invitations = sqlx::query!(
        r#"
        SELECT 
            ai.id,
            ai.email,
            r.name as role,
            ai.created_at,
            ai.expires_at,
            u.email as "invited_by_email"
        FROM admin_invitations ai
        JOIN roles r ON r.id = ai.role_id
        LEFT JOIN users u ON u.id = ai.invited_by
        WHERE ai.expires_at > NOW()
          AND ai.accepted_at IS NULL
        ORDER BY ai.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await;

    match invitations {
        Ok(rows) => {
            let data: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "email": r.email,
                        "role": r.role,
                        "created_at": r.created_at,
                        "expires_at": r.expires_at,
                        "invited_by": r.invited_by_email,
                    })
                })
                .collect();
            Ok(Json(data).into_response())
        }
        Err(e) => {
            tracing::error!("Failed to fetch invitations: {}", e);
            Ok(Json(serde_json::json!([])).into_response())
        }
    }
}

/// DELETE /api/admin/admins/invitations/:id - Revoke a pending invitation.
pub async fn api_admin_invitation_revoke(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(invite_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "admins.manage").await?;

    let r = sqlx::query!("DELETE FROM admin_invitations WHERE id = $1", invite_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to revoke invitation: {}", e);
            ApiError::Database(e)
        })?;

    if r.rows_affected() == 0 {
        return Err(ApiError::NotFound("Invitation not found".to_string()));
    }
    Ok(Json(serde_json::json!({"status": "success"})).into_response())
}

/// POST /api/admin/admins/invitations/:id/resend - Resend an invitation email.
pub async fn api_admin_invitation_resend(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(invite_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Response, ApiError> {
    admin.require_permission(&state.db, "admins.manage").await?;

    let record = sqlx::query!(
        "SELECT email FROM admin_invitations WHERE id = $1",
        invite_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to resend invitation: {}", e);
        ApiError::Database(e)
    })?
    .ok_or_else(|| ApiError::NotFound("Invitation not found".to_string()))?;

    let _ = sqlx::query!(
        "UPDATE admin_invitations SET expires_at = NOW() + INTERVAL '7 days' WHERE id = $1",
        invite_id
    )
    .execute(&state.db)
    .await;

    let new_token = uuid::Uuid::new_v4().to_string();
    let new_token_hash = crate::config::hash_token(&new_token);
    let _ = sqlx::query!(
        "UPDATE admin_invitations SET token_hash = $1 WHERE id = $2",
        new_token_hash,
        invite_id
    )
    .execute(&state.db)
    .await;

    let subject = "You have been invited to be a POOOL Admin";
    let body = format!(
        r#"
        <h2>Admin Invitation</h2>
        <p>You have been invited to join the POOOL admin dashboard.</p>
        <p>Please click the link below to accept the invitation:</p>
        <p><a href="{}/auth/admin/accept-invite?token={}">Accept Invitation</a></p>
        "#,
        state.config.base_url, new_token
    );
    let _ = crate::common::email::send_email(&record.email, subject, &body).await;

    tracing::info!("Resent invitation to {}", record.email);
    Ok(
        Json(serde_json::json!({"status": "success", "message": "Invitation resent"}))
            .into_response(),
    )
}
