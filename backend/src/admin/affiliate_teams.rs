//! Admin-Endpoints für Developer-Team-Affiliate-System (Phase 4).
//!
//! Gegated über `affiliates.team_manage`-Permission (siehe Migration 158).
//!
//! Routes:
//!   GET    /api/admin/affiliate-teams                              — List all teams + KPIs
//!   GET    /api/admin/affiliate-teams/:team_id                     — Team detail (members + counters + audit)
//!   POST   /api/admin/affiliate-teams/:team_id/suspend             — Status → paused
//!   POST   /api/admin/affiliate-teams/:team_id/resume              — Status → active
//!   POST   /api/admin/affiliate-teams/:team_id/terminate           — Status → terminated (hard)
//!   POST   /api/admin/affiliate-teams/members/:membership_id/remove — Admin-Override member-remove

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

// ── List ────────────────────────────────────────────────────────────────────

/// GET /api/admin/affiliate-teams — list all developer teams with KPI snapshots.
pub async fn api_admin_affiliate_teams_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.team_manage")
        .await?;

    let rows = sqlx::query!(
        r#"SELECT t.id, t.developer_user_id, t.display_name, t.public_slug, t.status,
                  t.created_at, t.terminated_at,
                  u.email::text AS developer_email,
                  COALESCE((SELECT COUNT(*) FROM developer_team_memberships m
                            WHERE m.team_id = t.id AND m.status = 'active'), 0) AS "active_members!",
                  COALESCE((SELECT COUNT(*) FROM developer_team_memberships m
                            WHERE m.team_id = t.id
                              AND m.status IN ('invited', 'pending_developer_approval')), 0) AS "pending_members!",
                  COALESCE((SELECT lifetime_commission_cents FROM affiliate_live_counters
                            WHERE payout_user_id = t.developer_user_id), 0) AS "lifetime_commission_cents!",
                  COALESCE((SELECT payable_commission_cents FROM affiliate_live_counters
                            WHERE payout_user_id = t.developer_user_id), 0) AS "payable_commission_cents!"
           FROM developer_teams t
           LEFT JOIN users u ON u.id = t.developer_user_id
           ORDER BY t.created_at DESC"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let teams: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "team_id": r.id,
                "developer_user_id": r.developer_user_id,
                "developer_email": r.developer_email,
                "display_name": r.display_name,
                "public_slug": r.public_slug,
                "status": r.status,
                "created_at": r.created_at,
                "terminated_at": r.terminated_at,
                "active_members": r.active_members,
                "pending_members": r.pending_members,
                "lifetime_commission_cents": r.lifetime_commission_cents,
                "payable_commission_cents": r.payable_commission_cents,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "teams": teams })).into_response())
}

// ── Detail ──────────────────────────────────────────────────────────────────

