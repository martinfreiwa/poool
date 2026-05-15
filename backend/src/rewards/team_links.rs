//! Service-Layer für `affiliate_links` (Phase 2).
//!
//! Verantwortlich für:
//!   * Erzeugen von Personal-Links (beim Affiliate-Approve)
//!   * Erzeugen von Team-Business-Links (durch Developer für aktiven Mitarbeiter)
//!   * Deaktivieren (Off-boarding, Admin-Suspend)
//!   * Lookups (per code, per user, per team)
//!
//! Alle mutierenden Operationen schreiben Audit-Log-Rows.

use crate::error::AppError;
use crate::rewards::team_models::{AffiliateLink, LinkType};
use sqlx::PgPool;
use uuid::Uuid;

/// Länge des generierten Codes (alphanumerisch, gemischt).
const CODE_LENGTH: usize = 10;

/// Generiert einen URL-safen Affiliate-Code. Crypto-RNG, kollisionsarm.
pub fn generate_unique_code() -> String {
    use rand::Rng;
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // ohne I,O,0,1,l für Lesbarkeit
    let mut rng = rand::thread_rng();
    (0..CODE_LENGTH)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// Loops bis max 5x neu gewürfelt, falls Race-Kollision. UNIQUE-Index in DB
/// schützt final.
async fn generate_available_code(pool: &PgPool) -> Result<String, AppError> {
    for _ in 0..5 {
        let code = generate_unique_code();
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM affiliate_links WHERE code = $1)")
                .bind(&code)
                .fetch_one(pool)
                .await
                .unwrap_or(false);
        if !exists {
            return Ok(code);
        }
    }
    Err(AppError::Internal(
        "Failed to generate unique affiliate-link code after 5 attempts".into(),
    ))
}

// ── Lookups ─────────────────────────────────────────────────────────────────

