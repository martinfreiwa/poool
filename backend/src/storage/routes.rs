/// HTTP route handlers for file upload endpoints.
///
/// POST /api/upload/avatar      – Upload/replace user profile photo.
/// POST /api/upload/kyc         – Upload a KYC identity document (private).
///
/// Both endpoints:
/// - Require the user to be authenticated (session cookie).
/// - Accept multipart/form-data with a single `file` field.
/// - Enforce MIME type allowlists.
/// - Enforce a maximum file size (5 MB avatars, 10 MB KYC docs).
/// - Store the public URL / GCS path in PostgreSQL.
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;
use std::collections::HashSet;
use uuid::Uuid;

use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;
use crate::common::sanitize::sanitize_text;

const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024; // 5 MB
const MAX_KYC_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const MAX_DEVELOPER_LOGO_BYTES: usize = 2 * 1024 * 1024; // 2 MB

// ─── Avatar ────────────────────────────────────────────────────

/// POST /api/upload/avatar
///
/// Replaces the authenticated user's profile photo.
///
/// Request: multipart/form-data with a single `file` field (JPEG / PNG / WebP, max 5 MB).
/// Response: { "avatar_url": "https://storage.googleapis.com/..." }
pub async fn upload_avatar(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    // Auth
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let bucket = state.config.gcs_bucket.clone();

    // Read multipart field
    let (file_bytes, mime_type) = match read_multipart_file(&mut multipart, MAX_AVATAR_BYTES).await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            )
                .into_response();
        }
    };

    // Validate MIME
    if let Err(e) = service::validate_image_mime(&mime_type) {
        return e.into_response();
    }

    let ext = service::extension_for_mime(&mime_type);
    let object_path = format!("avatars/{}/{}.{}", user.id, Uuid::new_v4(), ext);

    // Upload to GCS if configured, otherwise fall back to local filesystem.
    let avatar_url = if let Some(ref b) = bucket {
        let gcs_fut = service::upload_public(b, &object_path, file_bytes.clone(), &mime_type);
        match tokio::time::timeout(std::time::Duration::from_secs(15), gcs_fut).await {
            Ok(Ok(url)) => url,
            Ok(Err(e)) => {
                tracing::warn!("GCS avatar upload failed, falling back to local: {}", e);
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => {
                        tracing::error!("Local avatar fallback failed: {}", e2);
                        return e2.into_response();
                    }
                }
            }
            Err(_) => {
                tracing::warn!("GCS avatar upload timed out, falling back to local storage");
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => return e2.into_response(),
                }
            }
        }
    } else {
        match service::upload_local(&object_path, file_bytes).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Local avatar save failed: {}", e);
                return e.into_response();
            }
        }
    };

    // Persist URL in users table
    let result = sqlx::query("UPDATE users SET avatar_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(&avatar_url)
        .bind(user.id)
        .execute(&state.db)
        .await;

    if let Err(e) = result {
        tracing::error!("Failed to save avatar_url for {}: {}", user.id, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Upload succeeded but failed to save URL"})),
        )
            .into_response();
    }

    // Evict the community user_bridge cache so feed/comment/composer surfaces
    // pick up the new avatar immediately instead of serving the stale Redis
    // entry for up to 5 minutes.
    crate::community::user_bridge::invalidate_user_cache(state.redis.as_ref(), user.id).await;

    tracing::info!("Avatar updated for user {} → {}", user.id, avatar_url);

    Json(serde_json::json!({
        "status": "success",
        "avatar_url": avatar_url,
    }))
    .into_response()
}

// ─── Developer Logo ────────────────────────────────────────────

