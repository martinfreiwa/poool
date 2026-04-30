use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Returns true when transactional email delivery is configured.
pub fn resend_configured() -> bool {
    std::env::var("RESEND_API_KEY")
        .ok()
        .map(|key| !key.trim().is_empty())
        .unwrap_or(false)
}

/// Send an email using Resend API.
pub async fn send_email(
    to: &str,
    subject: &str,
    html_body: &str,
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

    let client = Client::new();
    let res = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&json!({
            "from": "POOOL <hello@poool.app>",
            "to": [to],
            "subject": subject,
            "html": html_body
        }))
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
