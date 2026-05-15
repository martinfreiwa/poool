//! Datenmodelle für das Developer-Team-Affiliate-System.
//!
//! Drei Kern-Entitäten:
//!   * `DeveloperTeam` — wirtschaftlicher Container, gehört einem Developer-User
//!   * `TeamMembership` — Mitgliedschaft eines Users in einem Team (1 User max 1 aktives Team)
//!   * `AffiliateLink` — physischer Link (personal oder team_business)
//!
//! Phase 2 von /docs/affiliate/AFFILIATE_TEAM_LINKS.md (Migrationen 156–160).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── developer_teams ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeveloperTeam {
    pub id: Uuid,
    pub developer_user_id: Uuid,
    pub display_name: String,
    pub public_slug: Option<String>,
    pub is_default: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub terminated_reason: Option<String>,
}

// ─── developer_team_memberships ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipStatus {
    Invited,
    PendingDeveloperApproval,
    Active,
    Removed,
}

impl MembershipStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::PendingDeveloperApproval => "pending_developer_approval",
            Self::Active => "active",
            Self::Removed => "removed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMembership {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub status: String,
    pub invitation_token_hash: Option<String>,
    pub invitation_expires_at: Option<DateTime<Utc>>,
    pub invited_by_user_id: Option<Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub removed_at: Option<DateTime<Utc>>,
    pub removed_reason: Option<String>,
    pub removed_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─── affiliate_links ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Personal,
    TeamBusiness,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::TeamBusiness => "team_business",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AffiliateLink {
    pub id: Uuid,
    pub code: String,
    pub link_type: String,
    pub attribution_user_id: Uuid,
    pub payout_user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub deactivated_reason: Option<String>,
}

impl AffiliateLink {
    pub fn is_personal(&self) -> bool {
        self.link_type == LinkType::Personal.as_str()
    }
    pub fn is_team_business(&self) -> bool {
        self.link_type == LinkType::TeamBusiness.as_str()
    }
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
}

// ─── Reporting-Aggregate ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AffiliateLiveCounter {
    pub payout_user_id: Uuid,
    pub lifetime_revenue_cents: i64,
    pub lifetime_commission_cents: i64,
    pub pending_commission_cents: i64,
    pub payable_commission_cents: i64,
    pub paid_commission_cents: i64,
    pub clawed_back_cents: i64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AffiliateDailyRollup {
    pub rollup_date: chrono::NaiveDate,
    pub link_id: Uuid,
    pub payout_user_id: Uuid,
    pub attribution_user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub link_type: String,
    pub clicks_count: i32,
    pub signups_count: i32,
    pub qualified_count: i32,
    pub gross_revenue_cents: i64,
    pub commission_cents: i64,
    pub updated_at: DateTime<Utc>,
}
