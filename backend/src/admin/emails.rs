use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use sqlx::Row;

/// Static catalog of every transactional event the platform emits.
///
/// Powers the Workflows tab in the admin email-marketing page so an admin
/// can see at a glance which events have wired bodies, what category they
/// belong to, what sample metadata they accept, and whether they're
/// optional (opt-out-able by the recipient) or mandatory.
///
/// Tuple shape: (event_type, category, summary, sample_metadata_json).
/// `sample_metadata_json` is consumed by the preview endpoint as the
/// default render context when no overrides are supplied. Keep the JSON
/// shape in sync with what `build_email_html` reads.
pub const EVENT_REGISTRY: &[(&str, &str, &str, &str)] = &[
    // ── Auth / security (handled by dedicated auth/service.rs paths) ──
    (
        "welcome",
        "Auth",
        "Sent after a new user verifies their email.",
        r#"{"first_name":"Maria"}"#,
    ),
    (
        "verify_email",
        "Auth",
        "One-time verification link for a new sign-up.",
        r#"{"verify_url":"https://platform.poool.app/verify?t=..."}"#,
    ),
    (
        "password_reset",
        "Auth",
        "Password reset code (separate outbox for retry).",
        r#"{"reset_url":"https://platform.poool.app/reset?t=..."}"#,
    ),
    (
        "2fa_setup",
        "Auth",
        "Confirmation that the user enrolled in 2FA.",
        r#"{}"#,
    ),
    (
        "new_login",
        "Auth",
        "Notice for a sign-in from a new device/location.",
        r#"{"location":"Munich, DE","ip":"203.0.113.42"}"#,
    ),
    // ── KYC ──────────────────────────────────────────────────────────
    (
        "kyc_submitted",
        "KYC",
        "Receipt confirmation after documents are uploaded.",
        r#"{}"#,
    ),
    (
        "kyc_approved",
        "KYC",
        "Identity verified — user can now invest.",
        r#"{}"#,
    ),
    (
        "kyc_rejected",
        "KYC",
        "Resubmission required (carries reason from compliance).",
        r#"{"rejection_reason":"ID photo too dark — please retake in daylight."}"#,
    ),
    // ── Wallet ───────────────────────────────────────────────────────
    (
        "deposit_submitted",
        "Wallet",
        "Proof of wire uploaded, awaiting verification.",
        r#"{"amount_display":"€2,500.00","reference":"POOOL-7F3A2B","processing_hours":24}"#,
    ),
    (
        "deposit_confirmed",
        "Wallet",
        "Wire confirmed and wallet credited.",
        r#"{"amount_display":"€2,500.00"}"#,
    ),
    (
        "withdraw_requested",
        "Wallet",
        "Withdrawal pending admin review.",
        r#"{"amount_display":"€500.00","destination":"DE89 …4567"}"#,
    ),
    (
        "withdraw_approved",
        "Wallet",
        "Withdrawal approved and SEPA dispatched.",
        r#"{"amount_display":"€500.00","destination":"DE89 …4567"}"#,
    ),
    (
        "withdraw_rejected",
        "Wallet",
        "Withdrawal rejected (funds returned to wallet).",
        r#"{"amount_display":"€500.00","admin_notes":"Beneficiary name mismatch."}"#,
    ),
    (
        "withdrawal_processed",
        "Wallet",
        "Bank settled — final confirmation.",
        r#"{"amount_display":"€500.00","destination":"DE89 …4567"}"#,
    ),
    // ── Returns / orders / invoices ─────────────────────────────────
    (
        "dividend_payout",
        "Returns",
        "Dividend credited to wallet.",
        r#"{"asset_name":"Villa Bali #12","amount_display":"€42.50"}"#,
    ),
    (
        "monthly_statement",
        "Returns",
        "Monthly performance/tax summary ready.",
        r#"{"month":"April 2026","download_url":"https://platform.poool.app/statements/2026-04"}"#,
    ),
    (
        "order_confirmation",
        "Orders",
        "Investment order confirmed.",
        r#"{"asset_name":"Penthouse Marbella","amount_display":"€1,000.00","order_id":"ord-7F3A2B"}"#,
    ),
    (
        "invoice_available",
        "Orders",
        "PDF invoice ready for download.",
        r#"{"invoice_number":"INV-2026-042","download_url":"https://platform.poool.app/invoices/INV-2026-042"}"#,
    ),
    (
        "asset_funded",
        "Assets",
        "An asset the user follows hit 100% funding.",
        r#"{"asset_name":"Penthouse Marbella","asset_url":"https://platform.poool.app/assets/penthouse-marbella"}"#,
    ),
    // ── Affiliate Partner Syndicate ─────────────────────────────────
    (
        "affiliate_application_received",
        "Affiliate",
        "Application acknowledged, review pending.",
        r#"{}"#,
    ),
    (
        "affiliate_approved",
        "Affiliate",
        "Application approved — link is live.",
        r#"{"tier":"Access","commission_rate_bps":50}"#,
    ),
    (
        "affiliate_rejected",
        "Affiliate",
        "Application rejected with reason.",
        r#"{"reason":"Audience does not align with POOOL investor profile."}"#,
    ),
    (
        "affiliate_suspended",
        "Affiliate",
        "Account on hold pending compliance review.",
        r#"{"reason":"Unusual referral concentration"}"#,
    ),
    (
        "affiliate_payout_released",
        "Affiliate",
        "Commission payout dispatched.",
        r#"{"amount_cents":12345,"currency":"EUR","bank_last4":"4567"}"#,
    ),
    (
        "affiliate_commission_earned",
        "Affiliate",
        "New commission tracked from referred investment.",
        r#"{"amount_cents":5000,"currency":"EUR","referred_name":"Anna L.","holdback_days":30}"#,
    ),
    // ── Developer-Team Affiliate ────────────────────────────────────
    (
        "team_invitation_received",
        "Team",
        "Invited to join a developer's affiliate team.",
        r#"{"team_name":"Acme Capital","inviter_name":"Maria","token":"abc123"}"#,
    ),
    (
        "team_member_approved",
        "Team",
        "Self-request to join a team was approved.",
        r#"{"team_name":"Acme Capital"}"#,
    ),
    (
        "team_member_removed",
        "Team",
        "Membership ended.",
        r#"{"team_name":"Acme Capital"}"#,
    ),
    (
        "team_self_request_received",
        "Team",
        "Inviter notified of a new join request.",
        r#"{"team_name":"Acme Capital","requester_email":"new@partner.test"}"#,
    ),
    (
        "team_invitation_accepted",
        "Team",
        "Inviter notified that their invitation was accepted.",
        r#"{"team_name":"Acme Capital","member_name":"Maria"}"#,
    ),
    // ── Support ─────────────────────────────────────────────────────
    (
        "support_ticket_reply",
        "Support",
        "Customer-facing: new reply on a ticket.",
        r#"{"ticket_subject":"KYC review status","reply_preview":"Hi — your documents are under review. We expect a decision within 24h.","ticket_id":"tk-123"}"#,
    ),
    (
        "support_ticket_new",
        "Support",
        "Admin-facing: new ticket submitted.",
        r#"{"ticket_subject":"Cannot deposit","user_email":"user@example.test","priority":"high"}"#,
    ),
    (
        "support_ticket_resolved",
        "Support",
        "Customer-facing: ticket marked resolved.",
        r#"{"ticket_subject":"KYC review status"}"#,
    ),
    // ── Villa-Returns operations ────────────────────────────────────
    (
        "operations_rejected",
        "Operations",
        "Monthly operations submission rejected.",
        r#"{"asset_name":"Villa Bali #12","rejection_reason":"Missing occupancy data for week 32"}"#,
    ),
    (
        "operations_approved",
        "Operations",
        "Operations approved, awaiting publish.",
        r#"{"asset_name":"Villa Bali #12"}"#,
    ),
    (
        "operations_published",
        "Operations",
        "Operations live on the asset page.",
        r#"{"asset_name":"Villa Bali #12"}"#,
    ),
    // ── Admin-triggered marketing ───────────────────────────────────
    (
        "marketing_campaign",
        "Marketing",
        "Custom template sent to a user segment.",
        r#"{"first_name":"Maria","email":"maria@example.test"}"#,
    ),
    // ── Affiliate lifecycle (direct-send paths bypassing trigger_transactional_email) ──
    (
        "affiliate_commission_qualified",
        "Affiliate",
        "Holdback period ended — commission is now payable.",
        r#"{}"#,
    ),
    (
        "affiliate_application_info_requested",
        "Affiliate",
        "Compliance needs more information before approving the application.",
        r#"{"message":"Please share the URL of your primary investment-content channel."}"#,
    ),
    (
        "affiliate_tier_promoted",
        "Affiliate",
        "Affiliate volume crossed a tier threshold — rate goes up.",
        r#"{"new_tier":"Pro","new_rate_bps":300,"volume_12m_cents":1500000}"#,
    ),
    (
        "affiliate_tier_demoted",
        "Affiliate",
        "Affiliate volume dropped — tier rebalanced down.",
        r#"{"previous_tier":"Pro","new_tier":"Plus","new_rate_bps":200,"volume_12m_cents":800000}"#,
    ),
    (
        "affiliate_material_approved",
        "Affiliate",
        "Custom marketing material approved by compliance.",
        r#"{"material_name":"Q2 Bali Villas banner"}"#,
    ),
    (
        "affiliate_material_rejected",
        "Affiliate",
        "Custom marketing material rejected — needs revision.",
        r#"{"material_name":"Q2 Bali Villas banner","reason":"Includes unapproved past-performance figures."}"#,
    ),
    // ── Developer-facing ───────────────────────────────────────────
    (
        "developer_project_revision_required",
        "Developer",
        "Compliance flagged the submitted project for revisions before publish.",
        r#"{"project_name":"Penthouse Marbella","revision_notes":"Provide notarised land title before resubmit."}"#,
    ),
    // ── Internal (recipient is admin@poool.app, not a customer) ───
    (
        "admin_invitation",
        "Internal",
        "New admin invited to the platform — internal sign-up email.",
        r#"{"invite_url":"https://platform.poool.app/admin/accept-invite?token=...","role":"compliance","inviter_email":"founder@poool.app"}"#,
    ),
    (
        "admin_new_affiliate_application",
        "Internal",
        "Routed to ops: a new Partner Syndicate application is waiting.",
        r#"{"applicant_email":"new@partner.test","user_id":"00000000-0000-0000-0000-000000000000"}"#,
    ),
    (
        "admin_payout_request",
        "Internal",
        "Affiliate manually requested a payout — ops review needed.",
        r#"{"affiliate_email":"earner@partner.test","referral_code":"ACME-2026","amount_display":"€420.00"}"#,
    ),
    (
        "admin_new_marketing_material",
        "Internal",
        "Affiliate uploaded a custom marketing asset — compliance review needed.",
        r#"{"affiliate_email":"earner@partner.test","material_name":"Q2 Bali Villas banner"}"#,
    ),
];

