/// GCS service – thin async wrapper over `google-cloud-storage`.
///
/// Uploads binary data to a given GCS object path and returns the public
/// HTTPS URL (for avatars / property images) or a signed URL
/// (for KYC documents, valid 15 minutes).
///
/// Authentication is handled entirely by Application Default Credentials
/// (ADC).  On Cloud Run the service account attached to the revision is
/// used automatically.  Locally run:
///     gcloud auth application-default login
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::sign::SignedURLOptions;

use crate::error::AppError;

// ─── Upload ────────────────────────────────────────────────────

/// Upload raw bytes to GCS under `object_path` and return its public HTTPS URL.
///
/// The returned URL is suitable for storing in the DB and serving directly
/// from Cloud CDN for non-sensitive content (avatars, property images).
///
/// For KYC documents call [`upload_private`] instead.
pub async fn upload_public(
    bucket: &str,
    object_path: &str,
    data: Vec<u8>,
    content_type: &str,
) -> Result<String, AppError> {
    let client = build_client().await?;

    let upload_type = UploadType::Simple(Media {
        name: object_path.to_string().into(),
        content_type: content_type.to_string().into(),
        content_length: Some(data.len() as u64),
    });

    client
        .upload_object(
            &UploadObjectRequest {
                bucket: bucket.to_string(),
                ..Default::default()
            },
            data,
            &upload_type,
        )
        .await
        .map_err(|e| AppError::Internal(format!("GCS upload failed: {}", e)))?;

    // Return proxy URL — the /api/proxy/gcs/:bucket/*path endpoint generates
    // short-lived signed URLs, which avoids 403 Forbidden when the bucket lacks
    // allUsers viewer permission.
    let url = format!("/api/proxy/gcs/{}/{}", bucket, object_path);
    Ok(url)
}

/// Upload raw bytes to GCS under `object_path` (private object) and
/// return the raw `gs://bucket/path` stored in the DB.
///
/// Signed URLs are generated on-demand by [`generate_signed_url`] only
/// when admins or the user need to view the document.  This avoids
/// requiring a service-account RSA key just for uploads.
pub async fn upload_private(
    bucket: &str,
    object_path: &str,
    data: Vec<u8>,
    content_type: &str,
) -> Result<String, AppError> {
    let client = build_client().await?;

    let upload_type = UploadType::Simple(Media {
        name: object_path.to_string().into(),
        content_type: content_type.to_string().into(),
        content_length: Some(data.len() as u64),
    });

    client
        .upload_object(
            &UploadObjectRequest {
                bucket: bucket.to_string(),
                ..Default::default()
            },
            data,
            &upload_type,
        )
        .await
        .map_err(|e| AppError::Internal(format!("GCS upload failed: {}", e)))?;

    // Return the raw GCS URI (never served to users directly)
    Ok(format!("gs://{}/{}", bucket, object_path))
}

/// Generate a time-limited signed URL for an existing private GCS object.
///
/// `expires_in_minutes` — how long the URL is valid (max 7 days = 10080 min).
pub async fn generate_signed_url(
    bucket: &str,
    object_path: &str,
    expires_in_minutes: u32,
) -> Result<String, AppError> {
    let client = build_client().await?;

    let opts = SignedURLOptions {
        expires: std::time::Duration::from_secs(expires_in_minutes as u64 * 60),
        ..Default::default()
    };

    client
        .signed_url(bucket, object_path, None, None, opts)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to generate signed URL: {}", e)))
}

/// Delete a GCS object by its path (e.g. when a user replaces their avatar).
#[allow(dead_code)]
pub async fn delete_object(bucket: &str, object_path: &str) -> Result<(), AppError> {
    let client = build_client().await?;

    client
        .delete_object(&DeleteObjectRequest {
            bucket: bucket.to_string(),
            object: object_path.to_string(),
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::Internal(format!("GCS delete failed: {}", e)))?;

    Ok(())
}

// ─── GcsService Struct (for Admin/other modules) ────────────────
pub struct GcsService {
    bucket: String,
}

impl GcsService {
    pub async fn new(bucket: &str) -> Self {
        Self {
            bucket: bucket.to_string(),
        }
    }

    pub async fn generate_signed_url(
        &self,
        path: &str,
        expires_in_seconds: u32,
    ) -> Result<String, AppError> {
        // Strip gs://bucket/ prefix if present
        let prefix = format!("gs://{}/", self.bucket);
        let object_path = path.strip_prefix(&prefix).unwrap_or(path);

        generate_signed_url(&self.bucket, object_path, (expires_in_seconds / 60).max(1)).await
    }
}

// ─── Local Filesystem Fallback (Dev Only) ──────────────────────

/// Save a file to the local filesystem under `../uploads/{object_path}`.
/// Returns a URL path like `/uploads/properties/abc/doc.pdf` which is
/// served by the static file handler added in main.rs.
pub async fn upload_local(object_path: &str, data: Vec<u8>) -> Result<String, AppError> {
    let base = std::path::Path::new("../uploads");
    let full_path = base.join(object_path);

    // Create parent directories
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create upload dir: {}", e)))?;
    }

    tokio::fs::write(&full_path, &data)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to write file: {}", e)))?;

    let url = format!("/uploads/{}", object_path);
    tracing::info!("Local file saved: {} ({} bytes)", url, data.len());
    Ok(url)
}

