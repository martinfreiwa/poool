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
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::sign::SignedURLOptions;

use crate::error::AppError;
use sqlx::PgPool;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

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
#[tracing::instrument(
    name = "storage.upload_private",
    skip(data),
    fields(bucket, object_path, bytes = data.len(), content_type)
)]
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

// ─── Classification Markers (Phase 4.3) ────────────────────────────
//
// Every private upload carries custom metadata that tells the system
// its sensitivity class. The reconciler + retention worker + DLP
// scanner dispatch on these markers instead of guessing from the path.
// See docs/storage/04-compliance-and-retention.md → "Layer 3 —
// Classification Markers" for the full table + audit script.

/// PII sensitivity class for storage objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiClass {
    /// Identity docs (KYC). 5y/10y retention, signed-URLs only,
    /// CMEK + AV-scan required.
    A,
    /// Asset docs (contracts, financial reports). Retention = max of
    /// business_end+5y and asset_disposal+5y.
    B,
    /// Public assets (avatars, post images, asset thumbnails). No
    /// retention requirement, deleted with the parent entity.
    C,
    /// Diagnostic / temp objects with no PII.
    None,
}

impl PiiClass {
    /// Canonical string value persisted in `x-goog-meta-pii-class`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::None => "none",
        }
    }

    /// Default retention trigger for this class (matches the
    /// stakeholder-doc Q7 decision tree). Returned as the canonical
    /// string used in `x-goog-meta-retention-trigger`.
    pub fn default_retention_trigger(&self) -> &'static str {
        match self {
            Self::A => "business_end+5y",
            Self::B => "business_end+5y",
            Self::C => "none",
            Self::None => "none",
        }
    }
}

/// Upload raw bytes to GCS *with* classification metadata attached.
/// Use this in preference to [`upload_private`] for any object that
/// carries PII — the reconciler, retention worker and DLP scanner all
/// rely on the `pii-class` marker existing.
///
/// The `uploaded_by_user_id` is captured for the BAIT 8.3 audit trail
/// (who uploaded the object, separate from who owns it).
#[tracing::instrument(
    name = "storage.upload_private_with_markers",
    skip(data),
    fields(
        bucket,
        object_path,
        bytes = data.len(),
        content_type,
        pii_class = pii_class.as_str(),
        uploaded_by_user_id = ?uploaded_by_user_id,
    )
)]
pub async fn upload_private_with_markers(
    bucket: &str,
    object_path: &str,
    data: Vec<u8>,
    content_type: &str,
    pii_class: PiiClass,
    uploaded_by_user_id: Option<uuid::Uuid>,
) -> Result<String, AppError> {
    use google_cloud_storage::http::objects::Object;
    let client = build_client().await?;

    let mut meta = std::collections::HashMap::new();
    meta.insert("pii-class".to_string(), pii_class.as_str().to_string());
    meta.insert(
        "retention-trigger".to_string(),
        pii_class.default_retention_trigger().to_string(),
    );
    if let Some(uid) = uploaded_by_user_id {
        meta.insert("uploaded-by-user-id".to_string(), uid.to_string());
    }
    meta.insert("uploaded-at".to_string(), chrono::Utc::now().to_rfc3339());

    let object = Object {
        name: object_path.to_string(),
        bucket: bucket.to_string(),
        content_type: Some(content_type.to_string()),
        size: data.len() as i64,
        metadata: Some(meta),
        ..Default::default()
    };

    let upload_type = UploadType::Multipart(Box::new(object));

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
        .map_err(|e| AppError::Internal(format!("GCS upload (markers) failed: {}", e)))?;

    Ok(format!("gs://{}/{}", bucket, object_path))
}

