// ═══════════════════════════════════════════════════════════════
// POOOL Marketing & Email Automation Background Service
// ═══════════════════════════════════════════════════════════════

use sqlx::PgPool;
use std::time::Duration;
use tracing::info;

/// Build an HTML email body for a transactional event. Exposed `pub(crate)`
/// so the admin preview + workflows-tab endpoints can render arbitrary
/// events against sample metadata without going through the database.
///
/// The body returned here goes through `wrap_with_shell` at the end so
/// every event ships in the same POOOL branded template — header with
/// logo, the event-specific content, and a footer with company info +
/// unsubscribe link (for optional events).
pub(crate) fn build_email_html(event_type: &str, metadata: &serde_json::Value) -> String {
    let inner_or_full = build_event_body(event_type, metadata);
    let inner = strip_legacy_wrapper(&inner_or_full);
    let is_optional = crate::common::email::is_optional_email_event(event_type);
    let preheader = preheader_for_event(event_type);
    wrap_with_shell(&inner, ShellOpts {
        is_optional,
        preheader,
    })
}

/// Short hidden preview text shown in the inbox before the body is opened.
/// Gmail / Apple Mail render this as the line after the subject. Defaults
/// to a generic line if no event-specific copy is registered.
fn preheader_for_event(event_type: &str) -> &'static str {
    match event_type {
        "welcome" => "Your POOOL account is live — let's get you investing.",
        "verify_email" => "Confirm your email address to activate POOOL.",
        "password_reset" => "Reset your POOOL password. Link expires in 1 hour.",
        "2fa_setup" => "Two-factor authentication is now protecting your account.",
        "new_login" => "We noticed a sign-in to your POOOL account.",
        "email_changed" => "Your account email address was just changed.",
        "password_changed" => "Your account password was just changed.",
        "2fa_disabled" => "Two-factor authentication is off — re-enable now.",
        "kyc_approved" => "Identity verified — you can now invest on POOOL.",
        "kyc_rejected" => "Your identity verification needs another look.",
        "kyc_submitted" => "We received your verification documents.",
        "deposit_confirmed" => "Your deposit has been credited to your wallet.",
        "deposit_submitted" => "We received your proof of transfer.",
        "withdraw_requested" => "Your withdrawal request is in review.",
        "withdraw_approved" => "Your withdrawal is on its way.",
        "withdraw_rejected" => "Your withdrawal request could not be processed.",
        "withdrawal_processed" => "Your withdrawal has settled.",
        "large_deposit_received" => "Compliance needs source-of-funds documentation.",
        "order_confirmation" => "Your investment order is confirmed.",
        "investment_confirmed" => "Your fractional tokens have been minted.",
        "invoice_available" => "Your invoice is ready to download.",
        "asset_funded" => "An asset you follow is 100% funded.",
        "asset_matured" => "An asset in your portfolio has matured.",
        "dividend_payout" => "A dividend has been credited to your POOOL wallet.",
        "dividend_announced" => "A new distribution has been declared.",
        "monthly_statement" => "Your monthly POOOL statement is ready.",
        "trade_executed" => "Your trade has been executed.",
        "order_filled" => "Your limit order matched at target price.",
        "order_cancelled" => "Your order has been cancelled.",
        "listing_expired" => "Your secondary-market listing expired.",
        "tax_document_available" => "Your annual tax summary is ready to download.",
        "terms_updated" => "POOOL Terms of Service have been updated.",
        "abandoned_cart" => "Pick up where you left off — your investment is still open.",
        "win_back" => "What's new on POOOL since your last visit.",
        "milestone_first_investment" => "Welcome to investing with POOOL.",
        "milestone_anniversary" => "Thanks for another year on POOOL.",
        "weekly_digest" => "Your weekly POOOL portfolio summary.",
        "monthly_affiliate_summary" => "Your monthly affiliate performance.",
        "referral_signed_up" => "Someone you referred just joined POOOL.",
        "support_ticket_reply" => "You have a new reply on your support ticket.",
        "support_ticket_resolved" => "Your support ticket has been resolved.",
        _ => "An update from POOOL",
    }
}

/// Options for `wrap_with_shell`. Kept as a struct so additions don't break callers.
pub(crate) struct ShellOpts {
    /// When `true`, the footer includes a visible unsubscribe link in addition
    /// to the `List-Unsubscribe` header. Marketing / drip / non-essential mail.
    pub is_optional: bool,
    /// Inbox preview text (rendered hidden but parsed by Gmail / Apple Mail).
    pub preheader: &'static str,
}

