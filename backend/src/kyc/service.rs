use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::didit::{DiditConfig, DiditProvider};
use super::models::{KycInitiateResponse, KycStatusResponse, KycSubmitRequest};
use super::provider::{KycProvider, KycStatusUpdate, ManualProvider};
use crate::error::AppError;

/// Build the active KYC provider based on environment configuration.
///
/// Priority:
/// 1. If DIDIT_API_KEY is set → use Didit.
/// 2. Otherwise → fall back to manual review.
///
/// This is where Sumsub can be added in the future:
/// ```ignore
/// if let Some(cfg) = SumsubConfig::from_env() {
///     return Arc::new(SumsubProvider::new(cfg));
/// }
/// ```
pub fn build_provider() -> Arc<dyn KycProvider> {
    if let Some(cfg) = DiditConfig::from_env() {
        tracing::info!("KYC provider: Didit (workflow_id={})", cfg.workflow_id);
        Arc::new(DiditProvider::new(cfg))
    } else {
        // Fallback to manual only in development, or if specifically desired.
        // If the user wants "Didit only", we should ensure this is known.
        tracing::warn!("KYC provider: Manual (Warning: Didit configuration is missing!)");
        Arc::new(ManualProvider)
    }
}

pub async fn get_kyc_status(pool: &PgPool, user_id: Uuid) -> Result<KycStatusResponse, AppError> {
    let rec = sqlx::query!(
        r#"
        SELECT status, rejection_reason, provider FROM kyc_records
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    match rec {
        Some(r) => Ok(KycStatusResponse {
            status: r.status,
            rejection_reason: r.rejection_reason,
            provider: Some(r.provider),
            verification_url: None,
        }),
        None => Ok(KycStatusResponse {
            status: "not_started".to_string(),
            rejection_reason: None,
            provider: None,
            verification_url: None,
        }),
    }
}

/// Initiate a KYC verification session using the active provider.
///
/// 1. Checks for existing pending/approved records.
/// 2. Creates a session with the provider (Didit/Sumsub).
/// 3. Inserts a `kyc_records` row with the provider ref ID.
/// 4. Returns the verification URL for frontend redirect.
pub async fn initiate_kyc(
    pool: &PgPool,
    provider: &dyn KycProvider,
    user_id: Uuid,
    user_email: Option<&str>,
    callback_url: &str,
    document_type: Option<&str>,
) -> Result<KycInitiateResponse, AppError> {
    // Serialize concurrent initiate calls for the same user via a Postgres
    // transaction-scoped advisory lock. Prior version read existing state
    // and then inserted a new row in separate statements — a second request
    // racing through the same gap could spawn duplicate provider sessions
    // or blow past the 'approved' guard.
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // advisory lock keyed on user UUID (hashed into two i32s for the 2-arg form).
    let uid_bytes = user_id.as_bytes();
    let key_hi = i32::from_be_bytes([uid_bytes[0], uid_bytes[1], uid_bytes[2], uid_bytes[3]]);
    let key_lo = i32::from_be_bytes([uid_bytes[4], uid_bytes[5], uid_bytes[6], uid_bytes[7]]);
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(key_hi)
        .bind(key_lo)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    let existing: Option<String> = sqlx::query_scalar(
        "SELECT status FROM kyc_records WHERE user_id = $1 AND status IN ('pending', 'in_review', 'approved') ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    if let Some(ref status) = existing {
        if status == "approved" {
            return Err(AppError::Conflict("KYC already approved".to_string()));
        }
        if status == "in_review" {
            return Err(AppError::Conflict(
                "KYC submission already pending review".to_string(),
            ));
        }
        // status == "pending": session was created but user abandoned the flow.
        if status == "pending" {
            tracing::info!(user_id = %user_id, "Deleting stale pending KYC record to allow restart");
            sqlx::query("DELETE FROM kyc_records WHERE user_id = $1 AND status = 'pending'")
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;
        }
    }

    // Create a session with the provider (external side-effect — safe to do
    // inside the tx since the advisory lock already blocks other initiators).
    let session_result = provider
        .create_session(user_id, user_email, callback_url)
        .await?;

    let kyc_id: String = sqlx::query_scalar(
        r#"INSERT INTO kyc_records (user_id, provider, provider_ref_id, status, document_type)
           VALUES ($1, $2, $3, 'pending', $4)
           RETURNING id::text"#,
    )
    .bind(user_id)
    .bind(provider.name())
    .bind(&session_result.session_id)
    .bind(document_type)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        "KYC initiated for user {} via {} (kyc_id={}, session={})",
        user_id,
        provider.name(),
        kyc_id,
        session_result.session_id
    );

    // Audit log
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'kyc.initiated', 'kyc_records', $2::uuid, $3)"#,
    )
    .bind(user_id)
    .bind(&kyc_id)
    .bind(serde_json::json!({
        "provider": provider.name(),
        "status": "pending",
    }))
    .execute(pool)
    .await;

    // Trigger email notification
    let metadata = serde_json::json!({
        "provider": provider.name(),
        "document_type": document_type,
    });
    let _ =
        crate::email::trigger_transactional_email(pool, &user_id, "kyc_submitted", metadata).await;

    Ok(KycInitiateResponse {
        success: true,
        kyc_id,
        provider: provider.name().to_string(),
        verification_url: session_result.verification_url,
        message: "KYC verification initiated. Please complete the verification process."
            .to_string(),
    })
}

