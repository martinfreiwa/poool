//! Resend webhook receiver — wires provider events back into
//! `email_logs` and the `email_suppressions` list.
//!
//! Resend posts events for `email.sent`, `email.delivered`,
//! `email.delivery_delayed`, `email.bounced`, `email.complained`,
//! `email.opened`, and `email.clicked`. We map each one to a row update
//! on `email_logs` keyed by `provider_id` (Resend's email id) so
//! delivery metrics in the admin dashboard track reality instead of
//! frozen-at-send-time snapshots.
//!
//! Signature verification uses the Svix scheme (Resend's transport):
//!   `svix-id`, `svix-timestamp`, `svix-signature`
//! HMAC-SHA256 of `<id>.<timestamp>.<body>` keyed by the webhook secret,
//! base64-encoded, prefixed with `v1,`. If `RESEND_WEBHOOK_SECRET` is
//! unset (dev mode) the handler logs a warning and accepts unsigned
//! posts so localhost workflows don't require the live secret.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;

use crate::auth::routes::AppState;

/// Process one Resend webhook event. Always returns 200 to the
/// provider unless the signature is explicitly bad — we don't want
/// Resend retrying forever on a transient DB hiccup.
pub async fn handle_resend_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Signature gate. In production the secret MUST be set; in dev the
    // handler accepts unsigned bodies so a curl smoke-test works.
    let secret = std::env::var("RESEND_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.trim().is_empty());

    if let Some(secret) = secret.as_deref() {
        if !verify_svix_signature(&headers, &body, secret) {
            tracing::warn!("Resend webhook: signature verification failed");
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error":"bad signature"})),
            )
                .into_response();
        }
    } else {
        let in_prod = std::env::var("APP_ENV")
            .map(|e| e.eq_ignore_ascii_case("production"))
            .unwrap_or(false);
        if in_prod {
            tracing::error!(
                "Resend webhook: RESEND_WEBHOOK_SECRET unset in production — \
                 rejecting unsigned event to avoid forgery."
            );
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({"error":"webhook secret not configured"})),
            )
                .into_response();
        }
        tracing::warn!(
            "Resend webhook: RESEND_WEBHOOK_SECRET unset — accepting unsigned event (dev mode)."
        );
    }

    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({"error":"invalid json"})),
            )
                .into_response()
        }
    };

    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let data = event
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    if let Err(err) = process_event(&state.db, event_type, &data).await {
        // Log + sentry, but ACK to Resend. They'll retry transient errors
        // for us; permanent ones (schema-shape change) we'd rather see
        // in observability than receive a deluge of webhook retries for.
        tracing::error!("Resend webhook process failed: {}", err);
        sentry::capture_message(
            &format!("Resend webhook process failed: {err}"),
            sentry::Level::Error,
        );
    }

    axum::Json(serde_json::json!({"status":"ok"})).into_response()
}