/// Generate a time-limited signed URL for an existing private GCS object.
///
/// `expires_in_minutes` — how long the URL is valid (max 7 days = 10080 min).
///
/// Calls through to [`generate_signed_url_with_disposition`] with the
/// `force_download_filename` set to `None`, preserving the prior caller
/// contract (browser may render the object inline depending on its
/// content-type — fine for asset images, NOT fine for KYC PDFs).
pub async fn generate_signed_url(
    bucket: &str,
    object_path: &str,
    expires_in_minutes: u32,
) -> Result<String, AppError> {
    generate_signed_url_with_disposition(bucket, object_path, expires_in_minutes, None).await
}

/// Generate a signed URL that forces the browser to download the object
/// (instead of rendering it inline). Used for PII-class-A documents (KYC)
/// where inline render = exploitable XSS surface via PDF-embedded scripts.
///
/// `force_download_filename`: when `Some`, GCS responds with
/// `Content-Disposition: attachment; filename="<name>"` regardless of the
/// stored object's content-type. The browser saves the file rather than
/// running its renderer pipeline.
///
/// Implementation: GCS supports the `response-content-disposition` query
/// parameter on signed URLs (signed alongside the rest of the URL so a
/// MitM can't strip it). We inject it via the `query_parameters` field
/// of `SignedURLOptions`.
#[tracing::instrument(
    name = "storage.signed_url",
    fields(bucket, object_path, expires_in_minutes, disposition = ?force_download_filename)
)]
pub async fn generate_signed_url_with_disposition(
    bucket: &str,
    object_path: &str,
    expires_in_minutes: u32,
    force_download_filename: Option<&str>,
) -> Result<String, AppError> {
    let client = build_client().await?;

    let mut query_parameters = std::collections::HashMap::new();
    if let Some(name) = force_download_filename {
        // Quote-safe filename per RFC 6266 — strip CR/LF and quotes to
        // avoid header injection.
        let safe = name
            .replace(['\r', '\n', '"'], "")
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>();
        let disposition = format!("attachment; filename=\"{}\"", safe);
        query_parameters.insert(
            "response-content-disposition".to_string(),
            vec![disposition],
        );
    }

    let opts = SignedURLOptions {
        expires: std::time::Duration::from_secs(expires_in_minutes as u64 * 60),
        query_parameters,
        ..Default::default()
    };

    client
        .signed_url(bucket, object_path, None, None, opts)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to generate signed URL: {}", e)))
}

/// Delete a GCS object by its path (e.g. when a user replaces their avatar).
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