/// Strip the legacy outer `<div style="font-family:sans-serif;max-width:600px;...">`
/// wrapper from a body, leaving just the inner content. Bodies that don't
/// match the pattern are returned unchanged.
fn strip_legacy_wrapper(body: &str) -> String {
    let trimmed = body.trim();
    const LEGACY_OPEN: &str =
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">"#;
    if let Some(rest) = trimmed.strip_prefix(LEGACY_OPEN) {
        if let Some(inner) = rest.trim_end().strip_suffix("</div>") {
            return inner.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Wrap event-specific inner HTML in the POOOL branded email shell.
///
/// Brand palette matches the live platform (`bundle.css`):
///   * `#0000FF` — primary electric blue (header background, links, CTAs)
///   * `#98FB96` — mint green (wordmark + CTA text — the signature POOOL combo)
///   * `#FAFAFA` — page background (same as `--content-bg`)
///   * `#181D27` — primary text (same as `--text-primary`)
///   * `#535862` — secondary text (same as `--text-secondary`)
///
/// Email-client safe: tables-only layout, inline styles, no SVG, no
/// background-image, no flexbox, max-width 600px.
pub(crate) fn wrap_with_shell(inner: &str, opts: ShellOpts) -> String {
    let unsubscribe_block = if opts.is_optional {
        r#"<p style="margin:0 0 8px;color:#535862;font-size:11px;">
You're receiving this because you opted in to POOOL updates. Manage email preferences in your
<a href="https://platform.poool.app/settings" style="color:#535862;text-decoration:underline;">account settings</a>
or use the one-click unsubscribe link in your email client.
</p>"#
    } else {
        r#"<p style="margin:0 0 8px;color:#535862;font-size:11px;">
This is a security or transactional message related to your POOOL account and cannot be unsubscribed from.
</p>"#
    };

    // Brand font stack:
    //   * 'TT Norms Pro' — the real brand font, picked up by recipients
    //     who happen to have it installed (rare but it's free for us).
    //   * 'Inter' — Google Font, closest open-source match to TT Norms
    //     Pro's geometric sans-serif feel. Pulled via <link> below.
    //     Most clients ignore @import inside <head>; Apple Mail, iOS
    //     Mail and Outlook 365 honour <link>. Gmail web silently strips
    //     it and falls back to the next entry.
    //   * System stack — last-resort defaults that ship on every OS.
    const BRAND_FONT: &str = "'Inter','TT Norms Pro',-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif";

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<meta name="color-scheme" content="light only" />
<meta name="supported-color-schemes" content="light only" />
<title>POOOL</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<!-- Inter @ 400/600/700/900 — closest free font to TT Norms Pro. Mail
     clients that don't honour <link> fall back to the system stack
     declared inline on every element below. -->
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700;900&display=swap" rel="stylesheet" />
<style>
  /* Mobile tweaks — only padding shrinks, never layout. */
  @media only screen and (max-width: 600px) {{
    .px {{ padding-left: 20px !important; padding-right: 20px !important; }}
    .pt {{ padding-top: 24px !important; }}
  }}
  /* Brand-locked link colour — every email client tends to recolour <a>. */
  a {{ color: #0000FF; }}
  /* Brand font everywhere — wins over the body inline font-family. */
  body, table, td, p, h1, h2, h3, h4, ul, ol, li, a {{
    font-family: {brand_font};
  }}
</style>
</head>
<body style="margin:0;padding:0;background:#FAFAFA;font-family:{brand_font};color:#181D27;-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale;">
  <!-- Hidden preheader — parsed by Gmail / Apple Mail as the inbox preview snippet. -->
  <div style="display:none;max-height:0;overflow:hidden;mso-hide:all;font-size:1px;line-height:1px;color:#FAFAFA;opacity:0;">
    {preheader}
  </div>

  <table role="presentation" cellspacing="0" cellpadding="0" border="0" width="100%" style="background:#FAFAFA;">
    <tr><td align="center" style="padding:32px 16px;">
      <table role="presentation" cellspacing="0" cellpadding="0" border="0" width="600" style="max-width:600px;width:100%;background:#FFFFFF;border-radius:16px;overflow:hidden;border:1px solid #E9EAEB;box-shadow:0 1px 2px rgba(10,13,18,0.05);">

        <!-- ─── Header — image-free POOOL wordmark, mint on brand-blue ─── -->
        <!--
          The wordmark is rendered as live text so:
            * no image hosting / Resend asset CDN required,
            * dark-mode mail clients can't invert our brand,
            * accessibility tools can read it.
          Visual character matches the SVG logo: ultra-bold weight, tight
          tracking, all-caps, the "three O" rhythm that IS the brand mark.
        -->
        <tr><td class="px" style="padding:28px 32px;background:#0000FF;text-align:left;font-family:{brand_font};">
          <table role="presentation" cellspacing="0" cellpadding="0" border="0" width="100%">
            <tr>
              <td align="left" valign="middle" style="font-family:{brand_font};">
                <a href="https://platform.poool.app/" style="text-decoration:none;color:#98FB96;font-weight:900;font-size:26px;letter-spacing:0.04em;line-height:1;font-family:{brand_font};display:inline-block;">POOOL</a>
              </td>
              <td align="right" valign="middle" style="font-family:{brand_font};font-size:10px;font-weight:600;color:#D4FFE9;letter-spacing:0.14em;text-transform:uppercase;line-height:1.4;">
                Tokenised real-asset investing
              </td>
            </tr>
          </table>
        </td></tr>

        <!-- ─── Body ─── -->
        <tr><td class="px pt" style="padding:36px 40px 28px;font-family:{brand_font};font-size:15px;line-height:1.6;color:#181D27;">
{inner}
        </td></tr>

        <!-- ─── Footer ─── -->
        <tr><td class="px" style="padding:24px 32px 32px;background:#FAFAFA;border-top:1px solid #E9EAEB;font-family:{brand_font};font-size:11px;line-height:1.55;color:#535862;">
          {unsubscribe_block}
          <p style="margin:8px 0 4px;font-family:{brand_font};">
            POOOL Capital GmbH · Maximilianstraße 13 · 80539 München · Germany ·
            <a href="mailto:support@poool.app" style="color:#535862;text-decoration:underline;">support@poool.app</a>
          </p>
          <p style="margin:0;color:#717680;font-family:{brand_font};">© POOOL Capital GmbH. All rights reserved.</p>
        </td></tr>

      </table>
    </td></tr>
  </table>
</body>
</html>"#,
        preheader = html_escape_email(opts.preheader),
        inner = inner,
        unsubscribe_block = unsubscribe_block,
        brand_font = BRAND_FONT,
    )
}

/// Renders the event-specific inner content. The outer shell is added by
/// `build_email_html`. New events: add a match arm that returns the inner
/// HTML — the header + footer come for free.
fn build_event_body(event_type: &str, metadata: &serde_json::Value) -> String {
    match event_type {
        // ── Auth / security ───────────────────────────────────────────
        //
        // These five events are dispatched from `auth/service.rs` via
        // dedicated send paths (token outbox, immediate Resend send),
        // not through `trigger_transactional_email`. The bodies are
        // mirrored here so the admin Workflows tab can preview them
        // exactly as the customer sees them. Keep the HTML in sync with
        // the auth service if you change either side.
        "welcome" => {
            let first_name = metadata.get("first_name").and_then(|v| v.as_str()).unwrap_or("there");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Welcome to POOOL, {first}</h2>
  <p>Your account is live. POOOL gives you fractional access to tokenised real estate and other yield-bearing assets, with built-in custody, payouts, and reporting.</p>
  <p>A quick checklist to get the most out of your first session:</p>
  <ul style="color:#414651;line-height:1.7;">
    <li>Complete identity verification (1–2 business days) so you can invest.</li>
    <li>Make a first deposit — wires or SEPA, no card fees.</li>
    <li>Browse the marketplace and follow assets you like.</li>
  </ul>
  <p><a href="https://platform.poool.app/" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Dashboard</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Need help? Reply to this email or visit our <a href="https://platform.poool.app/support" style="color:#0000FF;">support centre</a>.</p>
</div>"#, first = html_escape_email(first_name))
        }

        "verify_email" => {
            let verify_url = metadata.get("verify_url").and_then(|v| v.as_str())
                .unwrap_or("https://platform.poool.app/auth/verify-email?token=...");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Verify your POOOL email</h2>
  <p>Tap the button below to confirm your email address. The link is valid for 24 hours.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Verify Email</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">If you didn't sign up for POOOL, ignore this email — no account will be created without verification.</p>
</div>"#, url = html_escape_email(verify_url))
        }

        "password_reset" => {
            let reset_url = metadata.get("reset_url").and_then(|v| v.as_str())
                .unwrap_or("https://platform.poool.app/auth/reset-password?token=...");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Reset your POOOL password</h2>
  <p>You requested a password reset. Click the link below to set a new password — it expires in 1 hour.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Reset Password</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">If you did not request this reset, ignore this email and your password will stay unchanged. For any concern, contact <a href="mailto:security@poool.app" style="color:#0000FF;">security@poool.app</a>.</p>
</div>"#, url = html_escape_email(reset_url))
        }

        "2fa_setup" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Two-factor authentication is on ✓</h2>
  <p>Your POOOL account is now protected by an authenticator app. From now on you'll be asked for a 6-digit code on every sign-in, plus for sensitive actions like withdrawals and payment method changes.</p>
  <p style="background:#F4F5FF;border-left:3px solid #0000FF;padding:12px 16px;border-radius:4px;color:#344054;font-size:14px;line-height:1.6;">
    <strong>Save your recovery codes.</strong> If you lose access to your authenticator app, recovery codes are the only way back in. Find them in
    <a href="https://platform.poool.app/settings/security" style="color:#0000FF;">Settings → Security</a>.
  </p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Didn't enable 2FA? Sign in and disable it immediately, then contact <a href="mailto:security@poool.app" style="color:#0000FF;">security@poool.app</a>.</p>
</div>"#.to_string(),

        "new_login" => {
            let location = metadata.get("location").and_then(|v| v.as_str()).unwrap_or("a new location");
            let ip = metadata.get("ip").and_then(|v| v.as_str()).unwrap_or("");
            let device = metadata.get("device").and_then(|v| v.as_str()).unwrap_or("a new device");
            let ip_row = if ip.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;width:120px;">IP address</td><td style="padding:8px 0;color:#101828;font-family:ui-monospace,monospace;font-weight:500;">{}</td></tr>"#,
                    html_escape_email(ip))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">New sign-in to your account</h2>
  <p>We noticed a sign-in to your POOOL account from {device}.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:120px;">Location</td><td style="padding:8px 0;color:#101828;font-weight:500;">{location}</td></tr>
    {ip_row}
  </table>
  <p>If this was you, no action needed.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;">
    <strong>Wasn't you?</strong> Sign in and reset your password immediately, then revoke all sessions in
    <a href="https://platform.poool.app/settings/security" style="color:#B42318;text-decoration:underline;">Settings → Security</a>.
  </p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Concerns? Contact <a href="mailto:security@poool.app" style="color:#0000FF;">security@poool.app</a>.</p>
</div>"#,
                device = html_escape_email(device),
                location = html_escape_email(location),
                ip_row = ip_row)
        }

        "kyc_approved" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your identity has been verified ✓</h2>
  <p>Great news — your KYC application has been approved. You can now invest in tokenised assets on POOOL.</p>
  <p><a href="https://platform.poool.app/marketplace" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Browse Assets</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">If you have questions, reply to this email or visit our support centre.</p>
</div>"#.to_string(),

        "kyc_rejected" => {
            let reason = metadata.get("rejection_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Please review the requirements and resubmit.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Action required: KYC resubmission</h2>
  <p>Unfortunately your identity verification could not be approved at this time.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p>Please resubmit your documents addressing the issue above.</p>
  <p><a href="https://platform.poool.app/kyc" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Resubmit Verification</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Need help? Contact us at support@poool.app</p>
</div>"#, reason = html_escape_email(reason))
        }

        "kyc_submitted" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">We received your verification documents</h2>
  <p>Your KYC application is now under review. This typically takes 1–2 business days.</p>
  <p>We'll email you as soon as a decision is made.</p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact us at support@poool.app</p>
</div>"#.to_string(),

        "deposit_confirmed" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let amount_block = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<p style="background:#ECFDF5;border:1px solid #A7F3D0;border-radius:8px;padding:12px 16px;color:#065F46;font-weight:600;">Credited: {}</p>"#,
                    html_escape_email(amount))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your deposit has been received</h2>
  <p>Your wire transfer has been verified and your POOOL wallet balance has been updated.</p>
  {amount}
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Wallet</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact us at support@poool.app</p>
</div>"#, amount = amount_block)
        }

        "deposit_submitted" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let reference = metadata.get("reference").and_then(|v| v.as_str()).unwrap_or("");
            let processing_hours = metadata.get("processing_hours").and_then(|v| v.as_i64()).unwrap_or(24);
            let amount_row = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;width:140px;">Amount</td><td style="padding:8px 0;color:#101828;font-weight:600;">{}</td></tr>"#,
                    html_escape_email(amount))
            };
            let reference_row = if reference.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;">Reference</td><td style="padding:8px 0;color:#101828;font-family:ui-monospace,monospace;font-weight:600;">{}</td></tr>"#,
                    html_escape_email(reference))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">We received your proof of transfer</h2>
  <p>Thanks — your deposit has been submitted and is awaiting verification. Your wallet will be credited within {hours} hours after the wire is received.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    {amount_row}
    {reference_row}
  </table>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Track Deposit</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Make sure the reference above appears on the wire transfer — without it, we cannot match your deposit.</p>
</div>"#, hours = processing_hours, amount_row = amount_row, reference_row = reference_row)
        }

        "withdraw_requested" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let destination = metadata.get("destination").and_then(|v| v.as_str()).unwrap_or("your bank account");
            let amount_row = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;width:140px;">Amount</td><td style="padding:8px 0;color:#101828;font-weight:600;">{}</td></tr>"#,
                    html_escape_email(amount))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Withdrawal request received</h2>
  <p>Your withdrawal is pending admin review. We'll email you again as soon as the funds are released.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    {amount_row}
    <tr><td style="padding:8px 0;color:#717680;">Destination</td><td style="padding:8px 0;color:#101828;font-weight:500;">{dest}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Processing time</td><td style="padding:8px 0;color:#101828;font-weight:500;">1–3 business days</td></tr>
  </table>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Wallet</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">If you did not request this withdrawal, contact support@poool.app immediately.</p>
</div>"#, amount_row = amount_row, dest = html_escape_email(destination))
        }

        "withdraw_approved" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let destination = metadata.get("destination").and_then(|v| v.as_str()).unwrap_or("your bank account");
            let amount_block = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<p style="background:#ECFDF5;border:1px solid #A7F3D0;border-radius:8px;padding:12px 16px;color:#065F46;font-weight:600;">Sent: {}</p>"#,
                    html_escape_email(amount))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Withdrawal sent ✓</h2>
  <p>Your withdrawal has been approved and the funds are on their way to {dest}.</p>
  {amount}
  <p>Bank settlement typically takes 1–3 business days depending on your bank.</p>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Transactions</a></p>