//
//  Admin Email Marketing API
//

/// Maximum marketing campaigns that can be triggered in a rolling hour
/// across the whole system. A typo or runaway script multiplied by the
/// `all` audience would otherwise mail every user multiple times.
const CAMPAIGN_RATE_LIMIT_PER_HOUR: i64 = 5;

/// Event-type used for outbox rows produced by the admin campaign tool.
/// Classified as optional in `is_optional_email_event` so List-Unsubscribe
/// and `email_notifications=false` are honoured per recipient.
pub const MARKETING_CAMPAIGN_EVENT_TYPE: &str = "marketing_campaign";

/// GET /api/admin/emails — list templates (without bulky HTML), KPI stats, recent logs.
pub async fn api_admin_emails(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    // 1. Templates — exclude the heavy HTML column so the list endpoint
    // stays small. The detail endpoint (`GET /:id`) returns the full body.
    let t_rows = sqlx::query(
        "SELECT id::text, name, subject, version, description, updated_at::text, 'transactional' as type
         FROM email_templates ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let templates: Vec<serde_json::Value> = t_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"),
                "subject": r.get::<String, _>("subject"), "version": r.get::<i32, _>("version"),
                "description": r.get::<Option<String>, _>("description"),
                "updated_at": r.get::<String, _>("updated_at"),
                "type": r.get::<String, _>("type")
            })
        })
        .collect();

    // 2. KPI aggregates — current 30d window + previous 30d window so the
    // dashboard delivery trend is real instead of a hard-coded zero.
    let stats_row = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days')::bigint
                AS total_sent,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status IN ('delivered', 'opened', 'clicked'))::bigint
                AS total_delivered,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status IN ('opened', 'clicked'))::bigint
                AS total_opened,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status = 'clicked')::bigint
                AS total_clicked,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status = 'bounced')::bigint
                AS total_bounced,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '60 days'
                             AND sent_at <  NOW() - INTERVAL '30 days')::bigint
                AS prev_total_sent,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '60 days'
                             AND sent_at <  NOW() - INTERVAL '30 days'
                             AND status IN ('delivered', 'opened', 'clicked'))::bigint
                AS prev_total_delivered
        FROM email_logs
        WHERE status != 'queued'
        "#
    )
    .fetch_one(&state.db)
    .await;

    let (
        total_sent,
        total_delivered,
        total_opened,
        total_clicked,
        total_bounced,
        prev_total_sent,
        prev_total_delivered,
    ) = match stats_row {
        Ok(r) => (
            r.total_sent.unwrap_or(0),
            r.total_delivered.unwrap_or(0),
            r.total_opened.unwrap_or(0),
            r.total_clicked.unwrap_or(0),
            r.total_bounced.unwrap_or(0),
            r.prev_total_sent.unwrap_or(0),
            r.prev_total_delivered.unwrap_or(0),
        ),
        Err(_) => (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
    };

    let delivery_rate = if total_sent > 0 {
        (total_delivered as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };
    let prev_delivery_rate = if prev_total_sent > 0 {
        (prev_total_delivered as f64 / prev_total_sent as f64) * 100.0
    } else {
        0.0
    };
    let delivery_trend = delivery_rate - prev_delivery_rate;

    let open_rate = if total_delivered > 0 {
        (total_opened as f64 / total_delivered as f64) * 100.0
    } else {
        0.0
    };
    let click_rate = if total_opened > 0 {
        (total_clicked as f64 / total_opened as f64) * 100.0
    } else {
        0.0
    };
    let bounce_rate = if total_sent > 0 {
        (total_bounced as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    let stats = serde_json::json!({
        "deliveryRate": (delivery_rate * 10.0).round() / 10.0,
        "deliveryTrend": (delivery_trend * 10.0).round() / 10.0,
        "openRate": (open_rate * 10.0).round() / 10.0,
        "clickRate": (click_rate * 10.0).round() / 10.0,
        "bounceRate": (bounce_rate * 10.0).round() / 10.0,
        "bouncesTotal": total_bounced,
        "totalSent": total_sent
    });

    // 3. Recent delivery logs
    let log_rows = sqlx::query(
        r#"SELECT e.id::text, e.subject, e.recipient_email, e.status, e.sent_at::text
           FROM email_logs e
           ORDER BY e.sent_at DESC LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let logs: Vec<serde_json::Value> = log_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"), "subject": r.get::<String, _>("subject"),
                "recipient_email": r.get::<String, _>("recipient_email"),
                "status": r.get::<String, _>("status"), "sent_at": r.get::<String, _>("sent_at"),
            })
        })
        .collect();

    Ok(
        Json(serde_json::json!({ "templates": templates, "stats": stats, "logs": logs }))
            .into_response(),
    )
}

