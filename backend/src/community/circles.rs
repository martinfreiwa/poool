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
    // Slug is required by the multi-circle URLs (`/community/circle/:slug/…`)
    // and by the settings-page hydration (`hydrateForm()` reads `c.slug`).
    // The DB column is NOT NULL — every row has one.
    pub slug: String,
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
    // CO.2: optional custom banner image. NULL = default CSS background.
    #[sqlx(default)]
    pub banner_url: Option<String>,
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
///
/// `bypass_ownership_check` lets super-admins create unlimited circles
/// (used for moderation, recovery, and seeding). Regular users hit the
/// one-circle-per-owner limit.
pub async fn create_circle(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    description: Option<&str>,
    emoji: Option<&str>,
    bypass_ownership_check: bool,
) -> Result<Circle, AppError> {
    if !bypass_ownership_check {
        let existing: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM circles WHERE owner_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

        if existing.is_some() {
            return Err(AppError::BadRequest("You already own a circle".into()));
        }
    }

    // Slug: lowercase name, alphanumeric only, dash-collapsed. If the slug
    // is already taken (or the name is purely non-alphanumeric), suffix a
    // short random hex chunk for uniqueness.
    let base_slug = {
        let mut s: String = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        while s.contains("--") {
            s = s.replace("--", "-");
        }
        s.trim_matches('-').chars().take(54).collect::<String>()
    };
    let mut slug = if base_slug.is_empty() { "circle".to_string() } else { base_slug };
    let collision: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM circles WHERE LOWER(slug) = LOWER($1))",
    )
    .bind(&slug)
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if collision {
        let suffix: String = Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(6)
            .collect();
        slug = format!("{}-{}", slug, suffix);
    }

    let mut tx = pool.begin().await?;

    let circle = sqlx::query_as::<_, Circle>(
        r#"INSERT INTO circles (name, slug, description, owner_id, avatar_emoji, member_count)
           VALUES ($1, $2, $3, $4, $5, 1)
           RETURNING *"#,
    )
    .bind(name)
    .bind(&slug)
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

