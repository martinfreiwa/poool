use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub const REQUIREMENT_TYPES: &[&str] = &[
    "kyc_approved",
    "buy_asset",
    "write_review",
    "join_circle",
    "login_streak",
];

pub const FREQUENCIES: &[&str] = &["one_time", "daily", "weekly"];

const MAX_TITLE_LEN: usize = 255;
const MAX_DESCRIPTION_LEN: usize = 5_000;
const MAX_BADGE_CODE_LEN: usize = 50;
const MAX_XP_REWARD: i32 = 10_000;
const MAX_REQUIREMENT_VALUE: i32 = 10_000;

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
        "#,
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
    let matching_challenges: Vec<Challenge> =
        sqlx::query_as("SELECT * FROM challenges WHERE requirement_type = $1 AND is_active = true")
            .bind(requirement_type)
            .fetch_all(community_pool)
            .await?;

    let mut newly_completed = Vec::new();

    for challenge in matching_challenges {
        // Upsert progress
        let (current_val, is_completed): (i32, bool) = sqlx::query_as(
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
            // Mark completed
            sqlx::query(
                r#"
                UPDATE challenge_progress 
                SET is_completed = true, completed_at = NOW() 
                WHERE user_id = $1 AND challenge_id = $2
                "#,
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
            )
            .await;

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
    let title = validate_text_field("Title", title, MAX_TITLE_LEN)?;
    let description = validate_text_field("Description", description, MAX_DESCRIPTION_LEN)?;
    validate_range("XP reward", xp_reward, 0, MAX_XP_REWARD)?;
    validate_range("Requirement value", req_value, 1, MAX_REQUIREMENT_VALUE)?;
    validate_allowed("Requirement type", req_type, REQUIREMENT_TYPES)?;
    validate_allowed("Frequency", frequency, FREQUENCIES)?;

    let badge_reward = match badge_reward {
        Some(code) => {
            let code = validate_text_field("Badge reward", code, MAX_BADGE_CODE_LEN)?;
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM badges WHERE code = $1)")
                    .bind(code)
                    .fetch_one(community_pool)
                    .await?;

            if !exists {
                return Err(AppError::BadRequest(
                    "Badge reward must reference an existing badge code.".to_string(),
                ));
            }

            Some(code)
        }
        None => None,
    };

    let challenge = sqlx::query_as::<_, Challenge>(
        r#"
        INSERT INTO challenges (title, description, xp_reward, badge_reward, requirement_type, requirement_value, frequency)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(title)
    .bind(description)
    .bind(xp_reward)
    .bind(badge_reward)
    .bind(req_type)
    .bind(req_value)
    .bind(frequency)
    .fetch_one(community_pool)
    .await?;

    Ok(challenge)
}

pub async fn admin_toggle_challenge(
    community_pool: &PgPool,
    challenge_id: Uuid,
    is_active: bool,
) -> Result<Challenge, AppError> {
    let challenge = sqlx::query_as::<_, Challenge>(
        "UPDATE challenges SET is_active = $1 WHERE id = $2 RETURNING *",
    )
    .bind(is_active)
    .bind(challenge_id)
    .fetch_optional(community_pool)
    .await?;

    challenge.ok_or_else(|| AppError::NotFound("Challenge not found.".to_string()))
}

fn validate_text_field<'a>(
    name: &str,
    value: &'a str,
    max_len: usize,
) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{name} is required.")));
    }

    if value.chars().count() > max_len {
        return Err(AppError::BadRequest(format!(
            "{name} must be {max_len} characters or fewer."
        )));
    }

    Ok(value)
}

fn validate_range(name: &str, value: i32, min: i32, max: i32) -> Result<(), AppError> {
    if value < min || value > max {
        return Err(AppError::BadRequest(format!(
            "{name} must be between {min} and {max}."
        )));
    }

    Ok(())
}

fn validate_allowed(name: &str, value: &str, allowed: &[&str]) -> Result<(), AppError> {
    if allowed.contains(&value) {
        return Ok(());
    }

    Err(AppError::BadRequest(format!(
        "{name} must be one of: {}.",
        allowed.join(", ")
    )))
}