/// Download an object's raw bytes from GCS.
///
/// Returns `(content_type, bytes)`. Uses standard ADC auth which only
/// requires `storage.objects.get` — no signing / `signBlob` permission needed.
#[tracing::instrument(
    name = "storage.download",
    fields(bucket, object_path, bytes = tracing::field::Empty)
)]
pub async fn download_object(
    bucket: &str,
    object_path: &str,
) -> Result<(String, Vec<u8>), AppError> {
    if let Some(fake_root) = fake_gcs_download_root()? {
        return download_fake_gcs_object(&fake_root, bucket, object_path).await;
    }

    let client = build_client().await?;

    // Fetch object metadata first to get content_type
    let meta = client
        .get_object(&GetObjectRequest {
            bucket: bucket.to_string(),
            object: object_path.to_string(),
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::NotFound(format!("GCS object not found: {}", e)))?;

    let content_type = meta
        .content_type
        .unwrap_or_else(|| "application/octet-stream".to_string());

    // Download the bytes
    let data = client
        .download_object(
            &GetObjectRequest {
                bucket: bucket.to_string(),
                object: object_path.to_string(),
                ..Default::default()
            },
            &Range::default(),
        )
        .await
        .map_err(|e| AppError::Internal(format!("GCS download failed: {}", e)))?;

    Ok((content_type, data))
}

fn fake_gcs_download_root() -> Result<Option<PathBuf>, AppError> {
    let Ok(raw_root) = std::env::var("POOOL_GCS_DOWNLOAD_FAKE_ROOT") else {
        return Ok(None);
    };
    let raw_root = raw_root.trim();
    if raw_root.is_empty() {
        return Ok(None);
    }
    if !is_local_fallback_allowed() {
        return Err(AppError::Internal(
            "POOOL_GCS_DOWNLOAD_FAKE_ROOT is only allowed in development/dev/local environments."
                .into(),
        ));
    }
    Ok(Some(PathBuf::from(raw_root)))
}

async fn download_fake_gcs_object(
    root: &Path,
    bucket: &str,
    object_path: &str,
) -> Result<(String, Vec<u8>), AppError> {
    if bucket.is_empty()
        || object_path.is_empty()
        || object_path.starts_with('/')
        || object_path.contains('\\')
        || object_path.contains("//")
        || object_path.chars().any(char::is_control)
        || Path::new(object_path)
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(AppError::BadRequest("GCS object path is invalid.".into()));
    }

    let full_path = root.join(bucket).join(object_path);
    let data = tokio::fs::read(&full_path)
        .await
        .map_err(|e| AppError::NotFound(format!("Fake GCS object not found: {}", e)))?;
    let content_type = sniff_mime(&data)
        .unwrap_or("application/octet-stream")
        .to_string();

    Ok((content_type, data))
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

    /// Same as [`generate_signed_url`] but forces the browser to download
    /// the object via `Content-Disposition: attachment`. Use for KYC docs
    /// and any other content where inline-render = XSS risk.
    pub async fn generate_signed_url_with_disposition(
        &self,
        path: &str,
        expires_in_seconds: u32,
        force_download_filename: Option<&str>,
    ) -> Result<String, AppError> {
        let prefix = format!("gs://{}/", self.bucket);
        let object_path = path.strip_prefix(&prefix).unwrap_or(path);

        generate_signed_url_with_disposition(
            &self.bucket,
            object_path,
            (expires_in_seconds / 60).max(1),
            force_download_filename,
        )
        .await
    }
}

// ─── Local Filesystem Fallback (Dev Only) ──────────────────────

// ─── Antivirus Scan Result Lookup (Phase 2.3) ──────────────────
//
// The Cloud Function `av-scan-clamav` (see docs/storage/02-antivirus-
// scanning.md) writes scan status into the object's custom metadata:
//   x-goog-meta-av-status:   "clean" | "infected" | "error"
//   x-goog-meta-av-scanner:  e.g. "clamav-1.4.x"
//   x-goog-meta-av-scanned-at: ISO-8601 UTC
//
// Callers that serve a private object to a user (signed-URL generation,
// admin preview, batch export) should check `av_status()` first and
// refuse to serve anything but `Clean`. Until the Cloud Function is
// deployed, every object returns `NotYetScanned` — wiring callers to
// hard-reject on that status would block all downloads, so the typed
// enum is exposed but enforcement is opt-in per caller.

/// Outcome of the malware scan for a given object, derived from its
/// GCS custom metadata. `NotYetScanned` is also returned when the
/// metadata is absent (no scanner deployed yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvStatus {
    Clean,
    /// Malware detected. The `String` holds the detection name (e.g.
    /// "Eicar-Test-Signature") so logs are actionable.
    Infected(String),
    /// Scanner ran but failed. Object was left in the prod bucket; the
    /// caller should refuse to serve it and emit a Sentry warning.
    ScannerError(String),
    /// Object has no AV metadata yet — either the scanner is not
    /// deployed or the trigger hasn't fired yet (latency typically <5s).
    /// Callers serving PII-class-A content (KYC) should treat this as
    /// "deny" once the scanner is live; for now it's "allow with caveat".
    NotYetScanned,
}