/// POST /api/upload/developer-logo
///
/// Uploads/replaces the authenticated developer's public logo.
pub async fn upload_developer_logo(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let bucket = state.config.gcs_bucket.clone();

    let (file_bytes, mime_type) =
        match read_multipart_file(&mut multipart, MAX_DEVELOPER_LOGO_BYTES).await {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response();
            }
        };

    if let Err(e) = service::validate_image_mime(&mime_type) {
        return e.into_response();
    }

    let ext = service::extension_for_mime(&mime_type);
    let object_path = format!("developer-logos/{}/{}.{}", user.id, Uuid::new_v4(), ext);

    let logo_url = if let Some(ref b) = bucket {
        let gcs_fut = service::upload_public(b, &object_path, file_bytes.clone(), &mime_type);
        match tokio::time::timeout(std::time::Duration::from_secs(15), gcs_fut).await {
            Ok(Ok(url)) => url,
            Ok(Err(e)) => {
                tracing::warn!("GCS logo upload failed, falling back to local: {}", e);
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => return e2.into_response(),
                }
            }
            Err(_) => match service::upload_local(&object_path, file_bytes).await {
                Ok(url) => url,
                Err(e2) => return e2.into_response(),
            },
        }
    } else {
        match service::upload_local(&object_path, file_bytes).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Local logo save failed: {}", e);
                return e.into_response();
            }
        }
    };

    if let Err(e) =
        crate::settings::service::update_developer_logo(&state.db, user.id, &logo_url).await
    {
        tracing::warn!("Failed to save developer logo for {}: {}", user.id, e);
        let message = match e {
            crate::error::AppError::Unauthorized(msg) => msg,
            _ => "Upload succeeded but failed to save logo URL.".to_string(),
        };
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": message})),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "status": "success",
        "url": logo_url,
        "logo_url": logo_url,
    }))
    .into_response()
}

// ─── Community Post Image Upload ───────────────────────────────

const MAX_POST_IMAGE_BYTES: usize = 5 * 1024 * 1024; // 5 MB

/// POST /api/upload/post-image
///
/// Uploads an image for a community post.
/// Request: multipart/form-data with `file` (max 5 MB).
/// Response: { "status": "success", "image_url": "url" }
pub async fn upload_post_image(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let bucket = state.config.gcs_bucket.clone();

    // Read multipart field
    let (file_bytes, mime_type) =
        match read_multipart_file(&mut multipart, MAX_POST_IMAGE_BYTES).await {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e})),
                )
                    .into_response();
            }
        };

    if let Err(e) = service::validate_image_mime(&mime_type) {
        return e.into_response();
    }

    let ext = service::extension_for_mime(&mime_type);
    let object_path = format!("community/posts/{}/{}.{}", user.id, Uuid::new_v4(), ext);

    let image_url = if let Some(ref b) = bucket {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            service::upload_public(b, &object_path, file_bytes.clone(), &mime_type),
        )
        .await
        {
            Ok(Ok(url)) => url,
            Ok(Err(e)) => {
                tracing::warn!("Post image upload failed: {}; falling back to local.", e);
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e) => {
                        tracing::error!("Local post image save failed: {}", e);
                        return e.into_response();
                    }
                }
            }
            Err(_) => {
                tracing::warn!("GCS asset image upload timed out. Falling back to local.");
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e) => {
                        tracing::error!("Local post image save failed: {}", e);
                        return e.into_response();
                    }
                }
            }
        }
    } else {
        match service::upload_local(&object_path, file_bytes).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Local post image save failed: {}", e);
                return e.into_response();
            }
        }
    };

    Json(serde_json::json!({
        "status": "success",
        "image_url": image_url,
    }))
    .into_response()
}

// ─── KYC Document ──────────────────────────────────────────────

