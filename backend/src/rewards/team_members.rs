//! Team-Membership-Service (Phase 2).
//!
//! Beitritts-Flows:
//!   * `invite_by_email` — Developer schickt Email an Adresse, Token erzeugt.
//!     User klickt Link → `accept_invitation` setzt status = active.
//!   * `self_request` — User findet Developer-Slug, requestet Beitritt.
//!     Developer bestätigt via `approve_pending` → status = active.
//!
//! Off-Boarding:
//!   * `remove_member` — soft-delete, deaktiviert alle Team-Business-Links
//!     des Members (historische Commissions bleiben).
//!
//! Nutzung von `one_active_membership_per_user` (Partial-Unique-Index in
//! Mig 156) garantiert: User kann nicht parallel in zwei Teams sein.

use crate::error::AppError;
use crate::rewards::team_models::MembershipStatus;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Tokens leben 14 Tage. Hash davon (SHA-256) wird in DB gespeichert; der
/// Plain-Token wird nur per Email versendet und nie persistiert.
const INVITATION_VALIDITY_DAYS: i64 = 14;

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_token() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

// ── Invite-Flow ─────────────────────────────────────────────────────────────

/// Developer lädt einen User per email ein. Wenn User noch keinen Account
/// hat, kann der Email-Empfänger nach Registrierung den Token einlösen.
///
/// Rückgabe: (membership_id, plain_token). Plain-Token NUR per Email
/// versenden, NICHT loggen.
pub async fn invite_by_email(
    pool: &PgPool,
    team_id: Uuid,
    invited_email: &str,
    inviter_user_id: Uuid,
) -> Result<(Uuid, String), AppError> {
    // Resolve email → user_id (existiert der Account?)
    let target_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM users WHERE LOWER(email) = LOWER($1)")
            .bind(invited_email)
            .fetch_optional(pool)
            .await?;

    let target_user_id = target_user_id.ok_or_else(|| {
        AppError::BadRequest(
            "Cannot invite — user with this email is not registered yet.".into(),
        )
    })?;

    if target_user_id == inviter_user_id {
        return Err(AppError::BadRequest(
            "Cannot invite yourself to your own team.".into(),
        ));
    }

    // Hat target_user bereits eine aktive (oder pending) Membership irgendwo?
    let conflict: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
               SELECT 1 FROM developer_team_memberships
               WHERE user_id = $1
                 AND status IN ('invited', 'pending_developer_approval', 'active')
           )"#,
    )
    .bind(target_user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if conflict {
        return Err(AppError::Conflict(
            "User is already invited to or member of a team.".into(),
        ));
    }

    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(INVITATION_VALIDITY_DAYS);

    let row = match sqlx::query!(
        r#"INSERT INTO developer_team_memberships
              (team_id, user_id, status, invitation_token_hash, invitation_expires_at,
               invited_by_user_id, invited_at)
           VALUES ($1, $2, 'invited', $3, $4, $5, NOW())
           RETURNING id"#,
        team_id,
        target_user_id,
        token_hash,
        expires_at,
        inviter_user_id
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(db_err))
            if db_err.constraint() == Some("one_active_membership_per_user") =>
        {
            return Err(AppError::Conflict(
                "User already has an active or pending membership.".into(),
            ));
        }
        Err(e) => return Err(e.into()),
    };

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_invitation_sent', 'developer_team_memberships', $2, $3)",
    )
    .bind(inviter_user_id)
    .bind(row.id)
    .bind(serde_json::json!({
        "team_id": team_id,
        "invited_user_id": target_user_id,
    }))
    .execute(pool)
    .await;

    // Trigger Email (fire-and-forget; failure does NOT block invite creation).
    let team_meta = sqlx::query!(
        "SELECT t.display_name AS team_name, u.email::text AS inviter_email
         FROM developer_teams t
         LEFT JOIN users u ON u.id = $2
         WHERE t.id = $1",
        team_id,
        inviter_user_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let team_name = team_meta
        .as_ref()
        .map(|r| r.team_name.clone())
        .unwrap_or_else(|| "POOOL Affiliate Team".to_string());
    let inviter_email = team_meta
        .and_then(|r| r.inviter_email)
        .unwrap_or_else(|| "POOOL team".to_string());
    let metadata = serde_json::json!({
        "team_name": team_name,
        "inviter_name": inviter_email,
        "token": token,
    });
    let _ =
        crate::email::trigger_transactional_email(pool, &target_user_id, "team_invitation_received", metadata)
            .await;

    Ok((row.id, token))
}

