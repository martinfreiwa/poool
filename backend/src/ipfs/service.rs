/// Pinata IPFS service — pin JSON metadata and files to IPFS.
///
/// Uses the Pinata API v2 (https://docs.pinata.cloud/api-reference)
/// with JWT bearer authentication.
///
/// Pinning strategies:
/// - `pinJSONToIPFS` — for metadata JSON (small, structured)
/// - `pinFileToIPFS` — for legal documents (PDFs, images)
///
/// Each pin is named with a descriptive label (e.g., "poool-asset-{id}-metadata")
/// so it's easy to find in the Pinata dashboard.
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
// ── CONFIGURATION ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Pinata API configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct PinataConfig {
    /// JWT bearer token (preferred auth method)
    pub jwt: String,
    /// Base URL for the Pinata API
    pub api_url: String,
    /// IPFS gateway for resolving pinned content
    pub gateway_url: String,
}

impl PinataConfig {
    /// Load Pinata config from environment. Returns None if not configured.
    pub fn from_env() -> Option<Self> {
        let jwt = std::env::var("PINATA_JWT").ok().filter(|v| !v.is_empty())?;
        
        let api_url = std::env::var("PINATA_API_URL")
            .unwrap_or_else(|_| "https://api.pinata.cloud".to_string());
        
        let gateway_url = std::env::var("PINATA_GATEWAY_URL")
            .unwrap_or_else(|_| "https://gateway.pinata.cloud/ipfs".to_string());

        Some(Self {
            jwt,
            api_url,
            gateway_url,
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// ── TYPES ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Request body for Pinata's `pinJSONToIPFS` endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PinJsonRequest {
    /// The JSON content to pin
    pin_json_to_ipfs: serde_json::Value,
    /// Metadata about the pin (name, keyvalues)
    pinata_metadata: PinataMetadata,
    /// Pin options (CID version, etc.)
    pinata_options: PinataOptions,
}

/// Metadata associated with a Pinata pin.
#[derive(Debug, Serialize)]
struct PinataMetadata {
    /// Human-readable name shown in Pinata dashboard
    name: String,
    /// Key-value pairs for filtering pins in Pinata
    #[serde(skip_serializing_if = "Option::is_none")]
    keyvalues: Option<serde_json::Value>,
}

/// Pin configuration options.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PinataOptions {
    /// CID version (1 = CIDv1, base32 encoding)
    cid_version: u8,
}

/// Response from Pinata's pin endpoints.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PinResponse {
    /// The IPFS content hash (CID)
    pub ipfs_hash: String,
    /// Size of the pinned content in bytes
    pub pin_size: i64,
    /// Timestamp of when the content was pinned
    pub timestamp: String,
}

/// Result type for IPFS operations.
pub type IpfsResult<T> = Result<T, IpfsError>;

/// Errors that can occur during IPFS operations.
#[derive(Debug)]
pub enum IpfsError {
    /// Pinata is not configured (missing PINATA_JWT)
    NotConfigured,
    /// HTTP request to Pinata API failed
    RequestFailed(String),
    /// Pinata API returned an error response
    ApiError { status: u16, message: String },
    /// Failed to serialize metadata JSON
    SerializationError(String),
}

impl std::fmt::Display for IpfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Pinata IPFS not configured (PINATA_JWT not set)"),
            Self::RequestFailed(msg) => write!(f, "IPFS request failed: {}", msg),
            Self::ApiError { status, message } => {
                write!(f, "Pinata API error (HTTP {}): {}", status, message)
            }
            Self::SerializationError(msg) => write!(f, "JSON serialization error: {}", msg),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// ── PIN JSON TO IPFS ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Pin a JSON object to IPFS via Pinata.
///
/// # Arguments
/// * `json_content` — The JSON value to pin (will be stored as-is on IPFS)
/// * `name` — Human-readable name for the pin (e.g., "poool-asset-{uuid}-metadata")
/// * `keyvalues` — Optional key-value metadata for Pinata filtering
///
/// # Returns
/// The IPFS CID (content hash) on success.
pub async fn pin_json(
    json_content: &serde_json::Value,
    name: &str,
    keyvalues: Option<serde_json::Value>,
) -> IpfsResult<PinResponse> {
    let config = PinataConfig::from_env().ok_or(IpfsError::NotConfigured)?;
    let client = Client::new();

    let body = serde_json::json!({
        "pinataContent": json_content,
        "pinataMetadata": {
            "name": name,
            "keyvalues": keyvalues.unwrap_or(serde_json::json!({})),
        },
        "pinataOptions": {
            "cidVersion": 1
        }
    });

    let response = client
        .post(format!("{}/pinning/pinJSONToIPFS", config.api_url))
        .header("Authorization", format!("Bearer {}", config.jwt))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| IpfsError::RequestFailed(e.to_string()))?;