/// POST /api/upload/kyc
///
/// Upload a KYC identity document for the authenticated user.
/// The file is stored privately on GCS; a short-lived signed URL
/// is returned so the frontend can show a preview.
///
/// Request: multipart/form-data fields:
///   - `file`          – the document (JPEG / PNG / WebP / PDF, max 10 MB)
///   - `document_type` – one of: passport, national_id, driving_licence, proof_of_address
///
/// Response: {
///   "status": "success",
///   "document_id": "<uuid>",
///   "preview_url": "<signed-url valid 15 min>",
///   "kyc_status": "pending"
/// }
pub async fn upload_kyc_document(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    // Auth
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let bucket = state.config.gcs_bucket.clone();

    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("kyc:upload:{}", user.id))
        .await
    {
        return crate::error::AppError::RateLimited(retry_after).into_response();
    }

    // Read all multipart fields: `file` and `document_type`
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type = String::from("application/octet-stream");
    let mut document_type = String::from("other");

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "document_type" {
            if let Ok(text) = field.text().await {
                document_type = text;
            }
        } else if field_name == "file" {
            // Detect MIME from content-type header, fallback to mime_guess
            if let Some(ct) = field.content_type() {
                mime_type = ct.to_string();
            }

            // Stream the field chunk-by-chunk so an oversized upload is
            // rejected before the whole body is in memory. `field.bytes()`
            // would buffer the entire payload first and let a client burn
            // RAM up to the request body limit.
            let mut field = field;
            let mut bytes: Vec<u8> = Vec::with_capacity(8 * 1024);
            loop {
                match field.chunk().await {
                    Ok(Some(chunk)) => {
                        if bytes.len().saturating_add(chunk.len()) > MAX_KYC_BYTES {
                            return (
                                StatusCode::PAYLOAD_TOO_LARGE,
                                Json(serde_json::json!({"error": "File must be ≤ 10 MB"})),
                            )
                                .into_response();
                        }
                        bytes.extend_from_slice(&chunk);
                    }
                    Ok(None) => break,
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": "Failed to read uploaded file"})),
                        )
                            .into_response();
                    }
                }
            }

            file_bytes = Some(bytes);
        }
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No file field found in request"})),
            )
                .into_response();
        }
    };

    // Magic-byte sniff: trust file contents, not the client-declared MIME.
    let sniffed = match sniff_mime(&file_bytes) {
        Some(m) => m,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Unsupported or unrecognized file format"})),
            )
                .into_response();
        }
    };
    if !mime_matches(&mime_type, sniffed) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "File content does not match declared type"})),
        )
            .into_response();
    }
    mime_type = sniffed.to_string();

    // Validate MIME
    if let Err(e) = service::validate_kyc_mime(&mime_type) {
        return e.into_response();
    }

    let document_type = match crate::kyc::service::normalize_document_type(Some(&document_type)) {
        Ok(Some(value)) => value,
        Ok(None) => "other".to_string(),
        Err(e) => return e.into_response(),
    };

    let ext = service::extension_for_mime(&mime_type);
    let file_id = Uuid::new_v4();
    let object_path = format!("kyc/{}/{}.{}", user.id, file_id, ext);

    // Upload to GCS if configured, otherwise fall back to local filesystem.
    let gcs_path = if let Some(ref b) = bucket {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            service::upload_private(b, &object_path, file_bytes.clone(), &mime_type),
        )
        .await
        {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => {
                tracing::warn!("GCS KYC upload failed, falling back to local: {}", e);
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => return e2.into_response(),
                }
            }
            Err(_) => {
                tracing::warn!("GCS KYC upload timed out, falling back to local");
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => return e2.into_response(),
                }
            }
        }
    } else {
        match service::upload_local(&object_path, file_bytes).await {
            Ok(url) => url,
            Err(e) => return e.into_response(),
        }
    };

    let used_gcs = bucket.is_some();

    // Helper: best-effort cleanup of the uploaded file on DB failure.
    let cleanup = |path: String, bucket_name: Option<String>| {
        tokio::spawn(async move {
            if let Some(b) = bucket_name {
                if let Err(e) = service::delete_object(&b, &path).await {
                    tracing::warn!("GCS cleanup after DB failure failed for {}: {}", path, e);
                } else {
                    tracing::info!("GCS cleanup after DB failure succeeded for {}", path);
                }
            }
        });
    };

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                "Failed to begin KYC document transaction for {}: {}",
                user.id,
                e
            );
            if used_gcs {
                cleanup(object_path.clone(), bucket.clone());
            }
            return crate::error::AppError::Database(e).into_response();
        }
    };

    // Persist to kyc_documents table
    let doc_id: uuid::Uuid = match sqlx::query_scalar(
        r#"
        INSERT INTO kyc_documents
            (user_id, document_type, gcs_path, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING id
        "#,
    )
    .bind(user.id)
    .bind(&document_type)
    .bind(&gcs_path)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to insert kyc_document for {}: {}", user.id, e);
            let _ = tx.rollback().await;
            if used_gcs {
                cleanup(object_path.clone(), bucket.clone());
            }
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Upload succeeded but failed to save record"})),
            )
                .into_response();
        }
    };

    // Audit log
    if let Err(e) = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'kyc_document.uploaded', 'kyc_documents', $2, $3)"#,
    )
    .bind(user.id)
    .bind(doc_id)
    .bind(serde_json::json!({
        "document_type": &document_type,
        "gcs_path": &gcs_path,
    }))
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Failed to audit KYC document upload for {}: {}", user.id, e);
        let _ = tx.rollback().await;
        if used_gcs {
            cleanup(object_path.clone(), bucket.clone());
        }
        return crate::error::AppError::Database(e).into_response();
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            "Failed to commit KYC document upload for {}: {}",
            user.id,
            e
        );
        if used_gcs {
            cleanup(object_path.clone(), bucket.clone());
        }
        return crate::error::AppError::Database(e).into_response();
    }

    tracing::info!(
        "KYC document uploaded: user={}, doc_id={}, type={}",
        user.id,
        doc_id,
        document_type
    );

    Json(serde_json::json!({
        "status": "success",
        "document_id": doc_id,
        "kyc_status": "pending",
        "message": "Document uploaded. Our team will review it and update your KYC status."
    }))
    .into_response()
}

