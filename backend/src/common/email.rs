use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Render a `{{variable}}` template body against a JSON context using
/// MiniJinja. Errors short-circuit to the raw template so a typo in an
/// admin-written email cannot block a transactional send (the unrendered
/// `{{first_name}}` is ugly but still arrives).
///
/// Use it for both:
///   * Per-recipient rendering in the campaign loop (first_name, email)
///   * Admin "preview as" rendering in the workflows tab
pub fn render_template(template: &str, context: &serde_json::Value) -> String {
    let mut env = minijinja::Environment::new();
    // Auto-escape HTML output for any block whose name ends in .html — the
    // template_name is "email" below so we set auto-escape explicitly via
    // the per-environment setting instead.
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::Html);

    let Ok(tmpl) = env.template_from_str(template) else {
        tracing::warn!("render_template: failed to compile, returning raw body");
        return template.to_string();
    };

    match tmpl.render(context) {
        Ok(rendered) => rendered,
        Err(err) => {
            tracing::warn!(
                "render_template: render error {} — returning raw body",
                err
            );
            template.to_string()
        }
    }
}

/// Returns true when transactional email delivery is configured.
pub fn resend_configured() -> bool {
    std::env::var("RESEND_API_KEY")
        .ok()
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
}

/// Send an email using Resend API.
///
/// Phase-2 P0: `unsubscribe_url` enables RFC 2369 `List-Unsubscribe` +
/// RFC 8058 one-click headers for marketing / transactional bulk mail.
/// Pass `None` for purely transactional / security mail (password reset,
/// 2FA) where unsubscribe is not appropriate.
pub async fn send_email(
    to: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), crate::error::AppError> {
    send_email_with_headers(to, subject, html_body, None).await
}

/// Like `send_email` but adds RFC 2369 + RFC 8058 unsubscribe headers when
/// a URL is supplied. The URL is expected to accept HTTPS POST with an
/// empty body (one-click unsubscribe) AND a GET fallback that renders a
/// confirmation page.
pub async fn send_email_with_headers(
    to: &str,
    subject: &str,
    html_body: &str,
    unsubscribe_url: Option<&str>,
) -> Result<(), crate::error::AppError> {
    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => {
            if std::env::var("APP_ENV")
                .map(|env| env.eq_ignore_ascii_case("production"))
                .unwrap_or(false)
            {
                tracing::error!("RESEND_API_KEY not configured in production");
                return Err(crate::error::AppError::ServiceUnavailable(
                    "Transactional email is not configured".to_string(),
                ));
            }
            tracing::warn!("RESEND_API_KEY not configured — email to {} not sent", to);
            return Ok(()); // Silently skip in dev; in prod this env var must be set
        }
    };

    // Build optional `headers` map. Resend accepts arbitrary headers as a
    // string→string object. We always include a mailto: fallback and, when
    // a per-recipient URL is available, the https one-click variant + the
    // companion `List-Unsubscribe-Post` flag that signals RFC 8058 support
    // to Gmail / Apple Mail / Yahoo bulk-sender pipelines.
    let mut headers_json = serde_json::Map::new();
    let mailto_unsub = "<mailto:unsubscribe@poool.app?subject=unsubscribe>";
    let list_unsub_value = match unsubscribe_url {
        Some(url) if !url.is_empty() => format!("<{}>, {}", url, mailto_unsub),
        _ => mailto_unsub.to_string(),
    };
    headers_json.insert("List-Unsubscribe".to_string(), json!(list_unsub_value));
    if unsubscribe_url.is_some() {
        headers_json.insert(
            "List-Unsubscribe-Post".to_string(),
            json!("List-Unsubscribe=One-Click"),
        );
    }

    let mut payload = json!({
        "from": "POOOL <hello@poool.app>",
        "to": [to],
        "subject": subject,
        "html": html_body,
    });
    payload["headers"] = serde_json::Value::Object(headers_json);

    let client = Client::new();
    let res = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            crate::error::AppError::Internal(format!("Failed to send email request: {}", e))
        })?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        tracing::error!("Resend API error ({}): {}", status, text);
        return Err(crate::error::AppError::Internal(
            "Failed to send email via Resend".into(),
        ));
    }

    Ok(())
}

