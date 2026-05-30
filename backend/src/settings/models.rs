/// Settings data models – request/response structs for the Settings page API.
use serde::{Deserialize, Serialize};

// ─── Response: Full settings data ─────────────────────────────

/// Complete settings response returned by GET /api/settings.
/// Joins data from users, user_profiles, user_settings, and roles.
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub email: String,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub gender: Option<String>,
    pub phone_number: Option<String>,
    pub country: Option<String>,
    pub timezone: String,
    pub role: String,
    pub is_developer: bool,
    pub is_admin: bool,
    pub language: String,
    pub currency: String,
    pub annual_income_cents: Option<i64>,
    pub avatar_url: Option<String>,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub date_of_birth: Option<String>,
    pub nationality: Option<String>,
    pub tax_id: Option<String>,
    // Account status
    pub email_verified: bool,
    pub kyc_status: Option<String>, // pending | in_review | approved | rejected | null
    // Notification preferences
    pub email_notifications: bool,
    pub push_notifications: bool,
    // Security details
    pub totp_enabled: bool,
    pub passkey_count: i64,
    // Extensions for full SaaS profile
    pub referral_code: Option<String>,
    pub tier_name: Option<String>,
    pub investment_limit_cents: Option<i64>,
    pub invested_12m_cents: Option<i64>,
    pub limit_available_cents: Option<i64>,
    pub active_sessions: Vec<UserSessionInfo>,
    pub oauth_accounts: Vec<OauthAccountInfo>,
    pub latest_terms_version: Option<String>,
    pub latest_terms_accepted_at: Option<String>,
    // Leaderboard
    pub lb_visible: bool,
    pub lb_avatar: bool,
    pub lb_display_name: Option<String>,
    pub lb_bio: Option<String>,
    pub social_links: SocialLinks,
    pub developer_profile: Option<DeveloperProfileSettings>,
}

#[derive(Debug, Serialize)]
pub struct UserSessionInfo {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct OauthAccountInfo {
    pub id: String,
    pub provider: String,
    pub email: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SocialLinks {
    pub twitter: Option<String>,
    pub linkedin: Option<String>,
    pub instagram: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct DeveloperLinks {
    pub website: Option<String>,
    pub github: Option<String>,
    pub twitter: Option<String>,
    pub linkedin: Option<String>,
    pub youtube: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct DeveloperProfileSettings {
    pub company_name: Option<String>,
    pub logo_url: Option<String>,
    pub description: Option<String>,
    pub links: DeveloperLinks,
}

/// Standard API response for all settings mutation endpoints.
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

impl ApiResponse {
    pub fn ok(message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
        }
    }

    pub fn err(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
        }
    }
}

// ─── Request: My Details tab ──────────────────────────────────

/// Form data from the "My Details" tab Save button.
#[derive(Debug, Deserialize)]
pub struct UpdateProfileForm {
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub gender: Option<String>,
    pub phone_number: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<String>,
    pub annual_income_cents: Option<i64>,
    // Extended fields
    pub date_of_birth: Option<String>, // YYYY-MM-DD
    pub nationality: Option<String>,
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub tax_id: Option<String>,
    #[allow(dead_code)]
    pub profile_photo: Option<String>,
}

// ─── Request: Notifications tab ───────────────────────────────

/// Form data from the "Notifications" tab Save button.
#[derive(Debug, Deserialize)]
pub struct UpdateNotificationsForm {
    pub email_notifications: bool,
    pub push_notifications: bool,
}

// ─── Request: Leaderboard tab ─────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateLeaderboardForm {
    pub visible: bool,
    pub show_avatar: bool,
    pub display_name: Option<String>,
    pub bio: Option<String>,
}

// ─── Request: Preferences tab ─────────────────────────────────

/// Form data from the "Preferences" tab Save button.
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesForm {
    pub language: String,
    pub currency: String,
    pub timezone: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSocialLinksForm {
    pub twitter: Option<String>,
    pub linkedin: Option<String>,
    pub instagram: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeveloperProfileForm {
    pub company_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeveloperLinksForm {
    pub website: Option<String>,
    pub github: Option<String>,
    pub twitter: Option<String>,
    pub linkedin: Option<String>,
    pub youtube: Option<String>,
}

// ─── Request: Security tab ────────────────────────────────────

/// Form data for changing the user's email address.
/// Requires current password for verification.
#[derive(Debug, Deserialize)]
pub struct ChangeEmailForm {
    pub new_email: String,
    pub current_password: String,
}

/// Form data for changing the user's password.
/// Requires current password for verification.
#[derive(Debug, Deserialize)]
pub struct ChangePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

/// Form data for changing the user's phone number.
#[derive(Debug, Deserialize)]
pub struct ChangePhoneForm {
    pub new_phone: String,
}

// ─── Request: GDPR Account Deletion ───────────────────────────

/// Form data for GDPR selective account deletion (Art. 17).
/// Requires current password for verification.
#[derive(Debug, Deserialize)]
pub struct DeleteAccountForm {
    pub current_password: String,
    pub confirm: String,
}

/// Form data for disabling TOTP.
/// Requires current password and a fresh authenticator code.
#[derive(Debug, Deserialize)]
pub struct DisableTotpForm {
    pub current_password: String,
    pub code: String,
}