/// Liefert einen aktiven Link per code, oder None.
pub async fn find_active_by_code(
    pool: &PgPool,
    code: &str,
) -> Result<Option<AffiliateLink>, AppError> {
    let row = sqlx::query_as::<_, AffiliateLink>(
        r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                  status, created_at, updated_at, deactivated_at, deactivated_reason
           FROM affiliate_links
           WHERE code = $1 AND status = 'active'"#,
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Liefert alle aktiven Links eines Teams (für Developer-Dashboard).
pub async fn list_active_for_team(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<Vec<AffiliateLink>, AppError> {
    let rows = sqlx::query_as::<_, AffiliateLink>(
        r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                  status, created_at, updated_at, deactivated_at, deactivated_reason
           FROM affiliate_links
           WHERE team_id = $1 AND status = 'active'
           ORDER BY created_at DESC"#,
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Liefert alle Links eines Users (sowohl als attribution_user_id als auch
/// als payout_user_id — also "alles was ich verdienen oder ausführen kann").
pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<AffiliateLink>, AppError> {
    let rows = sqlx::query_as::<_, AffiliateLink>(
        r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                  status, created_at, updated_at, deactivated_at, deactivated_reason
           FROM affiliate_links
           WHERE (attribution_user_id = $1 OR payout_user_id = $1)
             AND status = 'active'
           ORDER BY link_type ASC, created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Create ──────────────────────────────────────────────────────────────────

/// Erzeugt einen Personal-Link für einen User. Wird typischerweise vom
/// Affiliate-Approve-Flow aufgerufen (siehe admin/rewards.rs).
///
/// Idempotent — wenn der User schon einen aktiven Personal-Link hat, gibt
/// die existierende Row zurück.
pub async fn create_personal_link(pool: &PgPool, user_id: Uuid) -> Result<AffiliateLink, AppError> {
    // Idempotenz: bereits aktiver Personal-Link?
    let existing = sqlx::query_as::<_, AffiliateLink>(
        r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                  status, created_at, updated_at, deactivated_at, deactivated_reason
           FROM affiliate_links
           WHERE attribution_user_id = $1
             AND link_type = 'personal'
             AND status = 'active'
           LIMIT 1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if let Some(link) = existing {
        return Ok(link);
    }

    let code = generate_available_code(pool).await?;
    let insert_res = sqlx::query_as::<_, AffiliateLink>(
        r#"INSERT INTO affiliate_links
              (code, link_type, attribution_user_id, payout_user_id, team_id, status)
           VALUES ($1, 'personal', $2, $2, NULL, 'active')
           RETURNING id, code, link_type, attribution_user_id, payout_user_id, team_id,
                     status, created_at, updated_at, deactivated_at, deactivated_reason"#,
    )
    .bind(&code)
    .bind(user_id)
    .fetch_one(pool)
    .await;

    // Race with concurrent caller? Partial-unique index uniq_personal_link_per_user
    // (mig 162) catches it. Re-fetch the winning row instead of failing.
    let row = match insert_res {
        Ok(r) => r,
        Err(sqlx::Error::Database(db_err))
            if db_err.constraint() == Some("uniq_personal_link_per_user") =>
        {
            sqlx::query_as::<_, AffiliateLink>(
                r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                          status, created_at, updated_at, deactivated_at, deactivated_reason
                   FROM affiliate_links
                   WHERE attribution_user_id = $1 AND link_type = 'personal' AND status = 'active'
                   LIMIT 1"#,
            )
            .bind(user_id)
            .fetch_one(pool)
            .await?
        }
        Err(e) => return Err(e.into()),
    };

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'affiliate_link_created', 'affiliate_links', $2, $3)",
    )
    .bind(user_id)
    .bind(row.id)
    .bind(serde_json::json!({
        "link_type": "personal",
        "code": code,
    }))
    .execute(pool)
    .await;

    Ok(row)
}

/// Erzeugt einen Team-Business-Link für ein aktives Team-Mitglied.
/// Provisionen + Auszahlungen fließen an den Developer (team owner).
///
/// Idempotent — wenn der Member im Team schon einen aktiven Team-Business-Link
/// hat, wird die existierende Row zurückgegeben.
///
/// Voraussetzungen (geprüft):
///   * Membership existiert, status = 'active'
///   * Membership gehört zu `team_id`
///   * Team status = 'active'
pub async fn create_team_business_link(
    pool: &PgPool,
    team_id: Uuid,
    member_user_id: Uuid,
    actor_user_id: Uuid,
) -> Result<AffiliateLink, AppError> {
    // 1) Validate team + membership
    let team = sqlx::query!(
        r#"SELECT id, developer_user_id, status FROM developer_teams WHERE id = $1"#,
        team_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Team not found".into()))?;
    if team.status != "active" {
        return Err(AppError::BadRequest(format!(
            "Team status is {}, expected active",
            team.status
        )));
    }

    let membership = sqlx::query!(
        r#"SELECT id, status FROM developer_team_memberships
           WHERE team_id = $1 AND user_id = $2"#,
        team_id,
        member_user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Membership not found".into()))?;
    if membership.status != "active" {
        return Err(AppError::BadRequest(format!(
            "Membership status is {}, expected active",
            membership.status
        )));
    }

    // 2) Idempotenz
    let existing = sqlx::query_as::<_, AffiliateLink>(
        r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                  status, created_at, updated_at, deactivated_at, deactivated_reason
           FROM affiliate_links
           WHERE team_id = $1
             AND attribution_user_id = $2
             AND link_type = 'team_business'
             AND status = 'active'
           LIMIT 1"#,
    )
    .bind(team_id)
    .bind(member_user_id)
    .fetch_optional(pool)
    .await?;
    if let Some(link) = existing {
        return Ok(link);
    }

    // 3) Insert. Partial-unique uniq_team_business_link_per_member (mig 162)
    // catches concurrent double-create — re-fetch winning row.
    let code = generate_available_code(pool).await?;
    let insert_res = sqlx::query_as::<_, AffiliateLink>(
        r#"INSERT INTO affiliate_links
              (code, link_type, attribution_user_id, payout_user_id, team_id, status)
           VALUES ($1, 'team_business', $2, $3, $4, 'active')
           RETURNING id, code, link_type, attribution_user_id, payout_user_id, team_id,
                     status, created_at, updated_at, deactivated_at, deactivated_reason"#,
    )
    .bind(&code)
    .bind(member_user_id)
    .bind(team.developer_user_id)
    .bind(team_id)
    .fetch_one(pool)
    .await;

    let row = match insert_res {
        Ok(r) => r,
        Err(sqlx::Error::Database(db_err))
            if db_err.constraint() == Some("uniq_team_business_link_per_member") =>
        {
            sqlx::query_as::<_, AffiliateLink>(
                r#"SELECT id, code, link_type, attribution_user_id, payout_user_id, team_id,
                          status, created_at, updated_at, deactivated_at, deactivated_reason
                   FROM affiliate_links
                   WHERE team_id = $1 AND attribution_user_id = $2
                     AND link_type = 'team_business' AND status = 'active'
                   LIMIT 1"#,
            )
            .bind(team_id)
            .bind(member_user_id)
            .fetch_one(pool)
            .await?
        }
        Err(e) => return Err(e.into()),
    };

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'affiliate_link_created', 'affiliate_links', $2, $3)",
    )
    .bind(actor_user_id)
    .bind(row.id)
    .bind(serde_json::json!({
        "link_type": "team_business",
        "code": code,
        "team_id": team_id,
        "attribution_user_id": member_user_id,
        "payout_user_id": team.developer_user_id,
    }))
    .execute(pool)
    .await;

    Ok(row)
}

/// Deaktiviert einen Link (soft). Historische Referrals + Commissions bleiben
/// auswertbar, neue Klicks/Signups werden nicht mehr attribuiert.
pub async fn deactivate_link(
    pool: &PgPool,
    link_id: Uuid,
    actor_user_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    let res = sqlx::query!(
        r#"UPDATE affiliate_links
           SET status = 'inactive',
               deactivated_at = NOW(),
               deactivated_reason = $2,
               updated_at = NOW()
           WHERE id = $1 AND status = 'active'"#,
        link_id,
        reason
    )
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Link not found or already inactive".into(),
        ));
    }

    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'affiliate_link_deactivated', 'affiliate_links', $2, $3)",
    )
    .bind(actor_user_id)
    .bind(link_id)
    .bind(serde_json::json!({ "reason": reason }))
    .execute(pool)
    .await;

    Ok(())
}

/// Bulk-Deaktivierung aller Team-Business-Links eines Members. Wird bei
/// Off-Boarding aufgerufen (Member removed from team).
pub async fn deactivate_all_team_business_for_member(
    pool: &PgPool,
    team_id: Uuid,
    member_user_id: Uuid,
    actor_user_id: Uuid,
    reason: &str,
) -> Result<u64, AppError> {
    let res = sqlx::query!(
        r#"UPDATE affiliate_links
           SET status = 'inactive',
               deactivated_at = NOW(),
               deactivated_reason = $3,
               updated_at = NOW()
           WHERE team_id = $1
             AND attribution_user_id = $2
             AND link_type = 'team_business'
             AND status = 'active'"#,
        team_id,
        member_user_id,
        reason
    )
    .execute(pool)
    .await?;

    let affected = res.rows_affected();
    if affected > 0 {
        let _ = sqlx::query(
            "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
             VALUES ($1, 'affiliate_link_member_offboarded', 'developer_team_memberships', $2, $3)",
        )
        .bind(actor_user_id)
        .bind(member_user_id)
        .bind(serde_json::json!({
            "team_id": team_id,
            "links_deactivated": affected,
            "reason": reason,
        }))
        .execute(pool)
        .await;
    }
    Ok(affected)
}

/// Stellt sicher dass `affiliates`-Row für einen Developer existiert (wird
/// als payout_user_id in Team-Business-Commissions referenziert).
/// Wenn nicht vorhanden, wird sie automatisch angelegt mit Default-Werten.
///
/// Compliance-Schuld: Developer als Team-Affiliate sollte vollständigen
/// Onboarding-Flow (Exam, Tax) durchlaufen — TODO Phase 5 / Admin.
pub async fn ensure_developer_has_affiliate_row(
    pool: &PgPool,
    developer_user_id: Uuid,
) -> Result<(), AppError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM affiliates WHERE user_id = $1)")
            .bind(developer_user_id)
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if exists {
        return Ok(());
    }

    // Placeholder referral_code analog zum bestehenden onboarding-Flow
    let placeholder_code = format!("PEND_{}", uuid::Uuid::new_v4().as_fields().0);
    let _ = sqlx::query(
        r#"INSERT INTO affiliates (user_id, referral_code, status)
           VALUES ($1, $2, 'active')
           ON CONFLICT (user_id) DO NOTHING"#,
    )
    .bind(developer_user_id)
    .bind(placeholder_code)
    .execute(pool)
    .await?;

    Ok(())
}