/// Read the AV scan result for a given object. Bucket + object_path are
/// the same shape used by `upload_private` / `download_object`. Failure
/// to reach GCS bubbles as `AppError::Internal`.
pub async fn av_status(bucket: &str, object_path: &str) -> Result<AvStatus, AppError> {
    let client = build_client().await?;
    let meta = client
        .get_object(&GetObjectRequest {
            bucket: bucket.to_string(),
            object: object_path.to_string(),
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::NotFound(format!("GCS object metadata: {}", e)))?;

    // Per the google-cloud-storage crate, custom metadata lives under
    // `meta.metadata` as Option<HashMap<String, String>>.
    let custom = meta.metadata.unwrap_or_default();
    match custom.get("av-status").map(String::as_str) {
        Some("clean") => Ok(AvStatus::Clean),
        Some("infected") => Ok(AvStatus::Infected(
            custom
                .get("av-detection")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        )),
        Some("error") => Ok(AvStatus::ScannerError(
            custom
                .get("av-error")
                .cloned()
                .unwrap_or_else(|| "scanner reported error".to_string()),
        )),
        _ => Ok(AvStatus::NotYetScanned),
    }
}

// ─── Per-User Storage Quota (Phase 2.6) ─────────────────────────
//
// Per-class quotas in BYTES. Bumping these is a code-deploy operation,
// not a DB edit — we want canary + revert via git, not silent prod
// changes. Source of truth is the `QuotaClass` enum + this map.
//
// Default quotas reflect realistic user behaviour:
//   - 25 MB avatar:        ~5 changes/year × 5 MB
//   - 100 MB post_image:   ~20 community posts × 5 MB
//   - 500 MB asset_image:  ~50 property pictures × 10 MB (developers)
//   - 1 GB asset_document: ~50 property docs × 20 MB (developers)
//   - 50 MB kyc_document:  ~5 KYC docs × 10 MB
//   - 10 MB developer_logo: ~5 logo iterations × 2 MB
//
// File-count limits (default 100/class) catch death-by-many-small-files
// abuse — a user uploading 1000 × 1KB images doesn't trip the byte
// quota but breaks the bucket-list latency.

/// Allowlist of storage classes that have a per-user quota. The string
/// values must match the CHECK constraint on `storage_user_quotas.class`.
#[derive(Debug, Clone, Copy)]
pub enum QuotaClass {
    Avatar,
    PostImage,
    AssetImage,
    AssetDocument,
    KycDocument,
    DeveloperLogo,
}

impl QuotaClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::PostImage => "post_image",
            Self::AssetImage => "asset_image",
            Self::AssetDocument => "asset_document",
            Self::KycDocument => "kyc_document",
            Self::DeveloperLogo => "developer_logo",
        }
    }

    /// Max cumulative bytes a single user may store in this class.
    pub fn quota_bytes(&self) -> i64 {
        match self {
            Self::Avatar => 25 * 1024 * 1024,          // 25 MB
            Self::PostImage => 100 * 1024 * 1024,      // 100 MB
            Self::AssetImage => 500 * 1024 * 1024,     // 500 MB
            Self::AssetDocument => 1024 * 1024 * 1024, // 1 GB
            Self::KycDocument => 50 * 1024 * 1024,     // 50 MB
            Self::DeveloperLogo => 10 * 1024 * 1024,   // 10 MB
        }
    }

    /// Max number of files a single user may keep in this class. Catches
    /// many-tiny-files abuse that the byte quota alone misses.
    pub fn quota_files(&self) -> i32 {
        match self {
            Self::Avatar => 50,
            Self::PostImage => 200,
            Self::AssetImage => 500,
            Self::AssetDocument => 200,
            Self::KycDocument => 25,
            Self::DeveloperLogo => 25,
        }
    }
}

