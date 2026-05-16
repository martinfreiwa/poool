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
    /// Asset-type breakdown for the bento donut (top-N only). Empty when
    /// the row is not in the top-N or when the user has no investments.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub asset_mix: Vec<AssetMixSlice>,
}

/// The logged-in user's own rank information.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MyRank {
    pub rank: Option<i32>,
    pub metric_value: i64,
    pub metrics: LeaderboardMetrics,
}

/// Raw metrics for the user — all 6 core leaderboard dimensions.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LeaderboardMetrics {
    pub total_invested_cents: i64,
    pub asset_count: i32,
    pub portfolio_roi_bps: i32,
    pub affiliate_count: i32,
    pub referral_network_value_cents: i64,
    pub highest_investment_cents: i64,
}

/// One historical snapshot point for a single user + metric. Returned by
/// `GET /api/leaderboard/snapshots/me` so the UI can plot a rank/value
/// sparkline ("you're up 3 ranks this week").
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSnapshotPoint {
    pub snapshot_date: chrono::NaiveDate,
    pub rank: i32,
    pub metric_value: i64,
}

/// Per-user investment breakdown by `assets.asset_type` (real_estate,
/// commercial_property, commodity, business, startup, land_plot).
///
/// Optional on the entry — only populated for entries that appear in the
/// top-N bento cards where a donut visualization is worth the extra query.
/// The table-row rendering ignores this field.
#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct AssetMixSlice {
    pub asset_type: String,
    pub invested_cents: i64,
    pub asset_count: i32,
}

/// Full response for the leaderboard API.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub rankings: Vec<LeaderboardEntry>,
    pub my_rank: MyRank,
    pub total_participants: i64,
    pub metric_type: String,
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
