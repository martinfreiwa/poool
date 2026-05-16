//! Phase-4: multi-payout-method per affiliate.
//!
//! Each affiliate may register multiple payout destinations (SEPA, PayPal,
//! Wise, USDC, Stripe Connect). One row per (affiliate, method) — the
//! `is_default = TRUE` row picks the destination the payout-batch worker
//! uses. A partial-unique index enforces at-most-one default per affiliate.
//!
//! Schema: `database/194_affiliate_payout_methods.sql`.
//!
//! Lives in its own module so the call sites in `routes.rs` reach
//! `rewards::payout_methods::create_payout_method(...)` rather than
//! piggy-backing on `rewards::service` (which keeps churning during
//! parallel-session edits).

use crate::error::AppError;
use crate::rewards::service::{
    bank_iban_last4, encrypt_bank_iban, validate_iban_mod97, BANK_IBAN_KEY_VERSION,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct PayoutMethodRow {
    pub id: Uuid,
    pub method_type: String,
    /// Masked or plain identifier, safe for client display.
    pub identifier_display: String,
    pub label: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Validate a (method_type, identifier) pair. Returns:
///   (canonical_method_type, identifier_plain, identifier_encrypted, last4, key_version).
///
/// Encryption (SEPA only) happens inside the fn so callers never hold
/// plaintext IBAN after validation.
pub fn validate_payout_method(
    method_type: &str,
    identifier: &str,
) -> Result<
    (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i32>,
    ),
    AppError,
> {
    let id = identifier.trim();
    if id.is_empty() {
        return Err(AppError::BadRequest("identifier is required".into()));
    }
    match method_type {
        "sepa_iban" => {
            let compact: String = id
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_uppercase();
            validate_iban_mod97(&compact)?;
            let encrypted = encrypt_bank_iban(&compact)?;
            let last4 = bank_iban_last4(&compact);
            Ok((
                "sepa_iban".into(),
                None,
                Some(encrypted),
                Some(last4),
                Some(BANK_IBAN_KEY_VERSION as i32),
            ))
        }
        "paypal_email" | "wise_email" => {
            if !id.contains('@') || id.len() > 254 {
                return Err(AppError::BadRequest("invalid email".into()));
            }
            Ok((method_type.into(), Some(id.into()), None, None, None))
        }
        "usdc_wallet" => {
            // ERC-20 0x-prefixed 40-hex address. Light validation only —
            // the connector does the heavy chain-side check.
            if !id.starts_with("0x") || id.len() != 42 {
                return Err(AppError::BadRequest(
                    "USDC wallet must be a 0x-prefixed 40-hex address".into(),
                ));
            }
            if !id[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::BadRequest("wallet must be hex".into()));
            }
            let last4 = id[id.len() - 4..].to_string();
            Ok((
                "usdc_wallet".into(),
                Some(id.into()),
                None,
                Some(last4),
                None,
            ))
        }
        "stripe_connect" => {
            if !id.starts_with("acct_") || id.len() < 10 || id.len() > 80 {
                return Err(AppError::BadRequest(
                    "Stripe Connect ID must start with 'acct_'".into(),
                ));
            }
            Ok(("stripe_connect".into(), Some(id.into()), None, None, None))
        }
        other => Err(AppError::BadRequest(format!(
            "unknown method_type: {}",
            other
        ))),
    }
}

pub async fn create_payout_method(
    pool: &PgPool,
    user_id: Uuid,
    method_type: &str,
    identifier: &str,
    label: Option<&str>,
    is_default: bool,
) -> Result<Uuid, AppError> {
    let (mt, plain, encrypted, last4, key_version) =
        validate_payout_method(method_type, identifier)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("tx begin: {e}")))?;
    if is_default {
        sqlx::query(
            r#"UPDATE affiliate_payout_methods
                  SET is_default = FALSE, updated_at = NOW()
                WHERE affiliate_id = $1 AND is_default = TRUE AND is_active = TRUE"#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("default flip: {e}")))?;
    }
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO affiliate_payout_methods
              (id, affiliate_id, method_type, identifier_encrypted, identifier_plain,
               identifier_last4, key_version, label, is_default, is_active)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE)"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&mt)
    .bind(&encrypted)
    .bind(&plain)
    .bind(&last4)
    .bind(key_version)
    .bind(label)
    .bind(is_default)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("payout method insert: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("tx commit: {e}")))?;
    Ok(id)
}