// ─── Shared helpers ────────────────────────────────────────────

/// Read bytes from the first `file` field in a multipart request.
/// Returns `(bytes, mime_type)` where `mime_type` is sniffed from the file's
/// magic bytes — never trusted directly from the client-supplied Content-Type,
/// which is attacker-controlled.
async fn read_multipart_file(
    multipart: &mut Multipart,
    max_bytes: usize,
) -> Result<(Vec<u8>, String), String> {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }

        let client_mime = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let bytes: Vec<u8> = field
            .bytes()
            .await
            .map_err(|_| "Failed to read uploaded file".to_string())?
            .to_vec();

        if bytes.len() > max_bytes {
            let limit_mb = max_bytes / (1024 * 1024);
            return Err(format!("File must be ≤ {} MB", limit_mb));
        }

        // Sniff magic bytes and reject if the file has no recognizable header
        // or if the client-declared MIME doesn't match what's actually in the
        // buffer. Mismatch indicates a spoofed upload.
        let sniffed = sniff_mime(&bytes)
            .ok_or_else(|| "Unsupported or unrecognized file format".to_string())?;
        if !mime_matches(&client_mime, sniffed) {
            return Err(format!(
                "File content does not match declared type ({})",
                client_mime
            ));
        }

        return Ok((bytes, sniffed.to_string()));
    }

    Err("No `file` field found in request".to_string())
}

/// Sniff the MIME type from the leading bytes of a buffer. Returns None for
/// formats we don't accept (executables, archives, office docs, etc.).
// Canonical implementations live in [`crate::storage::service`]; these are
// thin wrappers so the existing routes file doesn't need a wide-scale rename.
fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    crate::storage::service::sniff_mime(bytes)
}

fn mime_matches(client_mime: &str, sniffed: &str) -> bool {
    crate::storage::service::mime_matches(client_mime, sniffed)
}

fn canonical_asset_doc_mime(client_mime: &str, sniffed: &str) -> String {
    let c = client_mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if c == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        && sniffed == "application/zip"
    {
        c
    } else {
        sniffed.to_string()
    }
}

const MAX_ASSET_DOC_BYTES: usize = 20 * 1024 * 1024; // 20 MB
const MAX_ASSET_IMAGE_BYTES: usize = 10 * 1024 * 1024; // 10 MB

fn normalize_asset_document_type(value: &str) -> Option<&'static str> {
    match value.trim() {
        "proof_of_title" => Some("proof_of_title"),
        "legal_basis" => Some("legal_basis"),
        "building_permit" => Some("building_permit"),
        "tax_npwp" => Some("tax_npwp"),
        "id_card" => Some("id_card"),
        "other" => Some("other"),
        _ => None,
    }
}

// ─── Asset Document Upload ─────────────────────────────────────

