/// Didit.me KYC Provider Implementation
///
/// Integrates with the Didit V3 API for identity verification.
/// API Docs: https://docs.didit.me
///
/// Flow:
/// 1. Backend creates a session via POST https://verification.didit.me/v3/session/
/// 2. User is redirected to the returned `url` to complete verification.
/// 3. Didit sends a webhook to our endpoint when the status changes.
/// 4. We process the webhook and update `kyc_records` accordingly.
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use super::provider::{
    KycExtractedData, KycMappedStatus, KycProvider, KycSessionResult, KycStatusUpdate,
};
use crate::error::AppError;

type HmacSha256 = Hmac<Sha256>;

/// Configuration for the Didit provider, loaded from environment variables.
#[derive(Debug, Clone)]
pub struct DiditConfig {
    /// API key for authenticating with Didit (header: x-api-key).
    pub api_key: String,
    /// The workflow ID configured in the Didit Business Console.
    pub workflow_id: String,
    /// Webhook secret key for verifying HMAC signatures.
    pub webhook_secret: String,
}

impl DiditConfig {
    /// Load Didit config from environment variables.
    /// Returns None if the required DIDIT_API_KEY is not set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("DIDIT_API_KEY")
            .ok()
            .filter(|v| !v.is_empty())?;
        let workflow_id = std::env::var("DIDIT_WORKFLOW_ID").ok().unwrap_or_default();
        // Webhook secret is mandatory when provider is enabled — empty means
        // webhooks would be unauthenticated. Refuse to initialize provider.
        let webhook_secret = std::env::var("DIDIT_WEBHOOK_SECRET")
            .ok()
            .filter(|v| !v.is_empty())?;

        Some(Self {
            api_key,
            workflow_id,
            webhook_secret,
        })
    }
}

/// The Didit KYC provider.
pub struct DiditProvider {
    config: DiditConfig,
    http_client: reqwest::Client,
}