pub async fn list_payout_methods(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<PayoutMethodRow>, AppError> {
    let rows = sqlx::query(
        r#"SELECT id, method_type, identifier_plain, identifier_last4,
                  label, is_default, is_active, created_at
             FROM affiliate_payout_methods
            WHERE affiliate_id = $1 AND is_active = TRUE
         ORDER BY is_default DESC, created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(format!("payout method list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let method_type: String = r.try_get("method_type").unwrap_or_default();
            let plain: Option<String> = r.try_get("identifier_plain").ok();
            let last4: Option<String> = r.try_get("identifier_last4").ok();
            let display = match (plain.as_deref(), last4.as_deref()) {
                (Some(p), _) if p.contains('@') => {
                    let mut parts = p.splitn(2, '@');
                    let local = parts.next().unwrap_or("");
                    let domain = parts.next().unwrap_or("");
                    if local.is_empty() {
                        p.to_string()
                    } else {
                        let first = local.chars().next().unwrap_or('*');
                        format!("{}***@{}", first, domain)
                    }
                }
                (Some(p), _) if p.starts_with("0x") && p.len() == 42 => {
                    format!("{}…{}", &p[..6], &p[p.len() - 4..])
                }
                (Some(p), _) if p.starts_with("acct_") => {
                    if p.len() <= 14 {
                        p.to_string()
                    } else {
                        format!("{}…", &p[..14])
                    }
                }
                (None, Some(l)) => format!("**** **** **** {}", l),
                (Some(p), _) => p.to_string(),
                (None, None) => String::new(),
            };
            PayoutMethodRow {
                id: r.try_get("id").unwrap_or_else(|_| Uuid::nil()),
                method_type,
                identifier_display: display,
                label: r.try_get("label").ok(),
                is_default: r.try_get("is_default").unwrap_or(false),
                is_active: r.try_get("is_active").unwrap_or(false),
                created_at: r
                    .try_get("created_at")
                    .unwrap_or_else(|_| chrono::Utc::now()),
            }
        })
        .collect())
}

pub async fn set_default_payout_method(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("tx begin: {e}")))?;
    let exists: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM affiliate_payout_methods
            WHERE id = $1 AND affiliate_id = $2 AND is_active = TRUE"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("ownership check: {e}")))?;
    if exists == 0 {
        tx.rollback().await.ok();
        return Ok(false);
    }
    sqlx::query(
        r#"UPDATE affiliate_payout_methods
              SET is_default = FALSE, updated_at = NOW()
            WHERE affiliate_id = $1 AND is_default = TRUE AND is_active = TRUE"#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("default flip-off: {e}")))?;
    sqlx::query(
        r#"UPDATE affiliate_payout_methods
              SET is_default = TRUE, updated_at = NOW()
            WHERE id = $1 AND affiliate_id = $2"#,
    )
    .bind(id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("default set: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("tx commit: {e}")))?;
    Ok(true)
}

pub async fn deactivate_payout_method(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
) -> Result<bool, AppError> {
    let res = sqlx::query(
        r#"UPDATE affiliate_payout_methods
              SET is_active = FALSE, is_default = FALSE, updated_at = NOW()
            WHERE id = $1 AND affiliate_id = $2 AND is_active = TRUE"#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("payout method deactivate: {e}")))?;
    Ok(res.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_each_method_type() {
        // SEPA test path needs an encryption key. Use a deterministic test key
        // — does NOT leak (test scope only).
        std::env::set_var(
            "BANK_IBAN_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );

        // sepa_iban: encrypt + last4; valid IBAN required
        let (mt, plain, enc, last4, kv) =
            validate_payout_method("sepa_iban", "DE89 3704 0044 0532 0130 00").unwrap();
        assert_eq!(mt, "sepa_iban");
        assert!(plain.is_none());
        assert!(enc.unwrap().starts_with("biban:v1:"));
        assert_eq!(last4.unwrap(), "3000");
        assert_eq!(kv.unwrap(), 1);
        assert!(validate_payout_method("sepa_iban", "garbage").is_err());

        // paypal_email + wise_email: plain, must contain @
        for t in &["paypal_email", "wise_email"] {
            let (_, plain, enc, _, _) = validate_payout_method(t, "user@example.com").unwrap();
            assert_eq!(plain.unwrap(), "user@example.com");
            assert!(enc.is_none());
            assert!(validate_payout_method(t, "noatsign").is_err());
        }

        // usdc_wallet: 0x + 40 hex
        let (_, plain, _, last4, _) = validate_payout_method(
            "usdc_wallet",
            "0x1234567890abcdef1234567890abcdef12345678",
        )
        .unwrap();
        assert_eq!(plain.unwrap().len(), 42);
        assert_eq!(last4.unwrap(), "5678");
        assert!(validate_payout_method("usdc_wallet", "0x123").is_err());
        assert!(validate_payout_method(
            "usdc_wallet",
            "1234567890abcdef1234567890abcdef12345678"
        )
        .is_err()); // no 0x

        // stripe_connect: acct_xxx
        let (_, plain, _, _, _) =
            validate_payout_method("stripe_connect", "acct_1234567890").unwrap();
        assert!(plain.unwrap().starts_with("acct_"));
        assert!(validate_payout_method("stripe_connect", "1234567890").is_err());

        // unknown type
        assert!(validate_payout_method("crypto_btc", "bc1q...").is_err());
    }
}