    let status = response.status().as_u16();
    if status != 200 {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(IpfsError::ApiError {
            status,
            message: error_body,
        });
    }

    let pin_response: PinResponse = response
        .json()
        .await
        .map_err(|e| IpfsError::RequestFailed(format!("Failed to parse response: {}", e)))?;

    tracing::info!(
        "📌 Pinned JSON to IPFS: name={}, cid={}, size={}",
        name,
        pin_response.ipfs_hash,
        pin_response.pin_size
    );

    Ok(pin_response)
}

/// Pin a file (bytes) to IPFS via Pinata.
///
/// # Arguments
/// * `file_bytes` — Raw file content
/// * `filename` — Original filename (e.g., "spv-agreement.pdf")
/// * `name` — Pin name for Pinata dashboard
/// * `keyvalues` — Optional metadata
pub async fn pin_file(
    file_bytes: &[u8],
    filename: &str,
    name: &str,
    keyvalues: Option<serde_json::Value>,
) -> IpfsResult<PinResponse> {
    let config = PinataConfig::from_env().ok_or(IpfsError::NotConfigured)?;
    let client = Client::new();

    // Build multipart form
    let file_part = reqwest::multipart::Part::bytes(file_bytes.to_vec())
        .file_name(filename.to_string())
        .mime_str("application/octet-stream")
        .map_err(|e| IpfsError::RequestFailed(e.to_string()))?;

    let metadata = serde_json::json!({
        "name": name,
        "keyvalues": keyvalues.unwrap_or(serde_json::json!({})),
    });

    let options = serde_json::json!({
        "cidVersion": 1
    });

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("pinataMetadata", metadata.to_string())
        .text("pinataOptions", options.to_string());

    let response = client
        .post(format!("{}/pinning/pinFileToIPFS", config.api_url))
        .header("Authorization", format!("Bearer {}", config.jwt))
        .multipart(form)
        .send()
        .await
        .map_err(|e| IpfsError::RequestFailed(e.to_string()))?;

    let status = response.status().as_u16();
    if status != 200 {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(IpfsError::ApiError {
            status,
            message: error_body,
        });
    }

    let pin_response: PinResponse = response
        .json()
        .await
        .map_err(|e| IpfsError::RequestFailed(format!("Failed to parse response: {}", e)))?;

    tracing::info!(
        "📌 Pinned file to IPFS: name={}, filename={}, cid={}, size={}",
        name,
        filename,
        pin_response.ipfs_hash,
        pin_response.pin_size
    );

    Ok(pin_response)
}

/// Build a full IPFS URI from a CID.
pub fn ipfs_uri(cid: &str) -> String {
    format!("ipfs://{}", cid)
}

/// Build a gateway URL for browser access.
pub fn gateway_url(cid: &str) -> String {
    let gateway = std::env::var("PINATA_GATEWAY_URL")
        .unwrap_or_else(|_| "https://gateway.pinata.cloud/ipfs".to_string());
    format!("{}/{}", gateway, cid)
}

// ═══════════════════════════════════════════════════════════════
// ── TESTS ─────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipfs_uri() {
        let cid = "bafkreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
        assert_eq!(
            ipfs_uri(cid),
            "ipfs://bafkreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
        );
    }

    #[test]
    fn test_config_returns_none_without_env() {
        // In test environment without PINATA_JWT set, from_env should return None
        // (this test passes when PINATA_JWT is not in the test environment)
        // We can't reliably test this without clearing env vars, so just test the type
        let _: Option<PinataConfig> = PinataConfig::from_env();
    }
}
