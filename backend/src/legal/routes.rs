use crate::auth::routes::AppState;
use axum::{extract::State, response::IntoResponse};
use axum_extra::extract::cookie::CookieJar;

/// GET /terms  Terms and Conditions (protected).
pub async fn page_terms(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "terms.html").await
}

/// GET /privacy-policy  Privacy Policy (protected).
pub async fn page_privacy_policy(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "privacy-policy.html").await
}

/// GET /currency-policy  Currency Policy (protected).
pub async fn page_currency_policy(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "currency-policy.html").await
}

/// GET /cookies  Cookies Policy (protected).
pub async fn page_cookies(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "cookies.html").await
}
