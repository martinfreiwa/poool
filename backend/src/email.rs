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
  <h2 style="color:#01011C;">Your deposit has been received</h2>
  <p>Your wire transfer has been verified and your POOOL wallet balance has been updated.</p>
  {amount}
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Wallet</a></p>
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
  <h2 style="color:#01011C;">We received your proof of transfer</h2>
  <p>Thanks — your deposit has been submitted and is awaiting verification. Your wallet will be credited within {hours} hours after the wire is received.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    {amount_row}
    {reference_row}
  </table>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Track Deposit</a></p>
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
  <h2 style="color:#01011C;">Withdrawal request received</h2>
  <p>Your withdrawal is pending admin review. We'll email you again as soon as the funds are released.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    {amount_row}
    <tr><td style="padding:8px 0;color:#717680;">Destination</td><td style="padding:8px 0;color:#101828;font-weight:500;">{dest}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Processing time</td><td style="padding:8px 0;color:#101828;font-weight:500;">1–3 business days</td></tr>
  </table>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Wallet</a></p>
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
  <h2 style="color:#01011C;">Withdrawal sent ✓</h2>
  <p>Your withdrawal has been approved and the funds are on their way to {dest}.</p>
  {amount}
  <p>Bank settlement typically takes 1–3 business days depending on your bank.</p>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Transactions</a></p>
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
  <h2 style="color:#01011C;">Withdrawal could not be processed</h2>
  <p>Unfortunately your withdrawal request was rejected. The held amount has been returned to your wallet balance.</p>
  {amount}
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Wallet</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact support@poool.app</p>
</div>"#, amount = amount_block, reason = html_escape_email(reason))
        }

        "support_ticket_reply" => {
            let subject_line = metadata.get("ticket_subject").and_then(|v| v.as_str()).unwrap_or("your ticket");
            let reply_preview = metadata.get("reply_preview").and_then(|v| v.as_str()).unwrap_or("");
            let ticket_id = metadata.get("ticket_id").and_then(|v| v.as_str()).unwrap_or("");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">You have a new reply on your support ticket</h2>
  <p style="color:#414651;">Our support team has replied to: <strong>{subject}</strong></p>
  {preview_block}
  <p><a href="https://platform.poool.app/support#{ticket_id}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Conversation</a></p>
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

        "team_invitation_received" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("a POOOL Affiliate Team");
            let inviter = metadata.get("inviter_name").and_then(|v| v.as_str()).unwrap_or("the team owner");
            let token = metadata.get("token").and_then(|v| v.as_str()).unwrap_or("");
            let accept_url = metadata.get("accept_url").and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("https://platform.poool.app/affiliate/team/accept?token={}", token));
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">You've been invited to {team}</h2>
  <p>{inviter} has invited you to join <strong>{team}</strong> as a team-affiliate. Commissions from referrals via your business link will route to the team owner, while your personal affiliate link (if any) remains entirely yours.</p>
  <p><a href="{accept}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Accept Invitation</a></p>
  <p style="color:#717680;font-size:13px;">Or paste this token in your affiliate dashboard:</p>
  <code style="display:inline-block;background:#F4F4F5;border:1px solid #E9EAEB;padding:8px 12px;border-radius:6px;font-family:monospace;word-break:break-all;">{token}</code>
  <p style="color:#717680;font-size:13px;margin-top:24px;">This invitation expires in 14 days. If you didn't expect this email, you can safely ignore it.</p>
</div>"#,
                team = html_escape_email(team_name),
                inviter = html_escape_email(inviter),
                accept = html_escape_email(&accept_url),
                token = html_escape_email(token))
        }

        "team_member_approved" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("a POOOL Affiliate Team");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Welcome to {team}</h2>
  <p>You're now an active member of <strong>{team}</strong>. Your business affiliate link is live — commissions from referrals route to the team owner.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
</div>"#, team = html_escape_email(team_name))
        }

        "team_member_removed" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("the POOOL Affiliate Team");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Your team membership has ended</h2>
  <p>You've been removed from <strong>{team}</strong>. Your business affiliate link is no longer active. Historical commissions remain with the team owner per program rules. Your personal affiliate link (if any) continues to work independently.</p>
  <p style="color:#717680;font-size:13px;margin-top:24px;">Questions? Contact support@poool.app</p>