/// GET /api/admin/emails/templates/:id — full template incl. HTML body.
pub async fn api_admin_emails_get(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;
    let uid = ApiError::parse_uuid(&id)?;

    let row = sqlx::query(
        "SELECT id::text, name, subject, html_template, version, description, updated_at::text
         FROM email_templates WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::from)?;

    let Some(r) = row else {
        return Err(ApiError::NotFound("Template not found".to_string()));
    };

    Ok(Json(serde_json::json!({
        "id": r.get::<String, _>("id"),
        "name": r.get::<String, _>("name"),
        "subject": r.get::<String, _>("subject"),
        "html_template": r.get::<String, _>("html_template"),
        "version": r.get::<i32, _>("version"),
        "description": r.get::<Option<String>, _>("description"),
        "updated_at": r.get::<String, _>("updated_at"),
    }))
    .into_response())
}

/// POST /api/admin/emails/templates
pub async fn api_admin_emails_create(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;

    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let subject = body.get("subject").and_then(|v| v.as_str()).unwrap_or("");
    let description = body
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let html_template = body
        .get("html_template")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if name.is_empty() || subject.is_empty() || html_template.is_empty() {
        return Err(ApiError::BadRequest("Missing required fields".to_string()));
    }

    let result = sqlx::query(
        "INSERT INTO email_templates (name, subject, html_template, description, version)
         VALUES ($1, $2, $3, $4, 1) RETURNING id",
    )
    .bind(name)
    .bind(subject)
    .bind(html_template)
    .bind(description)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"created"})).into_response()),
        Err(e) => {
            // Unique-name collision is a user-fixable BadRequest, not a 500.
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Err(ApiError::Conflict(format!(
                        "Template name '{name}' already exists"
                    )));
                }
            }
            tracing::error!("Failed to create template: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// PUT /api/admin/emails/templates/:id
pub async fn api_admin_emails_update(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;
    let uid = ApiError::parse_uuid(&id)?;

    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let subject = body.get("subject").and_then(|v| v.as_str()).unwrap_or("");
    let description = body
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let html_template = body
        .get("html_template")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if name.is_empty() || subject.is_empty() || html_template.is_empty() {
        return Err(ApiError::BadRequest("Missing required fields".to_string()));
    }

    let result = sqlx::query(
        "UPDATE email_templates SET name = $1, subject = $2, html_template = $3,
         description = $4, version = version + 1, updated_at = NOW()
         WHERE id = $5 RETURNING id",
    )
    .bind(name)
    .bind(subject)
    .bind(html_template)
    .bind(description)
    .bind(uid)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(_)) => Ok(Json(serde_json::json!({"status":"updated"})).into_response()),
        Ok(None) => Err(ApiError::NotFound("Template not found".to_string())),
        Err(e) => {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Err(ApiError::Conflict(format!(
                        "Template name '{name}' already exists"
                    )));
                }
            }
            tracing::error!("Failed to update template: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// DELETE /api/admin/emails/templates/:id — hard delete. `email_logs.template_id`
/// has `ON DELETE SET NULL` so historical logs are preserved.
pub async fn api_admin_emails_delete(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;
    let uid = ApiError::parse_uuid(&id)?;

    let res = sqlx::query("DELETE FROM email_templates WHERE id = $1")
        .bind(uid)
        .execute(&state.db)
        .await
        .map_err(ApiError::from)?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("Template not found".to_string()));
    }

    Ok(Json(serde_json::json!({"status":"deleted"})).into_response())
}

