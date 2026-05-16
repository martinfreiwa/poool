//! Generic idempotency-key helper for mutating HTTP endpoints.
//!
//! Pattern (Stripe-style):
//!
//! 1. Client sends `Idempotency-Key: <opaque-string>` header on a POST.
//! 2. Server `try_reserve()` performs an atomic INSERT ... ON CONFLICT DO
//!    NOTHING into `idempotency_keys`. First request wins → returns
//!    `Reserved(key)`. Later requests see a conflict → if the first finished
//!    we return the cached response (`CachedJson` / `CachedRedirect`),
//!    otherwise we return `InProgress` so the caller can 409.
//! 3. After the handler succeeds, the caller invokes `commit_*` to persist
//!    the response body. Storing the body unblocks future replays so the
//!    same idempotency key produces the same observable result.
//!
//! Keys live in `idempotency_keys` (migration 013). Rows expire after 24h
//! via `expires_at`; a periodic cleanup task can delete expired rows.
//!
//! The helper falls back gracefully when no `Idempotency-Key` header is
//! present (`NoKey`) — endpoints can still execute, but the caller is on
//! the hook for any double-submit races.

use axum::http::{HeaderMap, StatusCode};
use sqlx::PgPool;
use uuid::Uuid;

/// Outcome of the initial reservation attempt.
#[derive(Debug)]
pub enum Reservation {
    /// Client did not send an `Idempotency-Key` header. Handler should run
    /// normally but no replay protection is in effect.
    NoKey,
    /// Reservation succeeded — caller owns the key and must invoke
    /// `commit_json` / `commit_redirect` once the handler finishes.
    Reserved(String),
    /// A previous request with this key already returned a JSON response.
    /// Return it as-is to the client.
    CachedJson {
        status: StatusCode,
        body: serde_json::Value,
    },
    /// A previous request with this key already returned a redirect.
    CachedRedirect { location: String },
    /// Another request with this key is mid-flight. Suggest the client
    /// retry shortly. Maps cleanly to HTTP 409 Conflict.
    InProgress,
}

const HEADER_NAME: &str = "Idempotency-Key";

/// Read the `Idempotency-Key` header. Returns `None` if missing or empty.
pub fn header_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_NAME)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Attempt to reserve the key for `(user_id, path, method)`. Atomic via
/// `INSERT ... ON CONFLICT DO NOTHING` so two concurrent requests cannot
/// both win the reservation.
pub async fn try_reserve(
    pool: &PgPool,
    headers: &HeaderMap,
    user_id: Uuid,
    path: &str,
    method: &str,
) -> Reservation {
    let Some(key) = header_key(headers) else {
        return Reservation::NoKey;
    };

    let insert_res = sqlx::query(
        r#"INSERT INTO idempotency_keys (key, user_id, request_path, request_method)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (key, user_id) DO NOTHING"#,
    )
    .bind(&key)
    .bind(user_id)
    .bind(path)
    .bind(method)
    .execute(pool)
    .await;

    match insert_res {
        Ok(res) if res.rows_affected() == 1 => Reservation::Reserved(key),
        Ok(_) => {
            // Conflict: another request already claimed this key. Look up
            // its cached response — if missing, the first request is still
            // in-flight.
            let cached = sqlx::query_as::<_, (Option<i32>, Option<serde_json::Value>)>(
                r#"SELECT response_status, response_body
                     FROM idempotency_keys
                    WHERE key = $1 AND user_id = $2"#,
            )
            .bind(&key)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

            match cached {
                Some((Some(status), Some(body))) => {
                    // Redirect responses are stored with a sentinel marker
                    // so the helper can rebuild the redirect headers.
                    if let Some(loc) = body.get("__redirect").and_then(|v| v.as_str()) {
                        Reservation::CachedRedirect {
                            location: loc.to_string(),
                        }
                    } else {
                        Reservation::CachedJson {
                            status: StatusCode::from_u16(status as u16).unwrap_or(StatusCode::OK),
                            body,
                        }
                    }
                }
                _ => Reservation::InProgress,
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Idempotency reserve failed, allowing handler to proceed");
            Reservation::NoKey
        }
    }
}

/// Persist a JSON response body so future requests with the same key get
/// the same answer.
pub async fn commit_json(pool: &PgPool, key: &str, status: StatusCode, body: &serde_json::Value) {
    let _ = sqlx::query(
        r#"UPDATE idempotency_keys
              SET response_status = $1, response_body = $2
            WHERE key = $3"#,
    )
    .bind(status.as_u16() as i32)
    .bind(body)
    .bind(key)
    .execute(pool)
    .await;
}

/// Persist a redirect response so future requests follow the same
/// destination. Encoded as `{"__redirect": "..."}` in `response_body`.
pub async fn commit_redirect(pool: &PgPool, key: &str, location: &str) {
    let body = serde_json::json!({ "__redirect": location });
    let _ = sqlx::query(
        r#"UPDATE idempotency_keys
              SET response_status = $1, response_body = $2
            WHERE key = $3"#,
    )
    .bind(303_i32)
    .bind(&body)
    .bind(key)
    .execute(pool)
    .await;
}

/// Drop a reservation when the handler fails before producing a final
/// response. Without this, a transient error would lock the user out of
/// retrying for 24h.
pub async fn release(pool: &PgPool, key: &str) {
    let _ = sqlx::query("DELETE FROM idempotency_keys WHERE key = $1 AND response_status IS NULL")
        .bind(key)
        .execute(pool)
        .await;
}
