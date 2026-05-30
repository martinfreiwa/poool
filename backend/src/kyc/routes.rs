use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;

use super::models::{KycInitiateRequest, KycSubmitRequest};
use super::provider::KycProvider;
use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;
use crate::error::AppError;

async fn require_user_id(
    jar: &CookieJar,
    state: &AppState,
) -> Result<uuid::Uuid, axum::response::Response> {
    match middleware::get_current_user(jar, &state.db).await {
        Some(user) => Ok(user.id),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Not authenticated"})),
        )
            .into_response()),
    }
}

async fn require_kyc_rate_limit(
    state: &AppState,
    user_id: uuid::Uuid,
    action: &str,
) -> Result<(), axum::response::Response> {
    match state
        .auth_rate_limiter
        .check(&format!("kyc:{}:{}", action, user_id))
        .await
    {
        Ok(_) => Ok(()),
        Err(retry_after) => Err(AppError::RateLimited(retry_after).into_response()),
    }
}

/// GET /api/kyc/status — Return the current user's KYC status.
pub async fn get_status(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_kyc_status(&state.db, user_id).await {
        Ok(status) => Json(status).into_response(),
        Err(e) => {
            tracing::error!("Failed to get KYC status for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to check status"})),
            )
                .into_response()
        }
    }
}

/// POST /api/kyc/submit — Legacy manual submission (backward-compat).
pub async fn submit(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<KycSubmitRequest>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    if let Err(resp) = require_kyc_rate_limit(&state, user_id, "submit").await {
        return resp;
    }

    match service::submit_kyc(&state.db, user_id, payload).await {
        Ok(_) => Json(serde_json::json!({"status": "in_review"})).into_response(),
        Err(e) => {
            tracing::error!("Failed to submit KYC for {}: {}", user_id, e.detail());
            e.into_response()
        }
    }
}

/// POST /api/kyc/initiate — Create a KYC session with the active provider.
///
/// Returns a `verification_url` for redirect-based providers (Didit, Sumsub).
/// For manual providers, the URL is empty and the frontend should show its own form.
pub async fn initiate(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<KycInitiateRequest>,
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

    let provider = service::build_provider();
    let callback_url = format!("{}/kyc", state.config.base_url);

    if let Err(resp) = require_kyc_rate_limit(&state, user.id, "initiate").await {
        return resp;
    }

    match service::initiate_kyc(
        &state.db,
        provider.as_ref(),
        user.id,
        Some(&user.email),
        &callback_url,
        payload.document_type.as_deref(),
    )
    .await
    {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => {
            tracing::error!("Failed to initiate KYC for {}: {}", user.id, e.detail());
            e.into_response()
        }
    }
}

/// POST /api/webhooks/kyc/didit — Webhook endpoint for Didit status updates.
///
/// Didit sends POST requests with:
/// - Body: JSON with session_id, status, decision, etc.
/// - Header: X-Signature-Simple (HMAC-SHA256 of "{timestamp}:{session_id}:{status}:{webhook_type}")
///
/// This endpoint is unauthenticated (webhooks come from Didit servers)
/// but validates the HMAC signature.
pub async fn didit_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    let provider = match super::didit::DiditConfig::from_env() {
        Some(cfg) => super::didit::DiditProvider::new(cfg),
        None => {
            tracing::error!("Didit webhook received but provider not configured (API key or webhook secret missing)");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "KYC provider not configured"})),
            )
                .into_response();
        }
    };

    // Try multiple signature headers in order of preference (V2 is recommended)
    let signature = headers
        .get("x-signature-v2")
        .or_else(|| headers.get("x-signature-simple"))
        .or_else(|| headers.get("x-signature"))
        .and_then(|v| v.to_str().ok());

    match provider.process_webhook(&body, signature).await {
        Ok(update) => {
            // Replay guard: the signature alone is a nonce — Didit signs
            // deterministic payloads, so a resent webhook carries the same
            // signature. SETNX the signature with a 10-minute TTL (covers the
            // 5-minute freshness window on both sides). If Redis is absent
            // we degrade open with a warning log.
            if let (Some(sig), Some(redis_pool)) = (signature, state.redis.as_ref()) {
                if let Ok(mut conn) = redis_pool.get().await {
                    let key = format!("kyc_webhook_nonce:{}", sig);
                    let res: Result<Option<String>, _> = redis::cmd("SET")
                        .arg(&key)
                        .arg("1")
                        .arg("NX")
                        .arg("EX")
                        .arg(600)
                        .query_async(&mut *conn)
                        .await;
                    if matches!(res, Ok(None)) {
                        tracing::warn!("Didit webhook replay blocked (nonce already seen)");
                        return (StatusCode::OK, Json(serde_json::json!({"ok": true})))
                            .into_response();
                    }
                }
            } else if state.redis.is_none() {
                tracing::warn!("KYC webhook processed without replay guard (Redis unavailable)");
            }

            let provider_name = provider.name();
            if let Err(e) = service::process_webhook_update(&state.db, update, provider_name).await
            {
                tracing::error!("Failed to process Didit webhook update: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Processing failed"})),
                )
                    .into_response();
            }
            (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
        }
        Err(e) => {
            tracing::error!("Didit webhook processing error: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid webhook"})),
            )
                .into_response()
        }
    }
}