/// SQL fragment selecting `(id, email, first_name)` triples for a given
/// audience segment. `first_name` is left-joined from `user_profiles` and
/// returned as empty string when missing, so the campaign render context
/// can always interpolate `{{first_name}}` without a NULL panic.
///
/// Centralised so the campaign endpoint and the recipient-count preview
/// stay in sync — and so the queries can be regression-tested in one place.
///
/// Returns `None` for an unknown segment (caller maps that to BadRequest).
fn audience_query(segment: &str) -> Option<&'static str> {
    match segment {
        "all" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE",
        ),
        "investors" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND EXISTS (SELECT 1 FROM investments i WHERE i.user_id = u.id)",
        ),
        "kyc_approved" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND EXISTS (
                  SELECT 1 FROM kyc_records k
                  WHERE k.user_id = u.id AND k.status = 'approved'
                )",
        ),
        // Tier 'Plus' has sort_order = 2 (Intro=1, Plus=2, Pro=3, Elite=4, Premium=5).
        // "Plus and above" = sort_order >= 2.
        "tier_plus" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
               JOIN user_tiers ut ON ut.user_id = u.id
               JOIN tiers t ON t.id = ut.tier_id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND t.sort_order >= 2",
        ),
        // Dormant = no successful login in the past 30 days. `user_sessions`
        // is the source of truth (one row per login). Users with no session
        // at all are also dormant.
        "dormant" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND NOT EXISTS (
                  SELECT 1 FROM user_sessions s
                  WHERE s.user_id = u.id
                    AND s.created_at >= NOW() - INTERVAL '30 days'
                )",
        ),
        _ => None,
    }
}