/// Look up the current `(bytes_used, file_count)` for a user × class.
/// A missing row returns `(0, 0)` — first upload creates the row.
pub async fn get_quota_usage(
    pool: &PgPool,
    user_id: Uuid,
    class: QuotaClass,
) -> Result<(i64, i32), AppError> {
    let row: Option<(i64, i32)> = sqlx::query_as(
        "SELECT bytes_used, file_count FROM storage_user_quotas
         WHERE user_id = $1 AND class = $2",
    )
    .bind(user_id)
    .bind(class.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(row.unwrap_or((0, 0)))
}

/// Returns Ok(()) when the user has headroom for `incoming_bytes` more
/// in `class`; Err with a structured `AppError::BadRequest` otherwise
/// so the caller can render a clean 4xx (not a 5xx). Checks BOTH the
/// byte cap and the file-count cap.
pub async fn check_quota_or_reject(
    pool: &PgPool,
    user_id: Uuid,
    class: QuotaClass,
    incoming_bytes: i64,
) -> Result<(), AppError> {
    let (used_bytes, used_files) = get_quota_usage(pool, user_id, class).await?;
    let cap_bytes = class.quota_bytes();
    let cap_files = class.quota_files();

    if used_bytes + incoming_bytes > cap_bytes {
        return Err(AppError::BadRequest(format!(
            "Storage quota exceeded for {}: {} used + {} incoming > {} cap",
            class.as_str(),
            used_bytes,
            incoming_bytes,
            cap_bytes,
        )));
    }
    if used_files + 1 > cap_files {
        return Err(AppError::BadRequest(format!(
            "File-count quota exceeded for {}: {} of {} files in use",
            class.as_str(),
            used_files,
            cap_files,
        )));
    }
    Ok(())
}

/// Atomically credit `(bytes, +1 file)` against a user's quota row.
/// Upserts the row on first use. Call AFTER a successful upload +
/// successful DB-record INSERT — otherwise the counter desyncs from
/// reality.
pub async fn increment_quota(
    pool: &PgPool,
    user_id: Uuid,
    class: QuotaClass,
    bytes: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO storage_user_quotas (user_id, class, bytes_used, file_count, updated_at)
           VALUES ($1, $2, $3, 1, NOW())
           ON CONFLICT (user_id, class) DO UPDATE SET
               bytes_used = storage_user_quotas.bytes_used + EXCLUDED.bytes_used,
               file_count = storage_user_quotas.file_count + 1,
               updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(class.as_str())
    .bind(bytes)
    .execute(pool)
    .await?;
    Ok(())
}

/// Decrement after a confirmed file deletion. Floored at 0 so a
/// reconciliation-fixed under-count doesn't go negative (CHECK
/// `bytes_used >= 0` would otherwise reject the UPDATE).
pub async fn decrement_quota(
    pool: &PgPool,
    user_id: Uuid,
    class: QuotaClass,
    bytes: i64,
) -> Result<(), AppError> {
    sqlx::query(
        r#"UPDATE storage_user_quotas
           SET bytes_used = GREATEST(0, bytes_used - $3),
               file_count = GREATEST(0, file_count - 1),
               updated_at = NOW()
           WHERE user_id = $1 AND class = $2"#,
    )
    .bind(user_id)
    .bind(class.as_str())
    .bind(bytes)
    .execute(pool)
    .await?;
    Ok(())
}

/// Compute a hex-encoded SHA-256 of the upload bytes. Used to populate
/// `content_sha256` on `kyc_documents` / `asset_documents` so we can
/// later detect tampering, corruption, or re-uploads of identical files.
/// SHA-256 is the deduplication-key recommended by GCS for object
/// integrity (Content-MD5 alternative; SHA-256 has stronger collision
/// resistance).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Returns true when local-FS fallback is allowed. **PRODUCTION SAFETY**:
/// on Cloud Run the container filesystem is ephemeral — files saved
/// locally vanish on every container restart while the DB row points at
/// a now-404 URL, causing silent data loss. The fallback is restricted
/// to `POOOL_ENV=development|dev|local` so production GCS-failures fail
/// loudly (5xx + Sentry capture) instead of degrading silently.
pub fn is_local_fallback_allowed() -> bool {
    matches!(
        std::env::var("POOOL_ENV").as_deref(),
        Ok("development") | Ok("dev") | Ok("local")
    )
}

/// Save a file to the local filesystem under `../uploads/{object_path}`.
/// Returns a URL path like `/uploads/properties/abc/doc.pdf` which is
/// served by the static file handler added in main.rs.
///
/// **Production safety**: fails with `AppError::Internal` when
/// `POOOL_ENV` is not `development|dev|local`. On Cloud Run a successful
/// local write would still cause silent data loss on container recycle,
/// which is strictly worse than a loud 5xx.
pub async fn upload_local(object_path: &str, data: Vec<u8>) -> Result<String, AppError> {
    if !is_local_fallback_allowed() {
        // Loud failure beats silent data loss. The fallback is intentional
        // dev-only convenience — production deployments must have a
        // healthy GCS path or the deploy is misconfigured.
        return Err(AppError::Internal(
            "Local-FS upload-fallback disabled in production environment \
             (POOOL_ENV is not development/dev/local). GCS upload must \
             succeed. Check service-account credentials and bucket reachability."
                .to_string(),
        ));
    }

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

/// Public wrapper around the private [`build_client`] used by sibling
/// modules in `crate::storage::*` (reconciler, future scrubber jobs).
/// Kept as a thin alias so cross-module callers don't depend on the
/// private free function's visibility staying narrow.
pub async fn build_client_public() -> Result<Client, AppError> {
    build_client().await
}

/// Magic-byte MIME sniffing. Trust file contents, not the client-declared
/// header. Returns the canonical MIME if recognised, `None` otherwise.
pub fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
    {
        return Some("application/zip");
    }
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Some("application/msword");
    }
    None
}

