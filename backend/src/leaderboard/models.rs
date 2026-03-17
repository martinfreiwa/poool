use serde::{Deserialize, Serialize};

/// A single entry in the leaderboard rankings table.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub tier_name: String,
    pub tier_badge_color: String,
    pub total_score: i32,
    pub is_current_user: bool,
    pub score_breakdown: ScoreBreakdown,
}

/// The logged-in user's own rank information.
#[derive(Debug, Serialize, Deserialize)]
pub struct MyRank {
    pub rank: Option<i32>,
    pub total_score: i32,
    pub delta_weekly: i32,
    pub score_breakdown: ScoreBreakdown,
}

/// Score breakdown by component.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub invest_score: i32,
    pub referral_score: i32,
    pub tier_score: i32,
    pub diversity_score: i32,
}

/// Full response for the leaderboard API.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub rankings: Vec<LeaderboardEntry>,
    pub my_rank: MyRank,
    pub total_participants: i64,
    pub timeframe: String,
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
