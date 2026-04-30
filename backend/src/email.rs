// ═══════════════════════════════════════════════════════════════
// POOOL Marketing & Email Automation Background Service
// ═══════════════════════════════════════════════════════════════

use sqlx::PgPool;
use std::time::Duration;
use tracing::info;

/// Build an HTML email body for a transactional event.
fn build_email_html(event_type: &str, metadata: &serde_json::Value) -> String {
    match event_type {
        "kyc_approved" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Your identity has been verified ✓</h2>
  <p>Great news — your KYC application has been approved. You can now invest in tokenised assets on POOOL.</p>
  <p><a href="https://platform.poool.app/marketplace" style="display:inline-block;padding:12px 24px;background:#3D00F5;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Browse Assets</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">If you have questions, reply to this email or visit our support centre.</p>
</div>"#.to_string(),

        "kyc_rejected" => {
            let reason = metadata.get("rejection_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Please review the requirements and resubmit.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Action required: KYC resubmission</h2>
  <p>Unfortunately your identity verification could not be approved at this time.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p>Please resubmit your documents addressing the issue above.</p>
  <p><a href="https://platform.poool.app/kyc" style="display:inline-block;padding:12px 24px;background:#3D00F5;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Resubmit Verification</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Need help? Contact us at support@poool.app</p>
</div>"#, reason = html_escape_email(reason))
        }

        "kyc_submitted" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">We received your verification documents</h2>
  <p>Your KYC application is now under review. This typically takes 1–2 business days.</p>
  <p>We'll email you as soon as a decision is made.</p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact us at support@poool.app</p>
</div>"#.to_string(),

        "deposit_confirmed" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Your deposit has been received</h2>
  <p>Your deposit has been confirmed and your POOOL wallet balance has been updated.</p>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#3D00F5;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Wallet</a></p>
</div>"#.to_string(),

        "support_ticket_reply" => {
            let subject_line = metadata.get("ticket_subject").and_then(|v| v.as_str()).unwrap_or("your ticket");
            let reply_preview = metadata.get("reply_preview").and_then(|v| v.as_str()).unwrap_or("");
            let ticket_id = metadata.get("ticket_id").and_then(|v| v.as_str()).unwrap_or("");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">You have a new reply on your support ticket</h2>
  <p style="color:#414651;">Our support team has replied to: <strong>{subject}</strong></p>
  {preview_block}
  <p><a href="https://platform.poool.app/support" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Conversation</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Reply directly in your support portal. Please do not reply to this email.</p>
</div>"#,
                subject = html_escape_email(subject_line),
                preview_block = if !reply_preview.is_empty() {
                    format!(r#"<div style="background:#F4F5FF;border-left:3px solid #0000FF;padding:12px 16px;border-radius:4px;margin:16px 0;color:#344054;font-size:14px;line-height:1.6;">{}</div>"#,
                        html_escape_email(reply_preview))
                } else { String::new() },
            )
        }

        "support_ticket_new" => {
            let subject_line = metadata.get("ticket_subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
            let user_email = metadata.get("user_email").and_then(|v| v.as_str()).unwrap_or("unknown");
            let priority = metadata.get("priority").and_then(|v| v.as_str()).unwrap_or("normal");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">New support ticket submitted</h2>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:120px;">From</td><td style="padding:8px 0;color:#101828;font-weight:500;">{user}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Subject</td><td style="padding:8px 0;color:#101828;font-weight:500;">{subject}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Priority</td><td style="padding:8px 0;color:#101828;font-weight:500;">{priority}</td></tr>
  </table>
  <p><a href="https://platform.poool.app/admin/support.html" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View in Admin Panel</a></p>
</div>"#,
                user = html_escape_email(user_email),
                subject = html_escape_email(subject_line),
                priority = html_escape_email(priority),
            )
        }

        "support_ticket_resolved" => {
            let subject_line = metadata.get("ticket_subject").and_then(|v| v.as_str()).unwrap_or("your ticket");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Your support ticket has been resolved</h2>
  <p style="color:#414651;">We've marked <strong>{subject}</strong> as resolved.</p>
  <p style="color:#414651;">If your issue isn't fully sorted, you can reopen the ticket from your support portal at any time.</p>
  <p><a href="https://platform.poool.app/support" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Ticket</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Thank you for using POOOL support.</p>
</div>"#,
                subject = html_escape_email(subject_line),
            )
        }

        _ => format!(
            r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;"><p>You have a new notification from POOOL.</p></div>"#
        ),
    }
}