/// GET /api/kyc/provider — Return info about the active KYC provider.
///
/// Used by the frontend to decide whether to show the manual form
/// or redirect to an external verification flow.
pub async fn get_provider_info(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let _user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let provider = service::build_provider();
    Json(serde_json::json!({
        "provider": provider.name(),
        "supports_redirect": provider.name() != "manual",
    }))
    .into_response()
}

/// GET /kyc — Render the identity verification page.
pub async fn page_kyc(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "kyc.html").await
}

// ═══════════════════════════════════════════════════════════════
// ── SIWE WALLET BINDING ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════
//
// Two-step flow for binding a sovereign Ethereum wallet to a POOOL
// account:
//
//   1. Frontend calls `POST /api/kyc/wallet/challenge` with the address
//      the user wants to bind. Backend stores a fresh nonce (random,
//      single-use, 5min TTL) keyed on (user_id, address) and returns
//      the message text to sign.
//
//   2. User signs that message in their wallet (MetaMask / WalletConnect
//      personal_sign). Frontend POSTs `{address, signature}` to
//      `POST /api/kyc/wallet/bind`. Backend verifies the EIP-191
//      signature, confirms the recovered address matches, consumes the
//      nonce, and writes `users.chain_wallet_address`.
//
// The KYC whitelist worker then picks up `(KYC approved AND wallet
// bound AND not yet whitelisted)` and batches them on-chain. Tokens
// will only ever be transferred to addresses the user has proven
// ownership of.

#[derive(serde::Deserialize)]
pub struct WalletChallengeRequest {
    pub address: String,
}

#[derive(serde::Deserialize)]
pub struct WalletBindRequest {
    pub address: String,
    pub signature: String,
}

fn normalize_address(addr: &str) -> Result<String, Box<axum::response::Response>> {
    let clean = addr.strip_prefix("0x").unwrap_or(addr).to_lowercase();
    if clean.len() != 40 || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Box::new(
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid Ethereum address"})),
            )
                .into_response(),
        ));
    }
    Ok(format!("0x{}", clean))
}

fn build_siwe_message(address: &str, nonce: &str) -> String {
    // Plain personal_sign payload — readable, no domain spoofing risk
    // because we don't follow the full EIP-4361 spec (overkill for our
    // needs). Frontend renders this verbatim; what the user signs is
    // what we verify.
    format!(
        "POOOL.app wallet binding\n\nAddress: {}\nNonce: {}\n\nBy signing you confirm ownership of this wallet for your POOOL account.",
        address, nonce
    )
}

/// POST /api/kyc/wallet/challenge
pub async fn wallet_challenge(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(req): Json<WalletChallengeRequest>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(r) => return r,
    };
    if let Err(r) = require_kyc_rate_limit(&state, user_id, "wallet_challenge").await {
        return r;
    }

    let address = match normalize_address(&req.address) {
        Ok(a) => a,
        Err(r) => return *r,
    };

    // Generate a 32-byte random nonce, hex-encoded.
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let nonce = hex::encode(bytes);

    // Persist the challenge with a 5-minute expiry. Replace any existing
    // open challenge for this (user, address) pair so re-issuing is fine.
    let res = sqlx::query(
        r#"INSERT INTO wallet_binding_challenges (user_id, address, nonce, expires_at)
           VALUES ($1, $2, $3, NOW() + INTERVAL '5 minutes')
           ON CONFLICT (user_id, address)
           DO UPDATE SET nonce = EXCLUDED.nonce, expires_at = EXCLUDED.expires_at, consumed_at = NULL"#,
    )
    .bind(user_id)
    .bind(&address)
    .bind(&nonce)
    .execute(&state.db)
    .await;
    if let Err(e) = res {
        tracing::error!("wallet_challenge persist failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Could not generate challenge"})),
        )
            .into_response();
    }

    let message = build_siwe_message(&address, &nonce);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "address": address,
            "nonce": nonce,
            "message": message,
        })),
    )
        .into_response()
}

