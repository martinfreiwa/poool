//! Phase-3 fresh: typed in-app notification wrappers for the affiliate domain.
//!
//! Centralises the event-type strings + metadata shape so producers call
//! a single typed function instead of remembering whitelist values from
//! migration 201. All helpers are best-effort: a notify failure does NOT
//! propagate to the caller — the underlying business event has already
//! committed. Failures are traced.
//!
//! Schema reference: `database/183_notifications_inbox.sql` (base) +
//! `database/201_notification_types_extended.sql` (the 3 new types used here).

use crate::common::notifications::enqueue_notification;
use sqlx::PgPool;
use uuid::Uuid;

/// commission earned — fires when `affiliate_commissions` row is created.
pub async fn notify_commission_earned(
    pool: &PgPool,
    affiliate_user_id: Uuid,
    amount_cents: i64,
    currency: &str,
    source_order_id: Uuid,
) {
    let title = format!(
        "Commission earned: {} {}",
        format_minor_units(amount_cents),
        currency
    );
    let meta = serde_json::json!({
        "amount_cents": amount_cents,
        "currency": currency,
        "source_order_id": source_order_id,
    });
    if let Err(e) = enqueue_notification(
        pool,
        affiliate_user_id,
        "affiliate_commission_earned",
        &title,
        Some("A new commission has been added to your ledger."),
        Some("/affiliate/dashboard"),
        meta,
    )
    .await
    {
        tracing::warn!(
            user_id = %affiliate_user_id,
            error = %e,
            "notify_commission_earned failed (non-fatal)"
        );
    }
}

/// payout released — fires once per invoice issued from a payout batch.
pub async fn notify_payout_released(
    pool: &PgPool,
    affiliate_user_id: Uuid,
    invoice_id: Uuid,
    amount_cents: i64,
    currency: &str,
) {
    let title = format!(
        "Payout released: {} {}",
        format_minor_units(amount_cents),
        currency
    );
    let meta = serde_json::json!({
        "invoice_id": invoice_id,
        "amount_cents": amount_cents,
        "currency": currency,
    });
    if let Err(e) = enqueue_notification(
        pool,
        affiliate_user_id,
        "affiliate_payout_released",
        &title,
        Some("Your invoice is available."),
        Some(&format!("/affiliate/invoices/{}", invoice_id)),
        meta,
    )
    .await
    {
        tracing::warn!(
            user_id = %affiliate_user_id,
            error = %e,
            "notify_payout_released failed (non-fatal)"
        );
    }
}

/// clawback — fires once per (affiliate, refunded investment) combination.
pub async fn notify_commission_clawed_back(
    pool: &PgPool,
    affiliate_user_id: Uuid,
    investment_id: Uuid,
    reason: &str,
) {
    let meta = serde_json::json!({
        "investment_id": investment_id,
        "reason": reason,
    });
    if let Err(e) = enqueue_notification(
        pool,
        affiliate_user_id,
        "affiliate_commission_clawed_back",
        "Commission reversed (refund)",
        Some(reason),
        Some("/affiliate/dashboard"),
        meta,
    )
    .await
    {
        tracing::warn!(
            user_id = %affiliate_user_id,
            error = %e,
            "notify_commission_clawed_back failed (non-fatal)"
        );
    }
}

/// policy re-acceptance — fires when an affiliate must re-accept an updated
/// policy version before the next dashboard load is unblocked.
pub async fn notify_policy_update_required(
    pool: &PgPool,
    user_id: Uuid,
    policy_name: &str,
    new_version: &str,
) {
    let title = format!("Action required: re-accept {}", policy_name);
    let meta = serde_json::json!({
        "policy_name": policy_name,
        "new_version": new_version,
    });
    if let Err(e) = enqueue_notification(
        pool,
        user_id,
        "affiliate_policy_update_required",
        &title,
        Some("A policy you previously accepted has been updated."),
        Some("/affiliate/dashboard"),
        meta,
    )
    .await
    {
        tracing::warn!(
            user_id = %user_id,
            error = %e,
            "notify_policy_update_required failed (non-fatal)"
        );
    }
}

/// tax doc — fires when an affiliate has earned commissions but has no tax
/// document on file (blocks next payout release).
pub async fn notify_tax_doc_required(pool: &PgPool, user_id: Uuid, pending_amount_cents: i64) {
    let meta = serde_json::json!({
        "pending_amount_cents": pending_amount_cents,
    });
    if let Err(e) = enqueue_notification(
        pool,
        user_id,
        "affiliate_tax_doc_required",
        "Tax document required for next payout",
        Some("Upload your tax document to unlock the next scheduled payout."),
        Some("/affiliate/settings"),
        meta,
    )
    .await
    {
        tracing::warn!(
            user_id = %user_id,
            error = %e,
            "notify_tax_doc_required failed (non-fatal)"
        );
    }
}

fn format_minor_units(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    let major = abs / 100;
    let minor = abs % 100;
    format!("{}{}.{:02}", sign, major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_minor_units_handles_signs_and_padding() {
        assert_eq!(format_minor_units(0), "0.00");
        assert_eq!(format_minor_units(7), "0.07");
        assert_eq!(format_minor_units(1234), "12.34");
        assert_eq!(format_minor_units(-1234), "-12.34");
        assert_eq!(format_minor_units(100_000), "1000.00");
    }
}
