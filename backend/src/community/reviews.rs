use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

// ─── Models ─────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct AssetReview {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub user_id: Uuid,
    pub rating: i16,
    pub content: String,
    pub is_owner: bool,
    pub helpful_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct AssetReviewWithUser {
    #[serde(flatten)]
    pub review: AssetReview,
    pub user_display_name: String,
    pub user_avatar_url: Option<String>,
    pub has_upvoted: bool,
}

// ─── CRUD Operations ────────────────────────────────────────────────

/// Check if a user currently owns tokens in an asset (using the main DB pool).
pub async fn check_verified_owner(main_pool: &PgPool, user_id: Uuid, asset_id: Uuid) -> Result<bool, AppError> {
    let owned: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(tokens_owned), 0)::BIGINT FROM investments WHERE user_id = $1 AND asset_id = $2"
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_optional(main_pool)
    .await?;

    Ok(owned.unwrap_or(0) > 0)
}

/// Create or update a review.
pub async fn upsert_review(
    community_pool: &PgPool,
    main_pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    rating: i16,
    content: &str,
) -> Result<AssetReview, AppError> {
    if !(1..=5).contains(&rating) {
        return Err(AppError::BadRequest("Rating must be between 1 and 5".into()));
    }

    if content.trim().is_empty() {
        return Err(AppError::BadRequest("Review content cannot be empty".into()));
    }

    // Check ownership status in main DB
    let is_owner = check_verified_owner(main_pool, user_id, asset_id).await?;

    let review = sqlx::query_as::<_, AssetReview>(
        r#"
        INSERT INTO asset_reviews (asset_id, user_id, rating, content, is_owner)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (asset_id, user_id) DO UPDATE SET
            rating = EXCLUDED.rating,
            content = EXCLUDED.content,
            is_owner = EXCLUDED.is_owner,
            updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(asset_id)
    .bind(user_id)
    .bind(rating)
    .bind(content)
    .bind(is_owner)
    .fetch_one(community_pool)
    .await?;

    // Award Gamification Challenge
    crate::community::challenges::increment_progress(community_pool, user_id, "write_review", 1).await?;

    Ok(review)
}

/// Get a specific user's review for an asset.
pub async fn get_my_review(
    community_pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<Option<AssetReview>, AppError> {
    let review = sqlx::query_as::<_, AssetReview>(
        "SELECT * FROM asset_reviews WHERE asset_id = $1 AND user_id = $2"
    )
    .bind(asset_id)
    .bind(user_id)
    .fetch_optional(community_pool)
    .await?;

    Ok(review)
}

/// Fetch reviews for an asset with user names attached.
pub async fn list_reviews_for_asset(
    community_pool: &PgPool,
    asset_id: Uuid,
    viewer_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AssetReviewWithUser>, AppError> {
    let reviews = sqlx::query_as::<_, AssetReview>(
        "SELECT * FROM asset_reviews WHERE asset_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(asset_id)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(community_pool)
    .await?;

    if reviews.is_empty() {
        return Ok(vec![]);
    }

    // Get display names
    let user_ids: Vec<Uuid> = reviews.iter().map(|r| r.user_id).collect();
    
    // Fetch from community_profiles table directly
    #[derive(sqlx::FromRow)]
    struct ProfileRow { user_id: Uuid, display_name: String, avatar_url: Option<String> }
    
    let profiles: Vec<ProfileRow> = sqlx::query_as(
        "SELECT user_id, display_name, avatar_url FROM community_profiles WHERE user_id = ANY($1)"
    )
    .bind(&user_ids[..])
    .fetch_all(community_pool)
    .await?;

    let mut bridge_info_map = std::collections::HashMap::new();
    for p in profiles {
        bridge_info_map.insert(p.user_id, p);
    }

    let mut result = Vec::with_capacity(reviews.len());
    for review in reviews {
        let u_info = bridge_info_map.get(&review.user_id);
        let display_name = u_info.map(|i| i.display_name.clone()).unwrap_or_else(|| "Anonymous".to_string());
        let avatar_url = u_info.and_then(|i| i.avatar_url.clone());
        
        let has_upvoted = match viewer_id {
            Some(v_id) => {
                let check: Option<Uuid> = sqlx::query_scalar(
                    "SELECT user_id FROM review_upvotes WHERE review_id = $1 AND user_id = $2"
                )
                .bind(review.id)
                .bind(v_id)
                .fetch_optional(community_pool)
                .await?;
                check.is_some()
            },
            None => false,
        };

        result.push(AssetReviewWithUser {
            user_display_name: display_name,
            user_avatar_url: avatar_url,
            has_upvoted,
            review,
        });
    }

    Ok(result)
}

/// Delete a review.
pub async fn delete_review(
    community_pool: &PgPool,
    user_id: Uuid,
    review_id: Uuid,
) -> Result<(), AppError> {
    let res = sqlx::query("DELETE FROM asset_reviews WHERE id = $1 AND user_id = $2")
        .bind(review_id)
        .bind(user_id)
        .execute(community_pool)
        .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Review not found or not owned by you".into()));
    }

    Ok(())
}

// ─── Upvoting ───────────────────────────────────────────────────────

/// Toggle upvote on a review. Returns the new total helpful_count.
pub async fn toggle_review_upvote(
    community_pool: &PgPool,
    user_id: Uuid,
    review_id: Uuid,
) -> Result<(bool, i32), AppError> {
    let mut tx = community_pool.begin().await?;

    let exists: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM review_upvotes WHERE review_id = $1 AND user_id = $2"
    )
    .bind(review_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let is_now_upvoted;

    if exists.is_some() {
        // Remove upvote
        sqlx::query("DELETE FROM review_upvotes WHERE review_id = $1 AND user_id = $2")
            .bind(review_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            
        sqlx::query("UPDATE asset_reviews SET helpful_count = GREATEST(0, helpful_count - 1) WHERE id = $1")
            .bind(review_id)
            .execute(&mut *tx)
            .await?;
        is_now_upvoted = false;
    } else {
        // Add upvote
        sqlx::query("INSERT INTO review_upvotes (review_id, user_id) VALUES ($1, $2)")
            .bind(review_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            
        sqlx::query("UPDATE asset_reviews SET helpful_count = helpful_count + 1 WHERE id = $1")
            .bind(review_id)
            .execute(&mut *tx)
            .await?;
        is_now_upvoted = true;
    }

    let new_count: i32 = sqlx::query_scalar("SELECT helpful_count FROM asset_reviews WHERE id = $1")
        .bind(review_id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((is_now_upvoted, new_count))
}

// ─── Stats ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct AssetReviewStats {
    pub average_rating: f32,
    pub total_reviews: i64,
}

pub async fn get_review_stats(
    community_pool: &PgPool,
    asset_id: Uuid,
) -> Result<AssetReviewStats, AppError> {
    let (avg, count): (Option<f64>, Option<i64>) = sqlx::query_as(
        "SELECT AVG(rating), COUNT(*) FROM asset_reviews WHERE asset_id = $1"
    )
    .bind(asset_id)
    .fetch_one(community_pool)
    .await?;

    Ok(AssetReviewStats {
        average_rating: avg.unwrap_or(0.0) as f32,
        total_reviews: count.unwrap_or(0),
    })
}