/// User akzeptiert eine Einladung per Token. Erzeugt sofort den
/// Team-Business-Link.
pub async fn accept_invitation(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
) -> Result<Uuid, AppError> {
    let token_hash = hash_token(token);

    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        r#"SELECT id, team_id, status, invitation_expires_at
           FROM developer_team_memberships
           WHERE user_id = $1
             AND invitation_token_hash = $2
             AND status = 'invited'
           FOR UPDATE"#,
        user_id,
        token_hash
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Invitation not found or already used.".into()))?;

    if let Some(exp) = row.invitation_expires_at {
        if exp < chrono::Utc::now() {
            return Err(AppError::BadRequest("Invitation expired.".into()));
        }
    }

    sqlx::query!(
        r#"UPDATE developer_team_memberships
           SET status = 'active',
               joined_at = NOW(),
               invitation_token_hash = NULL,
               updated_at = NOW()
           WHERE id = $1"#,
        row.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Team-Business-Link nach Commit erzeugen. Bei Fehler: laut loggen,
    // damit Operators / Admin-Repair-Tool das nachholen können. Die
    // Membership ist bereits aktiv; das Link-Backfill ist idempotent via
    // partial-unique (mig 162).
    if let Err(e) =
        super::team_links::create_team_business_link(pool, row.team_id, user_id, user_id).await
    {
        tracing::error!(
            user_id = %user_id,
            team_id = %row.team_id,
            membership_id = %row.id,
            error = ?e,
            "accept_invitation: post-commit business-link create FAILED; membership is active without link. Run admin repair."
        );
    }

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_invitation_accepted', 'developer_team_memberships', $2, $3)",
    )
    .bind(user_id)
    .bind(row.id)
    .bind(serde_json::json!({ "team_id": row.team_id }))
    .execute(pool)
    .await;

    Ok(row.id)
}

// ── Self-Request-Flow ───────────────────────────────────────────────────────

/// User self-requestet Beitritt via Developer-Slug. Erzeugt
/// `pending_developer_approval`-Row. Developer muss approven.
pub async fn self_request_join(
    pool: &PgPool,
    user_id: Uuid,
    developer_slug: &str,
) -> Result<Uuid, AppError> {
    let team = sqlx::query!(
        r#"SELECT id, developer_user_id FROM developer_teams
           WHERE LOWER(public_slug) = LOWER($1) AND status = 'active'"#,
        developer_slug
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Developer team not found.".into()))?;

    if team.developer_user_id == user_id {
        return Err(AppError::BadRequest(
            "You cannot join your own team.".into(),
        ));
    }

    let conflict: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
               SELECT 1 FROM developer_team_memberships
               WHERE user_id = $1
                 AND status IN ('invited', 'pending_developer_approval', 'active')
           )"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if conflict {
        return Err(AppError::Conflict(
            "You are already invited to or member of a team.".into(),
        ));
    }

    let row = match sqlx::query!(
        r#"INSERT INTO developer_team_memberships
              (team_id, user_id, status, invited_at)
           VALUES ($1, $2, 'pending_developer_approval', NOW())
           RETURNING id"#,
        team.id,
        user_id
    )
    .fetch_one(pool)
    .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(db_err))
            if db_err.constraint() == Some("one_active_membership_per_user") =>
        {
            return Err(AppError::Conflict(
                "You already have an active or pending membership.".into(),
            ));
        }
        Err(e) => return Err(e.into()),
    };

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_self_request', 'developer_team_memberships', $2, $3)",
    )
    .bind(user_id)
    .bind(row.id)
    .bind(serde_json::json!({ "team_id": team.id }))
    .execute(pool)
    .await;

    // Notify developer of the new request.
    let team_meta = sqlx::query!(
        "SELECT t.display_name AS team_name, u.email::text AS requester_email
         FROM developer_teams t
         LEFT JOIN users u ON u.id = $2
         WHERE t.id = $1",
        team.id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    if let Some(meta) = team_meta {
        let _ = crate::email::trigger_transactional_email(
            pool,
            &team.developer_user_id,
            "team_self_request_received",
            serde_json::json!({
                "team_name": meta.team_name,
                "requester_email": meta.requester_email,
            }),
        )
        .await;
    }

    Ok(row.id)
}

