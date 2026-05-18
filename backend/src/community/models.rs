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
    pub is_locked: Option<bool>,
    pub content_tags: Option<Vec<String>>,
    pub link_preview: Option<serde_json::Value>,
    pub reaction_count: i32,
    pub comment_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // UX.16 — quote-repost target (NULL for original posts).
    #[sqlx(default)]
    pub quoted_post_id: Option<Uuid>,
    // CO.7 — when set, the post is hidden from feeds until this timestamp.
    #[sqlx(default)]
    pub scheduled_for: Option<DateTime<Utc>>,
}

/// Brief render of a quoted post — excerpted content + author preview.
/// Always one level deep so quote chains can't recursively bloat payloads.
#[derive(Debug, Serialize, Clone)]
pub struct QuotedPostBrief {
    pub id: Uuid,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub content: String,
    pub created_at_display: String,
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
    pub is_pinned: Option<bool>,
    pub created_at: DateTime<Utc>,
    // 14.8.5 — populated after migration 028. NULL on comments that have
    // never been edited; non-NULL drives the "Edited" indicator on the
    // comment row.
    pub edited_at: Option<DateTime<Utc>>,
    // 14.8.6 — denormalized counter maintained by trigger on the new
    // comment_reactions table (migration 029).
    pub reaction_count: i32,
    // 14.8.12 — nullable self-FK; NULL for top-level comments.
    pub parent_comment_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reaction {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub reaction_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
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
    // Column on `community_profiles` is `xp_total` (added in 008_circles_xp.sql),
    // not `total_xp`; the previous mismatch crashed every `SELECT *` decode.
    #[sqlx(rename = "xp_total")]
    pub xp_total: i32,
    pub muted_until: Option<DateTime<Utc>>,
    pub mod_notes: Option<String>,
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
    pub created_at_display: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreatePostRequest {
    pub post_type: String, // 'general' or 'market_insight'
    pub content: String,
    pub asset_id: Option<Uuid>,
    pub image_urls: Option<Vec<String>>,
    // UX.11: Poll support — optional
    pub poll_question: Option<String>,
    pub poll_options: Option<Vec<String>>,
    pub poll_expires_hours: Option<i32>,
    // UX.16: Quote-repost — references the post being shared. The
    // backend stores the id; the feed read joins it back to a brief
    // "quoted card" payload so threads stay one level deep.
    pub quoted_post_id: Option<Uuid>,
    // CO.7: optional ISO8601 future timestamp. Post is created
    // immediately but hidden from feeds until this time. Past values
    // are rejected; NULL = publish now.
    pub scheduled_for: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PostDisplay {
    pub id: Uuid,
    pub author_name: String,
    pub author_initials: String,
    pub author_id: Uuid,
    pub author_avatar: Option<String>,
    pub author_badges: Vec<String>,
    pub post_type: String,
    pub content: String,
    pub rendered_content: String,
    pub asset_id: Option<Uuid>,
    pub image_urls: Vec<String>,
    pub link_preview: Option<serde_json::Value>,
    pub link_preview_domain: Option<String>,
    pub reaction_count: i32,
    pub comment_count: i32,
    pub current_user_reacted: bool,
    pub is_bookmarked: bool,
    pub is_hidden: bool,
    pub is_pinned: bool,
    pub disclaimer_shown: bool,
    pub verified_owner: bool,
    pub created_at: DateTime<Utc>,
    pub created_at_display: String,
    /// UX.20: estimated reading time (minutes) for long posts. `None` for
    /// posts under one minute — the partial only renders the badge when set.
    pub read_time_minutes: Option<i32>,
    /// M6-FEAT.3: rich-media embed kind extracted from `link_preview.url`.
    /// `Some("youtube"|"loom")` triggers the player-card render path.
    pub embed_kind: Option<String>,
    /// M6-FEAT.3: provider-specific id (e.g. YouTube video id) used to build
    /// thumbnail URLs without a second roundtrip.
    pub embed_id: Option<String>,
    /// UX.16: when this post quotes another, the brief render lives here.
    pub quoted: Option<QuotedPostBrief>,
    /// UX.14: optional author flair (short label + emoji). Hydrated via
    /// `get_flairs_batch` at the route layer; partial renders nothing
    /// when None.
    pub author_flair: Option<String>,
    /// UX.17: true when the author is currently in the top-N by XP.
    /// Recomputed on every feed read (cheap; one indexed query).
    pub author_top_contributor: bool,
    /// W3.4: portfolio-value tier (Bronze/Silver/Gold/Platinum) derived
    /// from the author's total invested capital. None when they have no
    /// holdings yet.
    pub author_tier: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContentReport {
    pub id: Uuid,
    pub post_id: Uuid,
    pub reporter_id: Uuid,
    pub reason: String,
    pub status: String,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateContentReportRequest {
    pub reason: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminReportDisplay {
    pub id: Uuid,
    pub post_id: Uuid,
    pub reporter_id: Uuid,
    pub reporter_name: String,
    pub post_author_id: Uuid,
    pub post_author_name: String,
    pub post_content: String,
    pub reason: String,
    pub status: String,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AdminReportActionRequest {
    pub action: String, // 'hide_post', 'dismiss_report'
    pub admin_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBanAppealReq {
    pub appeal_text: String,
}

#[derive(Debug, Serialize)]
pub struct BanAppealDisplay {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub appeal_text: String,
    pub status: String,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AdminReviewAppealReq {
    pub action: String, // 'approve', 'reject'
    pub admin_notes: Option<String>,
}
