use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

// ─── Models ─────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct Challenge {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub xp_reward: i32,
    pub badge_reward: Option<String>,
    pub requirement_type: String, // e.g., "buy_asset"
    pub requirement_value: i32,
    pub frequency: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ChallengeWithProgress {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub challenge: Challenge,
    pub current_value: i32,
    pub is_completed: bool,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ─── Core Logic ─────────────────────────────────────────────────────

/// List all active challenges, including the user's progress for each.
pub async fn list_challenges_for_user(
    community_pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ChallengeWithProgress>, AppError> {
    let rows = sqlx::query_as::<_, ChallengeWithProgress>(
        r#"
        SELECT 
            c.id, c.title, c.description, c.xp_reward, c.badge_reward, 
            c.requirement_type, c.requirement_value, c.frequency, c.is_active, c.created_at,
            COALESCE(cp.current_value, 0) AS current_value,
            COALESCE(cp.is_completed, false) AS is_completed,
            cp.completed_at
        FROM challenges c
        LEFT JOIN challenge_progress cp ON cp.challenge_id = c.id AND cp.user_id = $1
        WHERE c.is_active = true
        ORDER BY COALESCE(cp.is_completed, false) ASC, c.xp_reward DESC, c.created_at ASC
        "#
    )
    .bind(user_id)
    .fetch_all(community_pool)
    .await?;

    Ok(rows)
}

/// Increment progress on a challenge by type.
pub async fn increment_progress(
    community_pool: &PgPool,
    user_id: Uuid,
    requirement_type: &str,
    increment_by: i32,
) -> Result<Vec<Uuid>, AppError> {
    // Find all active challenges with this requirement_type
    let matching_challenges: Vec<Challenge> = sqlx::query_as(
        "SELECT * FROM challenges WHERE requirement_type = $1 AND is_active = true"
    )
    .bind(requirement_type)
    .fetch_all(community_pool)
    .await?;

    let mut newly_completed = Vec::new();

    for challenge in matching_challenges {
        // Upsert progress
        let (current_val, mut is_completed): (i32, bool) = sqlx::query_as(
            r#"
            INSERT INTO challenge_progress (user_id, challenge_id, current_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, challenge_id) DO UPDATE SET
                current_value = LEAST(challenge_progress.current_value + EXCLUDED.current_value, $4),
                updated_at = NOW()
            RETURNING current_value, is_completed
            "#
        )
        .bind(user_id)
        .bind(challenge.id)
        .bind(increment_by)
        .bind(challenge.requirement_value)
        .fetch_one(community_pool)
        .await?;

        // Check if just completed
        if !is_completed && current_val >= challenge.requirement_value {
            is_completed = true;

            // Mark completed
            sqlx::query(
                r#"
                UPDATE challenge_progress 
                SET is_completed = true, completed_at = NOW() 
                WHERE user_id = $1 AND challenge_id = $2
                "#
            )
            .bind(user_id)
            .bind(challenge.id)
            .execute(community_pool)
            .await?;

            // Grant XP immediately
            if challenge.xp_reward > 0 {
                let metadata = format!("Completed challenge: {}", challenge.title);
                crate::community::xp::award_xp(
                    community_pool,
                    user_id,
                    "challenge_completed",
                    Some(&metadata),
                    Some(challenge.xp_reward),
                )
                .await?;
            }

            // Notify user
            let notif_content = format!("You completed the '{}' challenge!", challenge.title);
            let _ = crate::community::notifications::notify_user(
                community_pool,
                user_id,
                None, // system
                "challenge_completed",
                Some(challenge.id),
                &notif_content,
                Some("/community?tab=challenges"),
            ).await;

            newly_completed.push(challenge.id);
        }
    }

    Ok(newly_completed)
}

// ─── Admin Logic ────────────────────────────────────────────────────

pub async fn admin_create_challenge(
    community_pool: &PgPool,
    title: &str,
    description: &str,
    xp_reward: i32,
    badge_reward: Option<&str>,
    req_type: &str,
    req_value: i32,
    frequency: &str,
) -> Result<Challenge, AppError> {
    let challenge = sqlx::query_as::<_, Challenge>(
        r#"
        INSERT INTO challenges (title, description, xp_reward, badge_reward, requirement_type, requirement_value, frequency)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(title)
    .bind(description)
    .bind(xp_reward.max(0))
    .bind(badge_reward)
    .bind(req_type)
    .bind(req_value.max(1))
    .bind(frequency)
    .fetch_one(community_pool)
    .await?;

    Ok(challenge)
}

pub async fn admin_toggle_challenge(
    community_pool: &PgPool,
    challenge_id: Uuid,
    is_active: bool,
) -> Result<(), AppError> {
    sqlx::query("UPDATE challenges SET is_active = $1 WHERE id = $2")
        .bind(is_active)
        .bind(challenge_id)
        .execute(community_pool)
        .await?;
    Ok(())
}
