/// Expert AMAs — CRUD, Questions, Upvotes, Admin Answers.
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

const VALID_AMA_STATUSES: &[&str] = &[
    "draft",
    "scheduled",
    "accepting_questions",
    "live",
    "closed",
    "archived",
];
const MAX_TITLE_CHARS: usize = 300;
const MAX_EXPERT_NAME_CHARS: usize = 200;
const MAX_EXPERT_TITLE_CHARS: usize = 300;
const MAX_DESCRIPTION_CHARS: usize = 5_000;
const MAX_ANSWER_CHARS: usize = 5_000;

// ─── Models ──────────────────────────────────────────────────────────

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct Ama {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub expert_name: String,
    pub expert_title: Option<String>,
    pub expert_avatar_url: Option<String>,
    pub status: String,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub max_questions: i32,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct AmaQuestion {
    pub id: Uuid,
    pub ama_id: Uuid,
    pub user_id: Uuid,
    pub question: String,
    pub answer: Option<String>,
    pub answered_by: Option<Uuid>,
    pub answered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub upvote_count: i32,
    pub is_featured: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct AmaDetail {
    pub ama: Ama,
    pub questions: Vec<AmaQuestionWithMeta>,
    pub question_count: i64,
    pub user_has_reminded: bool,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct AmaQuestionWithMeta {
    pub id: Uuid,
    pub ama_id: Uuid,
    pub user_id: Uuid,
    pub question: String,
    pub answer: Option<String>,
    pub answered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub upvote_count: i32,
    pub is_featured: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub user_has_upvoted: bool,
}

// ─── List AMAs ───────────────────────────────────────────────────────

/// List AMAs visible to users (scheduled, accepting_questions, live, closed, archived).
pub async fn list_amas(pool: &PgPool) -> Result<Vec<Ama>, AppError> {
    let amas = sqlx::query_as::<_, Ama>(
        r#"SELECT id, title, description, expert_name, expert_title, expert_avatar_url,
                  status, scheduled_at, started_at, ended_at, max_questions, created_by, created_at
           FROM amas
           WHERE status != 'draft'
           ORDER BY
               CASE status
                   WHEN 'live' THEN 1
                   WHEN 'accepting_questions' THEN 2
                   WHEN 'scheduled' THEN 3
                   WHEN 'closed' THEN 4
                   WHEN 'archived' THEN 5
                   ELSE 6
               END,
               scheduled_at DESC NULLS LAST"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(amas)
}

/// List ALL AMAs (admin view — includes drafts).
pub async fn list_amas_admin(pool: &PgPool) -> Result<Vec<Ama>, AppError> {
    let amas = sqlx::query_as::<_, Ama>(
        r#"SELECT id, title, description, expert_name, expert_title, expert_avatar_url,
                  status, scheduled_at, started_at, ended_at, max_questions, created_by, created_at
           FROM amas
           ORDER BY created_at DESC"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(amas)
}

// ─── Get AMA Detail ──────────────────────────────────────────────────

pub async fn get_ama_detail(
    pool: &PgPool,
    ama_id: Uuid,
    user_id: Uuid,
) -> Result<AmaDetail, AppError> {
    get_ama_detail_with_visibility(pool, ama_id, user_id, false).await
}

pub async fn get_ama_detail_admin(
    pool: &PgPool,
    ama_id: Uuid,
    admin_id: Uuid,
) -> Result<AmaDetail, AppError> {
    get_ama_detail_with_visibility(pool, ama_id, admin_id, true).await
}

async fn get_ama_detail_with_visibility(
    pool: &PgPool,
    ama_id: Uuid,
    user_id: Uuid,
    include_drafts: bool,
) -> Result<AmaDetail, AppError> {
    let ama = sqlx::query_as::<_, Ama>(
        r#"SELECT id, title, description, expert_name, expert_title, expert_avatar_url,
                  status, scheduled_at, started_at, ended_at, max_questions, created_by, created_at
           FROM amas WHERE id = $1 AND ($2 OR status != 'draft')"#,
    )
    .bind(ama_id)
    .bind(include_drafts)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("AMA not found".into()))?;

    // Questions with upvote status for current user
    let questions = sqlx::query_as::<_, AmaQuestionWithMeta>(
        r#"SELECT q.id, q.ama_id, q.user_id, q.question, q.answer, q.answered_at,
                  q.upvote_count, q.is_featured, q.created_at,
                  EXISTS(SELECT 1 FROM ama_question_upvotes u WHERE u.question_id = q.id AND u.user_id = $2) AS user_has_upvoted
           FROM ama_questions q
           WHERE q.ama_id = $1
           ORDER BY q.is_featured DESC, q.upvote_count DESC, q.created_at ASC"#
    )
    .bind(ama_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let question_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM ama_questions WHERE ama_id = $1")
            .bind(ama_id)
            .fetch_one(pool)
            .await?;

    // Check if user set a reminder (we'll store this in a simple way — check if user submitted a "remind" entry)
    // For now we use a simple approach: no separate table, just return false
    let user_has_reminded = false;

    Ok(AmaDetail {
        ama,
        questions,
        question_count,
        user_has_reminded,
    })
}

// ─── Submit Question ─────────────────────────────────────────────────

pub async fn submit_question(
    pool: &PgPool,
    ama_id: Uuid,
    user_id: Uuid,
    question: &str,
) -> Result<AmaQuestion, AppError> {
    // Validate AMA status
    let (status, max_q): (String, i32) =
        sqlx::query_as("SELECT status, max_questions FROM amas WHERE id = $1")
            .bind(ama_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("AMA not found".into()))?;

    if !["accepting_questions", "live"].contains(&status.as_str()) {
        return Err(AppError::BadRequest(
            "This AMA is not currently accepting questions.".into(),
        ));
    }

    // Check max questions
    let current_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM ama_questions WHERE ama_id = $1")
            .bind(ama_id)
            .fetch_one(pool)
            .await?;

    if current_count >= max_q as i64 {
        return Err(AppError::BadRequest(format!(
            "This AMA has reached the maximum of {} questions.",
            max_q
        )));
    }

    // Check user hasn't submitted more than 3 questions per AMA
    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM ama_questions WHERE ama_id = $1 AND user_id = $2",
    )
    .bind(ama_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if user_count >= 3 {
        return Err(AppError::BadRequest(
            "You can submit a maximum of 3 questions per AMA.".into(),
        ));
    }

    let q = sqlx::query_as::<_, AmaQuestion>(
        r#"INSERT INTO ama_questions (ama_id, user_id, question)
           VALUES ($1, $2, $3)
           RETURNING id, ama_id, user_id, question, answer, answered_by, answered_at, upvote_count, is_featured, created_at"#
    )
    .bind(ama_id)
    .bind(user_id)
    .bind(question)
    .fetch_one(pool)
    .await?;

    Ok(q)
}

// ─── Toggle Upvote ───────────────────────────────────────────────────

/// Toggle upvote on a question. Returns true if added, false if removed.
pub async fn toggle_upvote(
    pool: &PgPool,
    question_id: Uuid,
    user_id: Uuid,
) -> Result<bool, AppError> {
    // Attempt insert
    let inserted = sqlx::query(
        "INSERT INTO ama_question_upvotes (question_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
    .bind(question_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if inserted.rows_affected() > 0 {
        return Ok(true);
    }

    // Already existed — remove (toggle off)
    sqlx::query("DELETE FROM ama_question_upvotes WHERE question_id = $1 AND user_id = $2")
        .bind(question_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(false)
}

// ─── Admin: Create AMA ──────────────────────────────────────────────

pub async fn create_ama(
    pool: &PgPool,
    admin_id: Uuid,
    title: &str,
    description: Option<&str>,
    expert_name: &str,
    expert_title: Option<&str>,
    expert_avatar_url: Option<&str>,
    scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    status: Option<&str>,
) -> Result<Ama, AppError> {
    let title = validate_required_text(title, "Title", MAX_TITLE_CHARS)?;
    let expert_name = validate_required_text(expert_name, "Expert name", MAX_EXPERT_NAME_CHARS)?;
    let description = validate_optional_text(description, "Description", MAX_DESCRIPTION_CHARS)?;
    let expert_title =
        validate_optional_text(expert_title, "Expert title", MAX_EXPERT_TITLE_CHARS)?;
    let st = status.unwrap_or("scheduled");
    validate_ama_status(st)?;

    let ama = sqlx::query_as::<_, Ama>(
        r#"INSERT INTO amas (title, description, expert_name, expert_title, expert_avatar_url, scheduled_at, status, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, title, description, expert_name, expert_title, expert_avatar_url,
                     status, scheduled_at, started_at, ended_at, max_questions, created_by, created_at"#
    )
    .bind(&title)
    .bind(description.as_deref())
    .bind(&expert_name)
    .bind(expert_title.as_deref())
    .bind(expert_avatar_url)
    .bind(scheduled_at)
    .bind(st)
    .bind(admin_id)
    .fetch_one(pool)
    .await?;

    Ok(ama)
}

// ─── Admin: Update AMA Status ────────────────────────────────────────

pub async fn update_ama_status(
    pool: &PgPool,
    ama_id: Uuid,
    new_status: &str,
) -> Result<(), AppError> {
    validate_ama_status(new_status)?;

    // Set timestamps based on status
    let result =
        match new_status {
            "live" => sqlx::query(
                "UPDATE amas SET status = $1, started_at = NOW(), updated_at = NOW() WHERE id = $2",
            )
            .bind(new_status)
            .bind(ama_id)
            .execute(pool)
            .await?,
            "closed" | "archived" => sqlx::query(
                "UPDATE amas SET status = $1, ended_at = NOW(), updated_at = NOW() WHERE id = $2",
            )
            .bind(new_status)
            .bind(ama_id)
            .execute(pool)
            .await?,
            _ => {
                sqlx::query("UPDATE amas SET status = $1, updated_at = NOW() WHERE id = $2")
                    .bind(new_status)
                    .bind(ama_id)
                    .execute(pool)
                    .await?
            }
        };

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("AMA not found".into()));
    }

    Ok(())
}

// ─── Admin: Answer Question ──────────────────────────────────────────

pub async fn answer_question(
    pool: &PgPool,
    ama_id: Uuid,
    question_id: Uuid,
    admin_id: Uuid,
    answer: &str,
) -> Result<(Uuid, Uuid), AppError> {
    let answer = validate_required_text(answer, "Answer", MAX_ANSWER_CHARS)?;

    let result = sqlx::query(
        "UPDATE ama_questions SET answer = $1, answered_by = $2, answered_at = NOW() WHERE id = $3 AND ama_id = $4",
    )
    .bind(&answer)
    .bind(admin_id)
    .bind(question_id)
    .bind(ama_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Question not found".into()));
    }

    // Award XP to the question author and notify them
    let info: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_id, ama_id FROM ama_questions WHERE id = $1 AND ama_id = $2")
            .bind(question_id)
            .bind(ama_id)
            .fetch_optional(pool)
            .await?;

    if let Some((uid, ama_id)) = info {
        let _ = crate::community::xp::award_xp(
            pool,
            uid,
            "ama_question",
            Some("Your AMA question was answered!"),
            Some(50),
        )
        .await;

        let notif_content = "Your AMA question was answered by the expert!".to_string();
        let link = format!("/community/feed?ama={}", ama_id);
        let _ = crate::community::notifications::notify_user(
            pool,
            uid,
            None, // platform
            "ama_answer",
            Some(ama_id),
            &notif_content,
            Some(&link),
        )
        .await;

        return Ok((uid, ama_id));
    }

    Err(AppError::NotFound("Question not found".into()))
}

// ─── Admin: Feature/Unfeature Question ───────────────────────────────

pub async fn toggle_featured(
    pool: &PgPool,
    ama_id: Uuid,
    question_id: Uuid,
) -> Result<bool, AppError> {
    let new_val: bool = sqlx::query_scalar(
        "UPDATE ama_questions SET is_featured = NOT is_featured WHERE id = $1 AND ama_id = $2 RETURNING is_featured"
    )
    .bind(question_id)
    .bind(ama_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Question not found".into()))?;

    Ok(new_val)
}

// ─── Question Count per AMA ──────────────────────────────────────────

pub async fn get_question_count(pool: &PgPool, ama_id: Uuid) -> Result<i64, AppError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM ama_questions WHERE ama_id = $1")
            .bind(ama_id)
            .fetch_one(pool)
            .await?;

    Ok(count)
}

fn validate_ama_status(status: &str) -> Result<(), AppError> {
    if VALID_AMA_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Invalid AMA status: {}",
            status
        )))
    }
}

fn validate_required_text(value: &str, field: &str, max_chars: usize) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required.", field)));
    }
    if trimmed.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!(
            "{} must be {} characters or fewer.",
            field, max_chars
        )));
    }
    Ok(trimmed.to_string())
}

fn validate_optional_text(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!(
            "{} must be {} characters or fewer.",
            field, max_chars
        )));
    }
    Ok(Some(trimmed.to_string()))
}