</div>"#, amount = amount_block, dest = html_escape_email(destination))
        }

        "withdraw_rejected" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let reason = metadata.get("admin_notes").and_then(|v| v.as_str()).unwrap_or("Please contact support for details.");
            let amount_block = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<p style="color:#414651;"><strong>Amount:</strong> {}</p>"#,
                    html_escape_email(amount))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Withdrawal could not be processed</h2>
  <p>Unfortunately your withdrawal request was rejected. The held amount has been returned to your wallet balance.</p>
  {amount}
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Wallet</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact support@poool.app</p>
</div>"#, amount = amount_block, reason = html_escape_email(reason))
        }

        "support_ticket_reply" => {
            let subject_line = metadata.get("ticket_subject").and_then(|v| v.as_str()).unwrap_or("your ticket");
            let reply_preview = metadata.get("reply_preview").and_then(|v| v.as_str()).unwrap_or("");
            let ticket_id = metadata.get("ticket_id").and_then(|v| v.as_str()).unwrap_or("");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">You have a new reply on your support ticket</h2>
  <p style="color:#414651;">Our support team has replied to: <strong>{subject}</strong></p>
  {preview_block}
  <p><a href="https://platform.poool.app/support#{ticket_id}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Conversation</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Reply directly in your support portal. Please do not reply to this email.</p>
</div>"#,
                ticket_id = ticket_id,
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
  <h2 style="color:#181D27;">New support ticket submitted</h2>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:120px;">From</td><td style="padding:8px 0;color:#101828;font-weight:500;">{user}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Subject</td><td style="padding:8px 0;color:#101828;font-weight:500;">{subject}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Priority</td><td style="padding:8px 0;color:#101828;font-weight:500;">{priority}</td></tr>
  </table>
  <p><a href="https://platform.poool.app/admin/support.html" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View in Admin Panel</a></p>
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
  <h2 style="color:#181D27;">Your support ticket has been resolved</h2>
  <p style="color:#414651;">We've marked <strong>{subject}</strong> as resolved.</p>
  <p style="color:#414651;">If your issue isn't fully sorted, you can reopen the ticket from your support portal at any time.</p>
  <p><a href="https://platform.poool.app/support" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Ticket</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Thank you for using POOOL support.</p>
</div>"#,
                subject = html_escape_email(subject_line),
            )
        }

        "team_invitation_received" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("a POOOL Affiliate Team");
            let inviter = metadata.get("inviter_name").and_then(|v| v.as_str()).unwrap_or("the team owner");
            let token = metadata.get("token").and_then(|v| v.as_str()).unwrap_or("");
            let accept_url = metadata.get("accept_url").and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("https://platform.poool.app/affiliate/team/accept?token={}", token));
            // Phase-4: per-team branding overrides. Falls back to POOOL
            // wordmark + Electric Blue when team hasn't customised.
            let accent = metadata.get("accent_color").and_then(|v| v.as_str())
                .filter(|s| s.starts_with('#') && s.len() == 7)
                .unwrap_or("#0000FF");
            let logo_url = metadata.get("logo_url").and_then(|v| v.as_str()).unwrap_or("");
            let logo_block = if logo_url.is_empty() {
                String::new()
            } else {
                format!(r#"<div style="margin-bottom:24px;text-align:center;"><img src="{}" alt="{}" style="max-height:48px;max-width:180px;width:auto;height:auto;"/></div>"#,
                    html_escape_email(logo_url), html_escape_email(team_name))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  {logo}
  <h2 style="color:#181D27;">You've been invited to {team}</h2>
  <p>{inviter} has invited you to join <strong>{team}</strong> as a team-affiliate. Commissions from referrals via your business link will route to the team owner, while your personal affiliate link (if any) remains entirely yours.</p>
  <p><a href="{accept}" style="display:inline-block;padding:12px 24px;background:{accent};color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Accept Invitation</a></p>
  <p style="color:#717680;font-size:13px;">Or paste this token in your affiliate dashboard:</p>
  <code style="display:inline-block;background:#F4F4F5;border:1px solid #E9EAEB;padding:8px 12px;border-radius:6px;font-family:monospace;word-break:break-all;">{token}</code>
  <p style="color:#717680;font-size:13px;margin-top:24px;">This invitation expires in 14 days. If you didn't expect this email, you can safely ignore it.</p>
</div>"#,
                logo = logo_block,
                team = html_escape_email(team_name),
                inviter = html_escape_email(inviter),
                accept = html_escape_email(&accept_url),
                accent = html_escape_email(accent),
                token = html_escape_email(token))
        }

        "team_member_approved" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("a POOOL Affiliate Team");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Welcome to {team}</h2>
  <p>You're now an active member of <strong>{team}</strong>. Your business affiliate link is live — commissions from referrals route to the team owner.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
</div>"#, team = html_escape_email(team_name))
        }

        "team_member_removed" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("the POOOL Affiliate Team");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your team membership has ended</h2>
  <p>You've been removed from <strong>{team}</strong>. Your business affiliate link is no longer active. Historical commissions remain with the team owner per program rules. Your personal affiliate link (if any) continues to work independently.</p>
  <p style="color:#717680;font-size:13px;margin-top:24px;">Questions? Contact support@poool.app</p>
</div>"#, team = html_escape_email(team_name))
        }

        "team_self_request_received" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("your team");
            let requester = metadata.get("requester_email").and_then(|v| v.as_str()).unwrap_or("a user");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">New join request for {team}</h2>
  <p><strong>{requester}</strong> has requested to join your affiliate team. Review and approve the request in your team dashboard.</p>
  <p><a href="https://platform.poool.app/developer/affiliate-team" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Review Request</a></p>
</div>"#,
                team = html_escape_email(team_name),
                requester = html_escape_email(requester))
        }

        // Phase-2 P0: inviter is notified when their invitee accepts the
        // invitation. Closes a feedback loop the developer currently misses.
        "team_invitation_accepted" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("your team");
            let member = metadata.get("member_name")
                .or_else(|| metadata.get("member_email"))
                .and_then(|v| v.as_str())
                .unwrap_or("A new member");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">{member} joined {team}</h2>
  <p>Your invitation was accepted. {member} now has an active team-business affiliate link and any commissions they drive route directly to you.</p>
  <p><a href="https://platform.poool.app/developer/affiliate-team/members" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Team Members</a></p>
