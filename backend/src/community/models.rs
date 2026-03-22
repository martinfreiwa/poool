use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnouncementCategory {
    pub post_id: Uuid,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub post_type: String,
    pub content: String,
    pub content_sanitized: Option<String>,
    pub asset_id: Option<Uuid>,
    pub image_urls: Option<Vec<String>>,
    pub is_pinned: bool,
    pub is_hidden: bool,
    pub hidden_reason: Option<String>,
    pub disclaimer_shown: bool,
    pub reaction_count: i32,
    pub comment_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub content_sanitized: Option<String>,
    pub helpful_count: i32,
    pub is_hidden: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reaction {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub reaction_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommunityProfile {
    pub user_id: Uuid,
    pub bio: Option<String>,
    pub is_community_banned: bool,
    pub ban_reason: Option<String>,
    pub ban_expires_at: Option<DateTime<Utc>>,
    pub warning_count: i32,
    pub post_count: i32,
    pub follower_count: i32,
    pub following_count: i32,
    pub login_streak: i32,
    pub last_login_date: Option<chrono::NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A fully joined announcement for display
#[derive(Debug, Serialize)]
pub struct AnnouncementDisplay {
    pub id: Uuid,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub category: String,
    pub content: String,
    pub image_urls: Vec<String>,
    pub reaction_count: i32,
    pub comment_count: i32,
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
}
