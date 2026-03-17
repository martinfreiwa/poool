/// KYC Provider Abstraction Layer
///
/// Defines a generic interface for KYC verification providers.
/// This enables swapping between Didit, Sumsub, or any future provider
/// without modifying core business logic.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// Extracted identity data returned by a KYC provider after verification.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KycExtractedData {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub nationality: Option<String>,
    pub document_type: Option<String>,
    pub document_number: Option<String>,
    pub address: Option<String>,
    pub gender: Option<String>,
}

/// The result of creating a verification session with a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycSessionResult {
    /// The provider-specific session ID (stored as `provider_ref_id`).
    pub session_id: String,
    /// The URL where the user should be redirected to complete verification.
    pub verification_url: String,
}

/// Status update received from a provider webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycStatusUpdate {
    /// The provider-specific session ID.
    pub session_id: String,
    /// Our internal user ID, extracted from `vendor_data`.
    pub user_id: Option<Uuid>,
    /// New status mapped to our internal status vocabulary.
    pub status: KycMappedStatus,
    /// Rejection reason if declined.
    pub rejection_reason: Option<String>,
    /// Extracted identity data (only populated on approval).
    pub extracted_data: Option<KycExtractedData>,
    /// Whether the user passed the PEP check (None = not yet determined).
    pub pep_check_passed: Option<bool>,
    /// Whether the user passed the sanctions check.
    pub sanctions_check_passed: Option<bool>,
}

/// Internal KYC status vocabulary.  Provider implementations must map
/// their native statuses to one of these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KycMappedStatus {
    Pending,
    InReview,
    Approved,
    Rejected,
    Expired,
}

impl KycMappedStatus {
    /// Convert to the string representation stored in `kyc_records.status`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            KycMappedStatus::Pending => "pending",
            KycMappedStatus::InReview => "in_review",
            KycMappedStatus::Approved => "approved",
            KycMappedStatus::Rejected => "rejected",
            KycMappedStatus::Expired => "expired",
        }
    }
}

/// The generic KYC provider trait.
///
/// Any KYC vendor (Didit, Sumsub, manual) must implement this trait.
/// The active provider is selected at runtime via configuration.
#[async_trait]
pub trait KycProvider: Send + Sync {
    /// Human-readable provider name (e.g. "didit", "sumsub", "manual").
    fn name(&self) -> &'static str;

    /// Create a verification session for a user.
    ///
    /// Returns a `KycSessionResult` containing the external session ID
    /// and a URL to redirect the user for identity verification.
    async fn create_session(
        &self,
        user_id: Uuid,
        user_email: Option<&str>,
        callback_url: &str,
    ) -> Result<KycSessionResult, AppError>;

    /// Process an incoming webhook payload from the provider.
    ///
    /// The implementation must:
    /// 1. Validate the webhook signature.
    /// 2. Parse the payload into a `KycStatusUpdate`.
    /// 3. Map provider-specific statuses to `KycMappedStatus`.
    async fn process_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<KycStatusUpdate, AppError>;

    /// (Optional) Retrieve the current verification status from the provider.
    ///
    /// Used for polling or manual re-check from the admin panel.
    #[allow(dead_code)]
    async fn get_session_result(&self, _session_id: &str) -> Result<KycStatusUpdate, AppError> {
        Err(AppError::Internal(
            "get_session_result not implemented for this provider".to_string(),
        ))
    }
}

/// A no-op manual provider for testing / legacy manual reviews.
pub struct ManualProvider;

#[async_trait]
impl KycProvider for ManualProvider {
    fn name(&self) -> &'static str {
        "manual"
    }

    async fn create_session(
        &self,
        _user_id: Uuid,
        _user_email: Option<&str>,
        _callback_url: &str,
    ) -> Result<KycSessionResult, AppError> {
        // Manual provider does not redirect to an external URL.
        // The frontend form submits data directly and admin reviews manually.
        Ok(KycSessionResult {
            session_id: Uuid::new_v4().to_string(),
            verification_url: String::new(), // No redirect needed
        })
    }

    async fn process_webhook(
        &self,
        _payload: &[u8],
        _signature: Option<&str>,
    ) -> Result<KycStatusUpdate, AppError> {
        Err(AppError::BadRequest(
            "Manual provider does not accept webhooks".to_string(),
        ))
    }
}
