/// Circle System — CRUD, invites, membership management.
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

// ─── Models ─────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct Circle {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub avatar_emoji: Option<String>,
    pub member_count: i32,
    pub total_xp: i64,
    pub level: i32,
    pub level_name: String,
    pub is_public: bool,
    pub max_members: i32,
    // W3.1: Token-Gated Circles
    pub token_gate_asset_id: Option<Uuid>,
    pub token_gate_min_value_cents: Option<i64>,
    pub token_gate_asset_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CircleMember {
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CircleInvite {
    pub id: Uuid,
    pub circle_id: Uuid,
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
    pub status: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ─── Circle CRUD ────────────────────────────────────────────────────

/// Create a new circle. The creator becomes the owner.
pub async fn create_circle(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    description: Option<&str>,
    emoji: Option<&str>,
) -> Result<Circle, AppError> {
    // Check: user can only own one circle
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM circles WHERE owner_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest("You already own a circle".into()));
    }

    let mut tx = pool.begin().await?;

    let circle = sqlx::query_as::<_, Circle>(
        r#"INSERT INTO circles (name, description, owner_id, avatar_emoji, member_count)
           VALUES ($1, $2, $3, $4, 1)
           RETURNING *"#,
    )
    .bind(name)
    .bind(description)
    .bind(user_id)
    .bind(emoji.unwrap_or("🟢"))
    .fetch_one(&mut *tx)
    .await?;

    // Add owner as member
    sqlx::query("INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(circle.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Update profile circle_id
    sqlx::query("UPDATE community_profiles SET circle_id = $1 WHERE user_id = $2")
        .bind(circle.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(circle)
}

/// Get circle by ID.
pub async fn get_circle(pool: &PgPool, circle_id: Uuid) -> Result<Option<Circle>, AppError> {
    let circle = sqlx::query_as::<_, Circle>("SELECT * FROM circles WHERE id = $1")
        .bind(circle_id)
        .fetch_optional(pool)
        .await?;
    Ok(circle)
}

/// Get circle for a specific user (via their membership).
pub async fn get_my_circle(pool: &PgPool, user_id: Uuid) -> Result<Option<Circle>, AppError> {
    let circle = sqlx::query_as::<_, Circle>(
        r#"SELECT c.* FROM circles c
           JOIN circle_members cm ON cm.circle_id = c.id
           WHERE cm.user_id = $1
           LIMIT 1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(circle)
}

/// Update circle name/description (owner only).
pub async fn update_circle(
    pool: &PgPool,
    circle_id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    emoji: Option<&str>,
) -> Result<Circle, AppError> {
    // Verify ownership
    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM circles WHERE id = $1")
        .bind(circle_id)
        .fetch_optional(pool)
        .await?;

    if owner_id != Some(user_id) {
        return Err(AppError::Unauthorized(
            "Only the circle owner can edit".into(),
        ));
    }

    let circle = sqlx::query_as::<_, Circle>(
        r#"UPDATE circles SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            avatar_emoji = COALESCE($4, avatar_emoji),
            updated_at = NOW()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(circle_id)
    .bind(name)
    .bind(description)
    .bind(emoji)
    .fetch_one(pool)
    .await?;

    Ok(circle)
}

pub async fn admin_force_update_circle(
    pool: &PgPool,
    circle_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    emoji: Option<&str>,
    is_public: Option<bool>,
) -> Result<Circle, AppError> {
    if let Some(name) = name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("Circle name is required.".into()));
        }
        if name.chars().count() > 100 {
            return Err(AppError::BadRequest(
                "Circle name must be 100 characters or fewer.".into(),
            ));
        }
    }

    if let Some(description) = description {
        if description.chars().count() > 500 {
            return Err(AppError::BadRequest(
                "Circle description must be 500 characters or fewer.".into(),
            ));
        }
    }

    if let Some(emoji) = emoji {
        if emoji.chars().count() > 10 {
            return Err(AppError::BadRequest(
                "Circle emoji must be 10 characters or fewer.".into(),
            ));
        }
    }

    let circle = sqlx::query_as::<_, Circle>(
        r#"UPDATE circles SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            avatar_emoji = COALESCE($4, avatar_emoji),
            is_public = COALESCE($5, is_public),
            updated_at = NOW()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(circle_id)
    .bind(name)
    .bind(description)
    .bind(emoji)
    .bind(is_public)
    .fetch_one(pool)
    .await?;

    Ok(circle)
}

pub async fn admin_force_transfer_circle(
    community_pool: &PgPool,
    core_pool: &PgPool,
    circle_id: Uuid,
    new_owner_id: Uuid,
) -> Result<(), AppError> {
    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status <> 'deleted')",
    )
    .bind(new_owner_id)
    .fetch_one(core_pool)
    .await?;

    if !user_exists {
        return Err(AppError::BadRequest(
            "New owner must be an active platform user.".into(),
        ));
    }

    let mut tx = community_pool.begin().await?;

    sqlx::query_scalar::<_, Uuid>("SELECT id FROM circles WHERE id = $1 FOR UPDATE")
        .bind(circle_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    sqlx::query(
        "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(new_owner_id)
    .execute(&mut *tx)
    .await?;

    let existing_membership: Option<(Uuid, String)> =
        sqlx::query_as("SELECT circle_id, role FROM circle_members WHERE user_id = $1 FOR UPDATE")
            .bind(new_owner_id)
            .fetch_optional(&mut *tx)
            .await?;

    let is_member = match existing_membership {
        Some((existing_circle_id, role)) if existing_circle_id == circle_id => Some(role),
        Some(_) => {
            return Err(AppError::BadRequest(
                "New owner is already a member of another circle.".into(),
            ))
        }
        None => None,
    };

    // Current owner -> admin/member
    let demoted_owners = sqlx::query(
        "UPDATE circle_members SET role = 'member' WHERE circle_id = $1 AND role = 'owner'",
    )
    .bind(circle_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if demoted_owners == 0 {
        return Err(AppError::BadRequest(
            "Circle has no current owner to transfer from.".into(),
        ));
    }

    if is_member.is_some() {
        // Just upgrade them
        sqlx::query(
            "UPDATE circle_members SET role = 'owner' WHERE circle_id = $1 AND user_id = $2",
        )
        .bind(circle_id)
        .bind(new_owner_id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'owner')",
        )
        .bind(circle_id)
        .bind(new_owner_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE circles SET member_count = member_count + 1 WHERE id = $1")
            .bind(circle_id)
            .execute(&mut *tx)
            .await?;
    }

    let profile_rows =
        sqlx::query("UPDATE community_profiles SET circle_id = $1 WHERE user_id = $2")
            .bind(circle_id)
            .bind(new_owner_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

    if profile_rows != 1 {
        return Err(AppError::Internal(
            "Failed to attach new owner community profile.".into(),
        ));
    }

    // Set circles.owner_id
    let circle_rows =
        sqlx::query("UPDATE circles SET owner_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_owner_id)
            .bind(circle_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

    if circle_rows != 1 {
        return Err(AppError::NotFound("Circle not found".into()));
    }

    tx.commit().await?;

    Ok(())
}

/// Get all members of a circle.
pub async fn get_circle_members(
    pool: &PgPool,
    circle_id: Uuid,
) -> Result<Vec<CircleMember>, AppError> {
    let members = sqlx::query_as::<_, CircleMember>(
        "SELECT user_id, role, joined_at FROM circle_members WHERE circle_id = $1 ORDER BY joined_at ASC"
    )
    .bind(circle_id)
    .fetch_all(pool)
    .await?;
    Ok(members)
}

// ─── Join / Leave ───────────────────────────────────────────────────

/// Join a public circle (or accept an invite for private).
pub async fn join_circle(pool: &PgPool, user_id: Uuid, circle_id: Uuid) -> Result<(), AppError> {
    // Check if already in a circle
    let existing_circle: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM circle_members WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    if existing_circle.is_some() {
        return Err(AppError::BadRequest(
            "You are already in a circle. Leave first.".into(),
        ));
    }

    // Check circle exists and is public
    let (is_public, member_count, max_members): (bool, i32, i32) =
        sqlx::query_as("SELECT is_public, member_count, max_members FROM circles WHERE id = $1")
            .bind(circle_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    if !is_public {
        return Err(AppError::BadRequest(
            "This circle is private. You need an invite.".into(),
        ));
    }

    if member_count >= max_members {
        return Err(AppError::BadRequest("This circle is full.".into()));
    }

    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(circle_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "UPDATE circles SET member_count = member_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(circle_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE community_profiles SET circle_id = $1 WHERE user_id = $2")
        .bind(circle_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Award Gamification Challenge
    crate::community::challenges::increment_progress(pool, user_id, "join_circle", 1).await?;

    Ok(())
}

/// Leave a circle. Owner cannot leave (must transfer or delete).
pub async fn leave_circle(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let membership: Option<(Uuid, String)> =
        sqlx::query_as("SELECT circle_id, role FROM circle_members WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    let (circle_id, role) = match membership {
        Some(m) => m,
        None => return Err(AppError::BadRequest("You are not in a circle.".into())),
    };

    if role == "owner" {
        return Err(AppError::BadRequest(
            "Circle owners cannot leave. Transfer ownership or delete the circle.".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM circle_members WHERE user_id = $1 AND circle_id = $2")
        .bind(user_id)
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE circles SET member_count = GREATEST(0, member_count - 1), updated_at = NOW() WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE community_profiles SET circle_id = NULL WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

// ─── Invites ────────────────────────────────────────────────────────

/// Send an invite to a user. Only owner/admin can invite.
pub async fn send_invite(
    pool: &PgPool,
    inviter_id: Uuid,
    invitee_id: Uuid,
    circle_id: Uuid,
) -> Result<CircleInvite, AppError> {
    // Check inviter permission
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(inviter_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => {}
        _ => {
            return Err(AppError::Unauthorized(
                "Only circle owner/admin can invite".into(),
            ))
        }
    }

    // Check invitee not already in a circle
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM circle_members WHERE user_id = $1")
            .bind(invitee_id)
            .fetch_optional(pool)
            .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest("User is already in a circle.".into()));
    }

    let invite = sqlx::query_as::<_, CircleInvite>(
        r#"INSERT INTO circle_invites (circle_id, inviter_id, invitee_id)
           VALUES ($1, $2, $3) RETURNING *"#,
    )
    .bind(circle_id)
    .bind(inviter_id)
    .bind(invitee_id)
    .fetch_one(pool)
    .await?;

    Ok(invite)
}

/// Accept an invite.
pub async fn accept_invite(pool: &PgPool, user_id: Uuid, invite_id: Uuid) -> Result<(), AppError> {
    let invite: Option<CircleInvite> = sqlx::query_as(
        "SELECT * FROM circle_invites WHERE id = $1 AND invitee_id = $2 AND status = 'pending'",
    )
    .bind(invite_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let invite = invite.ok_or_else(|| AppError::NotFound("Invite not found or expired".into()))?;

    if invite.expires_at < chrono::Utc::now() {
        sqlx::query("UPDATE circle_invites SET status = 'expired' WHERE id = $1")
            .bind(invite_id)
            .execute(pool)
            .await?;
        return Err(AppError::BadRequest("This invite has expired.".into()));
    }

    // Use join_circle logic, then mark invite accepted
    join_circle(pool, user_id, invite.circle_id).await?;

    sqlx::query("UPDATE circle_invites SET status = 'accepted' WHERE id = $1")
        .bind(invite_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Decline an invite.
pub async fn decline_invite(pool: &PgPool, user_id: Uuid, invite_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE circle_invites SET status = 'declined' WHERE id = $1 AND invitee_id = $2 AND status = 'pending'")
        .bind(invite_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get pending invites for a user.
pub async fn get_my_invites(pool: &PgPool, user_id: Uuid) -> Result<Vec<CircleInvite>, AppError> {
    let invites = sqlx::query_as::<_, CircleInvite>(
        "SELECT * FROM circle_invites WHERE invitee_id = $1 AND status = 'pending' AND expires_at > NOW() ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(invites)
}

// ─── Circle Leaderboard ─────────────────────────────────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CircleLeaderboardEntry {
    pub id: Uuid,
    pub name: String,
    pub avatar_emoji: Option<String>,
    pub owner_id: Uuid,
    pub member_count: i32,
    pub total_xp: i64,
    pub level: i32,
    pub level_name: String,
}

pub async fn get_circle_leaderboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<CircleLeaderboardEntry>, AppError> {
    let entries = sqlx::query_as::<_, CircleLeaderboardEntry>(
        r#"SELECT id, name, avatar_emoji, owner_id, member_count, total_xp, level, level_name
           FROM circles
           ORDER BY total_xp DESC
           LIMIT $1"#,
    )
    .bind(limit.clamp(1, 50))
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

// ─── Auto-Join from Referral ────────────────────────────────────────

/// Called after referral signup: automatically join the referrer's circle.
pub async fn auto_join_referrer_circle(
    pool: &PgPool,
    new_user_id: Uuid,
    referrer_id: Uuid,
) -> Result<(), AppError> {
    // Find referrer's circle
    let circle_id: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM circle_members WHERE user_id = $1 LIMIT 1")
            .bind(referrer_id)
            .fetch_optional(pool)
            .await?;

    if let Some(cid) = circle_id {
        // Try to join, ignore errors (circle may be full, etc.)
        match join_circle(pool, new_user_id, cid).await {
            Ok(()) => {
                tracing::info!(user_id = %new_user_id, circle_id = %cid, "Auto-joined referrer's circle");
            }
            Err(e) => {
                tracing::warn!(user_id = %new_user_id, circle_id = %cid, error = %e, "Failed to auto-join referrer's circle");
            }
        }
    }

    Ok(())
}

// ─── Kick Member ────────────────────────────────────────────────────

/// Remove a member from a circle (owner/admin only).
pub async fn kick_member(
    pool: &PgPool,
    actor_id: Uuid,
    target_id: Uuid,
    circle_id: Uuid,
) -> Result<(), AppError> {
    // Check actor permission
    let actor_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(actor_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    if actor_id == target_id {
        return Err(AppError::BadRequest(
            "You cannot kick yourself. Please use the leave circle function.".into(),
        ));
    }

    match actor_role.as_deref() {
        Some("owner") => {}
        Some("admin") => {
            // Admin can only kick members, not other admins
            let target_role: Option<String> = sqlx::query_scalar(
                "SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2",
            )
            .bind(target_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;
            if target_role.as_deref() != Some("member") {
                return Err(AppError::Unauthorized(
                    "Admins can only kick members".into(),
                ));
            }
        }
        _ => return Err(AppError::Unauthorized("Not authorized to kick".into())),
    }

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM circle_members WHERE user_id = $1 AND circle_id = $2")
        .bind(target_id)
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE circles SET member_count = GREATEST(0, member_count - 1), updated_at = NOW() WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE community_profiles SET circle_id = NULL WHERE user_id = $1")
        .bind(target_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

// ─── Circle Roles & Settings (M4-BE.11, M4-BE.12, M4-BE.13) ──────────────────

/// Promote or demote a member. Owner can promote to admin or demote to member.
pub async fn update_member_role(
    pool: &PgPool,
    owner_id: Uuid,
    target_id: Uuid,
    circle_id: Uuid,
    new_role: &str,
) -> Result<(), AppError> {
    if new_role != "admin" && new_role != "member" {
        return Err(AppError::BadRequest(
            "Role must be 'admin' or 'member'.".into(),
        ));
    }

    // Verify actor is owner
    let owner_check: Option<Uuid> =
        sqlx::query_scalar("SELECT owner_id FROM circles WHERE id = $1")
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    if owner_check != Some(owner_id) {
        return Err(AppError::Unauthorized(
            "Only the circle owner can manage roles.".into(),
        ));
    }

    // Verify target is in circle
    let target_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(target_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    if target_role.is_none() {
        return Err(AppError::NotFound("User is not in this circle.".into()));
    }

    if target_role.as_deref() == Some("owner") {
        return Err(AppError::BadRequest(
            "Cannot change the owner's role directly. Use transfer ownership instead.".into(),
        ));
    }

    sqlx::query("UPDATE circle_members SET role = $1 WHERE user_id = $2 AND circle_id = $3")
        .bind(new_role)
        .bind(target_id)
        .bind(circle_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Transfer circle ownership.
pub async fn transfer_ownership(
    pool: &PgPool,
    current_owner_id: Uuid,
    new_owner_id: Uuid,
    circle_id: Uuid,
) -> Result<(), AppError> {
    // Verify current owner
    let owner_check: Option<Uuid> =
        sqlx::query_scalar("SELECT owner_id FROM circles WHERE id = $1")
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    if owner_check != Some(current_owner_id) {
        return Err(AppError::Unauthorized(
            "Only the circle owner can transfer ownership.".into(),
        ));
    }

    // Verify new owner is in circle
    let new_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(new_owner_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    if new_role.is_none() {
        return Err(AppError::BadRequest(
            "The new owner must be a member of the circle.".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    // Demote current owner to admin
    sqlx::query("UPDATE circle_members SET role = 'admin' WHERE user_id = $1 AND circle_id = $2")
        .bind(current_owner_id)
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    // Promote new owner
    sqlx::query("UPDATE circle_members SET role = 'owner' WHERE user_id = $1 AND circle_id = $2")
        .bind(new_owner_id)
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    // Update circles table
    sqlx::query("UPDATE circles SET owner_id = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_owner_id)
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

/// Update circle privacy setting.
pub async fn update_circle_privacy(
    pool: &PgPool,
    actor_id: Uuid,
    circle_id: Uuid,
    is_public: bool,
) -> Result<(), AppError> {
    // Verify actor is owner or admin
    let actor_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(actor_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    match actor_role.as_deref() {
        Some("owner") | Some("admin") => {}
        _ => {
            return Err(AppError::Unauthorized(
                "Only the circle owner or an admin can change privacy settings.".into(),
            ))
        }
    }

    sqlx::query("UPDATE circles SET is_public = $1, updated_at = NOW() WHERE id = $2")
        .bind(is_public)
        .bind(circle_id)
        .execute(pool)
        .await?;

    Ok(())
}

// ─── Circle Join Requests (M4-BE.15) — Private Circle Flow ─────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CircleJoinRequest {
    pub id: Uuid,
    pub circle_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Submit a join request for a private circle.
/// Returns Err if the user is already in a circle, already requested, or the circle is public.
pub async fn request_to_join(
    pool: &PgPool,
    user_id: Uuid,
    circle_id: Uuid,
) -> Result<CircleJoinRequest, AppError> {
    // Check user isn't already in a circle
    let already_member: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM circle_members WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    if already_member.is_some() {
        return Err(AppError::BadRequest(
            "You are already in a circle. Leave first.".into(),
        ));
    }

    // Get circle – must exist and be private
    let (is_public, member_count, max_members): (bool, i32, i32) =
        sqlx::query_as("SELECT is_public, member_count, max_members FROM circles WHERE id = $1")
            .bind(circle_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    if is_public {
        return Err(AppError::BadRequest(
            "This circle is public — just join directly.".into(),
        ));
    }

    if member_count >= max_members {
        return Err(AppError::BadRequest("This circle is full.".into()));
    }

    // Check for existing pending request (the unique index on pending prevents duplicates via DB constraint)
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM circle_join_requests WHERE circle_id = $1 AND user_id = $2 AND status = 'pending'"
    )
    .bind(circle_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest(
            "You already have a pending request for this circle.".into(),
        ));
    }

    let req = sqlx::query_as::<_, CircleJoinRequest>(
        "INSERT INTO circle_join_requests (circle_id, user_id) VALUES ($1, $2) RETURNING *",
    )
    .bind(circle_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(req)
}

/// Cancel your own pending join request.
pub async fn cancel_join_request(
    pool: &PgPool,
    user_id: Uuid,
    request_id: Uuid,
) -> Result<(), AppError> {
    let rows = sqlx::query(
        "DELETE FROM circle_join_requests WHERE id = $1 AND user_id = $2 AND status = 'pending'",
    )
    .bind(request_id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound(
            "Request not found or already processed.".into(),
        ));
    }

    Ok(())
}

/// List all pending join requests for a circle (owner/admin only).
pub async fn get_pending_join_requests(
    pool: &PgPool,
    actor_id: Uuid,
    circle_id: Uuid,
) -> Result<Vec<CircleJoinRequest>, AppError> {
    // Check actor is owner or admin
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(actor_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => {}
        _ => {
            return Err(AppError::Unauthorized(
                "Only circle owner/admin can view requests.".into(),
            ))
        }
    }

    let requests = sqlx::query_as::<_, CircleJoinRequest>(
        "SELECT * FROM circle_join_requests WHERE circle_id = $1 AND status = 'pending' ORDER BY created_at ASC"
    )
    .bind(circle_id)
    .fetch_all(pool)
    .await?;

    Ok(requests)
}

/// Get your own pending join requests (so users can see where they've requested to join).
pub async fn get_my_join_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CircleJoinRequest>, AppError> {
    let reqs = sqlx::query_as::<_, CircleJoinRequest>(
        "SELECT * FROM circle_join_requests WHERE user_id = $1 AND status = 'pending' ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(reqs)
}

/// Approve a join request (owner/admin action). Adds the requester to the circle.
pub async fn approve_join_request(
    pool: &PgPool,
    actor_id: Uuid,
    request_id: Uuid,
) -> Result<Uuid, AppError> {
    // Fetch the request
    let req: Option<CircleJoinRequest> =
        sqlx::query_as("SELECT * FROM circle_join_requests WHERE id = $1 AND status = 'pending'")
            .bind(request_id)
            .fetch_optional(pool)
            .await?;

    let req =
        req.ok_or_else(|| AppError::NotFound("Request not found or already processed.".into()))?;

    // Verify actor is owner or admin of that circle
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(actor_id)
            .bind(req.circle_id)
            .fetch_optional(pool)
            .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => {}
        _ => {
            return Err(AppError::Unauthorized(
                "Only circle owner/admin can approve requests.".into(),
            ))
        }
    }

    // Check circle not now full
    let (member_count, max_members): (i32, i32) =
        sqlx::query_as("SELECT member_count, max_members FROM circles WHERE id = $1")
            .bind(req.circle_id)
            .fetch_one(pool)
            .await?;

    if member_count >= max_members {
        return Err(AppError::BadRequest(
            "Cannot approve: circle is now full.".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    // Mark request as accepted
    sqlx::query("UPDATE circle_join_requests SET status = 'accepted' WHERE id = $1")
        .bind(request_id)
        .execute(&mut *tx)
        .await?;

    // Add to members
    sqlx::query(
        "INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'member') ON CONFLICT DO NOTHING"
    )
    .bind(req.circle_id)
    .bind(req.user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE circles SET member_count = member_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(req.circle_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE community_profiles SET circle_id = $1 WHERE user_id = $2")
        .bind(req.circle_id)
        .bind(req.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(req.user_id)
}

/// Decline a join request (owner/admin action).
pub async fn decline_join_request(
    pool: &PgPool,
    actor_id: Uuid,
    request_id: Uuid,
) -> Result<Uuid, AppError> {
    let req: Option<CircleJoinRequest> =
        sqlx::query_as("SELECT * FROM circle_join_requests WHERE id = $1 AND status = 'pending'")
            .bind(request_id)
            .fetch_optional(pool)
            .await?;

    let req =
        req.ok_or_else(|| AppError::NotFound("Request not found or already processed.".into()))?;

    // Verify actor is owner or admin
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(actor_id)
            .bind(req.circle_id)
            .fetch_optional(pool)
            .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => {}
        _ => {
            return Err(AppError::Unauthorized(
                "Only circle owner/admin can decline requests.".into(),
            ))
        }
    }

    sqlx::query("UPDATE circle_join_requests SET status = 'declined' WHERE id = $1")
        .bind(request_id)
        .execute(pool)
        .await?;

    Ok(req.user_id)
}

// ─── Owner Circle Deletion ──────────────────────────────────────────

/// Allow the circle owner to delete their own circle.
pub async fn delete_own_circle(
    pool: &PgPool,
    user_id: Uuid,
    circle_id: Uuid,
) -> Result<(), AppError> {
    // Verify the user is the owner
    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM circles WHERE id = $1")
        .bind(circle_id)
        .fetch_optional(pool)
        .await?;

    match owner_id {
        Some(oid) if oid == user_id => {}
        Some(_) => {
            return Err(AppError::Forbidden(
                "Only the circle owner can delete the circle.".into(),
            ))
        }
        None => return Err(AppError::NotFound("Circle not found.".into())),
    }

    let mut tx = pool.begin().await?;

    // Unlink members in community_profiles
    sqlx::query("UPDATE community_profiles SET circle_id = NULL WHERE circle_id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    // Delete members
    sqlx::query("DELETE FROM circle_members WHERE circle_id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    // Delete invites
    sqlx::query("DELETE FROM circle_invites WHERE circle_id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    // Delete join requests
    sqlx::query("DELETE FROM circle_join_requests WHERE circle_id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    // Delete circle
    let res = sqlx::query("DELETE FROM circles WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Circle not found.".into()));
    }

    tx.commit().await?;
    Ok(())
}

// ─── Admin Features ──────────────────────────────────────────────────

/// List circles for admin (ordered by creation date) with bounded pagination.
pub async fn admin_get_circles(
    pool: &PgPool,
    search: Option<&str>,
    visibility: Option<bool>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Circle>, i64, i64, i64), AppError> {
    let circles = sqlx::query_as::<_, Circle>(
        r#"
        SELECT * FROM circles
        WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%' OR id::text ILIKE '%' || $1 || '%')
          AND ($2::bool IS NULL OR is_public = $2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(search)
    .bind(visibility)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let totals = sqlx::query_as::<_, (i64, i64, i64)>(
        r#"
        SELECT
            COUNT(*)::BIGINT,
            COALESCE(SUM(member_count), 0)::BIGINT,
            COALESCE(SUM(total_xp), 0)::BIGINT
        FROM circles
        WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%' OR id::text ILIKE '%' || $1 || '%')
          AND ($2::bool IS NULL OR is_public = $2)
        "#,
    )
    .bind(search)
    .bind(visibility)
    .fetch_one(pool)
    .await?;

    Ok((circles, totals.0, totals.1, totals.2))
}

/// Admin completely deletes a circle and unlinks all members.
pub async fn admin_delete_circle(
    pool: &PgPool,
    circle_id: Uuid,
    actor_user_id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let circle = sqlx::query_as::<_, Circle>("SELECT * FROM circles WHERE id = $1 FOR UPDATE")
        .bind(circle_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    // Unlink members in community_profiles
    let profiles_unlinked =
        sqlx::query("UPDATE community_profiles SET circle_id = NULL WHERE circle_id = $1")
            .bind(circle_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

    // Delete members
    let members_deleted = sqlx::query("DELETE FROM circle_members WHERE circle_id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    // Delete invites
    let invites_deleted = sqlx::query("DELETE FROM circle_invites WHERE circle_id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    // Delete circle. Join requests are covered by ON DELETE CASCADE.
    sqlx::query("DELETE FROM circles WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    let details = serde_json::json!({
        "circle": circle,
        "profiles_unlinked": profiles_unlinked,
        "members_deleted": members_deleted,
        "invites_deleted": invites_deleted,
    });

    sqlx::query(
        r#"
        INSERT INTO community_audit_logs
            (actor_user_id, action, entity_type, entity_id, target_user_id, details)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(actor_user_id)
    .bind("circle.delete")
    .bind("circle")
    .bind(circle_id)
    .bind(circle.owner_id)
    .bind(details)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Admin removes a user from a circle. Owner cannot be removed.
pub async fn admin_remove_member(
    pool: &PgPool,
    circle_id: Uuid,
    target_id: Uuid,
) -> Result<(), AppError> {
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(target_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;

    let role = role.ok_or_else(|| AppError::NotFound("User not in circle".into()))?;
    if role == "owner" {
        return Err(AppError::BadRequest(
            "Cannot remove circle owner. Delete circle instead.".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM circle_members WHERE user_id = $1 AND circle_id = $2")
        .bind(target_id)
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE circles SET member_count = GREATEST(0, member_count - 1), updated_at = NOW() WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE community_profiles SET circle_id = NULL WHERE user_id = $1")
        .bind(target_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

// ─── W3.1: Token-Gated Circle Enforcement ───────────────────────────────

/// Check if a user meets the token gate requirement for a circle.
/// Reads the user's investments from the core DB (read-only cross-DB access).
/// Returns Ok(()) if the gate is passed or the circle has no gate.
pub async fn check_token_gate(
    community_pool: &PgPool,
    core_pool: &PgPool,
    user_id: Uuid,
    circle_id: Uuid,
) -> Result<(), AppError> {
    use sqlx::Row;

    // Fetch the circle's gate config from community DB
    let gate_row = sqlx::query(
        "SELECT token_gate_asset_id, token_gate_min_value_cents FROM circles WHERE id = $1",
    )
    .bind(circle_id)
    .fetch_optional(community_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    let gate_asset_id: Option<Uuid> = gate_row.try_get("token_gate_asset_id").ok().flatten();
    let gate_min_cents: Option<i64> = gate_row
        .try_get("token_gate_min_value_cents")
        .ok()
        .flatten();

    // No gate configured → everyone can join
    let asset_id = match gate_asset_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let min_cents = gate_min_cents.unwrap_or(0);
    if min_cents <= 0 {
        return Ok(());
    }

    // Read from core DB: user's holdings of this specific asset
    // current_value = tokens_owned × asset.token_price_cents
    let holding_value: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT (i.tokens_owned * a.token_price_cents)::BIGINT
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE i.user_id = $1 AND i.asset_id = $2 AND i.status != 'refunded'
        ORDER BY (i.tokens_owned * a.token_price_cents) DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_optional(core_pool)
    .await?;

    let actual_value = holding_value.unwrap_or(0);

    if actual_value < min_cents {
        let required_formatted = format!("${:.2}", min_cents as f64 / 100.0);
        let held_formatted = format!("${:.2}", actual_value as f64 / 100.0);
        return Err(AppError::BadRequest(format!(
            "Token gate requirement not met. You need at least {} of this asset to join. You currently hold {}.",
            required_formatted, held_formatted
        )));
    }

    Ok(())
}

/// Set or clear the token gate on a circle. Only the circle owner can do this.
pub async fn update_token_gate(
    community_pool: &PgPool,
    core_pool: &PgPool,
    user_id: Uuid,
    circle_id: Uuid,
    asset_id: Option<Uuid>,
    min_value_cents: Option<i64>,
) -> Result<Circle, AppError> {
    // Verify ownership
    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM circles WHERE id = $1")
        .bind(circle_id)
        .fetch_optional(community_pool)
        .await?;

    match owner_id {
        Some(oid) if oid == user_id => {}
        Some(_) => {
            return Err(AppError::Forbidden(
                "Only the circle owner can set token gates".into(),
            ))
        }
        None => return Err(AppError::NotFound("Circle not found".into())),
    }

    // If setting a gate, fetch the asset name from core DB for denormalization
    let asset_name: Option<String> = if let Some(aid) = asset_id {
        use sqlx::Row;
        let row = sqlx::query("SELECT title FROM assets WHERE id = $1")
            .bind(aid)
            .fetch_optional(core_pool)
            .await?;
        match row {
            Some(r) => Some(r.try_get("title")?),
            None => {
                return Err(AppError::NotFound(
                    "Asset not found. Cannot set token gate for a non-existent asset.".into(),
                ))
            }
        }
    } else {
        None
    };

    let circle = sqlx::query_as::<_, Circle>(
        r#"
        UPDATE circles
        SET token_gate_asset_id = $1,
            token_gate_min_value_cents = $2,
            token_gate_asset_name = $3,
            updated_at = NOW()
        WHERE id = $4
        RETURNING *
        "#,
    )
    .bind(asset_id)
    .bind(min_value_cents.unwrap_or(0))
    .bind(&asset_name)
    .bind(circle_id)
    .fetch_one(community_pool)
    .await?;

    Ok(circle)
}