// ─── Helpers ───────────────────────────────────────────────────

/// Build a GCS client using Application Default Credentials (ADC).
///
/// Locally:  run `gcloud auth application-default login` once.
/// Cloud Run: the attached service account is used automatically.
async fn build_client() -> Result<Client, AppError> {
    let config = ClientConfig::default()
        .with_auth()
        .await
        .map_err(|e| AppError::Internal(format!("GCS auth failed: {}", e)))?;

    Ok(Client::new(config))
}

/// Derive a safe file extension from the MIME type.
pub fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        _ => "bin",
    }
}

/// Validate that the MIME type is allowed for avatar uploads.
pub fn validate_image_mime(mime: &str) -> Result<(), AppError> {
    match mime {
        "image/jpeg" | "image/png" | "image/webp" => Ok(()),
        _ => Err(AppError::BadRequest(
            "Only JPEG, PNG, and WebP images are accepted for avatars.".to_string(),
        )),
    }
}

/// Validate that the MIME type is allowed for KYC document uploads.
pub fn validate_kyc_mime(mime: &str) -> Result<(), AppError> {
    match mime {
        "image/jpeg" | "image/png" | "image/webp" | "application/pdf" => Ok(()),
        _ => Err(AppError::BadRequest(
            "Only JPEG, PNG, WebP, and PDF files are accepted for identity documents.".to_string(),
        )),
    }
}

/// Validate MIME types for asset document uploads (legal docs, permits, etc.).
pub fn validate_asset_doc_mime(mime: &str) -> Result<(), AppError> {
    match mime {
        "image/jpeg"
        | "image/png"
        | "image/webp"
        | "application/pdf"
        | "application/msword"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/zip" => Ok(()),
        _ => Err(AppError::BadRequest(
            "Only JPEG, PNG, WebP, PDF, DOC, DOCX, and ZIP files are accepted.".to_string(),
        )),
    }
}

// ─── URL rewriting ─────────────────────────────────────────────
/// Rewrite a stored image/asset URL so that old direct GCS links
/// (`https://storage.googleapis.com/bucket/path`) become proxy
/// paths (`/api/proxy/gcs/bucket/path`). URLs that are already
/// proxy paths or relative paths are returned unchanged.
pub fn rewrite_gcs_url(url: &str) -> String {
    if url.starts_with("https://storage.googleapis.com/") {
        url.replacen("https://storage.googleapis.com/", "/api/proxy/gcs/", 1)
    } else {
        url.to_string()
    }
}

/// Same as [`rewrite_gcs_url`] but works on `Option<String>`.
pub fn rewrite_gcs_url_opt(url: Option<&str>) -> Option<String> {
    url.map(|u| rewrite_gcs_url(u))
}

/// Validate MIME types for asset image uploads (property photos).
pub fn validate_asset_image_mime(mime: &str) -> Result<(), AppError> {
    match mime {
        "image/jpeg" | "image/png" | "image/webp" | "image/gif" => Ok(()),
        _ => Err(AppError::BadRequest(
            "Only JPEG, PNG, WebP, and GIF images are accepted.".to_string(),
        )),
    }
}

/// Extended extension mapping for document uploads.
pub fn extension_for_doc_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/zip" => "zip",
        _ => "bin",
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_for_mime() {
        assert_eq!(extension_for_mime("image/jpeg"), "jpg");
        assert_eq!(extension_for_mime("image/png"), "png");
        assert_eq!(extension_for_mime("application/pdf"), "pdf");
        assert_eq!(extension_for_mime("text/plain"), "bin");
    }

    #[test]
    fn test_validate_image_mime() {
        assert!(validate_image_mime("image/jpeg").is_ok());
        assert!(validate_image_mime("image/png").is_ok());
        assert!(validate_image_mime("image/webp").is_ok());
        assert!(validate_image_mime("application/pdf").is_err());
        assert!(validate_image_mime("text/plain").is_err());
    }

    #[test]
    fn test_validate_kyc_mime() {
        assert!(validate_kyc_mime("image/jpeg").is_ok());
        assert!(validate_kyc_mime("application/pdf").is_ok());
        assert!(validate_kyc_mime("text/plain").is_err());
    }
}
