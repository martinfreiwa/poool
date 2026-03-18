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
use uuid::Uuid;

use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;

const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024; // 5 MB
const MAX_KYC_BYTES: usize = 10 * 1024 * 1024; // 10 MB

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
                .into_response()
        }
    };

    let bucket = match &state.config.gcs_bucket {
        Some(b) => b.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "File storage is not configured"})),
            )
                .into_response()
        }
    };

    // Read multipart field
    let (file_bytes, mime_type) = match read_multipart_file(&mut multipart, MAX_AVATAR_BYTES).await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    };

    // Validate MIME
    if let Err(e) = service::validate_image_mime(&mime_type) {
        return e.into_response();
    }

    let ext = service::extension_for_mime(&mime_type);
    let object_path = format!("avatars/{}/{}.{}", user.id, Uuid::new_v4(), ext);

    // Upload to GCS (public – served directly)
    let avatar_url =
        match service::upload_public(&bucket, &object_path, file_bytes, &mime_type).await {
            Ok(url) => url,
            Err(e) => {
                tracing::error!("Avatar upload failed for {}: {}", user.id, e);
                return e.into_response();
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

    tracing::info!("Avatar updated for user {} → {}", user.id, avatar_url);

    Json(serde_json::json!({
        "status": "success",
        "avatar_url": avatar_url,
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
                .into_response()
        }
    };

    let bucket = match &state.config.gcs_bucket {
        Some(b) => b.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "File storage is not configured"})),
            )
                .into_response()
        }
    };

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

            let bytes: Vec<u8> = match field.bytes().await {
                Ok(b) => b.to_vec(),
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "Failed to read uploaded file"})),
                    )
                        .into_response()
                }
            };

            if bytes.len() > MAX_KYC_BYTES {
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(serde_json::json!({"error": "File must be ≤ 10 MB"})),
                )
                    .into_response();
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
                .into_response()
        }
    };

    // Validate MIME
    if let Err(e) = service::validate_kyc_mime(&mime_type) {
        return e.into_response();
    }

    // Validate document_type
    let allowed_doc_types = [
        "passport",
        "national_id",
        "driving_licence",
        "proof_of_address",
        "other",
    ];
    if !allowed_doc_types.contains(&document_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid document_type"})),
        )
            .into_response();
    }

    let ext = service::extension_for_mime(&mime_type);
    let file_id = Uuid::new_v4();
    let object_path = format!("kyc/{}/{}.{}", user.id, file_id, ext);

    // Upload to GCS (private)
    let gcs_path =
        match service::upload_private(&bucket, &object_path, file_bytes, &mime_type).await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("KYC upload failed for {}: {}", user.id, e);
                return e.into_response();
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
    .fetch_one(&state.db)
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to insert kyc_document for {}: {}", user.id, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Upload succeeded but failed to save record"})),
            )
                .into_response();
        }
    };

    // Audit log
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'kyc_document.uploaded', 'kyc_documents', $2, $3)"#,
    )
    .bind(user.id)
    .bind(doc_id)
    .bind(serde_json::json!({
        "document_type": document_type,
        "gcs_path": gcs_path,
    }))
    .execute(&state.db)
    .await;

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
/// Returns `(bytes, mime_type)`.
async fn read_multipart_file(
    multipart: &mut Multipart,
    max_bytes: usize,
) -> Result<(Vec<u8>, String), String> {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }

        let mime = field
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

        return Ok((bytes, mime));
    }

    Err("No `file` field found in request".to_string())
}

const MAX_ASSET_DOC_BYTES: usize = 20 * 1024 * 1024; // 20 MB
const MAX_ASSET_IMAGE_BYTES: usize = 10 * 1024 * 1024; // 10 MB

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
                .into_response()
        }
    };

    // Verify ownership or Admin role and ensure asset is not deleted
    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL")
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
                            .into_response()
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
                .into_response()
        }
    };

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
                .into_response()
        }
    };

    // Verify ownership and ensure asset is not deleted
    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL")
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
                            .into_response()
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
                .into_response()
        }
    };

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
                .into_response()
        }
    };

    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL")
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
                .into_response()
        }
    };

    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL")
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
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response()
        }
    };

    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL")
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

    // First unset all covers to avoid duplicates
    let _ = sqlx::query("UPDATE asset_images SET is_cover = false WHERE asset_id = $1")
        .bind(asset_id)
        .execute(&mut *tx)
        .await;

    // Update each image
    for img in payload {
        let _ = sqlx::query(
            "UPDATE asset_images SET sort_order = $1, is_cover = $2 WHERE id = $3 AND asset_id = $4",
        )
        .bind(img.sort_order)
        .bind(img.is_cover)
        .bind(img.id)
        .bind(asset_id)
        .execute(&mut *tx)
        .await;
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
/// Only the developer who owns the asset or an admin can access this.
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
                .into_response()
        }
    };

    // Find the document and the owner of the asset
    let doc_meta: Option<(String, Uuid)> = sqlx::query_as(
        "SELECT d.file_url, a.developer_user_id 
         FROM asset_documents d 
         JOIN assets a ON d.asset_id = a.id 
         WHERE d.id = $1",
    )
    .bind(doc_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (file_url, owner_id) = match doc_meta {
        Some(m) => m,
        None => return (StatusCode::NOT_FOUND, "Document not found").into_response(),
    };

    // Check authorization: must be the asset owner OR an admin
    let is_admin = middleware::is_admin(&jar, &state.db).await;
    if owner_id != user.id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized to view this document"})),
        )
            .into_response();
    }

    // Generate response (Redirect for signed URL or static path)
    if file_url.starts_with("gs://") {
        // e.g. gs://bucket-name/properties/uuid/documents/uuid.pdf
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

        match service::generate_signed_url(bucket, object_path, 15).await {
            Ok(signed_url) => axum::response::Redirect::temporary(&signed_url).into_response(),
            Err(e) => {
                tracing::error!("Failed to generate signed url: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to generate download link",
                )
                    .into_response()
            }
        }
    } else {
        // Local fallback path e.g. /uploads/...
        axum::response::Redirect::temporary(&file_url).into_response()
    }
}