</div>"#,
                team = html_escape_email(team_name),
                member = html_escape_email(member))
        }

        // ── Affiliate Partner Syndicate lifecycle (Phase-2 P0) ────────────
        //
        // All six events previously fell through to the generic 1-line body
        // ("You have a new notification from POOOL"), which is a CAN-SPAM /
        // UX issue when real money or status changes are involved. Each body
        // below uses the standard POOOL email shell (max-width 600 px,
        // sans-serif, accent #0000FF) and includes a deep-link CTA where
        // applicable. Numeric amounts arrive as integer minor units in
        // metadata (`amount_cents`, `currency`) — formatted server-side.
        "affiliate_application_received" => {
            r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">We received your Partner Syndicate application</h2>
  <p>Thanks for applying to the POOOL Partner Syndicate. Our team reviews new applications within 1–3 business days.</p>
  <p>You'll receive a follow-up email as soon as a decision is made. In the meantime you can continue to use your investor account as usual.</p>
  <p><a href="https://platform.poool.app/affiliate" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Application</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact us at <a href="mailto:partners@poool.app" style="color:#0000FF;">partners@poool.app</a>.</p>
</div>"#.to_string()
        }

        "affiliate_approved" => {
            let tier = metadata.get("tier").and_then(|v| v.as_str()).unwrap_or("Access");
            let rate_bps = metadata.get("commission_rate_bps").and_then(|v| v.as_u64()).unwrap_or(50);
            let rate_pct = format!("{}.{:02}%", rate_bps / 100, rate_bps % 100);
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Welcome to the POOOL Partner Syndicate 🎉</h2>
  <p>Your application has been approved. You're starting at the <strong>{tier}</strong> tier with a commission rate of <strong>{rate}</strong>.</p>
  <p>Your personal affiliate link is ready in your dashboard. Share it to start tracking referrals and earning commissions on qualified investments.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Before your first payout you'll need to upload a valid tax document and confirm your payout details.</p>
</div>"#,
                tier = html_escape_email(tier),
                rate = html_escape_email(&rate_pct))
        }

        "affiliate_rejected" => {
            let reason = metadata.get("reason").and_then(|v| v.as_str())
                .unwrap_or("Our team reviewed your application and could not approve it at this time.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Update on your Partner Syndicate application</h2>
  <p>Thank you for your interest in becoming a POOOL Partner.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;">{reason}</p>
  <p>You're welcome to reapply after addressing the points above. Your investor account remains active and unaffected.</p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions about this decision? Reply to this email or contact <a href="mailto:partners@poool.app" style="color:#0000FF;">partners@poool.app</a>.</p>
</div>"#, reason = html_escape_email(reason))
        }

        "affiliate_suspended" => {
            let reason = metadata.get("reason").and_then(|v| v.as_str())
                .unwrap_or("a compliance review of recent activity");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Urgent: your affiliate account is on hold</h2>
  <p>Your POOOL affiliate account has been temporarily suspended pending {reason}. New referrals and payouts are paused while we complete this review.</p>
  <p>Existing referrals, commissions earned, and any positive balance remain intact and will be released once the account is reinstated.</p>
  <p>Please contact our partner team within 7 days to resolve this:</p>
  <p><a href="mailto:partners@poool.app" style="display:inline-block;padding:12px 24px;background:#B42318;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Contact Partner Team</a></p>
</div>"#, reason = html_escape_email(reason))
        }

        "affiliate_payout_released" => {
            let amount_cents = metadata.get("amount_cents").and_then(|v| v.as_i64()).unwrap_or(0);
            let currency = metadata.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
            let formatted_amount = format!("{} {}.{:02}",
                html_escape_email(currency), amount_cents / 100, (amount_cents.abs() % 100));
            let bank_last4 = metadata.get("bank_last4").and_then(|v| v.as_str()).unwrap_or("");
            let dest_line = if bank_last4.is_empty() {
                "Your POOOL wallet has been credited.".to_string()
            } else {
                format!("Funds are being transferred to your bank account ending in <strong>{}</strong>. Allow 1–3 business days for the SEPA transfer to settle.",
                    html_escape_email(bank_last4))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Payout released: {amount}</h2>
  <p>{dest}</p>
  <p>A detailed statement is available in your affiliate dashboard.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Payout Statement</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">For tax purposes, retain this email as confirmation of payout. Annual statements (1099 / VAT summary) are issued in January.</p>
</div>"#,
                amount = formatted_amount, dest = dest_line)
        }

        "affiliate_commission_earned" => {
            let amount_cents = metadata.get("amount_cents").and_then(|v| v.as_i64()).unwrap_or(0);
            let currency = metadata.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
            let formatted_amount = format!("{} {}.{:02}",
                html_escape_email(currency), amount_cents / 100, (amount_cents.abs() % 100));
            let referred_name = metadata.get("referred_name").and_then(|v| v.as_str()).unwrap_or("a new referral");
            let holdback_days = metadata.get("holdback_days").and_then(|v| v.as_i64()).unwrap_or(30);
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">New commission tracked: {amount}</h2>
  <p>You've earned a commission from a qualified investment by <strong>{referred}</strong>.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:140px;">Commission</td><td style="padding:8px 0;color:#101828;font-weight:600;">{amount}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Status</td><td style="padding:8px 0;color:#101828;">Under holdback ({holdback}-day refund window)</td></tr>
  </table>
  <p>Commissions become payable once the holdback period ends and the underlying investment remains active.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View in Dashboard</a></p>
</div>"#,
                amount = formatted_amount,
                referred = html_escape_email(referred_name),
                holdback = holdback_days)
        }

        // ── Wallet / payouts ──────────────────────────────────────────
        "withdrawal_processed" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let destination = metadata.get("destination").and_then(|v| v.as_str()).unwrap_or("your bank account");
            let amount_block = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<p style="background:#ECFDF5;border:1px solid #A7F3D0;border-radius:8px;padding:12px 16px;color:#065F46;font-weight:600;">Settled: {}</p>"#,
                    html_escape_email(amount))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Withdrawal processed ✓</h2>
  <p>Your withdrawal has been settled and credited to {dest}.</p>
  {amount}
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Transactions</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Keep this email for your records — it is your settlement confirmation.</p>
</div>"#, amount = amount_block, dest = html_escape_email(destination))
        }

        // ── Returns / dividends / statements ──────────────────────────
        "dividend_payout" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("one of your investments");
            let amount_block = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<p style="background:#ECFDF5;border:1px solid #A7F3D0;border-radius:8px;padding:12px 16px;color:#065F46;font-weight:600;">Credited: {}</p>"#,
                    html_escape_email(amount))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">You've earned a dividend</h2>
  <p>A dividend distribution from <strong>{asset}</strong> has just landed in your POOOL wallet.</p>
  {amount}
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Distribution</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Tax-relevant payout statements are available in your annual report.</p>
</div>"#, asset = html_escape_email(asset), amount = amount_block)
        }

        "monthly_statement" => {
            let month = metadata.get("month").and_then(|v| v.as_str()).unwrap_or("the last month");
            let download = metadata.get("download_url").and_then(|v| v.as_str()).unwrap_or("https://platform.poool.app/statements");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your POOOL statement for {month} is ready</h2>
  <p>Your performance, dividends, fees, and tax summary for {month} are now available in your account.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Statement</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Statements are kept for 10 years per regulatory retention rules.</p>
</div>"#, month = html_escape_email(month), url = html_escape_email(download))
        }

        // ── Orders / invoices ─────────────────────────────────────────
        "order_confirmation" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("your investment");
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let order_id = metadata.get("order_id").and_then(|v| v.as_str()).unwrap_or("");
            let amount_row = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;width:140px;">Amount</td><td style="padding:8px 0;color:#101828;font-weight:600;">{}</td></tr>"#,
                    html_escape_email(amount))
            };
            let order_row = if order_id.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;">Order ID</td><td style="padding:8px 0;color:#101828;font-family:ui-monospace,monospace;font-weight:600;">{}</td></tr>"#,
                    html_escape_email(order_id))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Order confirmed</h2>
  <p>Your investment in <strong>{asset}</strong> has been confirmed. Tokens will appear in your portfolio once settlement completes.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    {amount_row}
    {order_row}
  </table>
  <p><a href="https://platform.poool.app/portfolio" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Portfolio</a></p>
</div>"#, asset = html_escape_email(asset), amount_row = amount_row, order_row = order_row)
        }

        "invoice_available" => {
            let download = metadata.get("download_url").and_then(|v| v.as_str()).unwrap_or("https://platform.poool.app/invoices");
            let invoice_no = metadata.get("invoice_number").and_then(|v| v.as_str()).unwrap_or("");
            let header_suffix = if invoice_no.is_empty() {
                String::new()
            } else {
                format!(" #{}", html_escape_email(invoice_no))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Invoice{suffix} is ready</h2>
  <p>Your invoice is available to download. Keep it for your tax records.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Download Invoice (PDF)</a></p>
</div>"#, suffix = header_suffix, url = html_escape_email(download))
        }

        // ── Asset lifecycle ──────────────────────────────────────────
        "asset_funded" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset you follow");
            let asset_url = metadata.get("asset_url").and_then(|v| v.as_str()).unwrap_or("https://platform.poool.app/marketplace");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">{asset} is 100% funded</h2>
  <p><strong>{asset}</strong> has reached its funding target. The primary offering is now closed; the asset moves to operations and (if applicable) the secondary marketplace once settlement completes.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Asset Page</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">You're receiving this because you follow this asset. Manage your watchlist in <a href="https://platform.poool.app/settings" style="color:#0000FF;">settings</a>.</p>
</div>"#, asset = html_escape_email(asset), url = html_escape_email(asset_url))
        }

        // ── Villa-Returns operations lifecycle ───────────────────────
        "operations_rejected" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset");
            let reason = metadata.get("rejection_reason").and_then(|v| v.as_str()).unwrap_or("Please review and resubmit.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Action required: operations submission rejected</h2>
  <p>Your operations submission for <strong>{asset}</strong> could not be approved.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p>Address the points above and resubmit from your developer dashboard.</p>
  <p><a href="https://platform.poool.app/developer/operations" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Operations</a></p>
</div>"#, asset = html_escape_email(asset), reason = html_escape_email(reason))
        }

        "operations_approved" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Operations approved — pending publish</h2>
  <p>Your operations submission for <strong>{asset}</strong> has been reviewed and approved by the compliance team. It will be published live with the next scheduled NAV update.</p>
  <p><a href="https://platform.poool.app/developer/operations" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Submission</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        "operations_published" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Operations published — now live</h2>
  <p>The operations period for <strong>{asset}</strong> has gone live. Investors can now see the latest revenue, occupancy, and NAV figures on the asset page.</p>
  <p><a href="https://platform.poool.app/developer/operations" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Operations</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        // ── Affiliate lifecycle (mirroring direct rewards/* send paths) ──
        "affiliate_commission_qualified" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Commission qualified ✓</h2>
  <p>Great news — the 30-day holdback period for one of your referred investments has ended. The underlying commission has upgraded from <em>under holdback</em> to <strong>payable</strong> and will be included in the next batch payout cycle.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