impl DiditProvider {
    /// Create a new DiditProvider with the given configuration.
    pub fn new(config: DiditConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl KycProvider for DiditProvider {
    fn name(&self) -> &'static str {
        "didit"
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        user_email: Option<&str>,
        callback_url: &str,
    ) -> Result<KycSessionResult, AppError> {
        let mut body = serde_json::json!({
            "workflow_id": self.config.workflow_id,
            "vendor_data": user_id.to_string(),
            "callback": callback_url,
            "callback_method": "both",
        });

        // Attach contact details if email is available
        if let Some(email) = user_email {
            body["contact_details"] = serde_json::json!({
                "email": email,
                "send_notification_emails": false,
            });
        }

        let resp = self
            .http_client
            .post("https://verification.didit.me/v3/session/")
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Didit API request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            tracing::error!("Didit create_session failed ({}): {}", status, err_body);
            return Err(AppError::Internal(format!(
                "Didit returned HTTP {}: {}",
                status, err_body
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse Didit response: {e}")))?;

        let session_id = data["session_id"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Didit response missing session_id".to_string()))?
            .to_string();

        let verification_url = data["url"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Didit response missing url".to_string()))?
            .to_string();

        tracing::info!("Created Didit session {} for user {}", session_id, user_id);

        Ok(KycSessionResult {
            session_id,
            verification_url,
        })
    }

    async fn process_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<KycStatusUpdate, AppError> {
        // --- Signature Verification ---
        // Didit provides three signature types:
        // 1. X-Signature-V2 (Recommended): Signs unescaped Unicode JSON (José vs Jos\u00e9).
        // 2. X-Signature-Simple (Fallback): Signs core fields "{timestamp}:{session_id}:{status}:{webhook_type}".
        // 3. X-Signature: Signs raw JSON bytes.
        //
        // Our implementation tries V2 first if possible, then falls back to Simple.
        // Secret is mandatory (enforced in DiditConfig::from_env). Always verify.
        let sig = match signature {
            Some(s) => s,
            None => {
                tracing::warn!("Didit webhook received without signature header");
                return Err(AppError::Unauthorized(
                    "Missing webhook signature".to_string(),
                ));
            }
        };

        let sig_bytes = hex::decode(sig).map_err(|_| {
            AppError::Unauthorized("Invalid webhook signature encoding".to_string())
        })?;

        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| AppError::BadRequest(format!("Invalid webhook JSON: {e}")))?;

        let mut verified = false;

        // 1. Try X-Signature-V2 — constant-time compare via verify_slice
        if let Ok(v2_str) = self.serialize_v2(&body) {
            let mut mac = HmacSha256::new_from_slice(self.config.webhook_secret.as_bytes())
                .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
            mac.update(v2_str.as_bytes());
            if mac.verify_slice(&sig_bytes).is_ok() {
                verified = true;
                tracing::debug!("Didit webhook verified via X-Signature-V2");
            }
        }

        // 2. Fall back to X-Signature-Simple (signs core fields only)
        if !verified {
            let timestamp = body["timestamp"].as_i64().unwrap_or(0);
            let session_id = body["session_id"].as_str().unwrap_or("");
            let status = body["status"].as_str().unwrap_or("");
            let webhook_type = body["webhook_type"].as_str().unwrap_or("");

            let sign_payload = format!("{}:{}:{}:{}", timestamp, session_id, status, webhook_type);

            let mut mac = HmacSha256::new_from_slice(self.config.webhook_secret.as_bytes())
                .map_err(|e| AppError::Internal(format!("HMAC init failed: {e}")))?;
            mac.update(sign_payload.as_bytes());
            if mac.verify_slice(&sig_bytes).is_ok() {
                verified = true;
                tracing::debug!("Didit webhook verified via X-Signature-Simple");
            }
        }

        if !verified {
            tracing::warn!("Didit webhook signature mismatch");
            return Err(AppError::Unauthorized(
                "Invalid webhook signature".to_string(),
            ));
        }

        // --- Timestamp Freshness Check ---
        let timestamp = body["timestamp"].as_i64().unwrap_or(0);
        let now = chrono::Utc::now().timestamp();
        if (now - timestamp).abs() > 300 {
            tracing::warn!(
                "Didit webhook timestamp too old: {} (now={})",
                timestamp,
                now
            );
            return Err(AppError::Unauthorized(
                "Webhook timestamp expired".to_string(),
            ));
        }

        // --- Parse the webhook body ---
        self.parse_didit_body(payload)
    }

    async fn get_session_result(&self, session_id: &str) -> Result<KycStatusUpdate, AppError> {
        let url = format!(
            "https://verification.didit.me/v3/session/{}/decision/",
            session_id
        );

        let resp = self
            .http_client
            .get(&url)
            .header("x-api-key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Didit retrieve session failed: {e}")))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("Didit retrieve failed: {err}")));
        }

        let data = resp
            .bytes()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read Didit response: {e}")))?;

        // Parse the response body directly (no signature check needed for direct API calls)
        self.parse_didit_body(&data)
    }
}

impl DiditProvider {
    /// Parse a Didit webhook/decision JSON body into a KycStatusUpdate.
    /// Shared by both the webhook handler and direct API polling.
    fn parse_didit_body(&self, payload: &[u8]) -> Result<KycStatusUpdate, AppError> {
        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| AppError::BadRequest(format!("Invalid webhook JSON: {e}")))?;

        let session_id = body["session_id"].as_str().unwrap_or("").to_string();
        let didit_status = body["status"].as_str().unwrap_or("");

        // Map Didit statuses to our internal vocabulary
        let mapped_status = match didit_status {
            "Not Started" | "In Progress" => KycMappedStatus::Pending,
            "In Review" => KycMappedStatus::InReview,
            "Approved" => KycMappedStatus::Approved,
            "Declined" => KycMappedStatus::Rejected,
            "Abandoned" => KycMappedStatus::Expired,
            other => {
                tracing::warn!("Unknown Didit status: '{}'", other);
                KycMappedStatus::Pending
            }
        };

