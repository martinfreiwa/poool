pub mod routes;
/// Google Cloud Storage integration for POOOL.
///
/// # Authentication
/// On Cloud Run: automatic via the instance's attached service account (Workload Identity / ADC).
/// Locally: via `gcloud auth application-default login` or `GOOGLE_APPLICATION_CREDENTIALS`.
///
/// # Bucket layout
/// ```
/// gs://{bucket}/
///   avatars/{user_id}/{filename}          ← profile photos (public-readable)
///   kyc/{user_id}/{filename}              ← KYC identity docs (private, signed URLs only)
///   properties/{asset_id}/{filename}      ← property images & documents
/// ```
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

/// Compose all upload-domain routes into a single mountable Router.
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        .route("/api/upload/avatar", post(upload_avatar))
        .route("/api/upload/kyc", post(upload_kyc_document))
        .route("/api/upload/post-image", post(upload_post_image))
        // Asset file management (developer draft flow)
        .route(
            "/api/developer/draft/:id/documents",
            post(upload_asset_document),
        )
        .route("/api/developer/draft/:id/images", post(upload_asset_image))
        .route(
            "/api/developer/draft/:id/documents/:doc_id",
            delete(delete_asset_document),
        )
        .route(
            "/api/developer/draft/:id/images/:img_id",
            delete(delete_asset_image),
        )
        .route(
            "/api/developer/draft/:id/images/reorder",
            put(reorder_asset_images),
        )
        // Secure document viewing
        .route("/api/documents/:id/download", get(download_asset_document))
        // Proxy public images (bypasses 403 blocks)
        .route("/api/proxy/gcs/:bucket/*path", get(proxy_gcs_image))
}