/// POST /api/developer/draft/:id/documents
///
/// Upload a legal/property document for an asset draft.
/// Accepts: multipart/form-data with `file` + `document_type` fields.
pub async fn upload_asset_document(
    jar: CookieJar,
    axum::extract::Path(asset_id): axum::extract::Path<Uuid>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    // Verify ownership or Admin role and ensure asset is not deleted
    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let is_admin = middleware::is_admin(&jar, &state.db).await;
    if owner_id != Some(user.id) && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized or asset deleted"})),
        )
            .into_response();
    }

    let bucket = state.config.gcs_bucket.clone();

    // Read multipart fields
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type = String::from("application/octet-stream");
    let mut document_type = String::from("other");
    let mut title = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "document_type" => {
                if let Ok(text) = field.text().await {
                    document_type = text;
                }
            }
            "title" => {
                if let Ok(text) = field.text().await {
                    title = text;
                }
            }
            "file" => {
                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }
                let fname = field.file_name().unwrap_or("document").to_string();
                if title.is_empty() {
                    title = fname;
                }
                let bytes: Vec<u8> = match field.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": "Failed to read file"})),
                        )
                            .into_response();
                    }
                };
                if bytes.len() > MAX_ASSET_DOC_BYTES {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(serde_json::json!({"error": "File must be ≤ 20 MB"})),
                    )
                        .into_response();
                }
                file_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No file field in request"})),
            )
                .into_response();
        }
    };
    document_type = match normalize_asset_document_type(&document_type) {
        Some(value) => value.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid document type"})),
            )
                .into_response();
        }
    };
    title = sanitize_text(&title);
    if title.trim().is_empty() {
        title = format!(
            "{}.{}",
            document_type,
            service::extension_for_doc_mime(&mime_type)
        );
    }
    if title.len() > 180 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Document title must be 180 characters or fewer"})),
        )
            .into_response();
    }

    // Magic-byte sniff: don't trust client-declared Content-Type.
    let sniffed = match sniff_mime(&file_bytes) {
        Some(m) => m,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Unsupported or unrecognized file format"})),
            )
                .into_response();
        }
    };
    if !mime_matches(&mime_type, sniffed) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "File content does not match declared type"})),
        )
            .into_response();
    }
    mime_type = canonical_asset_doc_mime(&mime_type, sniffed);

    if let Err(e) = service::validate_asset_doc_mime(&mime_type) {
        return e.into_response();
    }

    let file_size = file_bytes.len() as i64;
    let ext = service::extension_for_doc_mime(&mime_type);
    let file_id = Uuid::new_v4();
    let object_path = format!("properties/{}/documents/{}.{}", asset_id, file_id, ext);

    // Upload to GCS if configured, otherwise fall back to local filesystem.
    // Use a 15-second timeout so dev environments (where GCS write perms may be
    // unavailable) don't hang forever — they fall back to local storage.
    let file_url = if let Some(ref b) = bucket {
        let gcs_fut = service::upload_private(b, &object_path, file_bytes.clone(), &mime_type);
        match tokio::time::timeout(std::time::Duration::from_secs(15), gcs_fut).await {
            Ok(Ok(url)) => url,
            Ok(Err(e)) => {
                tracing::warn!("GCS document upload failed, falling back to local: {}", e);
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => {
                        tracing::error!("Local fallback also failed: {}", e2);
                        return e2.into_response();
                    }
                }
            }
            Err(_) => {
                tracing::warn!("GCS document upload timed out, falling back to local storage");
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e2) => {
                        tracing::error!("Local fallback failed: {}", e2);
                        return e2.into_response();
                    }
                }
            }
        }
    } else {
        match service::upload_local(&object_path, file_bytes).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Local file save failed: {}", e);
                return e.into_response();
            }
        }
    };

    // Insert into asset_documents
    let doc_id: Uuid = match sqlx::query_scalar(
        "INSERT INTO asset_documents (asset_id, document_type, title, file_url, file_size_bytes)
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(asset_id)
    .bind(&document_type)
    .bind(&title)
    .bind(&file_url)
    .bind(file_size)
    .fetch_one(&state.db)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to insert asset_document: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save document record"})),
            )
                .into_response();
        }
    };

    Json(serde_json::json!({
        "status": "success",
        "document_id": doc_id.to_string(),
        "file_url": file_url,
        "title": title,
        "document_type": document_type,
    }))
    .into_response()
}

// ─── Asset Image Upload ────────────────────────────────────────

