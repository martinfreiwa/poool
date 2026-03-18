use serde::{Deserialize, Serialize};

/// A single entry in the leaderboard rankings table.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub tier_name: String,
    pub tier_badge_color: String,
    pub metric_value: i64,
    pub is_current_user: bool,
    pub metrics: LeaderboardMetrics,
}

/// The logged-in user's own rank information.
#[derive(Debug, Serialize, Deserialize)]
pub struct MyRank {
    pub rank: Option<i32>,
    pub metric_value: i64,
    pub delta_weekly: i32,
    pub metrics: LeaderboardMetrics,
}

/// Raw metrics for the user.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardMetrics {
    pub total_invested_cents: i64,
    pub asset_count: i32,
    pub portfolio_roi_bps: i32,
    pub affiliate_count: i32,
    pub referral_revenue_cents: i64,
    pub highest_investment_cents: i64,
}

/// Full response for the leaderboard API.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub rankings: Vec<LeaderboardEntry>,
    pub my_rank: MyRank,
    pub total_participants: i64,
    pub metric_type: String,
    pub last_updated: Option<String>,
    pub has_more: bool,
}

/// User preferences for leaderboard visibility.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardPreferences {
    pub visible: bool,
    pub show_avatar: bool,
    pub display_name: Option<String>,
}

/// Request body for updating preferences.
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub visible: Option<bool>,
    pub show_avatar: Option<bool>,
    pub display_name: Option<String>,
}