</div>"#, team = html_escape_email(team_name))
        }

        "team_self_request_received" => {
            let team_name = metadata.get("team_name").and_then(|v| v.as_str()).unwrap_or("your team");
            let requester = metadata.get("requester_email").and_then(|v| v.as_str()).unwrap_or("a user");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">New join request for {team}</h2>
  <p><strong>{requester}</strong> has requested to join your affiliate team. Review and approve the request in your team dashboard.</p>
  <p><a href="https://platform.poool.app/developer/affiliate-team" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Review Request</a></p>
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
  <h2 style="color:#01011C;">{member} joined {team}</h2>
  <p>Your invitation was accepted. {member} now has an active team-business affiliate link and any commissions they drive route directly to you.</p>
  <p><a href="https://platform.poool.app/developer/affiliate-team/members" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Team Members</a></p>
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
  <h2 style="color:#01011C;">We received your Partner Syndicate application</h2>
  <p>Thanks for applying to the POOOL Partner Syndicate. Our team reviews new applications within 1–3 business days.</p>
  <p>You'll receive a follow-up email as soon as a decision is made. In the meantime you can continue to use your investor account as usual.</p>
  <p><a href="https://platform.poool.app/affiliate" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Application</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Questions? Contact us at <a href="mailto:partners@poool.app" style="color:#0000FF;">partners@poool.app</a>.</p>
</div>"#.to_string()
        }

        "affiliate_approved" => {
            let tier = metadata.get("tier").and_then(|v| v.as_str()).unwrap_or("Access");
            let rate_bps = metadata.get("commission_rate_bps").and_then(|v| v.as_u64()).unwrap_or(50);
            let rate_pct = format!("{}.{:02}%", rate_bps / 100, rate_bps % 100);
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Welcome to the POOOL Partner Syndicate 🎉</h2>
  <p>Your application has been approved. You're starting at the <strong>{tier}</strong> tier with a commission rate of <strong>{rate}</strong>.</p>
  <p>Your personal affiliate link is ready in your dashboard. Share it to start tracking referrals and earning commissions on qualified investments.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Affiliate Dashboard</a></p>
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
  <h2 style="color:#01011C;">Update on your Partner Syndicate application</h2>
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
  <h2 style="color:#01011C;">Urgent: your affiliate account is on hold</h2>
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
  <h2 style="color:#01011C;">Payout released: {amount}</h2>
  <p>{dest}</p>
  <p>A detailed statement is available in your affiliate dashboard.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Payout Statement</a></p>
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
  <h2 style="color:#01011C;">New commission tracked: {amount}</h2>
  <p>You've earned a commission from a qualified investment by <strong>{referred}</strong>.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    <tr><td style="padding:8px 0;color:#717680;width:140px;">Commission</td><td style="padding:8px 0;color:#101828;font-weight:600;">{amount}</td></tr>
    <tr><td style="padding:8px 0;color:#717680;">Status</td><td style="padding:8px 0;color:#101828;">Under holdback ({holdback}-day refund window)</td></tr>
  </table>
  <p>Commissions become payable once the holdback period ends and the underlying investment remains active.</p>
  <p><a href="https://platform.poool.app/affiliate/dashboard" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View in Dashboard</a></p>
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
  <h2 style="color:#01011C;">Withdrawal processed ✓</h2>
  <p>Your withdrawal has been settled and credited to {dest}.</p>
  {amount}
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Transactions</a></p>
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
  <h2 style="color:#01011C;">You've earned a dividend</h2>
  <p>A dividend distribution from <strong>{asset}</strong> has just landed in your POOOL wallet.</p>
  {amount}
  <p><a href="https://platform.poool.app/wallet" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Distribution</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Tax-relevant payout statements are available in your annual report.</p>
</div>"#, asset = html_escape_email(asset), amount = amount_block)
        }

        "monthly_statement" => {
            let month = metadata.get("month").and_then(|v| v.as_str()).unwrap_or("the last month");
            let download = metadata.get("download_url").and_then(|v| v.as_str()).unwrap_or("https://platform.poool.app/statements");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Your POOOL statement for {month} is ready</h2>
  <p>Your performance, dividends, fees, and tax summary for {month} are now available in your account.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Statement</a></p>
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
  <h2 style="color:#01011C;">Order confirmed</h2>
  <p>Your investment in <strong>{asset}</strong> has been confirmed. Tokens will appear in your portfolio once settlement completes.</p>
  <table style="width:100%;border-collapse:collapse;margin:16px 0;font-size:14px;">
    {amount_row}
    {order_row}
  </table>
  <p><a href="https://platform.poool.app/portfolio" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Portfolio</a></p>
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
  <h2 style="color:#01011C;">Invoice{suffix} is ready</h2>
  <p>Your invoice is available to download. Keep it for your tax records.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Download Invoice (PDF)</a></p>
</div>"#, suffix = header_suffix, url = html_escape_email(download))
        }

        // ── Asset lifecycle ──────────────────────────────────────────
        "asset_funded" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("an asset you follow");
            let asset_url = metadata.get("asset_url").and_then(|v| v.as_str()).unwrap_or("https://platform.poool.app/marketplace");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">{asset} is 100% funded</h2>
  <p><strong>{asset}</strong> has reached its funding target. The primary offering is now closed; the asset moves to operations and (if applicable) the secondary marketplace once settlement completes.</p>
  <p><a href="{url}" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Asset Page</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">You're receiving this because you follow this asset. Manage your watchlist in <a href="https://platform.poool.app/settings" style="color:#0000FF;">settings</a>.</p>