/// Update `email_logs` + `email_suppressions` for a single event.
async fn process_event(
    pool: &PgPool,
    event_type: &str,
    data: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let email_id = data
        .get("email_id")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("id").and_then(|v| v.as_str()));
    let recipient = data
        .get("to")
        .and_then(|v| {
            // Resend sends `to` as either a string or an array of strings.
            v.as_str()
                .map(str::to_owned)
                .or_else(|| v.as_array()?.first()?.as_str().map(str::to_owned))
        })
        .unwrap_or_default();

    let new_status = match event_type {
        "email.sent" => Some("sent"),
        "email.delivered" => Some("delivered"),
        "email.opened" => Some("opened"),
        "email.clicked" => Some("clicked"),
        "email.bounced" => Some("bounced"),
        "email.complained" => Some("spam_complaint"),
        "email.delivery_delayed" => None, // No state change — still in flight.
        _ => None,
    };

    if let (Some(status), Some(eid)) = (new_status, email_id) {
        // Targeted update on email_logs.provider_id (indexed in migration 185).
        let timestamp_col = match status {
            "delivered" => "delivered_at",
            "opened" => "opened_at",
            "clicked" => "clicked_at",
            _ => "",
        };

        let sql = if timestamp_col.is_empty() {
            "UPDATE email_logs SET status = $1 WHERE provider_id = $2".to_string()
        } else {
            format!(
                "UPDATE email_logs SET status = $1, {timestamp_col} = NOW() \
                 WHERE provider_id = $2"
            )
        };

        sqlx::query(&sql)
            .bind(status)
            .bind(eid)
            .execute(pool)
            .await?;
    }

    // Bounce → suppression list (hard bounces + spam complaints).
    let suppression_reason = match event_type {
        "email.bounced" => {
            // Resend distinguishes hard vs soft inside `bounce.type` —
            // only hard bounces go on the suppression list. Anything we
            // can't classify defaults to hard so we don't keep mailing
            // a confirmed bad address.
            let bounce_type = data
                .get("bounce")
                .and_then(|b| b.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("hard");
            if bounce_type.eq_ignore_ascii_case("soft") {
                Some("soft_bounce")
            } else {
                Some("hard_bounce")
            }
        }
        "email.complained" => Some("spam_complaint"),
        _ => None,
    };

    if let Some(reason) = suppression_reason {
        if !recipient.is_empty() {
            // Only persist suppressions worth honouring at send time —
            // soft bounces are tracked separately as a counter but do
            // not block sends.
            let blocks_sends = matches!(reason, "hard_bounce" | "spam_complaint");
            let event_id = email_id.unwrap_or("");

            if blocks_sends {
                // Insert-or-bump. The partial unique index on LOWER(email)
                // WHERE cleared_at IS NULL means we ON CONFLICT on that
                // expression — Postgres doesn't support that directly, so
                // we do an explicit upsert via SELECT then INSERT.
                let existing = sqlx::query_scalar::<_, uuid::Uuid>(
                    "SELECT id FROM email_suppressions
                      WHERE LOWER(email) = LOWER($1) AND cleared_at IS NULL
                      LIMIT 1",
                )
                .bind(&recipient)
                .fetch_optional(pool)
                .await?;

                if let Some(id) = existing {
                    sqlx::query(
                        "UPDATE email_suppressions
                            SET bounce_count = bounce_count + 1,
                                last_event_at = NOW(),
                                provider_event_id = COALESCE(NULLIF($2,''), provider_event_id),
                                reason = CASE
                                    WHEN reason = 'spam_complaint' THEN reason
                                    ELSE $3
                                END
                          WHERE id = $1",
                    )
                    .bind(id)
                    .bind(event_id)
                    .bind(reason)
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        "INSERT INTO email_suppressions
                             (email, reason, bounce_count, provider_event_id)
                         VALUES ($1, $2, 1, NULLIF($3,''))",
                    )
                    .bind(&recipient)
                    .bind(reason)
                    .bind(event_id)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

/// Maximum allowed clock-skew between Resend's webhook timestamp and our
/// receive time. Svix's reference implementation uses 5 minutes — matches
/// Stripe and protects against captured-and-replayed events.
const SVIX_TIMESTAMP_TOLERANCE_SECS: i64 = 5 * 60;

/// Verify Resend's Svix-style HMAC signature AND timestamp freshness.
///
/// Format: header `svix-signature: v1,<base64>` (may contain multiple
/// space-separated entries; we accept if ANY entry verifies). Payload
/// signed: `<svix-id>.<svix-timestamp>.<raw_body>`.
///
/// The timestamp is compared against `now` (or `now_override` in tests)
/// with a ±5min window. Without this check, an attacker who once
/// captured a valid webhook could replay it forever.
fn verify_svix_signature(headers: &HeaderMap, body: &[u8], secret: &str) -> bool {
    verify_svix_signature_at(headers, body, secret, current_unix_ts())
}

fn current_unix_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn verify_svix_signature_at(headers: &HeaderMap, body: &[u8], secret: &str, now: i64) -> bool {
    let id = match headers.get("svix-id").and_then(|h| h.to_str().ok()) {
        Some(v) => v,
        None => return false,
    };
    let ts = match headers.get("svix-timestamp").and_then(|h| h.to_str().ok()) {
        Some(v) => v,
        None => return false,
    };
    // Reject events outside the tolerance window — both stale (replay)
    // and future-dated (forged timestamp).
    if let Ok(ts_int) = ts.parse::<i64>() {
        if (now - ts_int).abs() > SVIX_TIMESTAMP_TOLERANCE_SECS {
            return false;
        }
    } else {
        return false;
    }
    let sig_header = match headers.get("svix-signature").and_then(|h| h.to_str().ok()) {
        Some(v) => v,
        None => return false,
    };

    // Decode the secret — Resend webhook secrets are prefixed `whsec_`
    // followed by base64-encoded raw bytes. Strip the prefix and decode.
    let key_bytes = match secret.strip_prefix("whsec_") {
        Some(b64) => {
            use base64::Engine;
            match base64::engine::general_purpose::STANDARD.decode(b64) {
                Ok(b) => b,
                Err(_) => return false,
            }
        }
        // Allow raw secret for local testing convenience.
        None => secret.as_bytes().to_vec(),
    };

    let signed_payload = format!(
        "{id}.{ts}.{body}",
        body = std::str::from_utf8(body).unwrap_or("")
    );
    let mut mac = match Hmac::<Sha256>::new_from_slice(&key_bytes) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();
    use base64::Engine;
    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(expected);

    // Header may contain `v1,<sig> v1,<sig2>` — verify any entry.
    sig_header.split_whitespace().any(|entry| {
        if let Some(sig) = entry.strip_prefix("v1,") {
            // Constant-time compare via length-equal check, then xor sum.
            sig.len() == expected_b64.len()
                && sig
                    .as_bytes()
                    .iter()
                    .zip(expected_b64.as_bytes().iter())
                    .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                    == 0
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn make_sig(secret_raw: &[u8], id: &str, ts: &str, body: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret_raw).unwrap();
        mac.update(format!("{id}.{ts}.{body}").as_bytes());
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    }

    /// Tests treat `now` as the same epoch as `ts` so the timestamp
    /// tolerance check always passes — we want each test to isolate one
    /// failure mode.
    fn now_for(ts: &str) -> i64 {
        ts.parse::<i64>().unwrap_or(0)
    }

    #[test]
    fn verify_svix_signature_accepts_valid_signature_raw_secret() {
        let secret = "shhh-test-secret";
        let id = "msg_123";
        let ts = "1700000000";
        let body = r#"{"hello":"world"}"#;
        let sig = make_sig(secret.as_bytes(), id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        assert!(verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            secret,
            now_for(ts)
        ));
    }

    #[test]
    fn verify_svix_signature_accepts_whsec_prefixed_secret() {
        // Real Resend secrets are `whsec_<base64>`. Decode then HMAC.
        let raw_key = b"raw-key-bytes-for-test";
        let secret = format!(
            "whsec_{}",
            base64::engine::general_purpose::STANDARD.encode(raw_key)
        );
        let id = "msg_456";
        let ts = "1700000001";
        let body = r#"{"a":1}"#;
        let sig = make_sig(raw_key, id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        assert!(verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            &secret,
            now_for(ts)
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_wrong_secret() {
        let id = "msg_789";
        let ts = "1700000002";
        let body = r#"{"x":true}"#;
        let sig = make_sig(b"correct-secret", id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        assert!(!verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            "wrong-secret",
            now_for(ts)
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_tampered_body() {
        let secret = "s";
        let id = "i";
        let ts = "1700000000";
        let original = r#"{"amount":100}"#;
        let tampered = r#"{"amount":999}"#;
        let sig = make_sig(secret.as_bytes(), id, ts, original);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        assert!(!verify_svix_signature_at(
            &headers,
            tampered.as_bytes(),
            secret,
            now_for(ts)
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_missing_headers() {
        let headers = HeaderMap::new();
        assert!(!verify_svix_signature_at(&headers, b"{}", "s", 0));
    }

    #[test]
    fn verify_svix_signature_accepts_multiple_sigs_one_valid() {
        // Svix sends multiple signatures during secret rotation. We
        // must accept the payload if any verifies.
        let secret = "current";
        let id = "i";
        let ts = "1700000000";
        let body = "{}";
        let good = make_sig(secret.as_bytes(), id, ts, body);
        let bad = make_sig(b"old", id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert(
            "svix-signature",
            format!("v1,{bad} v1,{good}").parse().unwrap(),
        );
        assert!(verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            secret,
            now_for(ts)
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_no_v1_prefix() {
        let secret = "s";
        let id = "i";
        let ts = "1700000000";
        let body = "{}";
        let sig = make_sig(secret.as_bytes(), id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        // Missing "v1," prefix → rejected.
        headers.insert("svix-signature", sig.parse().unwrap());
        assert!(!verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            secret,
            now_for(ts)
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_stale_timestamp() {
        // Captured webhook replayed >5min later — must be rejected even
        // with a valid signature.
        let secret = "s";
        let id = "i";
        let ts = "1700000000";
        let body = "{}";
        let sig = make_sig(secret.as_bytes(), id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        // 6 minutes later = stale.
        let stale_now = now_for(ts) + (SVIX_TIMESTAMP_TOLERANCE_SECS + 60);
        assert!(!verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            secret,
            stale_now
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_future_timestamp() {
        // Forged future-dated event — must be rejected.
        let secret = "s";
        let id = "i";
        let ts = "1700000600"; // 10 min into the "future"
        let body = "{}";
        let sig = make_sig(secret.as_bytes(), id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        // "now" is 10 min before the claimed ts.
        assert!(!verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            secret,
            1_700_000_000
        ));
    }

    #[test]
    fn verify_svix_signature_rejects_non_numeric_timestamp() {
        // Garbage timestamp can't be parsed → reject.
        let secret = "s";
        let id = "i";
        let ts = "not-a-number";
        let body = "{}";
        let sig = make_sig(secret.as_bytes(), id, ts, body);
        let mut headers = HeaderMap::new();
        headers.insert("svix-id", id.parse().unwrap());
        headers.insert("svix-timestamp", ts.parse().unwrap());
        headers.insert("svix-signature", format!("v1,{sig}").parse().unwrap());
        assert!(!verify_svix_signature_at(
            &headers,
            body.as_bytes(),
            secret,
            1_700_000_000
        ));
    }
}