/// POST /api/developer/draft/:id/images
///
/// Upload a property image for an asset draft.
/// Accepts: multipart/form-data with `file` + optional `sort_order` + optional `is_cover`.
pub async fn upload_asset_image(
    jar: CookieJar,
    axum::extract::Path(asset_id): axum::extract::Path<Uuid>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    // Verify ownership and ensure asset is not deleted
    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let is_admin = middleware::is_admin(&jar, &state.db).await;
    if owner_id != Some(user.id) && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized or asset deleted"})),
        )
            .into_response();
    }

    let bucket = state.config.gcs_bucket.clone();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type = String::from("application/octet-stream");
    let mut sort_order: i32 = 0;
    let mut is_cover = false;
    let mut alt_text = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "sort_order" => {
                if let Ok(text) = field.text().await {
                    sort_order = text.parse().unwrap_or(0);
                }
            }
            "is_cover" => {
                if let Ok(text) = field.text().await {
                    is_cover = text == "true" || text == "1";
                }
            }
            "alt_text" => {
                if let Ok(text) = field.text().await {
                    alt_text = text;
                }
            }
            "file" => {
                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }
                let bytes: Vec<u8> = match field.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": "Failed to read file"})),
                        )
                            .into_response();
                    }
                };
                if bytes.len() > MAX_ASSET_IMAGE_BYTES {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(serde_json::json!({"error": "Image must be ≤ 10 MB"})),
                    )
                        .into_response();
                }
                file_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "No file field in request"})),
            )
                .into_response();
        }
    };

    // Magic-byte sniff: don't trust client-declared Content-Type.
    let sniffed = match sniff_mime(&file_bytes) {
        Some(m) => m,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Unsupported or unrecognized image format"})),
            )
                .into_response();
        }
    };
    if !mime_matches(&mime_type, sniffed) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "File content does not match declared type"})),
        )
            .into_response();
    }
    mime_type = sniffed.to_string();

    if let Err(e) = service::validate_asset_image_mime(&mime_type) {
        return e.into_response();
    }

    let ext = service::extension_for_mime(&mime_type);
    let file_id = Uuid::new_v4();
    let object_path = format!("properties/{}/images/{}.{}", asset_id, file_id, ext);

    let image_url = if let Some(ref b) = bucket {
        // Try GCS first (with a timeout so it doesn't hang forever locally!)
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            service::upload_public(b, &object_path, file_bytes.clone(), &mime_type),
        )
        .await
        {
            Ok(Ok(url)) => url,
            Ok(Err(e)) => {
                tracing::warn!("Asset image upload failed: {}; falling back to local.", e);
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e) => {
                        tracing::error!("Local image save failed: {}", e);
                        return e.into_response();
                    }
                }
            }
            Err(_) => {
                tracing::warn!("GCS asset image upload timed out. Falling back to local storage.");
                match service::upload_local(&object_path, file_bytes).await {
                    Ok(url) => url,
                    Err(e) => {
                        tracing::error!("Local image save failed: {}", e);
                        return e.into_response();
                    }
                }
            }
        }
    } else {
        match service::upload_local(&object_path, file_bytes).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Local image save failed: {}", e);
                return e.into_response();
            }
        }
    };

    // Use a transaction to atomically unset old cover + insert new image
    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to begin image insert transaction: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Server error"})),
            )
                .into_response();
        }
    };

    // If this is marked as cover, unset any existing cover
    if is_cover {
        let _ = sqlx::query(
            "UPDATE asset_images SET is_cover = false WHERE asset_id = $1 AND is_cover = true",
        )
        .bind(asset_id)
        .execute(&mut *tx)
        .await;
    }

    let img_id: Uuid = match sqlx::query_scalar(
        "INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(asset_id)
    .bind(&image_url)
    .bind(if alt_text.is_empty() {
        None
    } else {
        Some(&alt_text)
    })
    .bind(sort_order)
    .bind(is_cover)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => {
            let _ = tx.commit().await;
            id
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!("Failed to insert asset_image: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save image record"})),
            )
                .into_response();
        }
    };

    Json(serde_json::json!({
        "status": "success",
        "image_id": img_id.to_string(),
        "image_url": image_url,
        "is_cover": is_cover,
    }))
    .into_response()
}

// ─── Delete Asset Document ─────────────────────────────────────

/// DELETE /api/developer/draft/:id/documents/:doc_id
pub async fn delete_asset_document(
    jar: CookieJar,
    axum::extract::Path((asset_id, doc_id)): axum::extract::Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let is_admin = middleware::is_admin(&jar, &state.db).await;
    if owner_id != Some(user.id) && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized or asset deleted"})),
        )
            .into_response();
    }

    let result = sqlx::query("DELETE FROM asset_documents WHERE id = $1 AND asset_id = $2")
        .bind(doc_id)
        .bind(asset_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Json(serde_json::json!({"status": "success", "message": "Document deleted"}))
                .into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Document not found"})),
        )
            .into_response(),
    }
}