/// GET /api/admin/affiliate-teams/:team_id — team detail (members + counters + audit).
pub async fn api_admin_affiliate_team_detail(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(team_id): Path<Uuid>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.team_manage")
        .await?;

    let team = sqlx::query!(
        r#"SELECT t.id, t.developer_user_id, t.display_name, t.public_slug, t.status,
                  t.created_at, t.updated_at, t.terminated_at, t.terminated_reason,
                  u.email::text AS developer_email
           FROM developer_teams t
           LEFT JOIN users u ON u.id = t.developer_user_id
           WHERE t.id = $1"#,
        team_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    let members = sqlx::query!(
        r#"SELECT m.id, m.user_id, m.role, m.status, m.invited_at, m.joined_at, m.removed_at,
                  u.email::text AS email,
                  NULLIF(TRIM(BOTH ' ' FROM (
                      COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, '')
                  )), '') AS full_name,
                  al.code AS link_code, al.status AS link_status
           FROM developer_team_memberships m
           LEFT JOIN users u          ON u.id = m.user_id
           LEFT JOIN user_profiles up ON up.user_id = m.user_id
           LEFT JOIN affiliate_links al ON al.team_id = m.team_id
                                       AND al.attribution_user_id = m.user_id
                                       AND al.link_type = 'team_business'
           WHERE m.team_id = $1
           ORDER BY m.status DESC, m.created_at DESC"#,
        team_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let counters = sqlx::query!(
        r#"SELECT lifetime_revenue_cents, lifetime_commission_cents,
                  pending_commission_cents, payable_commission_cents,
                  paid_commission_cents, clawed_back_cents
           FROM affiliate_live_counters WHERE payout_user_id = $1"#,
        team.developer_user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let audit = sqlx::query!(
        r#"SELECT id, actor_user_id, action, entity_id, metadata, created_at
           FROM audit_logs
           WHERE entity_type IN ('developer_teams', 'developer_team_memberships', 'affiliate_links')
             AND (entity_id = $1
                  OR entity_id IN (SELECT id FROM developer_team_memberships WHERE team_id = $1)
                  OR entity_id IN (SELECT id FROM affiliate_links WHERE team_id = $1))
           ORDER BY created_at DESC
           LIMIT 50"#,
        team_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "team": {
            "team_id": team.id,
            "developer_user_id": team.developer_user_id,
            "developer_email": team.developer_email,
            "display_name": team.display_name,
            "public_slug": team.public_slug,
            "status": team.status,
            "created_at": team.created_at,
            "updated_at": team.updated_at,
            "terminated_at": team.terminated_at,
            "terminated_reason": team.terminated_reason,
        },
        "counters": counters.map(|c| serde_json::json!({
            "lifetime_revenue_cents":    c.lifetime_revenue_cents,
            "lifetime_commission_cents": c.lifetime_commission_cents,
            "pending_commission_cents":  c.pending_commission_cents,
            "payable_commission_cents":  c.payable_commission_cents,
            "paid_commission_cents":     c.paid_commission_cents,
            "clawed_back_cents":         c.clawed_back_cents,
        })),
        "members": members.into_iter().map(|m| serde_json::json!({
            "membership_id": m.id,
            "user_id": m.user_id,
            "email": m.email,
            "full_name": m.full_name,
            "role": m.role,
            "status": m.status,
            "invited_at": m.invited_at,
            "joined_at": m.joined_at,
            "removed_at": m.removed_at,
            "link_code": m.link_code,
            "link_status": m.link_status,
        })).collect::<Vec<_>>(),
        "audit": audit.into_iter().map(|a| serde_json::json!({
            "id": a.id,
            "actor_user_id": a.actor_user_id,
            "action": a.action,
            "entity_id": a.entity_id,
            "metadata": a.metadata,
            "created_at": a.created_at,
        })).collect::<Vec<_>>(),
    }))
    .into_response())
}

// ── Status transitions ──────────────────────────────────────────────────────

/// Body for team-status mutations (suspend/resume/terminate).
#[derive(Deserialize)]
pub struct StatusChangePayload {
    /// Free-text reason recorded in the audit log.
    pub reason: Option<String>,
}

async fn set_team_status(
    admin: &AdminUser,
    state: &AppState,
    team_id: Uuid,
    next_status: &str,
    reason: Option<&str>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.team_manage")
        .await?;

    let allowed = matches!(next_status, "active" | "paused" | "terminated");
    if !allowed {
        return Err(ApiError::BadRequest("Invalid status".into()));
    }

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;
    let row = sqlx::query!(
        r#"UPDATE developer_teams
           SET status = $2::varchar,
               terminated_at = CASE WHEN $2::varchar = 'terminated' THEN NOW() ELSE NULL END::timestamptz,
               terminated_reason = CASE WHEN $2::varchar = 'terminated' THEN $3::text ELSE NULL END,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, status, developer_user_id"#,
        team_id,
        next_status,
        reason
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Team not found".into()))?;

    // Bei terminate: alle Team-Business-Links deaktivieren
    if next_status == "terminated" {
        sqlx::query!(
            "UPDATE affiliate_links
             SET status = 'inactive', deactivated_at = NOW(),
                 deactivated_reason = 'team_terminated', updated_at = NOW()
             WHERE team_id = $1 AND link_type = 'team_business' AND status = 'active'",
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }

    tx.commit().await.map_err(ApiError::Database)?;

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, $2, 'developer_teams', $3, $4)",
    )
    .bind(admin.user.id)
    .bind(format!("team_status_{}", next_status))
    .bind(team_id)
    .bind(serde_json::json!({
        "next_status": next_status,
        "reason": reason,
    }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({"status": row.status, "team_id": row.id})).into_response())
}

/// POST /api/admin/affiliate-teams/:team_id/suspend — set team status to 'paused'.
pub async fn api_admin_affiliate_team_suspend(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<StatusChangePayload>,
) -> Result<axum::response::Response, ApiError> {
    set_team_status(&admin, &state, team_id, "paused", payload.reason.as_deref()).await
}

/// POST /api/admin/affiliate-teams/:team_id/resume — set team status back to 'active'.
pub async fn api_admin_affiliate_team_resume(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(team_id): Path<Uuid>,
) -> Result<axum::response::Response, ApiError> {
    set_team_status(&admin, &state, team_id, "active", None).await
}