/// Returns true when the client-declared MIME is acceptable for the sniffed
/// bytes. Accepts common aliases (image/jpg → image/jpeg) and
/// application/octet-stream as a fallback.
pub fn mime_matches(client_mime: &str, sniffed: &str) -> bool {
    let c = client_mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if c == sniffed {
        return true;
    }
    matches!(
        (c.as_str(), sniffed),
        ("image/jpg", "image/jpeg")
            | ("image/pjpeg", "image/jpeg")
            | (
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "application/zip",
            )
            | ("application/octet-stream", _)
    )
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
/// Rewrite a stored image/asset URL into a path the app can serve.
///
/// - Old direct GCS links (`https://storage.googleapis.com/bucket/path`) become
///   proxy paths (`/api/proxy/gcs/bucket/path`).
/// - Legacy seed rows stored flat `/images/<file>` paths, while the checked-in
///   property assets live under `/static/images/properties/<folder>/<file>`.
pub fn rewrite_gcs_url(url: &str) -> String {
    if url.starts_with("https://storage.googleapis.com/") {
        url.replacen("https://storage.googleapis.com/", "/api/proxy/gcs/", 1)
    } else if let Some(file_name) = url.strip_prefix("/images/") {
        legacy_static_image_url(file_name)
            .or_else(|| legacy_property_image_url(file_name))
            .unwrap_or_else(|| url.to_string())
    } else {
        url.to_string()
    }
}

fn legacy_static_image_url(file_name: &str) -> Option<String> {
    match file_name {
        "martin_pfp.png" | "martin_pfp.webp" => {
            Some("/static/images/profiles/martin_pfp.webp".to_string())
        }
        "villa1.jpg" | "villa1.webp" => Some("/static/images/seed/villa1.webp".to_string()),
        "villa1_2.jpg" | "villa1_2.webp" => Some("/static/images/seed/villa1_2.webp".to_string()),
        "villa1_3.jpg" | "villa1_3.webp" => Some("/static/images/seed/villa1_3.webp".to_string()),
        "villa1_4.jpg" | "villa1_4.webp" => Some("/static/images/seed/villa1_4.webp".to_string()),
        "villa2_1.jpg" | "villa2_1.webp" => Some("/static/images/seed/villa2_1.webp".to_string()),
        "villa2_2.jpg" | "villa2_2.webp" => Some("/static/images/seed/villa2_2.webp".to_string()),
        "villa3_1.jpg" | "villa3_1.webp" => Some("/static/images/seed/villa3_1.webp".to_string()),
        "villa3_2.jpg" | "villa3_2.webp" => Some("/static/images/seed/villa3_2.webp".to_string()),
        "villa4_1.jpg" | "villa4_1.webp" => Some("/static/images/seed/villa4_1.webp".to_string()),
        "villa4_2.jpg" | "villa4_2.webp" => Some("/static/images/seed/villa4_2.webp".to_string()),
        "villa5.jpg" | "villa5.webp" => Some("/static/images/seed/villa5.webp".to_string()),
        "villa6.jpg" | "villa6.webp" => Some("/static/images/seed/villa6.webp".to_string()),
        "tokenization_cover.png" | "tokenization_cover.webp" => {
            Some("/static/images/ui/tokenization_cover.webp".to_string())
        }
        "bali_property.png" | "bali_property.webp" => {
            Some("/static/images/seed/bali_property.webp".to_string())
        }
        "diversify_cover.png" | "diversify_cover.webp" => {
            Some("/static/images/ui/diversify_cover.webp".to_string())
        }
        "platform_update.png" | "platform_update.webp" => {
            Some("/static/images/ui/platform_update.webp".to_string())
        }
        "passive_income.png" | "passive_income.webp" => {
            Some("/static/images/ui/passive_income.webp".to_string())
        }
        _ => None,
    }
}

fn legacy_property_image_url(file_name: &str) -> Option<String> {
    const FOLDERS: &[&str] = &[
        "bali_agri",
        "bukit_villa",
        "cacao_bali",
        "canggu_flip",
        "canggu_pool",
        "canggu_surf",
        "canopy",
        "coffee_kintamani",
        "denpasar_cliff",
        "denpasar_plaza",
        "grand_resort",
        "green_field",
        "jimbaran_sunset",
        "sanur_beach",
        "seminyak_complex",
        "ubud_resort",
        "uluwatu_retreat",
        "uluwatu_temple",
    ];

    FOLDERS
        .iter()
        .find(|folder| file_name.starts_with(**folder))
        .map(|folder| format!("/static/images/properties/{}/{}", folder, file_name))
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

    #[test]
    fn test_rewrite_gcs_url_maps_legacy_property_images() {
        assert_eq!(
            rewrite_gcs_url("/images/ubud_resort_hero.webp"),
            "/static/images/properties/ubud_resort/ubud_resort_hero.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/bukit_villa_pool.webp"),
            "/static/images/properties/bukit_villa/bukit_villa_pool.webp"
        );
    }

    #[test]
    fn test_rewrite_gcs_url_maps_legacy_seed_and_blog_images() {
        assert_eq!(
            rewrite_gcs_url("/images/villa1.webp"),
            "/static/images/seed/villa1.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/villa1.jpg"),
            "/static/images/seed/villa1.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/villa4_2.jpg"),
            "/static/images/seed/villa4_2.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/tokenization_cover.png"),
            "/static/images/ui/tokenization_cover.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/bali_property.png"),
            "/static/images/seed/bali_property.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/martin_pfp.png"),
            "/static/images/profiles/martin_pfp.webp"
        );
        assert_eq!(
            rewrite_gcs_url("/images/martin_pfp.webp"),
            "/static/images/profiles/martin_pfp.webp"
        );
    }

    #[test]
    fn test_rewrite_gcs_url_keeps_unknown_legacy_images() {
        assert_eq!(
            rewrite_gcs_url("/images/custom-upload.webp"),
            "/images/custom-upload.webp"
        );
    }

    #[test]
    fn test_rewrite_gcs_url_maps_storage_googleapis_urls() {
        assert_eq!(
            rewrite_gcs_url("https://storage.googleapis.com/poool/assets/one.webp"),
            "/api/proxy/gcs/poool/assets/one.webp"
        );
    }
}