// ─── Delete Asset Image ────────────────────────────────────────

/// DELETE /api/developer/draft/:id/images/:img_id
pub async fn delete_asset_image(
    jar: CookieJar,
    axum::extract::Path((asset_id, img_id)): axum::extract::Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let is_admin = middleware::is_admin(&jar, &state.db).await;
    if owner_id != Some(user.id) && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized or asset deleted"})),
        )
            .into_response();
    }

    let result = sqlx::query("DELETE FROM asset_images WHERE id = $1 AND asset_id = $2")
        .bind(img_id)
        .bind(asset_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Json(serde_json::json!({"status": "success", "message": "Image deleted"}))
                .into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Image not found"})),
        )
            .into_response(),
    }
}

// ─── Reorder Asset Images ────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct ImageOrderUpdate {
    id: Uuid,
    sort_order: i32,
    is_cover: bool,
}

/// PUT /api/developer/draft/:id/images/reorder
pub async fn reorder_asset_images(
    jar: CookieJar,
    axum::extract::Path(asset_id): axum::extract::Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<Vec<ImageOrderUpdate>>,
) -> axum::response::Response {
    if payload.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "At least one image is required"})),
        )
            .into_response();
    }

    let cover_count = payload.iter().filter(|img| img.is_cover).count();
    if cover_count != 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Exactly one image must be marked as cover"})),
        )
            .into_response();
    }

    let mut seen = HashSet::with_capacity(payload.len());
    for img in &payload {
        if img.sort_order < 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "sort_order must not be negative"})),
            )
                .into_response();
        }
        if !seen.insert(img.id) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Duplicate image id in reorder payload"})),
            )
                .into_response();
        }
    }

    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let is_admin = middleware::is_admin(&jar, &state.db).await;
    if owner_id != Some(user.id) && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized or asset deleted"})),
        )
            .into_response();
    }

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Transaction failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response();
        }
    };

    // First unset all covers to avoid duplicates.
    if let Err(e) = sqlx::query("UPDATE asset_images SET is_cover = false WHERE asset_id = $1")
        .bind(asset_id)
        .execute(&mut *tx)
        .await
    {
        tracing::error!("Failed to clear existing cover image: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save order"})),
        )
            .into_response();
    }

    // Update each image and fail if the client referenced an image outside this asset.
    for img in &payload {
        let result = sqlx::query(
            "UPDATE asset_images SET sort_order = $1, is_cover = $2 WHERE id = $3 AND asset_id = $4",
        )
        .bind(img.sort_order)
        .bind(img.is_cover)
        .bind(img.id)
        .bind(asset_id)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(updated) if updated.rows_affected() == 1 => {}
            Ok(_) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Image not found for this asset"})),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::error!("Failed to update asset image order: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Failed to save order"})),
                )
                    .into_response();
            }
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit reorder: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save order"})),
        )
            .into_response();
    }

    Json(serde_json::json!({"status": "success", "message": "Images reordered"})).into_response()
}

// ─── Download Asset Document ─────────────────────────────────────

