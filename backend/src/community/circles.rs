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
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM circles WHERE owner_id = $1 LIMIT 1"
    )
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
           RETURNING *"#
    )
    .bind(name)
    .bind(description)
    .bind(user_id)
    .bind(emoji.unwrap_or("🟢"))
    .fetch_one(&mut *tx)
    .await?;

    // Add owner as member
    sqlx::query(
        "INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'owner')"
    )
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
    let circle = sqlx::query_as::<_, Circle>(
        "SELECT * FROM circles WHERE id = $1"
    )
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
           LIMIT 1"#
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
    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT owner_id FROM circles WHERE id = $1"
    )
    .bind(circle_id)
    .fetch_optional(pool)
    .await?;

    if owner_id != Some(user_id) {
        return Err(AppError::Unauthorized("Only the circle owner can edit".into()));
    }

    let circle = sqlx::query_as::<_, Circle>(
        r#"UPDATE circles SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            avatar_emoji = COALESCE($4, avatar_emoji),
            updated_at = NOW()
           WHERE id = $1 RETURNING *"#
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
    let circle = sqlx::query_as::<_, Circle>(
        r#"UPDATE circles SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            avatar_emoji = COALESCE($4, avatar_emoji),
            is_public = COALESCE($5, is_public),
            updated_at = NOW()
           WHERE id = $1 RETURNING *"#
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
    pool: &PgPool,
    circle_id: Uuid,
    new_owner_id: Uuid,
) -> Result<(), AppError> {
    
    // Check if new_owner is in circle_members. If not, add them.
    let is_member: Option<String> = sqlx::query_scalar(
        "SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2"
    )
    .bind(circle_id)
    .bind(new_owner_id)
    .fetch_optional(pool)
    .await?;
    
    // Begin transaction
    let mut tx = pool.begin().await?;

    // Current owner -> admin/member
    sqlx::query(
        "UPDATE circle_members SET role = 'member' WHERE circle_id = $1 AND role = 'owner'"
    )
    .bind(circle_id)
    .execute(&mut *tx)
    .await?;

    if is_member.is_some() {
        // Just upgrade them
        sqlx::query(
            "UPDATE circle_members SET role = 'owner' WHERE circle_id = $1 AND user_id = $2"
        )
        .bind(circle_id)
        .bind(new_owner_id)
        .execute(&mut *tx)
        .await?;
    } else {
        // Check if full (this is admin so forcibly allow them over limits maybe?)
        sqlx::query(
            "INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'owner')"
        )
        .bind(circle_id)
        .bind(new_owner_id)
        .execute(&mut *tx)
        .await?;
        
        sqlx::query("UPDATE circles SET member_count = member_count + 1 WHERE id = $1")
            .bind(circle_id)
            .execute(&mut *tx)
            .await?;
            
        sqlx::query("UPDATE community_profiles SET circle_id = $1 WHERE user_id = $2")
            .bind(circle_id)
            .bind(new_owner_id)
            .execute(&mut *tx)
            .await?;
    }

    // Set circles.owner_id
    sqlx::query(
        "UPDATE circles SET owner_id = $1 WHERE id = $2"
    )
    .bind(new_owner_id)
    .bind(circle_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Get all members of a circle.
pub async fn get_circle_members(pool: &PgPool, circle_id: Uuid) -> Result<Vec<CircleMember>, AppError> {
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
    let existing_circle: Option<Uuid> = sqlx::query_scalar(
        "SELECT circle_id FROM circle_members WHERE user_id = $1 LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if existing_circle.is_some() {
        return Err(AppError::BadRequest("You are already in a circle. Leave first.".into()));
    }

    // Check circle exists and is public
    let (is_public, member_count, max_members): (bool, i32, i32) = sqlx::query_as(
        "SELECT is_public, member_count, max_members FROM circles WHERE id = $1"
    )
    .bind(circle_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    if !is_public {
        return Err(AppError::BadRequest("This circle is private. You need an invite.".into()));
    }

    if member_count >= max_members {
        return Err(AppError::BadRequest("This circle is full.".into()));
    }

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO circle_members (circle_id, user_id, role) VALUES ($1, $2, 'member')"
    )
    .bind(circle_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE circles SET member_count = member_count + 1, updated_at = NOW() WHERE id = $1")
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
    let membership: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT circle_id, role FROM circle_members WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let (circle_id, role) = match membership {
        Some(m) => m,
        None => return Err(AppError::BadRequest("You are not in a circle.".into())),
    };

    if role == "owner" {
        return Err(AppError::BadRequest("Circle owners cannot leave. Transfer ownership or delete the circle.".into()));
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
pub async fn send_invite(pool: &PgPool, inviter_id: Uuid, invitee_id: Uuid, circle_id: Uuid) -> Result<CircleInvite, AppError> {
    // Check inviter permission
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2"
    )
    .bind(inviter_id)
    .bind(circle_id)
    .fetch_optional(pool)
    .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => {},
        _ => return Err(AppError::Unauthorized("Only circle owner/admin can invite".into())),
    }

    // Check invitee not already in a circle
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT circle_id FROM circle_members WHERE user_id = $1"
    )
    .bind(invitee_id)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest("User is already in a circle.".into()));
    }

    let invite = sqlx::query_as::<_, CircleInvite>(
        r#"INSERT INTO circle_invites (circle_id, inviter_id, invitee_id)
           VALUES ($1, $2, $3) RETURNING *"#
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
        "SELECT * FROM circle_invites WHERE id = $1 AND invitee_id = $2 AND status = 'pending'"
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

pub async fn get_circle_leaderboard(pool: &PgPool, limit: i64) -> Result<Vec<CircleLeaderboardEntry>, AppError> {
    let entries = sqlx::query_as::<_, CircleLeaderboardEntry>(
        r#"SELECT id, name, avatar_emoji, owner_id, member_count, total_xp, level, level_name
           FROM circles
           ORDER BY total_xp DESC
           LIMIT $1"#
    )
    .bind(limit.clamp(1, 50))
    .fetch_all(pool)
    .await?;
    Ok(entries)
}

// ─── Auto-Join from Referral ────────────────────────────────────────

/// Called after referral signup: automatically join the referrer's circle.
pub async fn auto_join_referrer_circle(pool: &PgPool, new_user_id: Uuid, referrer_id: Uuid) -> Result<(), AppError> {
    // Find referrer's circle
    let circle_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT circle_id FROM circle_members WHERE user_id = $1 LIMIT 1"
    )
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
pub async fn kick_member(pool: &PgPool, actor_id: Uuid, target_id: Uuid, circle_id: Uuid) -> Result<(), AppError> {
    // Check actor permission
    let actor_role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2"
    )
    .bind(actor_id)
    .bind(circle_id)
    .fetch_optional(pool)
    .await?;

    match actor_role.as_deref() {
        Some("owner") => {},
        Some("admin") => {
            // Admin can only kick members, not other admins
            let target_role: Option<String> = sqlx::query_scalar(
                "SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2"
            )
            .bind(target_id)
            .bind(circle_id)
            .fetch_optional(pool)
            .await?;
            if target_role.as_deref() != Some("member") {
                return Err(AppError::Unauthorized("Admins can only kick members".into()));
            }
        },
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

// ─── Admin Features ──────────────────────────────────────────────────

/// List all circles for admin (ordered by creation date).
pub async fn admin_get_all_circles(pool: &PgPool) -> Result<Vec<Circle>, AppError> {
    let circles = sqlx::query_as::<_, Circle>(
        "SELECT * FROM circles ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(circles)
}

/// Admin completely deletes a circle and unlinks all members.
pub async fn admin_delete_circle(pool: &PgPool, circle_id: Uuid) -> Result<(), AppError> {
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

    // Delete circle
    let res = sqlx::query("DELETE FROM circles WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;
        
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Circle not found".into()));
    }

    tx.commit().await?;
    Ok(())
}

/// Admin removes a user from a circle. Owner cannot be removed.
pub async fn admin_remove_member(pool: &PgPool, circle_id: Uuid, target_id: Uuid) -> Result<(), AppError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2"
    )
    .bind(target_id)
    .bind(circle_id)
    .fetch_optional(pool)
    .await?;

    let role = role.ok_or_else(|| AppError::NotFound("User not in circle".into()))?;
    if role == "owner" {
        return Err(AppError::BadRequest("Cannot remove circle owner. Delete circle instead.".into()));
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