</div>"#.to_string(),

        "affiliate_application_info_requested" => {
            let message = metadata.get("message").and_then(|v| v.as_str())
                .unwrap_or("Please reply with the additional details requested by our compliance team.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Additional information requested</h2>
  <p>Thank you for applying to the POOOL Partner Syndicate. Before we can complete the review of your application we need a bit more information from you:</p>
  <blockquote style="background:#F4F5FF;border-left:3px solid #0000FF;padding:12px 16px;border-radius:4px;color:#344054;font-size:14px;line-height:1.6;">{message}</blockquote>
  <p>Please reply to this email with the requested details. Your application will remain on file in <em>pending</em> status until we hear back.</p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact us at <a href="mailto:partners@poool.app" style="color:#0000FF;">partners@poool.app</a>.</p>
</div>"#, message = html_escape_email(message))
        }

        "affiliate_tier_promoted" => {
            let new_tier = metadata.get("new_tier").and_then(|v| v.as_str()).unwrap_or("a higher");
            let rate_bps = metadata.get("new_rate_bps").and_then(|v| v.as_u64()).unwrap_or(0);
            let volume_cents = metadata.get("volume_12m_cents").and_then(|v| v.as_i64()).unwrap_or(0);
            let rate_pct = format!("{}.{:02}%", rate_bps / 100, rate_bps % 100);
            let volume_display = format!("${:.2}", (volume_cents as f64) / 100.0);
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Tier upgrade — welcome to {tier} 🎉</h2>
  <p>Based on your qualified referral volume in the last 12 months (<strong>{volume}</strong>), you've been promoted to the <strong>{tier}</strong> tier.</p>
  <p>Your new commission rate is <strong>{rate}</strong> ({bps} bps) and applies to all future commissions. Earnings already accrued at the previous rate are unaffected.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Your Tier</a></p>
</div>"#,
                tier = html_escape_email(new_tier),
                volume = volume_display,
                rate = rate_pct,
                bps = rate_bps)
        }

        "affiliate_tier_demoted" => {
            let previous = metadata.get("previous_tier").and_then(|v| v.as_str()).unwrap_or("your previous tier");
            let new_tier = metadata.get("new_tier").and_then(|v| v.as_str()).unwrap_or("a different");
            let rate_bps = metadata.get("new_rate_bps").and_then(|v| v.as_u64()).unwrap_or(0);
            let volume_cents = metadata.get("volume_12m_cents").and_then(|v| v.as_i64()).unwrap_or(0);
            let rate_pct = format!("{}.{:02}%", rate_bps / 100, rate_bps % 100);
            let volume_display = format!("${:.2}", (volume_cents as f64) / 100.0);
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Tier update</h2>
  <p>Your tier has moved from <strong>{prev}</strong> to <strong>{tier}</strong> based on your qualified referral volume in the last 12 months ({volume}).</p>
  <p>Your new commission rate is <strong>{rate}</strong> ({bps} bps) and applies to all future commissions. Earnings already accrued at the previous rate are unaffected.</p>
  <p>To climb back, focus on qualified referrals — the next-tier threshold is shown in your dashboard.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Dashboard</a></p>
</div>"#,
                prev = html_escape_email(previous),
                tier = html_escape_email(new_tier),
                volume = volume_display,
                rate = rate_pct,
                bps = rate_bps)
        }

        "affiliate_material_approved" => {
            let material = metadata.get("material_name").and_then(|v| v.as_str()).unwrap_or("your custom marketing material");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Marketing material approved ✓</h2>
  <p>Your custom marketing material <strong>{material}</strong> has been reviewed and approved. You may now use it in your campaigns.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
</div>"#, material = html_escape_email(material))
        }

        "affiliate_material_rejected" => {
            let material = metadata.get("material_name").and_then(|v| v.as_str()).unwrap_or("your custom marketing material");
            let reason = metadata.get("reason").and_then(|v| v.as_str()).unwrap_or("No reason provided");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Marketing material requires changes</h2>
  <p>Your custom marketing material <strong>{material}</strong> could not be approved at this time.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p>Please revise and resubmit from your affiliate dashboard.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Resubmit Material</a></p>
</div>"#,
                material = html_escape_email(material),
                reason = html_escape_email(reason))
        }

        // ── Developer-facing ───────────────────────────────────────────
        "developer_project_revision_required" => {
            let project = metadata.get("project_name").and_then(|v| v.as_str()).unwrap_or("your project");
            let notes = metadata.get("revision_notes").and_then(|v| v.as_str())
                .unwrap_or("See the admin review notes in your developer dashboard for details.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Action required: revision needed for {project}</h2>
  <p>The compliance team has reviewed your project submission and requires revisions before it can be approved for the marketplace.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Review notes:</strong> {notes}</p>
  <p><a href="https://platform.poool.app/developer/projects" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Project</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Reach the compliance team at <a href="mailto:compliance@poool.app" style="color:#0000FF;">compliance@poool.app</a>.</p>
</div>"#,
                project = html_escape_email(project),
                notes = html_escape_email(notes))
        }

        // ── Admin-facing (recipient is admin@poool.app) ───────────────
        "admin_invitation" => {
            let invite_url = metadata.get("invite_url").and_then(|v| v.as_str())
                .unwrap_or("https://platform.poool.app/admin/accept-invite?token=...");
            let role = metadata.get("role").and_then(|v| v.as_str()).unwrap_or("admin");
            let inviter = metadata.get("inviter_email").and_then(|v| v.as_str()).unwrap_or("a POOOL admin");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">You've been invited to the POOOL Admin portal</h2>
  <p>{inviter} has invited you to join the POOOL admin team as <strong>{role}</strong>.</p>
  <p>Accept the invitation to set your password and enable two-factor authentication. The link is valid for 72 hours.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Accept Invitation</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">If you weren't expecting this, ignore the email — the invitation expires automatically.</p>
</div>"#,
                inviter = html_escape_email(inviter),
                role = html_escape_email(role),
                url = html_escape_email(invite_url))
        }

        "admin_new_affiliate_application" => {
            let applicant = metadata.get("applicant_email").and_then(|v| v.as_str()).unwrap_or("a new applicant");
            let user_id = metadata.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
            let id_row = if user_id.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;width:140px;">User ID</td><td style="padding:8px 0;color:#101828;font-family:ui-monospace,monospace;font-weight:500;">{}</td></tr>"#,
                    html_escape_email(user_id))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">New affiliate application</h2>
  <p>A new POOOL Partner Syndicate application has been submitted. Please log into the admin portal to review.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:140px;">From</td><td style="padding:8px 0;color:#101828;font-weight:500;">{applicant}</td></tr>
    {id_row}
  </table>
  <p><a href="https://platform.poool.app/admin/affiliate-applications" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Review Application</a></p>
</div>"#,
                applicant = html_escape_email(applicant),
                id_row = id_row)
        }

        "admin_payout_request" => {
            let affiliate_email = metadata.get("affiliate_email").and_then(|v| v.as_str()).unwrap_or("an affiliate");
            let referral_code = metadata.get("referral_code").and_then(|v| v.as_str()).unwrap_or("");
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let amount_row = if amount.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;width:160px;">Requested amount</td><td style="padding:8px 0;color:#101828;font-weight:600;">{}</td></tr>"#,
                    html_escape_email(amount))
            };
            let code_row = if referral_code.is_empty() {
                String::new()
            } else {
                format!(r#"<tr><td style="padding:8px 0;color:#717680;">Referral code</td><td style="padding:8px 0;color:#101828;font-family:ui-monospace,monospace;font-weight:500;">{}</td></tr>"#,
                    html_escape_email(referral_code))
            };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Manual payout request</h2>
  <p>An affiliate has requested a manual payout of their payable commissions. Review and batch in the admin rewards panel.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:160px;">Affiliate</td><td style="padding:8px 0;color:#101828;font-weight:500;">{email}</td></tr>
    {code_row}
    {amount_row}
  </table>
  <p><a href="https://platform.poool.app/admin/rewards" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Pending Payouts</a></p>
</div>"#,
                email = html_escape_email(affiliate_email),
                code_row = code_row,
                amount_row = amount_row)
        }

        "admin_new_marketing_material" => {
            let affiliate = metadata.get("affiliate_email").and_then(|v| v.as_str()).unwrap_or("an affiliate");
            let material = metadata.get("material_name").and_then(|v| v.as_str()).unwrap_or("a custom marketing material");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">New marketing material pending review</h2>
  <p>{affiliate} has uploaded a custom marketing material named <strong>{material}</strong> that requires compliance review before it can be used in campaigns.</p>
  <p><a href="https://platform.poool.app/admin/affiliate-fraud" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Compliance Panel</a></p>
</div>"#,
                affiliate = html_escape_email(affiliate),
                material = html_escape_email(material))
        }

        // ── Account security (audit logs already fire these; wire when send sites exist) ──
        "email_changed" => {
            let old_email = metadata.get("old_email").and_then(|v| v.as_str()).unwrap_or("your previous address");
            let new_email = metadata.get("new_email").and_then(|v| v.as_str()).unwrap_or("a new address");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your POOOL email was changed</h2>
  <p>The email address on your account was changed from <strong>{old}</strong> to <strong>{new}</strong>.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Didn't change this?</strong> Sign in and reset your password immediately, then contact <a href="mailto:security@poool.app" style="color:#B42318;text-decoration:underline;">security@poool.app</a>.</p>
</div>"#, old = html_escape_email(old_email), new = html_escape_email(new_email))
        }

        "password_changed" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your POOOL password was changed</h2>
  <p>Your account password was just updated. If this was you, no further action is needed.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Wasn't you?</strong> Reset your password immediately and revoke all sessions in <a href="https://platform.poool.app/settings/security" style="color:#B42318;text-decoration:underline;">Settings → Security</a>, then contact <a href="mailto:security@poool.app" style="color:#B42318;text-decoration:underline;">security@poool.app</a>.</p>
</div>"#.to_string(),

        "2fa_disabled" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Two-factor authentication was disabled</h2>
  <p>2FA is no longer required to sign in or perform sensitive actions. We strongly recommend re-enabling it — accounts without 2FA have significantly higher takeover risk.</p>
  <p><a href="https://platform.poool.app/settings/security" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Re-enable 2FA</a></p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Didn't disable 2FA?</strong> Contact <a href="mailto:security@poool.app" style="color:#B42318;text-decoration:underline;">security@poool.app</a> immediately.</p>