/// Update circle name/description/banner (owner only). Pass `Some("")` for
/// `banner_url` to clear it; pass `None` to leave the existing value untouched.
pub async fn update_circle(
    pool: &PgPool,
    circle_id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    emoji: Option<&str>,
    banner_url: Option<&str>,
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

    // Convert "" → SQL NULL so owners can clear the banner; treat anything
    // else as a literal value to set. None leaves the column alone.
    let banner_arg: Option<Option<&str>> =
        banner_url.map(|s| if s.is_empty() { None } else { Some(s) });

    let circle = sqlx::query_as::<_, Circle>(
        r#"UPDATE circles SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            avatar_emoji = COALESCE($4, avatar_emoji),
            banner_url = CASE WHEN $5::BOOL THEN $6 ELSE banner_url END,
            updated_at = NOW()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(circle_id)
    .bind(name)
    .bind(description)
    .bind(emoji)
    .bind(banner_arg.is_some())
    .bind(banner_arg.flatten())
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
/// As of 2026-05-16 the single-circle restriction is gone — users may
/// belong to as many circles as they like. UNIQUE(circle_id, user_id)
/// in the DB still prevents joining the SAME circle twice.
pub async fn join_circle(pool: &PgPool, user_id: Uuid, circle_id: Uuid) -> Result<(), AppError> {
    // Idempotency: if user is already a member of THIS circle, no-op.
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM circle_members WHERE circle_id = $1 AND user_id = $2)",
    )
    .bind(circle_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if already {
        return Ok(());
    }

    // Ban check: don't let a banned user re-join.
    let banned: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(SELECT 1 FROM circle_bans
             WHERE circle_id = $1 AND banned_user_id = $2
               AND (expires_at IS NULL OR expires_at > NOW()))"#,
    )
    .bind(circle_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if banned {
        return Err(AppError::Forbidden(
            "You are banned from this circle.".into(),
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

    // `community_profiles.circle_id` = the user's PRIMARY circle (UI default).
    // Only set on FIRST circle join — subsequent multi-joins do not steal
    // primary status. User can change primary explicitly via settings page.
    sqlx::query(
        "UPDATE community_profiles SET circle_id = $1 WHERE user_id = $2 AND circle_id IS NULL",
    )
    .bind(circle_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Award Gamification Challenge
    crate::community::challenges::increment_progress(pool, user_id, "join_circle", 1).await?;

    // CO.5: auto-send a welcome notification to the new member so the
    // first interaction with the circle is something concrete (not a
    // "you joined" silence). Fire-and-forget — failure here must NOT
    // unwind the join itself.
    if let Ok(Some(name)) =
        sqlx::query_scalar::<_, String>("SELECT name FROM circles WHERE id = $1")
            .bind(circle_id)
            .fetch_optional(pool)
            .await
    {
        let welcome = format!("Welcome to {}! Say hello in the circle feed.", name);
        let _ = crate::community::notifications::notify_user(
            pool,
            user_id,
            None,
            "system_alert",
            Some(circle_id),
            &welcome,
            Some("/community?tab=circle"),
        )
        .await;
    }

    Ok(())
}

/// Leave a circle. Owner cannot leave (must transfer or delete).
/// Multi-join era: caller MUST specify `target_circle_id` — the days of
/// "user is in exactly one circle" are over.
pub async fn leave_circle(
    pool: &PgPool,
    user_id: Uuid,
    target_circle_id: Uuid,
) -> Result<(), AppError> {
    // Multi-join era: caller MUST specify which circle to leave. Look up
    // the user's role IN THIS SPECIFIC circle to enforce owner-cannot-leave.
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE user_id = $1 AND circle_id = $2")
            .bind(user_id)
            .bind(target_circle_id)
            .fetch_optional(pool)
            .await?;

    let role = match role {
        Some(r) => r,
        None => return Err(AppError::BadRequest("You are not in this circle.".into())),
    };

    if role == "owner" {
        return Err(AppError::BadRequest(
            "Circle owners cannot leave. Transfer ownership or delete the circle.".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM circle_members WHERE user_id = $1 AND circle_id = $2")
        .bind(user_id)
        .bind(target_circle_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE circles SET member_count = GREATEST(0, member_count - 1), updated_at = NOW() WHERE id = $1")
        .bind(target_circle_id)
        .execute(&mut *tx)
        .await?;

    // If the user's primary (community_profiles.circle_id) was the one they
    // just left, demote primary status. Their next remaining circle (if any)
    // can become primary on next join or via explicit settings choice.
    sqlx::query(
        "UPDATE community_profiles SET circle_id = NULL
         WHERE user_id = $1 AND circle_id = $2",
    )
    .bind(user_id)
    .bind(target_circle_id)
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

    // Check invitee not already in THIS circle (multi-circle era).
    let already_in_this: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM circle_members WHERE user_id = $1 AND circle_id = $2)",
    )
    .bind(invitee_id)
    .bind(circle_id)
    .fetch_one(pool)
    .await?;
    if already_in_this {
        return Err(AppError::BadRequest(
            "User is already in this circle.".into(),
        ));
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
    pub is_public: bool,
}

pub async fn get_circle_leaderboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<CircleLeaderboardEntry>, AppError> {
    let entries = sqlx::query_as::<_, CircleLeaderboardEntry>(
        r#"SELECT id, name, avatar_emoji, owner_id, member_count, total_xp, level, level_name, is_public
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
    // Multi-circle era: check user isn't already in THIS specific circle.
    let already_in_this: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM circle_members WHERE user_id = $1 AND circle_id = $2)",
    )
    .bind(user_id)
    .bind(circle_id)
    .fetch_one(pool)
    .await?;
    if already_in_this {
        return Err(AppError::BadRequest(
            "You are already in this circle.".into(),
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-circle discovery, search, slug-lookup, bans (2026-05-16 rework).
// ═══════════════════════════════════════════════════════════════════════

/// Slim Circle row for discover / search list cards. Smaller payload than
/// the full Circle struct — no XP totals, no level metadata.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct CircleCardRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub avatar_emoji: Option<String>,
    pub banner_url: Option<String>,
    pub member_count: i32,
    pub max_members: i32,
    pub is_public: bool,
    pub is_featured: bool,
    pub recent_post_count: i32,
}

/// Discover endpoint payload: 3 curated lists.
#[derive(Debug, serde::Serialize)]
pub struct DiscoverPayload {
    pub featured: Vec<CircleCardRow>,
    pub trending: Vec<CircleCardRow>,
    pub new: Vec<CircleCardRow>,
}

/// `GET /api/community/circles/discover` data: 3 sections × up to 10 each.
/// - `featured` = `is_featured = TRUE`, sorted by `featured_at DESC`.
/// - `trending` = sorted by `recent_post_count DESC` (refreshed by background
///   job; fallback to member_count DESC on cold start).
/// - `new`      = sorted by `created_at DESC`, last 30 days only.
pub async fn discover_circles(pool: &PgPool) -> Result<DiscoverPayload, AppError> {
    const COLS: &str = "id, slug, name, description, avatar_emoji, banner_url, \
                        member_count, max_members, is_public, is_featured, recent_post_count";

    let featured: Vec<CircleCardRow> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM circles
         WHERE is_featured = TRUE AND is_public = TRUE
         ORDER BY featured_at DESC NULLS LAST, member_count DESC
         LIMIT 10",
    ))
    .fetch_all(pool)
    .await?;

    let trending: Vec<CircleCardRow> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM circles
         WHERE is_public = TRUE AND is_featured = FALSE
         ORDER BY recent_post_count DESC, member_count DESC, created_at DESC
         LIMIT 10",
    ))
    .fetch_all(pool)
    .await?;

    let new_list: Vec<CircleCardRow> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM circles
         WHERE is_public = TRUE AND is_featured = FALSE
           AND created_at >= NOW() - INTERVAL '30 days'
         ORDER BY created_at DESC
         LIMIT 10",
    ))
    .fetch_all(pool)
    .await?;

    Ok(DiscoverPayload {
        featured,
        trending,
        new: new_list,
    })
}

/// Compact member preview used to render face-avatar stacks on Discover
/// cards. Five faces is enough to communicate "real community" without
/// blowing up the payload.
#[derive(Debug, serde::Serialize, Clone)]
pub struct MemberMini {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Fetches up to `limit` most-recent members per circle (community DB),
/// then hydrates display_name + avatar_url from the core DB via the
/// existing user-bridge batch helper. Returns a map keyed by circle_id
/// so callers can attach the slice to their card payload in O(1).
///
/// Silent on failure — face stacks are a nice-to-have, not load-bearing,
/// so the caller falls back to the placeholder stack when this returns
/// an empty map.
pub async fn get_member_previews(
    community_pool: &PgPool,
    core_pool: &PgPool,
    redis_pool: Option<&deadpool_redis::Pool>,
    circle_ids: &[Uuid],
    limit: i32,
) -> std::collections::HashMap<Uuid, Vec<MemberMini>> {
    use sqlx::Row;
    let mut out: std::collections::HashMap<Uuid, Vec<MemberMini>> =
        std::collections::HashMap::new();
    if circle_ids.is_empty() {
        return out;
    }

    // First N per circle via window function. ROW_NUMBER OVER PARTITION
    // is the canonical "top N per group" pattern and lets us hit the
    // (circle_id, user_id) index efficiently.
    let rows = match sqlx::query(
        r#"
        SELECT circle_id, user_id
        FROM (
            SELECT circle_id, user_id,
                   ROW_NUMBER() OVER (PARTITION BY circle_id ORDER BY joined_at DESC) AS rn
            FROM circle_members
            WHERE circle_id = ANY($1)
        ) ranked
        WHERE rn <= $2
        "#,
    )
    .bind(circle_ids)
    .bind(limit as i64)
    .fetch_all(community_pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("get_member_previews query failed: {}", e);
            return out;
        }
    };

    if rows.is_empty() {
        return out;
    }

    // Collect distinct user_ids → batch-hydrate from core db
    let mut pairs: Vec<(Uuid, Uuid)> = Vec::with_capacity(rows.len());
    let mut user_ids: Vec<Uuid> = Vec::with_capacity(rows.len());
    for r in &rows {
        let circle_id: Uuid = match r.try_get("circle_id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let user_id: Uuid = match r.try_get("user_id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        pairs.push((circle_id, user_id));
        user_ids.push(user_id);
    }
    user_ids.sort();
    user_ids.dedup();

    let info_map = match crate::community::user_bridge::get_users_info_batch(
        core_pool, redis_pool, &user_ids,
    )
    .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("get_member_previews bridge failed: {}", e);
            return out;
        }
    };

    for (circle_id, user_id) in pairs {
        let entry = out.entry(circle_id).or_default();
        if let Some(info) = info_map.get(&user_id) {
            entry.push(MemberMini {
                user_id,
                display_name: info.display_name.clone(),
                avatar_url: info.avatar_url.clone(),
            });
        }
    }
    out
}

/// `GET /api/community/circles/search?q=…&page=…` — paginated search over
/// public circles by name + description. Uses pg_trgm indexes added in
/// migration 045 for fast ILIKE lookups.
pub async fn search_circles(
    pool: &PgPool,
    query: &str,
    page: i64,
    per_page: i64,
) -> Result<(Vec<CircleCardRow>, i64), AppError> {
    let q = query.trim();
    if q.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let offset = (page.max(1) - 1) * per_page;
    let needle = format!("%{}%", q.to_lowercase());

    let rows: Vec<CircleCardRow> = sqlx::query_as(
        r#"SELECT id, slug, name, description, avatar_emoji, banner_url,
                  member_count, max_members, is_public, is_featured, recent_post_count
           FROM circles
           WHERE is_public = TRUE
             AND (LOWER(name) LIKE $1 OR LOWER(COALESCE(description, '')) LIKE $1)
           ORDER BY
               -- exact-prefix match first, then by member count
               CASE WHEN LOWER(name) LIKE LOWER($2) || '%' THEN 0 ELSE 1 END,
               member_count DESC, created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(&needle)
    .bind(q)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT FROM circles
           WHERE is_public = TRUE
             AND (LOWER(name) LIKE $1 OR LOWER(COALESCE(description, '')) LIKE $1)"#,
    )
    .bind(&needle)
    .fetch_one(pool)
    .await?;

    Ok((rows, total))
}

/// `GET /api/community/circles/by-slug/:slug` — resolve slug to full Circle
/// row + viewer's role (None if not a member).
pub async fn get_circle_by_slug(
    pool: &PgPool,
    slug: &str,
    viewer_id: Option<Uuid>,
) -> Result<(Circle, Option<String>), AppError> {
    let circle: Circle = sqlx::query_as("SELECT * FROM circles WHERE LOWER(slug) = LOWER($1)")
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    let role: Option<String> = if let Some(uid) = viewer_id {
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle.id)
            .bind(uid)
            .fetch_optional(pool)
            .await?
    } else {
        None
    };

    Ok((circle, role))
}

/// `GET /api/community/me/circles` — every circle the viewer is a member of,
/// with role. Drives the "My Circles" sidebar list on the new UI.
pub async fn list_my_circles(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(CircleCardRow, String)>, AppError> {
    let rows = sqlx::query(
        r#"SELECT c.id, c.slug, c.name, c.description, c.avatar_emoji, c.banner_url,
                  c.member_count, c.max_members, c.is_public, c.is_featured,
                  c.recent_post_count, cm.role
           FROM circles c
           JOIN circle_members cm ON cm.circle_id = c.id
           WHERE cm.user_id = $1
           ORDER BY cm.joined_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let card = CircleCardRow {
            id: r.try_get("id")?,
            slug: r.try_get("slug")?,
            name: r.try_get("name")?,
            description: r.try_get("description").ok(),
            avatar_emoji: r.try_get("avatar_emoji").ok(),
            banner_url: r.try_get("banner_url").ok(),
            member_count: r.try_get("member_count")?,
            max_members: r.try_get("max_members")?,
            is_public: r.try_get("is_public")?,
            is_featured: r.try_get("is_featured")?,
            recent_post_count: r.try_get("recent_post_count")?,
        };
        let role: String = r.try_get("role")?;
        out.push((card, role));
    }
    Ok(out)
}

/// Promote member to moderator (or demote). Only owner can grant/revoke
/// the moderator role. Existing `update_circle_member_role` handles
/// 'admin' ↔ 'member' transitions; this helper is for the new role.
pub async fn set_member_moderator(
    pool: &PgPool,
    actor_id: Uuid,
    circle_id: Uuid,
    target_user_id: Uuid,
    moderator: bool,
) -> Result<(), AppError> {
    // Only the OWNER can mint or revoke moderators (admins cannot).
    let actor_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(actor_id)
            .fetch_optional(pool)
            .await?;
    if actor_role.as_deref() != Some("owner") {
        return Err(AppError::Forbidden(
            "Only the circle owner can set moderators.".into(),
        ));
    }

    let target_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(target_user_id)
            .fetch_optional(pool)
            .await?;
    let current = target_role.ok_or_else(|| {
        AppError::BadRequest("Target user is not a member of this circle.".into())
    })?;
    if current == "owner" {
        return Err(AppError::BadRequest(
            "Cannot change the owner's role.".into(),
        ));
    }

    let new_role = if moderator { "moderator" } else { "member" };
    sqlx::query("UPDATE circle_members SET role = $1 WHERE circle_id = $2 AND user_id = $3")
        .bind(new_role)
        .bind(circle_id)
        .bind(target_user_id)
        .execute(pool)
        .await?;
    // Audit trail — fire-and-forget, never blocks the response.
    crate::community::audit::log(
        pool,
        actor_id,
        if moderator {
            "circle.promote_moderator"
        } else {
            "circle.demote_moderator"
        },
        "circle",
        Some(circle_id),
        Some(target_user_id),
        Some(serde_json::json!({
            "new_role": new_role,
            "previous_role": current,
        })),
    )
    .await;
    Ok(())
}

/// Ban a member: insert into `circle_bans` + remove from `circle_members`.
/// Allowed for: owner, admin, moderator. Targets at moderator+ rank
/// cannot be banned by a peer (moderator can't ban another moderator).
pub async fn ban_member(
    pool: &PgPool,
    actor_id: Uuid,
    circle_id: Uuid,
    target_user_id: Uuid,
    reason: Option<String>,
) -> Result<(), AppError> {
    let actor_role: String =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(actor_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::Forbidden("You are not a member of this circle.".into()))?;

    if !matches!(actor_role.as_str(), "owner" | "admin" | "moderator") {
        return Err(AppError::Forbidden(
            "Only owners, admins or moderators can ban.".into(),
        ));
    }

    let target_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(target_user_id)
            .fetch_optional(pool)
            .await?;
    if let Some(t) = target_role.as_deref() {
        if t == "owner" {
            return Err(AppError::BadRequest("Cannot ban the circle owner.".into()));
        }
        // Mods can't ban mods (or admins). Owner/admin can ban anyone below.
        let actor_rank = role_rank(&actor_role);
        let target_rank = role_rank(t);
        if target_rank >= actor_rank {
            return Err(AppError::Forbidden(
                "Cannot ban a member with equal or higher rank.".into(),
            ));
        }
    }

    // Clone for the audit-log call after the transaction commits — the
    // .bind() below moves the original into the prepared statement.
    let reason_for_audit = reason.clone();

    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"INSERT INTO circle_bans (circle_id, banned_user_id, banned_by, reason)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (circle_id, banned_user_id) DO UPDATE
              SET banned_by = EXCLUDED.banned_by,
                  reason    = EXCLUDED.reason,
                  banned_at = NOW(),
                  expires_at = NULL"#,
    )
    .bind(circle_id)
    .bind(target_user_id)
    .bind(actor_id)
    .bind(reason)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM circle_members WHERE circle_id = $1 AND user_id = $2")
        .bind(circle_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE circles SET member_count = GREATEST(0, member_count - 1) WHERE id = $1")
        .bind(circle_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE community_profiles SET circle_id = NULL WHERE user_id = $1 AND circle_id = $2",
    )
    .bind(target_user_id)
    .bind(circle_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Audit trail for ban — includes the actor's role + the target's
    // prior role so compliance can reconstruct authority chains.
    crate::community::audit::log(
        pool,
        actor_id,
        "circle.ban_member",
        "circle",
        Some(circle_id),
        Some(target_user_id),
        Some(serde_json::json!({
            "actor_role": actor_role,
            "target_role_before": target_role,
            "reason": reason_for_audit,
        })),
    )
    .await;
    Ok(())
}

/// Unban a user. Owner-only.
pub async fn unban_member(
    pool: &PgPool,
    actor_id: Uuid,
    circle_id: Uuid,
    target_user_id: Uuid,
) -> Result<(), AppError> {
    let actor_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(actor_id)
            .fetch_optional(pool)
            .await?;
    if !matches!(actor_role.as_deref(), Some("owner") | Some("admin")) {
        return Err(AppError::Forbidden("Only owner/admin can unban.".into()));
    }
    sqlx::query("DELETE FROM circle_bans WHERE circle_id = $1 AND banned_user_id = $2")
        .bind(circle_id)
        .bind(target_user_id)
        .execute(pool)
        .await?;
    crate::community::audit::log(
        pool,
        actor_id,
        "circle.unban_member",
        "circle",
        Some(circle_id),
        Some(target_user_id),
        None,
    )
    .await;
    Ok(())
}

fn role_rank(role: &str) -> i32 {
    match role {
        "owner" => 4,
        "admin" => 3,
        "moderator" => 2,
        "member" => 1,
        _ => 0,
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct CircleBanRow {
    pub banned_user_id: Uuid,
    pub banned_by: Uuid,
    pub reason: Option<String>,
    pub banned_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// List active bans on a circle. Owner/admin/moderator only.
/// Filters out expired bans (`expires_at < NOW()`) so the UI only shows
/// currently-enforced bans. Permanent bans (`expires_at IS NULL`) always
/// show.
pub async fn list_circle_bans(
    pool: &PgPool,
    viewer_id: Uuid,
    circle_id: Uuid,
) -> Result<Vec<CircleBanRow>, AppError> {
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(viewer_id)
            .fetch_optional(pool)
            .await?;
    if !matches!(
        role.as_deref(),
        Some("owner") | Some("admin") | Some("moderator")
    ) {
        return Err(AppError::Forbidden(
            "Only owner/admin/moderator can view ban list.".into(),
        ));
    }

    let rows = sqlx::query_as::<_, CircleBanRow>(
        r#"SELECT banned_user_id, banned_by, reason, banned_at, expires_at
           FROM circle_bans
           WHERE circle_id = $1
             AND (expires_at IS NULL OR expires_at > NOW())
           ORDER BY banned_at DESC"#,
    )
    .bind(circle_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Set or clear the cover-photo URL on the user's community profile.
/// Pass `Some(url)` to set, `None` to clear. Idempotent — running twice
/// with the same value is a no-op.
pub async fn set_profile_banner(
    pool: &PgPool,
    user_id: Uuid,
    banner_url: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE community_profiles SET banner_url = $1 WHERE user_id = $2")
        .bind(banner_url)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
