use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct KycStatusResponse {
    pub status: String,
    pub rejection_reason: Option<String>,
    /// The KYC provider that processed this record.
    pub provider: Option<String>,
    /// If the provider supports redirect-based verification,
    /// this URL is returned for the frontend to redirect the user.
    pub verification_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KycSubmitRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub date_of_birth: Option<String>,
    pub nationality: Option<String>,
    pub address_line1: Option<String>,
    pub address_city: Option<String>,
    pub address_country: Option<String>,
    // Real implementation would have multipart for documents
    // and would interact with a provider like Didit or Sumsub.
    pub document_type: Option<String>,
    pub document_id: Option<uuid::Uuid>,
    pub frontend_completed: Option<bool>,
}

/// Request to initiate a KYC session with the active provider.
/// The backend creates a session with the provider and returns a redirect URL.
#[derive(Serialize, Deserialize, Debug)]
pub struct KycInitiateRequest {
    /// Optional document type hint (passport, national_id, driving_license).
    pub document_type: Option<String>,
}

/// Response returned when initiating a KYC session.
#[derive(Serialize, Deserialize, Debug)]
pub struct KycInitiateResponse {
    pub success: bool,
    /// The kyc_records row ID.
    pub kyc_id: String,
    /// The KYC provider name.
    pub provider: String,
    /// URL to redirect user to for identity verification.
    /// Empty if using manual review flow.
    pub verification_url: String,
    /// Human-readable message.
    pub message: String,
}