</div>"#.to_string(),

        "payment_method_added" => {
            let method_type = metadata.get("method_type").and_then(|v| v.as_str()).unwrap_or("payment method");
            let last4 = metadata.get("last4").and_then(|v| v.as_str()).unwrap_or("");
            let suffix = if last4.is_empty() { String::new() } else { format!(" ending in {}", html_escape_email(last4)) };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">A new payment method was added</h2>
  <p>A new {method}{suffix} was added to your POOOL account.</p>
  <p style="color:#717680;font-size:13px;">If you didn't do this, sign in to <a href="https://platform.poool.app/settings/payment-methods" style="color:#0000FF;">Settings → Payment Methods</a> to review and remove it, then contact <a href="mailto:security@poool.app" style="color:#0000FF;">security@poool.app</a>.</p>
</div>"#, method = html_escape_email(method_type), suffix = suffix)
        }

        "payment_method_removed" => {
            let method_type = metadata.get("method_type").and_then(|v| v.as_str()).unwrap_or("payment method");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">A payment method was removed</h2>
  <p>A {method} was removed from your account. Existing deposits and withdrawals are unaffected.</p>
  <p style="color:#717680;font-size:13px;">Didn't do this? <a href="mailto:security@poool.app" style="color:#0000FF;">Contact security</a>.</p>
</div>"#, method = html_escape_email(method_type))
        }

        // ── Compliance / wallet ───────────────────────────────────────
        "large_deposit_received" => {
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let amount_line = if amount.is_empty() { String::new() } else { format!("<p style=\"background:#F4F5FF;border-left:3px solid #0000FF;padding:12px 16px;border-radius:4px;color:#344054;\">Amount: <strong>{}</strong></p>", html_escape_email(amount)) };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Source-of-funds documentation requested</h2>
  <p>Thanks for your deposit. Because of its size, our compliance policy requires us to confirm the source of funds before the deposit can be credited.</p>
  {amount_line}
  <p>Please upload a recent bank statement, salary slip, or other documentation showing the funds' origin.</p>
  <p><a href="https://platform.poool.app/wallet/source-of-funds" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Upload Documentation</a></p>
</div>"#, amount_line = amount_line)
        }

        "compliance_alert_user" => {
            let summary = metadata.get("summary").and_then(|v| v.as_str())
                .unwrap_or("Our compliance team flagged recent activity for review.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Action required on your account</h2>
  <p>{summary}</p>
  <p>Please sign in and follow the on-screen instructions to resolve this.</p>
  <p><a href="https://platform.poool.app/" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open POOOL</a></p>
</div>"#, summary = html_escape_email(summary))
        }

        // ── Marketplace (secondary) ───────────────────────────────────
        "trade_executed" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset");
            let side = metadata.get("side").and_then(|v| v.as_str()).unwrap_or("trade");
            let amount = metadata.get("amount_display").and_then(|v| v.as_str()).unwrap_or("");
            let amount_line = if amount.is_empty() { String::new() } else { format!("<p>Amount: <strong>{}</strong></p>", html_escape_email(amount)) };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Trade executed ✓</h2>
  <p>Your {side} of <strong>{asset}</strong> has been executed on the POOOL marketplace.</p>
  {amount_line}
  <p><a href="https://platform.poool.app/portfolio" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Portfolio</a></p>
</div>"#, side = html_escape_email(side), asset = html_escape_email(asset), amount_line = amount_line)
        }

        "order_filled" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your limit order has been filled</h2>
  <p>Your limit order on <strong>{asset}</strong> matched at your target price.</p>
  <p><a href="https://platform.poool.app/marketplace/orders" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Orders</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        "order_cancelled" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset");
            let reason = metadata.get("reason").and_then(|v| v.as_str()).unwrap_or("");
            let reason_line = if reason.is_empty() { String::new() } else { format!("<p style=\"color:#414651;\">Reason: <em>{}</em></p>", html_escape_email(reason)) };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your order has been cancelled</h2>
  <p>Your order on <strong>{asset}</strong> has been cancelled. Any escrowed funds have been returned to your wallet.</p>
  {reason_line}
</div>"#, asset = html_escape_email(asset), reason_line = reason_line)
        }

        "listing_expired" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your listing has expired</h2>
  <p>Your secondary-market listing for <strong>{asset}</strong> has expired without filling. You can relist anytime.</p>
  <p><a href="https://platform.poool.app/marketplace" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Relist</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        // ── Investment lifecycle ──────────────────────────────────────
        "investment_confirmed" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("your asset");
            let tokens = metadata.get("token_count").and_then(|v| v.as_u64()).unwrap_or(0);
            let token_line = if tokens == 0 { String::new() } else { format!("<p>You now own <strong>{}</strong> fractional tokens of this asset.</p>", tokens) };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your investment in {asset} is live</h2>
  <p>Settlement is complete and your fractional tokens have been minted on-chain.</p>
  {token_line}
  <p><a href="https://platform.poool.app/portfolio" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Position</a></p>
</div>"#, asset = html_escape_email(asset), token_line = token_line)
        }

        "asset_matured" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">{asset} has matured</h2>
  <p>The investment period has ended. Principal plus realised yield is being processed for return to your POOOL wallet.</p>
</div>"#, asset = html_escape_email(asset))
        }

        "dividend_announced" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset");
            let pay_date = metadata.get("pay_date").and_then(|v| v.as_str()).unwrap_or("the upcoming distribution date");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">A dividend has been announced</h2>
  <p>A new distribution has been declared for <strong>{asset}</strong>. Funds will be credited to your wallet on {date}.</p>
</div>"#, asset = html_escape_email(asset), date = html_escape_email(pay_date))
        }

        // ── Tax & legal ───────────────────────────────────────────────
        "tax_document_available" => {
            let year = metadata.get("tax_year").and_then(|v| v.as_str()).unwrap_or("this year");
            let download = metadata.get("download_url").and_then(|v| v.as_str()).unwrap_or("https://platform.poool.app/tax-documents");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your {year} tax document is ready</h2>
  <p>Your annual tax summary for {year} is now available — dividends, realised gains, fees, and withholding tax for your filing.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Download Tax Document (PDF)</a></p>
</div>"#, year = html_escape_email(year), url = html_escape_email(download))
        }

        "terms_updated" => {
            let effective = metadata.get("effective_date").and_then(|v| v.as_str()).unwrap_or("the next billing cycle");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">POOOL Terms of Service updated</h2>
  <p>We've updated our Terms of Service. Changes take effect on <strong>{date}</strong>. Continued use after that date constitutes acceptance.</p>
  <p><a href="https://platform.poool.app/legal/terms" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Review new Terms</a></p>
</div>"#, date = html_escape_email(effective))
        }

        // ── Marketing drips (scheduler stubs in this file) ────────────
        "onboarding_drip_24h" => {
            let first = metadata.get("first_name").and_then(|v| v.as_str()).unwrap_or("there");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Hi {first} — let's get your account live</h2>
  <p>Welcome to POOOL. To start investing, we just need a quick identity verification. Takes about 2 minutes; most users are approved within hours.</p>
  <p><a href="https://platform.poool.app/kyc" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Verify Identity (2 min)</a></p>
</div>"#, first = html_escape_email(first))
        }

        "onboarding_drip_72h" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Need help getting started?</h2>
  <p>We noticed you haven't finished your POOOL onboarding yet. Two steps to your first investment: verify identity, fund wallet (SEPA / wire). Reply to this email if anything blocks you — a real person will help.</p>
  <p><a href="https://platform.poool.app/" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open POOOL</a></p>
</div>"#.to_string(),

        "abandoned_cart" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset you were viewing");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Still thinking about it?</h2>
  <p>You left without completing your investment in <strong>{asset}</strong>. The offering is still open.</p>
  <p><a href="https://platform.poool.app/marketplace" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Return to Marketplace</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        "win_back" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">We miss you 👋</h2>
  <p>It's been a while since your last POOOL visit. New assets, secondary market trading, improved Plus+ rates — see what's new.</p>
  <p><a href="https://platform.poool.app/" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open POOOL</a></p>
</div>"#.to_string(),

        "milestone_first_investment" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("your first POOOL asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Welcome to investing with POOOL 🎉</h2>
  <p>Congratulations on your first investment in <strong>{asset}</strong>. Your fractional ownership is now live on-chain.</p>
  <p><a href="https://platform.poool.app/portfolio" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Portfolio</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        "milestone_anniversary" => {
            let years = metadata.get("years").and_then(|v| v.as_u64()).unwrap_or(1);
            let plural = if years == 1 { "year" } else { "years" };
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Happy POOOL anniversary 🎂</h2>
  <p>It's been <strong>{years} {plural}</strong> since you joined POOOL. Thanks for trusting us with your portfolio.</p>
</div>"#, years = years, plural = plural)
        }

        "weekly_digest" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your POOOL weekly</h2>
  <p>A quick summary of what happened across your POOOL holdings this week — performance, dividends, new offerings.</p>
  <p><a href="https://platform.poool.app/portfolio" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Dashboard</a></p>
</div>"#.to_string(),

        "monthly_affiliate_summary" => r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">Your monthly affiliate summary</h2>
  <p>This month's POOOL Partner Syndicate performance — clicks, signups, qualified investments, commissions earned and pending.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
</div>"#.to_string(),

        "referral_signed_up" => {
            let referred = metadata.get("referred_name").and_then(|v| v.as_str()).unwrap_or("someone you referred");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#181D27;">{referred} just joined POOOL</h2>
  <p>Through your referral link, {referred} created a POOOL account. You'll earn a commission once they complete a qualified investment.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#98FB96;text-decoration:none;border-radius:8px;font-weight:600;">View Referrals</a></p>
</div>"#, referred = html_escape_email(referred))
        }

        _ => format!(
            r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;"><p>You have a new notification from POOOL.</p></div>"#
        ),
    }
}

