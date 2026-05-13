//! Developer-role extractor with asset-link enforcement (Villa-Returns P2).
//!
//! Layers on top of the existing `require_developer_api` helper in this module's
//! `routes.rs` to add the asset-link check: a Developer user may only write data
//! for villas they are explicitly linked to via `developer_asset_links`.

use crate::admin::extractors::ApiError;
use crate::auth::middleware;
use crate::auth::models::User;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use sqlx::PgPool;
use uuid::Uuid;

/// An authenticated Developer user. Use as a handler parameter for JSON endpoints
/// under `/api/developer/...`. Returns 401 if not logged in, 403 if not a developer.
///
/// For per-asset enforcement, call `require_asset_link(...)` after extraction.
pub struct DeveloperUser {
    pub user: User,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for DeveloperUser
where
    S: Send + Sync,
    sqlx::PgPool: axum::extract::FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = <sqlx::PgPool as axum::extract::FromRef<S>>::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);

        let user = middleware::get_current_user(&jar, &pool)
            .await
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        // Developer role check matches the existing developer/routes.rs helper.
        let is_developer: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_roles ur
                JOIN roles r ON r.id = ur.role_id
                WHERE ur.user_id = $1
                  AND r.name IN ('developer', 'asset_owner')
                  AND ur.is_active = TRUE
            )
            "#,
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if !is_developer {
            return Err(ApiError::Forbidden(
                "Developer access required".to_string(),
            ));
        }

        Ok(DeveloperUser { user })
    }
}

impl DeveloperUser {
    /// Enforce that this developer is currently authorised to submit data for `asset_id`.
    /// Returns 403 if no active link exists. Active = `effective_until IS NULL`.
    pub async fn require_asset_link(
        &self,
        pool: &PgPool,
        asset_id: Uuid,
    ) -> Result<(), ApiError> {
        let linked: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM developer_asset_links
                WHERE developer_user_id = $1
                  AND asset_id          = $2
                  AND effective_until   IS NULL
            )
            "#,
        )
        .bind(self.user.id)
        .bind(asset_id)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;

        if !linked {
            return Err(ApiError::Forbidden(
                "You are not authorised to submit data for this asset".to_string(),
            ));
        }
        Ok(())
    }
}