/// POST /api/admin/emails/campaigns
///
/// Resolves the template + audience, then enqueues one row per recipient
/// into `transactional_email_outbox`. The existing outbox worker delivers
/// via Resend, retries with backoff, and honours per-user opt-out via
/// `is_optional_email_event` — campaigns are classified optional, so users
/// who toggled off `email_notifications` (or used the inbox unsubscribe
/// button) are skipped at send time.
///
/// Rate-limited globally to `CAMPAIGN_RATE_LIMIT_PER_HOUR` campaigns in a
/// rolling hour so a runaway script cannot mail the entire user base
/// repeatedly.
pub async fn api_admin_emails_campaign(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.send").await?;

    let template_id = body
        .get("templateId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let audience = body
        .get("audience")
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    if template_id.is_empty() {
        return Err(ApiError::BadRequest("Template ID required".to_string()));
    }

    let Some(audience_sql) = audience_query(audience) else {
        return Err(ApiError::BadRequest(format!(
            "Unknown audience segment: {audience}"
        )));
    };

    let uid = ApiError::parse_uuid(template_id)?;

    // Rate limit: count distinct campaign batches in the last hour. Each
    // batch shares a `created_at` second within ~1ms, so we approximate by
    // bucketing outbox rows on `event_type` + subject within the window.
    let recent_batches = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT date_trunc('second', created_at))
           FROM transactional_email_outbox
          WHERE event_type = $1
            AND created_at > NOW() - INTERVAL '1 hour'",
    )
    .bind(MARKETING_CAMPAIGN_EVENT_TYPE)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    if recent_batches >= CAMPAIGN_RATE_LIMIT_PER_HOUR {
        return Err(ApiError::TooManyRequests(format!(
            "Campaign rate limit reached ({CAMPAIGN_RATE_LIMIT_PER_HOUR}/hour). \
             Wait before sending the next campaign."
        )));
    }

    let t_row = sqlx::query("SELECT subject, html_template FROM email_templates WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::from)?;
    let Some(r) = t_row else {
        return Err(ApiError::NotFound("Template not found".to_string()));
    };
    let subject: String = r.get("subject");
    let html_body: String = r.get("html_template");

    let users = sqlx::query(audience_sql)
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::from)?;
    let mut queued_count: i64 = 0;

    for row in users {
        let u_id: sqlx::types::Uuid = row.get("id");
        let u_email: String = row.get("email");
        let first_name: String = row.try_get("first_name").unwrap_or_default();

        // Per-recipient render: {{first_name}} / {{email}} interpolated via
        // MiniJinja. Subject is rendered too so admins can personalise the
        // email subject line (`Hi {{first_name}}, your asset shipped`).
        let ctx = serde_json::json!({
            "first_name": first_name,
            "email": u_email,
            "user_id": u_id.to_string(),
        });
        let rendered_subject = crate::common::email::render_template(&subject, &ctx);
        let rendered_body = crate::common::email::render_template(&html_body, &ctx);

        // Durable enqueue. Worker picks it up via process_transactional_email_outbox.
        let outbox_result = sqlx::query(
            "INSERT INTO transactional_email_outbox
                (user_id, event_type, recipient_email, subject, html_body)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(u_id)
        .bind(MARKETING_CAMPAIGN_EVENT_TYPE)
        .bind(&u_email)
        .bind(&rendered_subject)
        .bind(&rendered_body)
        .execute(&state.db)
        .await;

        if outbox_result.is_err() {
            continue;
        }

        // Mirror to email_logs so the delivery-logs tab in the admin shows
        // the queue immediately (worker will update status on send).
        let _ = sqlx::query(
            "INSERT INTO email_logs
                (user_id, template_id, subject, recipient_email, status, sent_at)
             VALUES ($1, $2, $3, $4, 'queued', NOW())",
        )
        .bind(u_id)
        .bind(uid)
        .bind(&rendered_subject)
        .bind(&u_email)
        .execute(&state.db)
        .await;

        queued_count += 1;
    }

    // Best-effort audit trail. Lets admins answer "who sent campaign X?".
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'email_campaign_queued', 'email_template', $2, $3)",
    )
    .bind(admin.user.id)
    .bind(uid)
    .bind(serde_json::json!({
        "audience": audience,
        "queued_count": queued_count,
        "subject": subject,
    }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({
        "status": "campaign_queued",
        "target_count": queued_count,
    }))
    .into_response())
}