/// Try to deliver one password-reset email outbox row immediately.
///
/// Provider failures are captured back onto the outbox row for retry; callers
/// should not expose them to users because that would reintroduce account
/// existence and provider-health side channels.
pub async fn send_password_reset_outbox_item(pool: &PgPool, outbox_id: Uuid) {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            tracing::error!("Password reset outbox begin failed: {}", err);
            return;
        }
    };

    let row = match sqlx::query_as::<_, (Uuid, String, String, String, i32)>(
        r#"
        UPDATE password_reset_email_outbox
           SET status = 'sending',
               attempts = attempts + 1,
               updated_at = NOW()
         WHERE id = $1
           AND status IN ('queued', 'failed')
           AND next_attempt_at <= NOW()
           AND attempts < 10
           AND EXISTS (
               SELECT 1
                 FROM password_reset_tokens prt
                WHERE prt.id = password_reset_email_outbox.password_reset_token_id
                  AND prt.used_at IS NULL
                  AND prt.expires_at > NOW()
           )
        RETURNING id, recipient_email, subject, html_body, attempts
        "#,
    )
    .bind(outbox_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            let _ = tx.rollback().await;
            return;
        }
        Err(err) => {
            let _ = tx.rollback().await;
            tracing::error!("Password reset outbox claim failed: {}", err);
            return;
        }
    };

    if let Err(err) = tx.commit().await {
        tracing::error!("Password reset outbox claim commit failed: {}", err);
        return;
    }

    let (id, recipient_email, subject, html_body, attempts) = row;
    match send_email(&recipient_email, &subject, &html_body).await {
        Ok(()) => {
            if let Err(err) = sqlx::query(
                r#"
                UPDATE password_reset_email_outbox
                   SET status = 'sent',
                       sent_at = NOW(),
                       last_error = NULL,
                       updated_at = NOW()
                 WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(pool)
            .await
            {
                tracing::error!("Password reset outbox sent mark failed: {}", err);
            }
        }
        Err(err) => {
            let retry_delay_secs = retry_delay_seconds(attempts);
            let error_detail = err.detail();
            tracing::error!(
                "Password reset outbox delivery failed; queued retry in {}s: {}",
                retry_delay_secs,
                error_detail
            );
            sentry::capture_message(
                "Password reset outbox delivery failed",
                sentry::Level::Error,
            );

            if let Err(update_err) = sqlx::query(
                r#"
                UPDATE password_reset_email_outbox
                   SET status = 'failed',
                       last_error = $2,
                       next_attempt_at = NOW() + ($3::TEXT || ' seconds')::INTERVAL,
                       updated_at = NOW()
                 WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(error_detail)
            .bind(retry_delay_secs)
            .execute(pool)
            .await
            {
                tracing::error!("Password reset outbox failed mark failed: {}", update_err);
            }
        }
    }
}

/// Retry a bounded batch of queued/failed password-reset emails.
pub async fn process_password_reset_outbox(pool: &PgPool, limit: i64) {
    let ids = match sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
          FROM password_reset_email_outbox
         WHERE status IN ('queued', 'failed')
           AND next_attempt_at <= NOW()
           AND attempts < 10
           AND EXISTS (
               SELECT 1
                 FROM password_reset_tokens prt
                WHERE prt.id = password_reset_email_outbox.password_reset_token_id
                  AND prt.used_at IS NULL
                  AND prt.expires_at > NOW()
           )
         ORDER BY created_at ASC
         LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    {
        Ok(ids) => ids,
        Err(err) => {
            tracing::error!("Password reset outbox scan failed: {}", err);
            return;
        }
    };

    for id in ids {
        send_password_reset_outbox_item(pool, id).await;
    }
}

fn retry_delay_seconds(attempts: i32) -> i64 {
    let capped = attempts.clamp(1, 6) as u32;
    (60_i64 * 2_i64.pow(capped - 1)).min(1_800)
}

/// Deliver one transactional_email_outbox row immediately.
/// On provider failure, marks the row failed with a backoff timestamp for retry.
pub async fn send_transactional_outbox_item(pool: &PgPool, outbox_id: Uuid) {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            tracing::error!("Transactional email outbox begin failed: {}", err);
            return;
        }
    };

    // Phase-2 P0: claim now also returns user_id + event_type so we can:
    //   (a) build a per-recipient One-Click unsubscribe URL for marketing
    //       and affiliate event classes (List-Unsubscribe header)
    //   (b) skip delivery entirely if the user has opted out of optional
    //       email notifications for that class (security / payment events
    //       always send regardless).
    let row = match sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, i32)>(
        r#"UPDATE transactional_email_outbox
              SET status = 'sending',
                  attempts = attempts + 1,
                  updated_at = NOW()
            WHERE id = $1
              AND status IN ('queued', 'failed')
              AND next_attempt_at <= NOW()
              AND attempts < 10
           RETURNING id, user_id, event_type, recipient_email, subject, html_body, attempts"#,
    )
    .bind(outbox_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            let _ = tx.rollback().await;
            return;
        }
        Err(err) => {
            let _ = tx.rollback().await;
            tracing::error!("Transactional email outbox claim failed: {}", err);
            return;
        }
    };

    if let Err(err) = tx.commit().await {
        tracing::error!("Transactional email outbox claim commit failed: {}", err);
        return;
    }

    let (id, user_id, event_type, recipient_email, subject, html_body, attempts) = row;

    // Pref-gate optional event classes. We honour `user_settings.email_notifications`
    // for the classes audit-flagged as opt-out-able (affiliate + team
    // notifications). Security / payment events bypass this so users can't
    // accidentally silence a password-reset.
    if is_optional_email_event(&event_type) {
        let opted_out = sqlx::query_scalar::<_, bool>(
            "SELECT NOT COALESCE(email_notifications, true)
               FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
        if opted_out {
            tracing::info!(
                outbox_id = %id,
                user_id = %user_id,
                event_type = %event_type,
                "Recipient opted out of optional email notifications — skipping send."
            );
            let _ = sqlx::query(
                "UPDATE transactional_email_outbox
                   SET status='skipped', sent_at=NOW(),
                       last_error='recipient opted out', updated_at=NOW()
                 WHERE id=$1",
            )
            .bind(id)
            .execute(pool)
            .await;
            return;
        }
    }

    let unsubscribe_url = if is_optional_email_event(&event_type) {
        build_unsubscribe_url(user_id, &event_type)
    } else {
        None
    };

    match send_email_with_headers(
        &recipient_email,
        &subject,
        &html_body,
        unsubscribe_url.as_deref(),
    )
    .await
    {
        Ok(()) => {
            let _ = sqlx::query(
                "UPDATE transactional_email_outbox SET status='sent', sent_at=NOW(), last_error=NULL, updated_at=NOW() WHERE id=$1",
            )
            .bind(id)
            .execute(pool)
            .await;
        }
        Err(err) => {
            let delay = retry_delay_seconds(attempts);
            tracing::error!(
                "Transactional email outbox delivery failed; retry in {}s: {}",
                delay,
                err.detail()
            );
            let _ = sqlx::query(
                r#"UPDATE transactional_email_outbox
                      SET status='failed', last_error=$2,
                          next_attempt_at=NOW() + ($3::TEXT || ' seconds')::INTERVAL,
                          updated_at=NOW()
                    WHERE id=$1"#,
            )
            .bind(id)
            .bind(err.detail())
            .bind(delay.to_string())
            .execute(pool)
            .await;
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Unsubscribe / preference-gate helpers (Phase-2 P0)
//
// Event-class taxonomy:
//   * Optional (user can opt out via `user_settings.email_notifications`):
//     - affiliate_*, team_*  (all affiliate-program notifications)
//     - asset_funded, monthly_statement, dividend_payout, milestone_*
//   * Mandatory (always sent — security, legal, payment correctness):
//     - welcome, verify_email, password_reset, 2fa_setup, new_login
//     - kyc_*, deposit_confirmed, withdrawal_processed
//     - support_ticket_*, operations_*, invoice_available
//
// We err on the side of "mandatory" for ambiguous events: skipping a
// payout or compliance email has worse blast radius than sending an
// extra one.
// ───────────────────────────────────────────────────────────────────────────

/// Returns true when the event-type is opt-out-able by the recipient.
pub fn is_optional_email_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "affiliate_application_received"
            | "affiliate_approved"
            | "affiliate_rejected"
            | "affiliate_suspended"
            | "affiliate_payout_released"
            | "affiliate_commission_earned"
            | "team_invitation_received"
            | "team_member_approved"
            | "team_member_removed"
            | "team_self_request_received"
            | "team_invitation_accepted"
            | "asset_funded"
            | "monthly_statement"
            | "dividend_payout"
            // Admin-triggered marketing blasts. List-Unsubscribe + the
            // `email_notifications=false` preference both silence these.
            | "marketing_campaign"
    ) || event_type.starts_with("milestone_")
}

/// Public so other modules can render the same URL (e.g. the future
/// `/email/preferences` settings link in transactional bodies).
pub fn email_base_url() -> String {
    std::env::var("PUBLIC_APP_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://platform.poool.app".to_string())
}

fn unsubscribe_secret() -> String {
    std::env::var("EMAIL_UNSUBSCRIBE_SECRET")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("SESSION_SECRET").ok())
        .or_else(|| std::env::var("JWT_SECRET").ok())
        .unwrap_or_else(|| "dev-only-unsubscribe-secret".to_string())
}