/// GET /api/documents/:id/download
/// Securely retrieves a time-limited signed URL for a private asset document.
/// Developers/admins can access every asset document. Authenticated users can
/// access the investor-visible document subset for published assets.
pub async fn download_asset_document(
    jar: CookieJar,
    axum::extract::Path(doc_id): axum::extract::Path<Uuid>,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    // Find the document and the owner/visibility state of the asset.
    let doc_meta: Option<(String, Option<Uuid>, bool, bool, String)> = sqlx::query_as(
        "SELECT d.file_url, a.developer_user_id, COALESCE(d.is_investor_visible, false), COALESCE(a.published, false), COALESCE(d.title, '')
         FROM asset_documents d
         JOIN assets a ON d.asset_id = a.id
         WHERE d.id = $1",
    )
    .bind(doc_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (file_url, owner_id, is_investor_visible, asset_published, doc_title) = match doc_meta {
        Some(m) => m,
        None => return (StatusCode::NOT_FOUND, "Document not found").into_response(),
    };

    // Check authorization: must be the asset owner, an admin, or an
    // authenticated investor viewing a published asset's public diligence doc.
    let is_admin = middleware::is_admin(&jar, &state.db).await;
    let investor_visible = asset_published && is_investor_visible;
    if owner_id != Some(user.id) && !is_admin && !investor_visible {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized to view this document"})),
        )
            .into_response();
    }

    // Generate response. For private GCS objects we stream the bytes
    // through the API rather than redirecting to a signed URL — the
    // Cloud Run service account does not have `iam.serviceAccounts.signBlob`,
    // so signed-URL generation fails. Streaming uses standard ADC and only
    // requires `storage.objects.get`.
    if file_url.starts_with("gs://") {
        let parts: Vec<&str> = file_url.splitn(4, '/').collect();
        if parts.len() < 4 {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid storage URL format",
            )
                .into_response();
        }
        let bucket = parts[2];
        let object_path = parts[3];

        match service::download_object(bucket, object_path).await {
            Ok((content_type, data)) => {
                let mut headers = axum::http::HeaderMap::new();
                if let Ok(v) = content_type.parse() {
                    headers.insert(axum::http::header::CONTENT_TYPE, v);
                }
                headers.insert(
                    axum::http::header::HeaderName::from_static("x-content-type-options"),
                    "nosniff".parse().unwrap(),
                );
                headers.insert(
                    axum::http::header::CACHE_CONTROL,
                    "private, max-age=0, no-store".parse().unwrap(),
                );
                let raw_filename = if !doc_title.is_empty() {
                    doc_title.clone()
                } else {
                    object_path
                        .rsplit('/')
                        .next()
                        .unwrap_or("document")
                        .to_string()
                };
                let safe_filename = raw_filename.replace(['"', '\r', '\n'], "");
                if let Ok(v) = format!("attachment; filename=\"{}\"", safe_filename).parse() {
                    headers.insert(axum::http::header::CONTENT_DISPOSITION, v);
                }
                (headers, data).into_response()
            }
            Err(e) => {
                tracing::error!("Failed to download asset document {}: {}", doc_id, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch document",
                )
                    .into_response()
            }
        }
    } else {
        // Local fallback path e.g. /uploads/...
        axum::response::Redirect::temporary(&file_url).into_response()
    }
}

// ─── Proxy Endpoint for Public URLs ────────────────────────
/// GET /api/proxy/gcs/:bucket/*path
///
/// Downloads the object from GCS and streams the raw bytes to the client.
/// This avoids the need for `iam.serviceAccounts.signBlob` permission
/// that signed URL generation requires. Only `storage.objects.get` is needed.
///
/// Responses are cached for 1 hour via Cache-Control headers.
pub async fn proxy_gcs_image(
    State(state): State<AppState>,
    axum::extract::Path((bucket, object_path)): axum::extract::Path<(String, String)>,
) -> axum::response::Response {
    // Bucket allowlist: only serve the configured project bucket. Without this
    // check, the endpoint would act as an open GCS reverse proxy to any bucket
    // an attacker names in the URL.
    let allowed = state.config.gcs_bucket.as_deref();
    if allowed != Some(bucket.as_str()) {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }
    // Reject object path traversal or encoded separators that could escape
    // the intended prefix.
    if object_path.contains("..") || object_path.contains("//") {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    match super::service::download_object(&bucket, &object_path).await {
        Ok((content_type, data)) => {
            // Force download (attachment) for non-image types so browsers
            // cannot render hostile HTML/PDF inline. Images may still be
            // served inline for thumbnail rendering.
            let is_inline_safe = content_type.starts_with("image/");
            let mut headers = axum::http::HeaderMap::new();
            if let Ok(v) = content_type.parse() {
                headers.insert(axum::http::header::CONTENT_TYPE, v);
            }
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".parse().unwrap(),
            );
            headers.insert(
                axum::http::header::HeaderName::from_static("x-content-type-options"),
                "nosniff".parse().unwrap(),
            );
            if !is_inline_safe {
                let filename = object_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("file")
                    .replace('"', "");
                if let Ok(v) = format!("attachment; filename=\"{}\"", filename).parse() {
                    headers.insert(axum::http::header::CONTENT_DISPOSITION, v);
                }
            }
            (headers, data).into_response()
        }
        Err(e) => {
            tracing::error!("GCS proxy failed for {}/{}: {}", bucket, object_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Image failed to load").into_response()
        }
    }
}