/// GET /api/admin/emails/logs?status=&search=&days=
///
/// Filterable delivery logs with sane bounds — `days` defaults to 7,
/// caps at 90; `status` filters to one of the known states (queued,
/// sent, delivered, opened, clicked, bounced, failed, spam_complaint,
/// skipped); `search` matches subject OR recipient_email substring.
/// Result is capped at 500 rows; pagination on top is fine because the
/// admin UI also paginates client-side.
pub async fn api_admin_emails_logs(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<LogsQuery>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    let days = q.days.unwrap_or(7).clamp(1, 90);
    let status_filter = match q.status.as_deref() {
        Some("all") | None | Some("") => None,
        Some(s) => Some(s.to_string()),
    };
    let search = q.search.unwrap_or_default();
    let search_pattern = if search.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            search.replace('%', "\\%").replace('_', "\\_")
        ))
    };

    let rows = sqlx::query(
        r#"SELECT e.id::text, e.subject, e.recipient_email, e.status, e.sent_at::text
             FROM email_logs e
            WHERE e.sent_at >= NOW() - ($1::TEXT || ' days')::INTERVAL
              AND ($2::TEXT IS NULL OR e.status = $2)
              AND ($3::TEXT IS NULL
                   OR e.subject ILIKE $3
                   OR e.recipient_email ILIKE $3)
            ORDER BY e.sent_at DESC
            LIMIT 500"#,
    )
    .bind(days.to_string())
    .bind(status_filter.as_deref())
    .bind(search_pattern.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let logs: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "subject": r.get::<String, _>("subject"),
                "recipient_email": r.get::<String, _>("recipient_email"),
                "status": r.get::<String, _>("status"),
                "sent_at": r.get::<String, _>("sent_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "logs": logs, "filters": { "days": days } })).into_response())
}

/// GET /api/admin/emails/suppressions — list active suppressions.
///
/// Resend keeps its own internal suppression list, but mirroring it
/// locally means the outbox worker can skip a bounced address without
/// a Resend API round-trip per send and admins can audit the list from
/// the platform UI.
pub async fn api_admin_emails_suppressions(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    let rows = sqlx::query(
        "SELECT id::text, email, reason, bounce_count, last_event_at::text,
                created_at::text, cleared_at::text
           FROM email_suppressions
          WHERE cleared_at IS NULL
          ORDER BY last_event_at DESC
          LIMIT 500",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let suppressions: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "email": r.get::<String, _>("email"),
                "reason": r.get::<String, _>("reason"),
                "bounce_count": r.get::<i32, _>("bounce_count"),
                "last_event_at": r.get::<String, _>("last_event_at"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "suppressions": suppressions })).into_response())
}

/// DELETE /api/admin/emails/suppressions/:id — manually clear a
/// suppression (e.g. after the recipient updated their mailbox quota).
/// Sets `cleared_at` rather than deleting so the history is preserved.
pub async fn api_admin_emails_suppression_clear(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;
    let uid = ApiError::parse_uuid(&id)?;

    let res = sqlx::query(
        "UPDATE email_suppressions
            SET cleared_at = NOW(), cleared_by_admin = $2
          WHERE id = $1 AND cleared_at IS NULL",
    )
    .bind(uid)
    .bind(admin.user.id)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "Suppression not found or already cleared".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({"status":"cleared"})).into_response())
}

/// Query string for `api_admin_emails_logs`.
#[derive(serde::Deserialize)]
pub struct LogsQuery {
    /// Status filter — `"all"`, empty, or `None` returns every row.
    pub status: Option<String>,
    /// Substring matched against subject OR recipient_email (case-insensitive).
    pub search: Option<String>,
    /// Lookback window in days. Clamped to 1..=90 server-side; defaults to 7.
    pub days: Option<i64>,
}

/// GET /api/admin/emails/audiences/:segment/count — recipient count for a
/// segment without sending. Powers the campaign preview "this will mail N
/// users" UI introduced in Commit 5.
pub async fn api_admin_emails_audience_count(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(segment): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    let Some(sql) = audience_query(&segment) else {
        return Err(ApiError::BadRequest(format!(
            "Unknown audience segment: {segment}"
        )));
    };

    let count_sql = format!("SELECT COUNT(*) FROM ({sql}) AS _seg");
    let count: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(serde_json::json!({ "segment": segment, "count": count })).into_response())
}