/// Process a webhook status update from a KYC provider.
///
/// 1. Finds the `kyc_records` row by `provider_ref_id`.
/// 2. Updates the status, rejection reason, and check results.
/// 3. Optionally updates user profile with extracted data.
pub async fn process_webhook_update(
    pool: &PgPool,
    update: KycStatusUpdate,
    provider_name: &str,
) -> Result<(), AppError> {
    let new_status = update.status.as_db_str();

    // Find the kyc record by provider ref ID
    let kyc_record: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id FROM kyc_records WHERE provider_ref_id = $1 AND provider = $2 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&update.session_id)
    .bind(provider_name)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    let (kyc_id, user_id) = match kyc_record {
        Some(r) => r,
        None => {
            tracing::warn!(
                "Webhook for unknown KYC session: provider={}, session={}",
                provider_name,
                update.session_id
            );
            return Ok(()); // Silently ignore unknown sessions (might be from old provider)
        }
    };

    tracing::info!(
        "Processing KYC webhook: user={}, session={}, status={}",
        user_id,
        update.session_id,
        new_status
    );

    // Update the KYC record
    let verified_at = if new_status == "approved" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    // Set expiry to 1 year after approval
    let expires_at = if new_status == "approved" {
        Some(chrono::Utc::now() + chrono::Duration::days(365))
    } else {
        None
    };

    sqlx::query(
        r#"UPDATE kyc_records
           SET status = $1,
               rejection_reason = $2,
               pep_check_passed = COALESCE($3, pep_check_passed),
               sanctions_check = COALESCE($4, sanctions_check),
               verified_at = COALESCE($5, verified_at),
               expires_at = COALESCE($6, expires_at)
           WHERE id = $7"#,
    )
    .bind(new_status)
    .bind(&update.rejection_reason)
    .bind(update.pep_check_passed)
    .bind(update.sanctions_check_passed)
    .bind(verified_at)
    .bind(expires_at)
    .bind(kyc_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    // If approved, update user profile with extracted identity data
    if let Some(ref data) = update.extracted_data {
        if new_status == "approved" {
            let _ = sqlx::query(
                r#"UPDATE user_profiles
                   SET first_name = COALESCE($1, first_name),
                       last_name = COALESCE($2, last_name),
                       date_of_birth = COALESCE($3::date, date_of_birth),
                       nationality = COALESCE($4, nationality)
                   WHERE user_id = $5"#,
            )
            .bind(&data.first_name)
            .bind(&data.last_name)
            .bind(&data.date_of_birth)
            .bind(&data.nationality)
            .bind(user_id)
            .execute(pool)
            .await;
        }
    }

    // Phase 18 / 19: Intermediate Funnel Checkpoints (Affiliate Sync)
    // Update any pending referral stages to 'kyc_approved'
    if new_status == "approved" {
        let _ = sqlx::query(
            r#"UPDATE affiliate_referrals 
               SET status = 'kyc_approved', updated_at = NOW() 
               WHERE referred_user_id = $1 AND status IN ('attributed', 'registered')"#,
        )
        .bind(user_id)
        .execute(pool)
        .await;
    }

    // Audit log
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, $2, 'kyc_records', $3, $4)"#,
    )
    .bind(user_id)
    .bind(format!("kyc.{}", new_status))
    .bind(kyc_id)
    .bind(serde_json::json!({
        "provider": provider_name,
        "status": new_status,
        "rejection_reason": update.rejection_reason,
    }))
    .execute(pool)
    .await;

    // Send notification to user
    let (title, message, notif_type) = match new_status {
        "approved" => (
            "KYC Approved ✓",
            "Your identity verification has been approved. You can now access all platform features.",
            "kyc",
        ),
        "rejected" => (
            "KYC Declined",
            "Your identity verification was declined. Please check the reason and resubmit.",
            "kyc",
        ),
        _ => return Ok(()),
    };

    let _ = sqlx::query(
        "INSERT INTO notifications (user_id, title, message, type) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(title)
    .bind(message)
    .bind(notif_type)
    .execute(pool)
    .await;

    Ok(())
}

/// Legacy manual submission handler (kept for backward compatibility).
pub async fn submit_kyc(
    pool: &PgPool,
    user_id: Uuid,
    payload: KycSubmitRequest,
) -> Result<(), AppError> {
    // Update user profiles with personal info if provided
    if let (Some(ref fname), Some(ref lname)) = (&payload.first_name, &payload.last_name) {
        let _ = sqlx::query!(
            "UPDATE user_profiles SET first_name = $1, last_name = $2 WHERE user_id = $3",
            fname.clone(),
            lname.clone(),
            user_id
        )
        .execute(pool)
        .await;
    }

    // Upsert a pending kyc record
    let kyc_record_id: uuid::Uuid = sqlx::query_scalar!(
        r#"
        INSERT INTO kyc_records (user_id, status, provider, document_type)
        VALUES ($1, 'pending', 'manual', $2)
        RETURNING id
        "#,
        user_id,
        payload.document_type
    )
    .fetch_one(pool)
    .await?;

    // Link the document if provided
    if let Some(doc_id) = payload.document_id {
        let _ = sqlx::query!(
            "UPDATE kyc_documents SET kyc_record_id = $1 WHERE id = $2 AND user_id = $3",
            kyc_record_id,
            doc_id,
            user_id
        )
        .execute(pool)
        .await;
    }

    // Call transactional email service
    let metadata = serde_json::json!({
        "document_type": payload.document_type
    });
    let _ =
        crate::email::trigger_transactional_email(pool, &user_id, "kyc_submitted", metadata).await;

    Ok(())
}