        // Extract the user_id from vendor_data
        let user_id = body["vendor_data"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok());

        // Extract identity data from the decision object (if present)
        let mut extracted_data = None;
        let mut pep_check = None;
        let mut sanctions_check = None;
        let mut rejection_reason = None;

        if let Some(decision) = body.get("decision") {
            // Extract from first id_verification record
            if let Some(id_verifications) =
                decision.get("id_verifications").and_then(|v| v.as_array())
            {
                if let Some(first) = id_verifications.first() {
                    extracted_data = Some(KycExtractedData {
                        first_name: first["first_name"].as_str().map(String::from),
                        last_name: first["last_name"].as_str().map(String::from),
                        date_of_birth: first["date_of_birth"].as_str().map(String::from),
                        nationality: first["nationality"].as_str().map(String::from),
                        document_type: first["document_type"].as_str().map(String::from),
                        document_number: first["document_number"].as_str().map(String::from),
                        address: first["formatted_address"].as_str().map(String::from),
                        gender: first["gender"].as_str().map(String::from),
                    });
                }
            }

            // Extract AML screening results
            if let Some(aml) = decision.get("aml_screenings").and_then(|v| v.as_array()) {
                if let Some(first) = aml.first() {
                    let aml_status = first["status"].as_str().unwrap_or("");
                    // If AML status is "Approved" = clear, "In Review" = unclear, "Declined" = hit
                    sanctions_check = Some(aml_status == "Approved");

                    // Check if any PEP hits exist
                    if let Some(hits) = first.get("hits").and_then(|h| h.as_array()) {
                        let has_pep = hits.iter().any(|hit| {
                            hit.get("datasets")
                                .and_then(|d| d.as_array())
                                .map(|ds| ds.iter().any(|d| d.as_str() == Some("PEP")))
                                .unwrap_or(false)
                        });
                        pep_check = Some(!has_pep);
                    } else {
                        pep_check = Some(true); // No AML hits = PEP clear
                    }
                }
            }

            // Extract rejection reason from reviews
            if mapped_status == KycMappedStatus::Rejected {
                if let Some(reviews) = decision.get("reviews").and_then(|v| v.as_array()) {
                    rejection_reason = reviews
                        .last()
                        .and_then(|r| r["comment"].as_str())
                        .map(String::from);
                }
                if rejection_reason.is_none() {
                    rejection_reason = Some("Verification declined by provider".to_string());
                }
            }
        }

        Ok(KycStatusUpdate {
            session_id,
            user_id,
            status: mapped_status,
            rejection_reason,
            extracted_data,
            pep_check_passed: pep_check,
            sanctions_check_passed: sanctions_check,
        })
    }
}

impl DiditProvider {
    /// Serialize JSON for V2 signature verification.
    /// Matches Didit's server-side logic: sort keys, no spaces after separators, no Unicode escaping.
    fn serialize_v2(&self, value: &serde_json::Value) -> Result<String, AppError> {
        // serde_json::to_string typically matches what's needed for the V2 signature
        // when passed through a canonicalization step.
        let mut processed = value.clone();
        self.shorten_floats(&mut processed);

        // We need to ensure keys are sorted. serde_json::Value and its default serializer
        // do not guarantee sorting if it's already an Object.
        // However, serde_json::to_string on a Map DOES sort keys.
        let serialized = serde_json::to_string(&processed)
            .map_err(|e| AppError::Internal(format!("V2 serialization failed: {e}")))?;

        Ok(serialized)
    }

    /// Recursively ensure floats match server-side behavior (e.g. 1.0 -> 1).
    fn shorten_floats(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for v in map.values_mut() {
                    self.shorten_floats(v);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.shorten_floats(v);
                }
            }
            serde_json::Value::Number(num) => {
                if let Some(f) = num.as_f64() {
                    if f.fract() == 0.0 {
                        *value = serde_json::Value::Number(serde_json::Number::from(f as i64));
                    }
                }
            }
            _ => {}
        }
    }
}