/// GET /api/admin/emails/workflows — list every transactional event the
/// platform emits, with its category, summary, default subject, optional
/// vs mandatory classification, and the sample metadata used by the
/// preview endpoint.
pub async fn api_admin_emails_workflows(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    let workflows: Vec<serde_json::Value> = EVENT_REGISTRY
        .iter()
        .map(|(event, category, summary, sample_json)| {
            let sample: serde_json::Value =
                serde_json::from_str(sample_json).unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "event_type": event,
                "category": category,
                "summary": summary,
                "subject": crate::email::subject_for_event(event),
                "optional": crate::common::email::is_optional_email_event(event),
                "sample_metadata": sample,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "workflows": workflows })).into_response())
}

/// POST /api/admin/emails/preview
///
/// Renders an email body (and subject) without sending. Two modes:
///   * `{event_type, sample_data?}` — renders a transactional event with
///     its registered sample metadata. Pass `sample_data` to override.
///   * `{template_id, sample_data?}` — renders a stored template via
///     MiniJinja with the supplied context.
///
/// Always returns `{subject, html, mode, event_type?}` — never sends a
/// real email.
pub async fn api_admin_emails_preview(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<PreviewRequest>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    let (subject, html) = render_preview_payload(&state, &body).await?;
    let mode = if body.event_type.is_some() {
        "event"
    } else {
        "template"
    };

    Ok(Json(serde_json::json!({
        "mode": mode,
        "event_type": body.event_type,
        "template_id": body.template_id,
        "subject": subject,
        "html": html,
    }))
    .into_response())
}

/// POST /api/admin/emails/test-send
///
/// Renders the same way as `/preview` then dispatches a single mail to
/// the calling admin's own address via Resend. Bypasses the outbox so
/// admins get immediate feedback. Requires `emails.send` (test sends
/// still cost provider quota and could be abused).
pub async fn api_admin_emails_test_send(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<PreviewRequest>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.send").await?;

    // Render via the same helpers the preview endpoint uses — pure function
    // calls, no double DB round-trip.
    let (subject, html) = render_preview_payload(&state, &body).await?;

    let admin_email = admin.user.email.clone();
    let prefixed_subject = format!("[TEST] {subject}");
    crate::common::email::send_email(&admin_email, &prefixed_subject, &html)
        .await
        .map_err(|e| ApiError::Internal(format!("send_email failed: {}", e.detail())))?;

    Ok(Json(serde_json::json!({
        "status": "sent",
        "to": admin_email,
        "subject": prefixed_subject,
    }))
    .into_response())
}