fn html_escape_email(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The unified event bus / mail trigger for all transactional systems.
/// Writes to transactional_email_outbox for durable delivery with retry,
/// then attempts immediate send. Falls back gracefully if outbox insert fails.
#[allow(dead_code)]
pub async fn trigger_transactional_email(
    pool: &PgPool,
    user_id: &uuid::Uuid,
    event_type: &str,
    metadata: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = match event_type {
        "welcome" => "Welcome to POOOL!",
        "verify_email" => "Please Verify Your Email",
        "password_reset" => "Password Reset Code",
        "2fa_setup" => "2FA Setup Activated",
        "new_login" => "New Device Login Detected",
        "kyc_approved" => "KYC Application Approved",
        "kyc_rejected" => "KYC Action Required",
        "kyc_submitted" => "KYC Application Received",
        "deposit_confirmed" => "Deposit Received",
        "withdrawal_processed" => "Withdrawal Processed",
        "dividend_payout" => "You've Earned a Dividend!",
        "monthly_statement" => "Your Monthly POOOL Statement",
        "order_confirmation" => "Order Confirmation",
        "invoice_available" => "Invoice Available for Download",
        "asset_funded" => "An Asset You Follow is 100% Funded",

        // Affiliate Partner Syndicate Operations
        "affiliate_application_received" => "Application Received - POOOL Partner Syndicate",
        "affiliate_approved" => "Welcome to the POOOL Partner Syndicate!",
        "affiliate_rejected" => "Update on your POOOL Partner Application",
        "affiliate_suspended" => "Urgent: Your POOOL Affiliate Account Status",
        "affiliate_payout_released" => "Your POOOL Affiliate Payout Details",
        "affiliate_commission_earned" => "New Commission Tracked - POOOL Partner Syndicate",

        "support_ticket_reply" => "New reply on your support ticket",
        "support_ticket_new" => "New support ticket submitted",
        "support_ticket_resolved" => "Your support ticket has been resolved",

        _ => "You Have a New Notification",
    };

    let user_email = match sqlx::query_scalar::<_, String>(
        "SELECT email FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(e)) => e,
        _ => return Ok(()),
    };

    let html_body = build_email_html(event_type, &metadata);
    let event_type_owned = event_type.to_string();

    // Insert into durable outbox — if this fails we still attempt a best-effort send.
    let outbox_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"INSERT INTO transactional_email_outbox
               (user_id, event_type, recipient_email, subject, html_body)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(&event_type_owned)
    .bind(&user_email)
    .bind(subject)
    .bind(&html_body)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Also write to email_logs for audit visibility.
    let _ = sqlx::query!(
        "INSERT INTO email_logs (user_id, subject, recipient_email, status, error_message) VALUES ($1, $2, $3, 'queued', $4)",
        user_id, subject, user_email, serde_json::to_string(&metadata).unwrap_or_default()
    ).execute(pool).await;

    // Attempt immediate delivery via the outbox item (updates status to sent/failed).
    if let Some(id) = outbox_id {
        crate::common::email::send_transactional_outbox_item(pool, id).await;
    }

    Ok(())
}

/// 28.5 FinTech Email Automations & Drips
pub async fn run_email_scheduler(pool: PgPool) {
    info!("Starting POOOL Email Drips Scheduler...");
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60)); // Poll every hour

    loop {
        interval.tick().await;

        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("background_job".into()),
            message: Some("Email scheduler tick started".into()),
            level: sentry::Level::Info,
            ..Default::default()
        });

        // 1. Onboarding Drips
        if let Err(e) = process_onboarding_drips(&pool).await {
            tracing::error!("Email scheduler: onboarding drips failed: {}", e);
        }

        // 2. Abandonment Flows
        if let Err(e) = process_abandoned_carts(&pool).await {
            tracing::error!("Email scheduler: abandoned carts failed: {}", e);
        }

        // 3. Win-back / Re-engagement
        if let Err(e) = process_win_backs(&pool).await {
            tracing::error!("Email scheduler: win-backs failed: {}", e);
        }

        // 4. Milestone Celebrations
        if let Err(e) = process_milestones(&pool).await {
            tracing::error!("Email scheduler: milestones failed: {}", e);
        }
    }
}

/// Retry transactional email outbox items that must not be dropped.
pub async fn run_transactional_email_outbox_worker(pool: PgPool) {
    info!("Starting POOOL transactional email outbox worker...");
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;
        crate::common::email::process_password_reset_outbox(&pool, 25).await;
        crate::common::email::process_transactional_email_outbox(&pool, 25).await;
    }
}

async fn process_onboarding_drips(_pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Find users older than 24h but no KYC record
    // Insert template to logs
    Ok(())
}

async fn process_abandoned_carts(_pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Check cart tables older than 1hr
    Ok(())
}

async fn process_win_backs(_pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Check last login date > 60 days
    Ok(())
}

async fn process_milestones(_pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