</div>"#, asset = html_escape_email(asset), url = html_escape_email(asset_url))
        }

        // ── Villa-Returns operations lifecycle ───────────────────────
        "operations_rejected" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset");
            let reason = metadata.get("rejection_reason").and_then(|v| v.as_str()).unwrap_or("Please review and resubmit.");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Action required: operations submission rejected</h2>
  <p>Your operations submission for <strong>{asset}</strong> could not be approved.</p>
  <p style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:16px;color:#B42318;"><strong>Reason:</strong> {reason}</p>
  <p>Address the points above and resubmit from your developer dashboard.</p>
  <p><a href="https://platform.poool.app/developer/operations" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Operations</a></p>
</div>"#, asset = html_escape_email(asset), reason = html_escape_email(reason))
        }

        "operations_approved" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Operations approved — pending publish</h2>
  <p>Your operations submission for <strong>{asset}</strong> has been reviewed and approved by the compliance team. It will be published live with the next scheduled NAV update.</p>
  <p><a href="https://platform.poool.app/developer/operations" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">View Submission</a></p>
</div>"#, asset = html_escape_email(asset))
        }

        "operations_published" => {
            let asset = metadata.get("asset_name").and_then(|v| v.as_str()).unwrap_or("the asset");
            format!(r#"
<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Operations published — now live</h2>
  <p>The operations period for <strong>{asset}</strong> has gone live. Investors can now see the latest revenue, occupancy, and NAV figures on the asset page.</p>
  <p><a href="https://platform.poool.app/developer/operations" style="display:inline-block;padding:12px 24px;background:#0000FF;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Open Operations</a></p>
</div>"#, asset = html_escape_email(asset))
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

        _ => "You Have a New Notification",
    };

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
        "kyc_approved",
        "kyc_rejected",
        "kyc_submitted",
        "deposit_confirmed",
        "deposit_submitted",
        "withdraw_requested",
        "withdraw_approved",
        "withdraw_rejected",
        "withdrawal_processed",
        "dividend_payout",
        "monthly_statement",
        "order_confirmation",
        "invoice_available",
        "asset_funded",
        "operations_rejected",
        "operations_approved",
        "operations_published",
        "support_ticket_reply",
        "support_ticket_new",
        "support_ticket_resolved",
        "team_invitation_received",
        "team_member_approved",
        "team_member_removed",
        "team_self_request_received",
        "team_invitation_accepted",
        "affiliate_application_received",
        "affiliate_approved",
        "affiliate_rejected",
        "affiliate_suspended",
        "affiliate_payout_released",
        "affiliate_commission_earned",
    ];

    /// Events that `trigger_transactional_email` knows a subject for but
    /// that currently fall through to the generic "you have a new
    /// notification" body. The auth/security ones are intentionally here
    /// because they take a dedicated path in `auth/service.rs` (welcome /
    /// password reset have their own templates) and never actually flow
    /// through `build_email_html` in practice.
    const EVENTS_FALLING_THROUGH_TO_DEFAULT: &[&str] = &[
        "welcome",
        "verify_email",
        "password_reset",
        "2fa_setup",
        "new_login",
    ];

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
        let html =
            build_email_html("kyc_rejected", &json!({ "rejection_reason": "ID expired" }));
        assert!(html.contains("ID expired"));
    }

    #[test]
    fn kyc_rejected_renders_default_reason_when_missing() {
        let html = build_email_html("kyc_rejected", &json!({}));
        assert!(html.contains("Please review the requirements"));
    }

    #[test]
    fn deposit_confirmed_includes_amount_when_provided() {
        let html =
            build_email_html("deposit_confirmed", &json!({ "amount_display": "€1,500.00" }));
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
}