/// POST /api/admin/affiliate-teams/:team_id/terminate — set status to 'terminated' and deactivate links.
pub async fn api_admin_affiliate_team_terminate(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<StatusChangePayload>,
) -> Result<axum::response::Response, ApiError> {
    set_team_status(
        &admin,
        &state,
        team_id,
        "terminated",
        payload.reason.as_deref(),
    )
    .await
}

// ── Admin-Override: Member entfernen ────────────────────────────────────────

/// Body for the admin-override member-remove endpoint.
#[derive(Deserialize)]
pub struct AdminRemoveMemberPayload {
    /// Audit-trail reason; required (non-empty).
    pub reason: String,
}

/// Body for admin-override member-move endpoint.
#[derive(Deserialize)]
pub struct AdminMoveMemberPayload {
    /// Destination team to move the member into.
    pub target_team_id: Uuid,
    /// Audit-trail reason; required (non-empty).
    pub reason: String,
}

/// POST /api/admin/affiliate-teams/members/:membership_id/move
/// Admin-Override: hängt einen Member aus Team A in Team B um.
///
/// Atomar: alte Membership wird soft-removed (status='removed'), aktive
/// Team-Business-Links der alten Zuordnung deaktiviert, neue Membership in
/// Ziel-Team angelegt (status='active'), neuer Team-Business-Link erzeugt.
/// Historische Commissions bleiben unverändert (payout an alten Developer).
pub async fn api_admin_affiliate_team_member_move(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(membership_id): Path<Uuid>,
    Json(payload): Json<AdminMoveMemberPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.team_manage")
        .await?;

    if payload.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("Reason is required".into()));
    }

    // Resolve source membership + target team in one shot
    let src = sqlx::query!(
        r#"SELECT m.id, m.team_id AS src_team_id, m.user_id, m.status, m.role
           FROM developer_team_memberships m
           WHERE m.id = $1"#,
        membership_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Source membership not found".into()))?;

    if src.src_team_id == payload.target_team_id {
        return Err(ApiError::BadRequest(
            "Source and target team are identical".into(),
        ));
    }

    let target = sqlx::query!(
        r#"SELECT id, status FROM developer_teams WHERE id = $1"#,
        payload.target_team_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Target team not found".into()))?;

    if target.status != "active" {
        return Err(ApiError::BadRequest(
            "Target team must be active".into(),
        ));
    }

    // 1) Remove from source — same path as developer remove, but actor=admin.
    crate::rewards::team_members::remove_member(
        &state.db,
        membership_id,
        admin.user.id,
        &format!("moved_by_admin:{}", payload.reason),
    )
    .await
    .map_err(|e| match e {
        crate::error::AppError::NotFound(m) => ApiError::NotFound(m),
        _ => ApiError::Internal("source remove failed".into()),
    })?;

    // 2) Insert new active membership in target team. The partial-unique
    // constraint `one_active_membership_per_user` is now free because the
    // old membership flipped to 'removed' above.
    let new_id = sqlx::query!(
        r#"INSERT INTO developer_team_memberships
              (team_id, user_id, status, role, joined_at, invited_by_user_id, invited_at)
           VALUES ($1, $2, 'active', $3, NOW(), $4, NOW())
           RETURNING id"#,
        payload.target_team_id,
        src.user_id,
        src.role,
        admin.user.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?
    .id;

    // 3) New business-link in target team (idempotent).
    let _ = crate::rewards::team_links::create_team_business_link(
        &state.db,
        payload.target_team_id,
        src.user_id,
        admin.user.id,
    )
    .await;

    // 4) Audit
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_member_moved', 'developer_team_memberships', $2, $3)",
    )
    .bind(admin.user.id)
    .bind(new_id)
    .bind(serde_json::json!({
        "user_id": src.user_id,
        "from_team_id": src.src_team_id,
        "to_team_id": payload.target_team_id,
        "reason": payload.reason,
    }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({
        "status": "moved",
        "new_membership_id": new_id,
        "from_team_id": src.src_team_id,
        "to_team_id": payload.target_team_id,
    }))
    .into_response())
}

/// POST /api/admin/affiliate-teams/members/:membership_id/remove — admin-override removal.
pub async fn api_admin_affiliate_team_member_remove(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(membership_id): Path<Uuid>,
    Json(payload): Json<AdminRemoveMemberPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.team_manage")
        .await?;

    if payload.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("Reason is required".into()));
    }

    crate::rewards::team_members::remove_member(
        &state.db,
        membership_id,
        admin.user.id,
        &payload.reason,
    )
    .await
    .map_err(|e| match e {
        crate::error::AppError::NotFound(m) => ApiError::NotFound(m),
        _ => ApiError::Internal("remove failed".into()),
    })?;

    Ok(Json(serde_json::json!({"status": "removed"})).into_response())
}