/// POST /api/kyc/wallet/bind
pub async fn wallet_bind(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(req): Json<WalletBindRequest>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(r) => return r,
    };
    if let Err(r) = require_kyc_rate_limit(&state, user_id, "wallet_bind").await {
        return r;
    }

    let address = match normalize_address(&req.address) {
        Ok(a) => a,
        Err(r) => return *r,
    };

    // Look up the active challenge.
    let row: Option<(String,)> = match sqlx::query_as(
        r#"SELECT nonce
           FROM wallet_binding_challenges
           WHERE user_id = $1 AND address = $2
             AND consumed_at IS NULL
             AND expires_at > NOW()"#,
    )
    .bind(user_id)
    .bind(&address)
    .fetch_optional(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("wallet_bind challenge lookup failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "DB error"})),
            )
                .into_response();
        }
    };
    let (nonce,) = match row {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "No active challenge — request /wallet/challenge first or check expiry"
                })),
            )
                .into_response();
        }
    };

    // Verify the EIP-191 signature.
    let message = build_siwe_message(&address, &nonce);
    if let Err(e) = crate::blockchain::signing::verify_personal_sign(
        message.as_bytes(),
        &req.signature,
        &address,
    ) {
        tracing::warn!(
            "wallet_bind signature verification failed: user={} addr={} err={}",
            user_id,
            address,
            e
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid signature"})),
        )
            .into_response();
    }

    // Bind in a tx so the consumed_at flip and the user update commit
    // atomically. If two concurrent binds race for the same address (a
    // user re-clicking), the FOR UPDATE prevents double-application.
    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("wallet_bind tx begin failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "DB error"})),
            )
                .into_response();
        }
    };

    let consumed = sqlx::query(
        r#"UPDATE wallet_binding_challenges
           SET consumed_at = NOW()
           WHERE user_id = $1 AND address = $2
             AND consumed_at IS NULL
             AND expires_at > NOW()"#,
    )
    .bind(user_id)
    .bind(&address)
    .execute(&mut *tx)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0);
    if consumed != 1 {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "Challenge already consumed"})),
        )
            .into_response();
    }

    // Reject re-binding to a DIFFERENT address than what's already set.
    // Allow re-binding the SAME address (idempotent — covers retries
    // after a network hiccup mid-bind).
    let update_result = match sqlx::query(
        r#"UPDATE users SET chain_wallet_address = $1, chain_whitelisted_at = NULL
           WHERE id = $2
             AND (chain_wallet_address IS NULL OR chain_wallet_address = $1)"#,
    )
    .bind(&address)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("wallet_bind user update failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "DB error"})),
            )
                .into_response();
        }
    };

    // Zero rows updated = the WHERE NULL-or-equal guard rejected the bind
    // because the user already has a different `chain_wallet_address` set.
    // Without this check we'd silently 200-OK a no-op while telling the
    // client (and the audit row below) that the new address was bound.
    if update_result.rows_affected() == 0 {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "Account already linked to a different wallet address"
            })),
        )
            .into_response();
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("wallet_bind commit failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "DB error"})),
        )
            .into_response();
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (user_id, action, details, ip_address, created_at)
           VALUES ($1, 'wallet_bound', $2, '0.0.0.0', NOW())"#,
    )
    .bind(user_id)
    .bind(serde_json::json!({"address": &address}).to_string())
    .execute(&state.db)
    .await;

    // User now has a wallet — promote any of their already-completed
    // primary order items from "no destination address" to settlement-
    // eligible. Best-effort; logged on failure.
    match crate::blockchain::primary_settlement::mark_user_eligible_after_wallet_bind(
        &state.db, user_id,
    )
    .await
    {
        Ok(0) => {}
        Ok(n) => tracing::info!(
            "wallet_bind: marked {} order items eligible for primary settlement (user={})",
            n,
            user_id
        ),
        Err(e) => tracing::error!(
            "wallet_bind: failed to lift order items for user {}: {}",
            user_id,
            e
        ),
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"ok": true, "address": address})),
    )
        .into_response()
}