/// Sign `<user_id>|<event_class>` with HMAC-SHA256. Hex-encoded; ~64 chars.
/// Event-class is currently a fixed value ("optional") because we toggle a
/// single boolean; extending to per-channel prefs is a one-line change.
fn sign_unsubscribe_token(user_id: Uuid, class: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let payload = format!("{}|{}", user_id, class);
    let mut mac = Hmac::<Sha256>::new_from_slice(unsubscribe_secret().as_bytes())
        .expect("HMAC-SHA256 accepts any key length");
    mac.update(payload.as_bytes());
    let bytes = mac.finalize().into_bytes();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes.iter() {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

fn verify_unsubscribe_token(user_id: Uuid, class: &str, token: &str) -> bool {
    // Constant-time-ish comparison: same length first, then strict eq.
    let expected = sign_unsubscribe_token(user_id, class);
    if expected.len() != token.len() {
        return false;
    }
    expected.as_bytes().iter().zip(token.as_bytes().iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Build the per-recipient one-click unsubscribe URL for the
/// `List-Unsubscribe` header. Returns `None` only on truly impossible
/// failures (current impl: always returns `Some`).
pub fn build_unsubscribe_url(user_id: Uuid, _event_type: &str) -> Option<String> {
    let class = "optional";
    let token = sign_unsubscribe_token(user_id, class);
    Some(format!(
        "{}/email/unsubscribe?u={}&c={}&t={}",
        email_base_url(),
        user_id,
        class,
        token
    ))
}

/// Confirmation HTML rendered by `handle_unsubscribe` after a successful
/// unsubscribe. Plain, standalone — no template engine required.
fn unsubscribe_confirmation_html(success: bool, kind: &str) -> String {
    if success {
        format!(
            r#"<!doctype html><html><head><meta charset="utf-8"><title>Unsubscribed — POOOL</title>
<style>body{{font-family:sans-serif;max-width:520px;margin:80px auto;padding:24px;color:#101828;}}
.ok{{background:#ECFDF3;color:#027A48;padding:16px;border-radius:8px;}}
a{{color:#0000FF;}}</style></head><body>
<h1>You've been unsubscribed</h1>
<p class="ok">You will no longer receive {kind} from POOOL.</p>
<p>You can re-enable these emails any time in your <a href="{base}/settings">account settings</a>.</p>
<p style="color:#717680;font-size:13px;margin-top:32px;">Security and payment-related emails (password reset, payout confirmations) will continue to be sent regardless of this preference, per regulatory requirements.</p>
</body></html>"#,
            kind = kind,
            base = email_base_url(),
        )
    } else {
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Link expired — POOOL</title>
<style>body{font-family:sans-serif;max-width:520px;margin:80px auto;padding:24px;color:#101828;}
.err{background:#FEF3F2;color:#B42318;padding:16px;border-radius:8px;}</style></head><body>
<h1>This unsubscribe link is no longer valid</h1>
<p class="err">The signature did not verify. The link may be malformed, expired, or tampered with.</p>
<p>Open your <a href="https://platform.poool.app/settings">account settings</a> to update your email preferences directly.</p>
</body></html>"#.to_string()
    }
}

/// HTTP handler: `GET /email/unsubscribe?u=<uuid>&c=<class>&t=<hmac>`
///
/// Also accepts `POST` for RFC 8058 one-click compliance — Gmail / Apple
/// Mail / Yahoo bulk-sender pipelines POST to this URL when the user hits
/// the inbox-level unsubscribe button. The behaviour is identical: verify
/// the signed token, flip the user's `email_notifications` pref, render
/// the confirmation page.
pub async fn handle_unsubscribe(
    axum::extract::State(state): axum::extract::State<crate::auth::routes::AppState>,
    axum::extract::Query(q): axum::extract::Query<UnsubscribeQuery>,
) -> axum::response::Response {
    use axum::response::{Html, IntoResponse};
    let user_id = match Uuid::parse_str(&q.u) {
        Ok(id) => id,
        Err(_) => return Html(unsubscribe_confirmation_html(false, "")).into_response(),
    };
    let class = q.c.as_deref().unwrap_or("optional");
    if !verify_unsubscribe_token(user_id, class, &q.t) {
        return Html(unsubscribe_confirmation_html(false, "")).into_response();
    }
    // Idempotent UPSERT — turn email_notifications off.
    let _ = sqlx::query(
        r#"INSERT INTO user_settings (user_id, email_notifications)
           VALUES ($1, FALSE)
           ON CONFLICT (user_id) DO UPDATE
              SET email_notifications = FALSE, updated_at = NOW()"#,
    )
    .bind(user_id)
    .execute(&state.db)
    .await;
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'email_unsubscribed', 'user_settings', $2, $3)",
    )
    .bind(user_id)
    .bind(user_id)
    .bind(serde_json::json!({ "class": class }))
    .execute(&state.db)
    .await;
    Html(unsubscribe_confirmation_html(
        true,
        "marketing and affiliate-program emails",
    ))
    .into_response()
}

#[derive(serde::Deserialize)]
pub struct UnsubscribeQuery {
    pub u: String,
    pub c: Option<String>,
    pub t: String,
}

/// Poll transactional_email_outbox for items that are ready to retry.
pub async fn process_transactional_email_outbox(pool: &PgPool, batch_size: i64) {
    let ids = match sqlx::query_scalar::<_, Uuid>(
        r#"SELECT id FROM transactional_email_outbox
            WHERE status IN ('queued', 'failed')
              AND next_attempt_at <= NOW()
              AND attempts < 10
            ORDER BY next_attempt_at
            LIMIT $1"#,
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await
    {
        Ok(ids) => ids,
        Err(err) => {
            tracing::error!("Transactional email outbox scan failed: {}", err);
            return;
        }
    };

    for id in ids {
        send_transactional_outbox_item(pool, id).await;
    }
}

// ─── Baseline tests for delivery classification + unsubscribe HMAC ────────
//
// Pure unit tests (no DB / no network). They pin:
//   * The optional-vs-mandatory event classification — a regression here
//     would either silence security mail or leak marketing mail past an
//     opt-out, both of which are user-visible incidents.
//   * The HMAC-SHA256 unsubscribe token round-trip and tampering rejection
//     — verifying a forged token would let anyone unsubscribe anyone else.
//   * The exponential backoff schedule used by the outbox worker so we
//     catch accidental "retries every second" regressions.

#[cfg(test)]
mod tests {
    use super::*;

    /// Mandatory event classes — security, payment, legal, support. Must
    /// always send regardless of the user's `email_notifications` setting.
    const MANDATORY_EVENTS: &[&str] = &[
        "welcome",
        "verify_email",
        "password_reset",
        "2fa_setup",
        "new_login",
        "kyc_submitted",
        "kyc_approved",
        "kyc_rejected",
        "deposit_submitted",
        "deposit_confirmed",
        "withdraw_requested",
        "withdraw_approved",
        "withdraw_rejected",
        "withdrawal_processed",
        "order_confirmation",
        "invoice_available",
        "support_ticket_reply",
        "support_ticket_new",
        "support_ticket_resolved",
        "operations_rejected",
        "operations_approved",
        "operations_published",
    ];

    /// Optional event classes — user can silence via List-Unsubscribe or
    /// settings without breaking platform correctness.
    const OPTIONAL_EVENTS: &[&str] = &[
        "affiliate_application_received",
        "affiliate_approved",
        "affiliate_rejected",
        "affiliate_suspended",
        "affiliate_payout_released",
        "affiliate_commission_earned",
        "team_invitation_received",
        "team_member_approved",
        "team_member_removed",
        "team_self_request_received",
        "team_invitation_accepted",
        "asset_funded",
        "monthly_statement",
        "dividend_payout",
        "marketing_campaign",
        "milestone_first_investment",
        "milestone_anniversary",
    ];

    #[test]
    fn mandatory_events_are_never_classified_optional() {
        for event in MANDATORY_EVENTS {
            assert!(
                !is_optional_email_event(event),
                "event '{}' must be mandatory (security/payment/legal/support) \
                 but is_optional_email_event returned true — this would silence \
                 the email when the user opts out of marketing",
                event
            );
        }
    }

    #[test]
    fn optional_events_are_classified_optional() {
        for event in OPTIONAL_EVENTS {
            assert!(
                is_optional_email_event(event),
                "event '{}' should be optional (marketing / affiliate / team) \
                 but is_optional_email_event returned false — opt-out would not \
                 silence the email",
                event
            );
        }
    }

    #[test]
    fn unknown_events_default_to_mandatory() {
        // Err on the side of "send it" for unrecognised events — a missing
        // payout email is worse than an extra unrelated one.
        assert!(!is_optional_email_event("totally_unknown_event"));
        assert!(!is_optional_email_event(""));
    }

    #[test]
    fn milestone_prefix_matches_any_suffix() {
        assert!(is_optional_email_event("milestone_first_investment"));
        assert!(is_optional_email_event("milestone_1year"));
        // Edge: bare "milestone_" (no suffix) still matches the prefix rule.
        assert!(is_optional_email_event("milestone_"));
    }

    #[test]
    fn unsubscribe_token_roundtrip_with_same_input() {
        let user = uuid::Uuid::nil();
        let class = "optional";
        let token = sign_unsubscribe_token(user, class);
        assert!(verify_unsubscribe_token(user, class, &token));
        // Token is hex-encoded HMAC-SHA256 → 64 chars.
        assert_eq!(token.len(), 64);
    }

    #[test]
    fn unsubscribe_token_rejects_wrong_user() {
        let user_a = uuid::Uuid::nil();
        let user_b = uuid::Uuid::from_u128(1);
        let token = sign_unsubscribe_token(user_a, "optional");
        assert!(!verify_unsubscribe_token(user_b, "optional", &token));
    }

    #[test]
    fn unsubscribe_token_rejects_wrong_class() {
        let user = uuid::Uuid::nil();
        let token = sign_unsubscribe_token(user, "optional");
        assert!(!verify_unsubscribe_token(user, "marketing", &token));
    }

    #[test]
    fn unsubscribe_token_rejects_tampered_token() {
        let user = uuid::Uuid::nil();
        let token = sign_unsubscribe_token(user, "optional");
        // Flip the first hex character.
        let mut tampered = token.clone();
        let first = tampered.chars().next().unwrap();
        let replacement = if first == '0' { '1' } else { '0' };
        tampered.replace_range(..1, &replacement.to_string());
        assert!(!verify_unsubscribe_token(user, "optional", &tampered));
    }

    #[test]
    fn unsubscribe_token_rejects_wrong_length() {
        let user = uuid::Uuid::nil();
        // Short and long inputs both fail without panicking.
        assert!(!verify_unsubscribe_token(user, "optional", ""));
        assert!(!verify_unsubscribe_token(user, "optional", "short"));
        assert!(!verify_unsubscribe_token(
            user,
            "optional",
            &"a".repeat(128)
        ));
    }

    #[test]
    fn build_unsubscribe_url_returns_well_formed_link() {
        let user = uuid::Uuid::nil();
        let url = build_unsubscribe_url(user, "affiliate_approved").expect("url");
        assert!(url.contains("/email/unsubscribe"));
        assert!(url.contains(&format!("u={}", user)));
        assert!(url.contains("c=optional"));
        assert!(url.contains("t="));
    }

    #[test]
    fn retry_delay_backoff_is_exponential_and_capped() {
        // attempt 1 → 60s, 2 → 120s, 3 → 240s, 4 → 480s, 5 → 960s, 6+ → 1800s cap.
        assert_eq!(retry_delay_seconds(1), 60);
        assert_eq!(retry_delay_seconds(2), 120);
        assert_eq!(retry_delay_seconds(3), 240);
        assert_eq!(retry_delay_seconds(4), 480);
        assert_eq!(retry_delay_seconds(5), 960);
        assert_eq!(retry_delay_seconds(6), 1_800);
        // Cap holds beyond clamp range — large attempts must not overflow.
        assert_eq!(retry_delay_seconds(10), 1_800);
        assert_eq!(retry_delay_seconds(100), 1_800);
    }

    #[test]
    fn retry_delay_handles_zero_and_negative_attempts() {
        // Defensive: clamp guarantees we never panic on weird inputs.
        assert_eq!(retry_delay_seconds(0), 60);
        assert_eq!(retry_delay_seconds(-5), 60);
    }

    // ── render_template (MiniJinja interpolation) ─────────────────────

    #[test]
    fn render_template_interpolates_known_vars() {
        let ctx = serde_json::json!({ "first_name": "Maria", "email": "m@x.test" });
        let out = render_template("Hi {{first_name}} ({{email}})", &ctx);
        assert_eq!(out, "Hi Maria (m@x.test)");
    }

    #[test]
    fn render_template_leaves_unknown_vars_blank() {
        // MiniJinja's default behaviour: undefined → empty string.
        let ctx = serde_json::json!({ "first_name": "Maria" });
        let out = render_template("{{first_name}} / {{last_name}}", &ctx);
        assert_eq!(out, "Maria / ");
    }

    #[test]
    fn render_template_auto_escapes_html_in_context() {
        // Auto-escape is on so a user-controlled first_name cannot inject
        // markup into the rendered email body.
        let ctx = serde_json::json!({ "first_name": "<script>x</script>" });
        let out = render_template("Hello {{first_name}}", &ctx);
        assert!(!out.contains("<script>"));
        assert!(out.contains("&lt;script&gt;"));
    }

    #[test]
    fn render_template_returns_raw_on_syntax_error() {
        // Bad template should not crash the send loop — return the raw
        // body so the unrendered `{{` is at least delivered.
        let ctx = serde_json::json!({});
        let bad = "{{ unbalanced";
        let out = render_template(bad, &ctx);
        assert_eq!(out, bad);
    }

    #[test]
    fn render_template_passthrough_when_no_variables() {
        let ctx = serde_json::json!({});
        let html = "<h1>Welcome</h1><p>Static content.</p>";
        assert_eq!(render_template(html, &ctx), html);
    }

    #[test]
    fn render_template_handles_if_blocks() {
        // Lets admins write conditional bodies like
        // `{% if first_name %}Hi {{first_name}}{% else %}Hi there{% endif %}`.
        let ctx_named = serde_json::json!({ "first_name": "Sam" });
        let ctx_anon = serde_json::json!({ "first_name": "" });
        let tmpl = "{% if first_name %}Hi {{first_name}}{% else %}Hi there{% endif %}!";
        assert_eq!(render_template(tmpl, &ctx_named), "Hi Sam!");
        assert_eq!(render_template(tmpl, &ctx_anon), "Hi there!");
    }
}