fn html_escape_email(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Subject-line lookup for a given event-type. Exposed `pub` so the admin
/// workflows + preview endpoints can show the subject without needing to
/// trigger an actual send. Adding a new event? Update the match arm AND
/// the `EVENT_REGISTRY` consumed by the workflows view.
pub fn subject_for_event(event_type: &str) -> &'static str {
    match event_type {
        "welcome" => "Welcome to POOOL!",
        "verify_email" => "Please Verify Your Email",
        "password_reset" => "Password Reset Code",
        "2fa_setup" => "2FA Setup Activated",
        "new_login" => "New Device Login Detected",
        "kyc_approved" => "KYC Application Approved",
        "kyc_rejected" => "KYC Action Required",
        "kyc_submitted" => "KYC Application Received",
        "deposit_submitted" => "Deposit Submitted — Awaiting Verification",
        "deposit_confirmed" => "Deposit Received",
        "withdraw_requested" => "Withdrawal Request Received",
        "withdraw_approved" => "Withdrawal Sent",
        "withdraw_rejected" => "Withdrawal Could Not Be Processed",
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

        // Developer-Team-Affiliate (Phase 2+)
        "team_invitation_received" => "You've been invited to a POOOL Affiliate Team",
        "team_member_approved" => "You're now an active POOOL Affiliate Team member",
        "team_member_removed" => "You were removed from a POOOL Affiliate Team",
        "team_self_request_received" => "New team join request — POOOL Affiliate",
        "team_invitation_accepted" => "Your team invitation was accepted",

        "support_ticket_reply" => "New reply on your support ticket",
        "support_ticket_new" => "New support ticket submitted",
        "support_ticket_resolved" => "Your support ticket has been resolved",

        // Villa-Returns operations lifecycle
        "operations_rejected" => "Action Required: Operations Submission Rejected",
        "operations_approved" => "Operations Approved — Pending Publish",
        "operations_published" => "Operations Published — Now Live",

        // Affiliate lifecycle (mirroring direct send_email paths in rewards/*)
        "affiliate_commission_qualified" => "You earned a new POOOL Affiliate Commission!",
        "affiliate_application_info_requested" => {
            "More information needed for your POOOL Affiliate Application"
        }
        "affiliate_tier_promoted" => "You've been promoted to a new affiliate tier!",
        "affiliate_tier_demoted" => "Your affiliate tier has changed",
        "affiliate_material_approved" => "Your marketing material has been approved",
        "affiliate_material_rejected" => "Your marketing material requires changes",

        // Developer-facing
        "developer_project_revision_required" => "Revision Required for your POOOL project",

        // Admin-facing (sent to admin@poool.app)
        "admin_invitation" => "You have been invited to be a POOOL Admin",
        "admin_new_affiliate_application" => "New Affiliate Application",
        "admin_payout_request" => "Affiliate Commission Payout Request",
        "admin_new_marketing_material" => "New Affiliate Marketing Material Pending Review",

        // Account security — not yet wired to send sites but the audit
        // logs already fire these event types, so emails are a one-line
        // hook-up away.
        "email_changed" => "Your POOOL email address was changed",
        "password_changed" => "Your POOOL password was changed",
        "2fa_disabled" => "Two-factor authentication was disabled on your account",
        "payment_method_added" => "A new payment method was added to your account",
        "payment_method_removed" => "A payment method was removed from your account",

        // Compliance / wallet alerts
        "large_deposit_received" => {
            "Large deposit received — source-of-funds documentation requested"
        }
        "compliance_alert_user" => "Action required on your POOOL account",

        // Marketplace (secondary market)
        "trade_executed" => "Your trade has been executed",
        "order_filled" => "Your limit order has been filled",
        "order_cancelled" => "Your order has been cancelled",
        "listing_expired" => "Your asset listing has expired",

        // Investment lifecycle
        "investment_confirmed" => "Your investment has been confirmed",
        "asset_matured" => "An asset in your portfolio has matured",
        "dividend_announced" => "A dividend has been announced for one of your assets",

        // Tax & legal
        "tax_document_available" => "Your annual tax document is available",
        "terms_updated" => "POOOL Terms of Service updated",

        // Marketing drips (Phase-1 stubs in email.rs scheduler)
        "onboarding_drip_24h" => "Complete your POOOL onboarding in 2 minutes",
        "onboarding_drip_72h" => "Need help getting started with POOOL?",
        "abandoned_cart" => "Your investment is still waiting",
        "win_back" => "We miss you — what's new at POOOL",
        "milestone_first_investment" => "Congrats on your first POOOL investment!",
        "milestone_anniversary" => "Happy POOOL anniversary",
        "weekly_digest" => "Your weekly POOOL summary",
        "monthly_affiliate_summary" => "Your monthly affiliate performance",
        "referral_signed_up" => "Your referral just joined POOOL",

        _ => "You Have a New Notification",
    }
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
    // System-wide workflow toggle. Mandatory events bypass this. A
    // disabled non-mandatory event is silently dropped at the source —
    // no outbox row, no log entry. Defense-in-depth re-check fires at
    // the outbox worker (so toggling mid-flight is honoured too).
    if !crate::common::email::workflow_is_enabled(pool, event_type).await {
        tracing::info!(
            event_type = %event_type,
            user_id = %user_id,
            "Workflow disabled — skipping enqueue."
        );
        return Ok(());
    }

    let subject = subject_for_event(event_type);

    let user_email = match sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE id = $1")
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

// ─── Baseline tests for the transactional email body builder ──────────────
//
// Pure unit tests (no DB / no network). These pin the current behaviour of
// `build_email_html` and `html_escape_email` so subsequent refactors cannot
// silently break a customer-facing email. The catalog test is the canonical
// list of "events that already have a hand-written body" — the inverse list
// (events that still fall through to the generic body) is captured by
// `EVENTS_FALLING_THROUGH_TO_DEFAULT` so the gap is visible in CI rather
// than buried in `email.rs`.

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Events that have a custom HTML body in `build_email_html`. Adding a
    /// new branch to the match arm? Add it here too and the assertion will
    /// confirm it produces something useful.
    const EVENTS_WITH_CUSTOM_BODY: &[&str] = &[
        // Auth / security — dispatched from auth/service.rs in production
        // but mirrored here so the admin Workflows preview works.
        "welcome",
        "verify_email",
        "password_reset",
        "2fa_setup",
        "new_login",
        // KYC
        "kyc_approved",
        "kyc_rejected",
        "kyc_submitted",
        // Wallet
        "deposit_confirmed",
        "deposit_submitted",
        "withdraw_requested",
        "withdraw_approved",
        "withdraw_rejected",
        "withdrawal_processed",
        // Returns / orders
        "dividend_payout",
        "monthly_statement",
        "order_confirmation",
        "invoice_available",
        "asset_funded",
        // Operations
        "operations_rejected",
        "operations_approved",
        "operations_published",
        // Support
        "support_ticket_reply",
        "support_ticket_new",
        "support_ticket_resolved",
        // Team
        "team_invitation_received",
        "team_member_approved",
        "team_member_removed",
        "team_self_request_received",
        "team_invitation_accepted",
        // Affiliate
        "affiliate_application_received",
        "affiliate_approved",
        "affiliate_rejected",
        "affiliate_suspended",
        "affiliate_payout_released",
        "affiliate_commission_earned",
        "affiliate_commission_qualified",
        "affiliate_application_info_requested",
        "affiliate_tier_promoted",
        "affiliate_tier_demoted",
        "affiliate_material_approved",
        "affiliate_material_rejected",
        // Developer
        "developer_project_revision_required",
        // Internal (admin@poool.app)
        "admin_invitation",
        "admin_new_affiliate_application",
        "admin_payout_request",
        "admin_new_marketing_material",
    ];

    /// Events that `trigger_transactional_email` knows a subject for but
    /// that intentionally fall through to the generic body. Currently
    /// empty — every catalogued event has a hand-written body. New
    /// events should land in `EVENTS_WITH_CUSTOM_BODY` from day one.
    const EVENTS_FALLING_THROUGH_TO_DEFAULT: &[&str] = &[];

    /// Sentinel substring of the generic fallback body. If a "custom body"
    /// event suddenly emits this string, the match arm was removed.
    const GENERIC_FALLBACK_MARKER: &str = "You have a new notification from POOOL";

    #[test]
    fn all_known_custom_body_events_render_non_generic_html() {
        for event in EVENTS_WITH_CUSTOM_BODY {
            let html = build_email_html(event, &json!({}));
            assert!(
                !html.contains(GENERIC_FALLBACK_MARKER),
                "event '{}' fell through to generic body — match arm removed?",
                event
            );
            assert!(
                html.contains("<div") || html.contains("<p"),
                "event '{}' produced no HTML block: {:?}",
                event,
                html
            );
        }
    }

    #[test]
    fn fallthrough_events_are_documented() {
        for event in EVENTS_FALLING_THROUGH_TO_DEFAULT {
            let html = build_email_html(event, &json!({}));
            assert!(
                html.contains(GENERIC_FALLBACK_MARKER),
                "event '{}' is in fallthrough list but now has a custom body — \
                 move it into EVENTS_WITH_CUSTOM_BODY",
                event
            );
        }
    }

    #[test]
    fn unknown_event_falls_back_to_generic_notice() {
        let html = build_email_html("this_event_does_not_exist", &json!({}));
        assert!(html.contains(GENERIC_FALLBACK_MARKER));
    }

    #[test]
    fn kyc_rejected_renders_reason_when_provided() {
        let html = build_email_html("kyc_rejected", &json!({ "rejection_reason": "ID expired" }));
        assert!(html.contains("ID expired"));
    }

    #[test]
    fn kyc_rejected_renders_default_reason_when_missing() {
        let html = build_email_html("kyc_rejected", &json!({}));
        assert!(html.contains("Please review the requirements"));
    }

    #[test]
    fn deposit_confirmed_includes_amount_when_provided() {
        let html = build_email_html(
            "deposit_confirmed",
            &json!({ "amount_display": "€1,500.00" }),
        );
        assert!(html.contains("€1,500.00"));
        assert!(html.contains("Credited:"));
    }

    #[test]
    fn deposit_confirmed_skips_amount_block_when_missing() {
        let html = build_email_html("deposit_confirmed", &json!({}));
        assert!(!html.contains("Credited:"));
    }

    #[test]
    fn affiliate_payout_formats_minor_units_as_currency() {
        let html = build_email_html(
            "affiliate_payout_released",
            &json!({ "amount_cents": 12345_i64, "currency": "EUR", "bank_last4": "4242" }),
        );
        assert!(html.contains("EUR 123.45"));
        assert!(html.contains("4242"));
    }

    #[test]
    fn affiliate_commission_uses_default_holdback_when_missing() {
        let html = build_email_html(
            "affiliate_commission_earned",
            &json!({ "amount_cents": 5000_i64, "currency": "EUR" }),
        );
        // Default holdback = 30 days per `build_email_html`.
        assert!(html.contains("30-day refund window"));
    }

    #[test]
    fn team_invitation_uses_provided_accept_url_over_default() {
        let html = build_email_html(
            "team_invitation_received",
            &json!({
                "team_name": "Acme Capital",
                "inviter_name": "Maria",
                "token": "abc123",
                "accept_url": "https://example.test/accept?t=abc123",
            }),
        );
        assert!(html.contains("https://example.test/accept?t=abc123"));
        assert!(html.contains("Acme Capital"));
        assert!(html.contains("Maria"));
    }

    #[test]
    fn html_escape_email_escapes_dangerous_chars() {
        let escaped = html_escape_email(r#"<script>alert("xss")</script> & friends"#);
        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt; &amp; friends"
        );
    }

    #[test]
    fn html_escape_email_is_idempotent_for_safe_text() {
        let plain = "Hello world 12345";
        assert_eq!(html_escape_email(plain), plain);
    }

    #[test]
    fn user_supplied_metadata_is_html_escaped_in_bodies() {
        // Reason fields are user-controlled inputs from admin notes — must
        // not allow HTML injection into the rendered customer email.
        let html = build_email_html(
            "kyc_rejected",
            &json!({ "rejection_reason": "<img src=x onerror=alert(1)>" }),
        );
        assert!(!html.contains("<img src=x"));
        assert!(html.contains("&lt;img src=x"));
    }

    // ── New bodies added in Commit 4 (fallthrough fix) ────────────────

    #[test]
    fn dividend_payout_renders_asset_and_amount() {
        let html = build_email_html(
            "dividend_payout",
            &json!({ "asset_name": "Villa Bali #12", "amount_display": "€42.50" }),
        );
        assert!(html.contains("Villa Bali #12"));
        assert!(html.contains("€42.50"));
        assert!(html.contains("dividend"));
    }

    #[test]
    fn monthly_statement_renders_month_and_link() {
        let html = build_email_html(
            "monthly_statement",
            &json!({ "month": "April 2026", "download_url": "https://x.test/s/123" }),
        );
        assert!(html.contains("April 2026"));
        assert!(html.contains("https://x.test/s/123"));
    }

    #[test]
    fn order_confirmation_skips_optional_blocks_when_missing() {
        let html = build_email_html("order_confirmation", &json!({}));
        assert!(html.contains("Order confirmed"));
        assert!(!html.contains("Order ID"));
        assert!(!html.contains("Amount"));
    }

    #[test]
    fn order_confirmation_includes_amount_and_id_when_present() {
        let html = build_email_html(
            "order_confirmation",
            &json!({
                "asset_name": "Test Asset",
                "amount_display": "$1,000",
                "order_id": "ord-abc123",
            }),
        );
        assert!(html.contains("Test Asset"));
        assert!(html.contains("$1,000"));
        assert!(html.contains("ord-abc123"));
    }

    #[test]
    fn invoice_available_includes_invoice_number_when_present() {
        let html = build_email_html(
            "invoice_available",
            &json!({ "invoice_number": "INV-2026-042" }),
        );
        assert!(html.contains("INV-2026-042"));
    }

    #[test]
    fn invoice_available_works_without_invoice_number() {
        let html = build_email_html("invoice_available", &json!({}));
        assert!(html.contains("Invoice"));
        assert!(html.contains("Download Invoice"));
    }

    #[test]
    fn asset_funded_includes_asset_name_and_url() {
        let html = build_email_html(
            "asset_funded",
            &json!({
                "asset_name": "Penthouse Marbella",
                "asset_url": "https://platform.poool.app/assets/abc",
            }),
        );
        assert!(html.contains("Penthouse Marbella"));
        assert!(html.contains("https://platform.poool.app/assets/abc"));
        assert!(html.contains("100% funded"));
    }

    #[test]
    fn operations_rejected_renders_reason() {
        let html = build_email_html(
            "operations_rejected",
            &json!({
                "asset_name": "Villa X",
                "rejection_reason": "Missing occupancy data for week 32",
            }),
        );
        assert!(html.contains("Villa X"));
        assert!(html.contains("Missing occupancy data for week 32"));
    }

    #[test]
    fn operations_approved_published_render_asset_name() {
        for event in ["operations_approved", "operations_published"] {
            let html = build_email_html(event, &json!({ "asset_name": "Casa Vista" }));
            assert!(
                html.contains("Casa Vista"),
                "event '{event}' should include asset name"
            );
        }
    }

    #[test]
    fn withdrawal_processed_renders_destination_and_amount() {
        let html = build_email_html(
            "withdrawal_processed",
            &json!({ "amount_display": "€500.00", "destination": "DE89 …4567" }),
        );
        assert!(html.contains("€500.00"));
        assert!(html.contains("DE89 …4567"));
        assert!(html.contains("processed"));
    }

    // ── Auth / security event bodies ──────────────────────────────────

    #[test]
    fn welcome_renders_first_name() {
        let html = build_email_html("welcome", &json!({ "first_name": "Maria" }));
        assert!(html.contains("Welcome to POOOL, Maria"));
        assert!(html.contains("identity verification"));
    }

    #[test]
    fn welcome_falls_back_to_there_without_name() {
        let html = build_email_html("welcome", &json!({}));
        assert!(html.contains("Welcome to POOOL, there"));
    }

    #[test]
    fn verify_email_renders_provided_url() {
        let html = build_email_html(
            "verify_email",
            &json!({ "verify_url": "https://x.test/verify?t=abc" }),
        );
        assert!(html.contains("https://x.test/verify?t=abc"));
        assert!(html.contains("24 hours"));
    }

    #[test]
    fn password_reset_renders_provided_url() {
        let html = build_email_html(
            "password_reset",
            &json!({ "reset_url": "https://x.test/reset?t=xyz" }),
        );
        assert!(html.contains("https://x.test/reset?t=xyz"));
        assert!(html.contains("expires in 1 hour"));
    }

    #[test]
    fn twofa_setup_includes_recovery_codes_callout() {
        let html = build_email_html("2fa_setup", &json!({}));
        assert!(html.contains("Two-factor authentication is on"));
        assert!(html.contains("recovery codes"));
    }

    #[test]
    fn new_login_renders_location_device_and_ip() {
        let html = build_email_html(
            "new_login",
            &json!({
                "location": "Munich, DE",
                "ip": "203.0.113.42",
                "device": "Chrome on macOS",
            }),
        );
        assert!(html.contains("Munich, DE"));
        assert!(html.contains("203.0.113.42"));
        assert!(html.contains("Chrome on macOS"));
        assert!(html.contains("Wasn't you"));
    }

    #[test]
    fn new_login_omits_ip_row_when_missing() {
        let html = build_email_html("new_login", &json!({}));
        assert!(html.contains("New sign-in"));
        assert!(!html.contains("IP address"));
    }

    #[test]
    fn auth_event_bodies_html_escape_user_metadata() {
        // Welcome takes first_name straight from the caller — must escape.
        let html = build_email_html(
            "welcome",
            &json!({ "first_name": "<script>alert(1)</script>" }),
        );
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
    }

    // ── Newly catalogued events (direct send_email paths) ─────────────

    #[test]
    fn affiliate_tier_promoted_renders_tier_rate_and_volume() {
        let html = build_email_html(
            "affiliate_tier_promoted",
            &json!({
                "new_tier": "Pro",
                "new_rate_bps": 300_u64,
                "volume_12m_cents": 1_500_000_i64,
            }),
        );
        assert!(html.contains("Pro"));
        assert!(html.contains("3.00%"));
        assert!(html.contains("300 bps"));
        assert!(html.contains("$15000.00"));
    }

    #[test]
    fn affiliate_tier_demoted_shows_previous_and_new_tier() {
        let html = build_email_html(
            "affiliate_tier_demoted",
            &json!({ "previous_tier": "Pro", "new_tier": "Plus", "new_rate_bps": 200_u64 }),
        );
        assert!(html.contains("Pro"));
        assert!(html.contains("Plus"));
        assert!(html.contains("2.00%"));
    }

    #[test]
    fn affiliate_material_rejected_renders_reason_html_escaped() {
        let html = build_email_html(
            "affiliate_material_rejected",
            &json!({
                "material_name": "Q2 banner",
                "reason": "<b>past-performance</b> claims not permitted",
            }),
        );
        assert!(html.contains("Q2 banner"));
        assert!(!html.contains("<b>past-performance</b>"));
        assert!(html.contains("&lt;b&gt;past-performance&lt;/b&gt;"));
    }

    #[test]
    fn developer_revision_required_renders_project_and_notes() {
        let html = build_email_html(
            "developer_project_revision_required",
            &json!({
                "project_name": "Villa Sunset",
                "revision_notes": "Provide land title before resubmit.",
            }),
        );
        assert!(html.contains("Villa Sunset"));
        assert!(html.contains("Provide land title before resubmit."));
    }

    #[test]
    fn admin_invitation_renders_role_and_inviter_and_url() {
        let html = build_email_html(
            "admin_invitation",
            &json!({
                "invite_url": "https://x.test/admin/accept?t=abc",
                "role": "compliance",
                "inviter_email": "ceo@poool.app",
            }),
        );
        assert!(html.contains("https://x.test/admin/accept?t=abc"));
        assert!(html.contains("compliance"));
        assert!(html.contains("ceo@poool.app"));
    }

    #[test]
    fn admin_new_affiliate_application_includes_applicant_email() {
        let html = build_email_html(
            "admin_new_affiliate_application",
            &json!({ "applicant_email": "x@test.test", "user_id": "abc-123" }),
        );
        assert!(html.contains("x@test.test"));
        assert!(html.contains("abc-123"));
    }

    #[test]
    fn admin_payout_request_renders_amount_and_code() {
        let html = build_email_html(
            "admin_payout_request",
            &json!({
                "affiliate_email": "earner@test.test",
                "referral_code": "ACME-2026",
                "amount_display": "€420.00",
            }),
        );
        assert!(html.contains("earner@test.test"));
        assert!(html.contains("ACME-2026"));
        assert!(html.contains("€420.00"));
    }

    #[test]
    fn affiliate_commission_qualified_has_dashboard_cta() {
        let html = build_email_html("affiliate_commission_qualified", &json!({}));
        assert!(html.contains("Commission qualified"));
        assert!(html.contains("Affiliate Dashboard"));
    }

    #[test]
    fn affiliate_application_info_requested_renders_message() {
        let html = build_email_html(
            "affiliate_application_info_requested",
            &json!({ "message": "Send your VAT number." }),
        );
        assert!(html.contains("Send your VAT number."));
    }
}