/// Developer bestätigt einen pending self-request. Erzeugt sofort den
/// Team-Business-Link.
pub async fn approve_pending(
    pool: &PgPool,
    membership_id: Uuid,
    developer_user_id: Uuid,
) -> Result<Uuid, AppError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        r#"SELECT m.id, m.team_id, m.user_id, m.status,
                  t.developer_user_id, t.status as team_status
           FROM developer_team_memberships m
           JOIN developer_teams t ON t.id = m.team_id
           WHERE m.id = $1
             AND m.status = 'pending_developer_approval'
           FOR UPDATE"#,
        membership_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Pending membership not found.".into()))?;

    if row.developer_user_id != developer_user_id {
        return Err(AppError::Forbidden(
            "Only the team's developer can approve.".into(),
        ));
    }
    if row.team_status != "active" {
        return Err(AppError::BadRequest("Team is not active.".into()));
    }

    sqlx::query!(
        r#"UPDATE developer_team_memberships
           SET status = 'active', joined_at = NOW(), updated_at = NOW()
           WHERE id = $1"#,
        row.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let member_uid = row.user_id;
    if let Err(e) = super::team_links::create_team_business_link(
        pool,
        row.team_id,
        member_uid,
        developer_user_id,
    )
    .await
    {
        tracing::error!(
            user_id = %member_uid,
            team_id = %row.team_id,
            membership_id = %row.id,
            error = ?e,
            "approve_pending: post-commit business-link create FAILED; membership is active without link. Run admin repair."
        );
    }

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_member_approved', 'developer_team_memberships', $2, $3)",
    )
    .bind(developer_user_id)
    .bind(row.id)
    .bind(serde_json::json!({
        "team_id": row.team_id,
        "user_id": row.user_id,
    }))
    .execute(pool)
    .await;

    // Confirmation email to the member.
    let team_name: String = sqlx::query_scalar("SELECT display_name FROM developer_teams WHERE id = $1")
        .bind(row.team_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "POOOL Affiliate Team".to_string());
    let _ = crate::email::trigger_transactional_email(
        pool,
        &row.user_id,
        "team_member_approved",
        serde_json::json!({ "team_name": team_name }),
    )
    .await;

    Ok(row.id)
}

// ── Off-Boarding ────────────────────────────────────────────────────────────

/// Developer (oder Admin) entfernt einen Member. Soft-delete: status='removed'.
/// Deaktiviert auch alle aktiven Team-Business-Links dieses Members.
/// Historische Commissions + Referrals bleiben unverändert (payout an
/// Developer historisch bestehen).
pub async fn remove_member(
    pool: &PgPool,
    membership_id: Uuid,
    actor_user_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        r#"SELECT m.id, m.team_id, m.user_id, m.status
           FROM developer_team_memberships m
           WHERE m.id = $1 FOR UPDATE"#,
        membership_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Membership not found.".into()))?;

    if row.status == MembershipStatus::Removed.as_str() {
        return Ok(()); // idempotent
    }

    sqlx::query!(
        r#"UPDATE developer_team_memberships
           SET status = 'removed',
               removed_at = NOW(),
               removed_reason = $2,
               removed_by_user_id = $3,
               invitation_token_hash = NULL,
               updated_at = NOW()
           WHERE id = $1"#,
        row.id,
        reason,
        actor_user_id
    )
    .execute(&mut *tx)
    .await?;

    // CRITICAL: deactivate business-links INSIDE the same transaction so
    // membership-removed and link-inactive land atomically. Without this,
    // a removed member's business links could keep routing commissions to
    // the developer until manual cleanup. (Audit P0 #4.)
    let deactivated = sqlx::query!(
        r#"UPDATE affiliate_links
           SET status = 'inactive',
               deactivated_at = NOW(),
               deactivated_reason = $3,
               updated_at = NOW()
           WHERE team_id = $1
             AND attribution_user_id = $2
             AND link_type = 'team_business'
             AND status = 'active'"#,
        row.team_id,
        row.user_id,
        reason
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    tx.commit().await?;

    if deactivated > 0 {
        let _ = sqlx::query(
            "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
             VALUES ($1, 'affiliate_link_member_offboarded', 'developer_team_memberships', $2, $3)",
        )
        .bind(actor_user_id)
        .bind(row.user_id)
        .bind(serde_json::json!({
            "team_id": row.team_id,
            "links_deactivated": deactivated,
            "reason": reason,
        }))
        .execute(pool)
        .await;
    }

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_member_removed', 'developer_team_memberships', $2, $3)",
    )
    .bind(actor_user_id)
    .bind(row.id)
    .bind(serde_json::json!({
        "team_id": row.team_id,
        "user_id": row.user_id,
        "reason": reason,
    }))
    .execute(pool)
    .await;

    // Notification email to the member.
    let team_name: String =
        sqlx::query_scalar("SELECT display_name FROM developer_teams WHERE id = $1")
            .bind(row.team_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "POOOL Affiliate Team".to_string());
    let _ = crate::email::trigger_transactional_email(
        pool,
        &row.user_id,
        "team_member_removed",
        serde_json::json!({ "team_name": team_name, "reason": reason }),
    )
    .await;

    Ok(())
}
