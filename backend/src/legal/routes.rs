use crate::auth::routes::AppState;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::cookie::CookieJar;

/// GET /terms  Terms and Conditions.
pub async fn page_terms(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "terms.html",
        serde_json::json!({}),
    )
    .await
}

/// GET /privacy-policy  Privacy Policy.
pub async fn page_privacy_policy(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "privacy-policy.html",
        serde_json::json!({}),
    )
    .await
}

/// GET /currency-policy  Currency Policy.
pub async fn page_currency_policy(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "currency-policy.html",
        serde_json::json!({}),
    )
    .await
}

/// GET /cookies  Cookies Policy.
pub async fn page_cookies(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "cookies.html",
        serde_json::json!({}),
    )
    .await
}

/// GET /imprint  Legal imprint / Impressum.
pub async fn page_imprint(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "imprint.html",
        serde_json::json!({}),
    )
    .await
}

/// GET /gdpr-data-request  GDPR data request form.
pub async fn page_gdpr_data_request(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "gdpr-data-request.html",
        serde_json::json!({}),
    )
    .await
}

/// GET /aml-kyc-policy  AML/KYC policy.
pub async fn page_aml_kyc_policy(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_public_with_context(
        jar,
        &state,
        "aml-kyc-policy.html",
        serde_json::json!({}),
    )
    .await
}