/// Shared preview rendering — used by both `api_admin_emails_preview`
/// (returns JSON to the browser) and `api_admin_emails_test_send`
/// (forwards the body to Resend). Returns `(subject, html)`.
async fn render_preview_payload(
    state: &AppState,
    body: &PreviewRequest,
) -> Result<(String, String), ApiError> {
    if let Some(event_type) = body.event_type.as_deref() {
        let registry_entry = EVENT_REGISTRY.iter().find(|(e, _, _, _)| *e == event_type);
        let default_sample: serde_json::Value = registry_entry
            .and_then(|(_, _, _, json)| serde_json::from_str(json).ok())
            .unwrap_or(serde_json::Value::Null);
        let metadata = body.sample_data.clone().unwrap_or(default_sample);
        let html = crate::email::build_email_html(event_type, &metadata);
        let subject = crate::email::subject_for_event(event_type).to_string();
        return Ok((subject, html));
    }

    if let Some(template_id) = body.template_id.as_deref() {
        let uid = ApiError::parse_uuid(template_id)?;
        let row = sqlx::query("SELECT subject, html_template FROM email_templates WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?;
        let Some(r) = row else {
            return Err(ApiError::NotFound("Template not found".to_string()));
        };
        let subject: String = r.get("subject");
        let html: String = r.get("html_template");
        let ctx = body.sample_data.clone().unwrap_or_else(|| {
            serde_json::json!({
                "first_name": "Maria",
                "email": "maria@example.test",
            })
        });
        let rendered_subject = crate::common::email::render_template(&subject, &ctx);
        let rendered_html = crate::common::email::render_template(&html, &ctx);
        return Ok((rendered_subject, rendered_html));
    }

    Err(ApiError::BadRequest(
        "Either event_type or template_id is required".to_string(),
    ))
}

/// Request body shared by `api_admin_emails_preview` and `api_admin_emails_test_send`.
#[derive(serde::Deserialize)]
pub struct PreviewRequest {
    /// Render a built-in transactional event (one of the entries in `EVENT_REGISTRY`).
    pub event_type: Option<String>,
    /// Render a stored email_templates row by UUID. Mutually exclusive with event_type.
    pub template_id: Option<String>,
    /// Optional JSON context override. Defaults to the event's registered sample.
    pub sample_data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audience_query_known_segments_return_sql() {
        for seg in ["all", "investors", "kyc_approved", "tier_plus", "dormant"] {
            assert!(
                audience_query(seg).is_some(),
                "segment '{seg}' should be supported"
            );
        }
    }

    #[test]
    fn audience_query_unknown_segment_returns_none() {
        assert!(audience_query("totally_unknown").is_none());
        assert!(audience_query("").is_none());
        assert!(audience_query("ALL").is_none(), "case-sensitive on purpose");
    }

    #[test]
    fn audience_queries_all_select_id_email_and_first_name() {
        // Every segment query must expose `id`, `email`, and `first_name`
        // (left-joined from user_profiles, COALESCED to empty string) so
        // the campaign render loop can construct a per-user context.
        for seg in ["all", "investors", "kyc_approved", "tier_plus", "dormant"] {
            let sql = audience_query(seg).unwrap();
            assert!(sql.contains("u.id"), "segment '{seg}' missing u.id column");
            assert!(
                sql.contains("u.email"),
                "segment '{seg}' missing u.email column"
            );
            assert!(
                sql.contains("first_name"),
                "segment '{seg}' missing first_name column — render loop \
                 would crash trying to read it"
            );
            assert!(
                sql.contains("LEFT JOIN user_profiles"),
                "segment '{seg}' must LEFT JOIN user_profiles so users \
                 with no profile row still receive campaign mail"
            );
        }
    }

    #[test]
    fn audience_queries_filter_inactive_and_unverified() {
        // All audiences must exclude unverified or suspended users —
        // otherwise we'd mail people who never confirmed their address.
        for seg in ["all", "investors", "kyc_approved", "tier_plus", "dormant"] {
            let sql = audience_query(seg).unwrap();
            assert!(
                sql.contains("status = 'active'"),
                "segment '{seg}' must filter active users"
            );
            assert!(
                sql.contains("email_verified = TRUE"),
                "segment '{seg}' must filter verified emails"
            );
        }
    }

    #[test]
    fn marketing_campaign_event_type_is_optional_class() {
        // The outbox worker pref-gates optional events. Campaigns MUST be
        // optional so a user who unsubscribed via List-Unsubscribe is
        // skipped at send time.
        assert!(crate::common::email::is_optional_email_event(
            MARKETING_CAMPAIGN_EVENT_TYPE
        ));
    }

    // ── EVENT_REGISTRY (Workflows tab + Preview endpoint) ─────────────

    #[test]
    fn event_registry_entries_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for (event, _, _, _) in EVENT_REGISTRY {
            assert!(
                seen.insert(*event),
                "duplicate event_type in EVENT_REGISTRY: {event}"
            );
        }
    }

    #[test]
    fn event_registry_sample_metadata_is_valid_json() {
        for (event, _, _, sample) in EVENT_REGISTRY {
            let parsed: serde_json::Value = serde_json::from_str(sample)
                .unwrap_or_else(|e| panic!("event '{event}' has invalid sample JSON: {e}"));
            assert!(
                parsed.is_object() || parsed.is_null(),
                "event '{event}' sample must be a JSON object or null"
            );
        }
    }

    #[test]
    fn event_registry_covers_all_subjects_in_email_rs() {
        // Every event we render a body for via build_email_html should be
        // in the workflows registry so the admin UI can list it. A
        // missing entry means the workflows tab silently omits a real
        // production email.
        let registry_events: std::collections::HashSet<&str> =
            EVENT_REGISTRY.iter().map(|(e, _, _, _)| *e).collect();
        for event in [
            "welcome",
            "verify_email",
            "password_reset",
            "2fa_setup",
            "new_login",
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
            "affiliate_commission_qualified",
            "affiliate_application_info_requested",
            "affiliate_tier_promoted",
            "affiliate_tier_demoted",
            "affiliate_material_approved",
            "affiliate_material_rejected",
            "developer_project_revision_required",
            "admin_invitation",
            "admin_new_affiliate_application",
            "admin_payout_request",
            "admin_new_marketing_material",
        ] {
            assert!(
                registry_events.contains(event),
                "event '{event}' has a build_email_html body but is missing \
                 from EVENT_REGISTRY — admins won't see it in the workflows \
                 tab"
            );
        }
    }

    #[test]
    fn event_registry_categories_are_known() {
        let known: std::collections::HashSet<&str> = [
            "Auth",
            "KYC",
            "Wallet",
            "Returns",
            "Orders",
            "Assets",
            "Affiliate",
            "Team",
            "Support",
            "Operations",
            "Marketing",
            "Developer",
            "Internal",
        ]
        .iter()
        .copied()
        .collect();
        for (event, category, _, _) in EVENT_REGISTRY {
            assert!(
                known.contains(*category),
                "event '{event}' has unknown category '{category}' — update \
                 the known-set or fix the registry"
            );
        }
    }
}
