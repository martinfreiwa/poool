use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;
use crate::common::sanitize;

/// Ensure user's payment_methods exist, get them list.
pub async fn list_user_payment_methods(
    pool: &PgPool,
    user_id: &Uuid,
    method_type_filter: Option<&str>,
) -> Result<Vec<PaymentMethod>, sqlx::Error> {
    let mut query =
        String::from("SELECT * FROM payment_methods WHERE user_id = $1 AND status != 'failed'");

    let methods = if let Some(t) = method_type_filter {
        query.push_str(" AND method_type = $2 ORDER BY is_default DESC, created_at DESC");
        sqlx::query_as::<_, PaymentMethod>(&query)
            .bind(user_id)
            .bind(t)
            .fetch_all(pool)
            .await?
    } else {
        query.push_str(" ORDER BY is_default DESC, created_at DESC");
        sqlx::query_as::<_, PaymentMethod>(&query)
            .bind(user_id)
            .fetch_all(pool)
            .await?
    };

    Ok(methods)
}

/// Attach secure card token (mocking Stripe fetch for simplicity)
pub async fn attach_card(
    pool: &PgPool,
    user_id: &Uuid,
    form: AttachCardTokenForm,
) -> Result<PaymentMethod, sqlx::Error> {
    // Extract brand from label if in "Brand ending in XXXX" format
    // e.g. label = "Visa ending in 4242" → brand = "Visa", last4 = "4242"
    let label_str = form.label.as_deref().unwrap_or("");
    let (extracted_brand, extracted_last4) = if label_str.contains("ending in") {
        let parts: Vec<&str> = label_str.splitn(2, " ending in ").collect();
        let brand = parts.first().unwrap_or(&"Card").trim().to_string();
        let last4 = parts.get(1).unwrap_or(&"****").trim().to_string();
        (brand, last4)
    } else {
        // Fall back: try to parse last4 from the token (e.g. "visa_4242_1234567890")
        let token_parts: Vec<&str> = form.stripe_payment_method_id.splitn(3, '_').collect();
        let brand = token_parts
            .first()
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .unwrap_or_else(|| "Card".to_string());
        let last4 = token_parts.get(1).unwrap_or(&"****").to_string();
        (brand, last4)
    };

    let label = form
        .label
        .map(|l| sanitize::sanitize_text(&l))
        .unwrap_or_else(|| format!("{} ending in {}", extracted_brand, extracted_last4));

    let holder_name = sanitize::sanitize_text(&form.holder_name);

    let res = sqlx::query_as::<_, PaymentMethod>(
        r#"
        INSERT INTO payment_methods (
            user_id, method_type, processor_type, processor_token,
            brand, last_four, holder_name, label, is_default, status
        )
        VALUES ($1, 'card', 'manual', $2, $3, $4, $5, $6, false, 'active')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&form.stripe_payment_method_id)
    .bind(&extracted_brand)
    .bind(&extracted_last4)
    .bind(holder_name)
    .bind(&label)
    .fetch_one(pool)
    .await?;

    Ok(res)
}

/// Secure bank details
pub async fn add_bank(
    pool: &PgPool,
    user_id: &Uuid,
    form: AddBankForm,
) -> Result<PaymentMethod, sqlx::Error> {
    // For SEPA/SWIFT the "account_number" is the IBAN — mask everything except the last 4
    let last_four = if form.account_number.len() >= 4 {
        form.account_number[form.account_number.len() - 4..].to_string()
    } else {
        form.account_number.clone()
    };

    let processor_token = format!(
        "bank_{}",
        &form.account_number[..form.account_number.len().min(8)]
    );

    let bank_name = sanitize::sanitize_text(&form.bank_name);
    let holder_name = sanitize::sanitize_text(&form.account_holder_name);
    let default_label = format!("{} ending in {}", bank_name, last_four);
    let final_label = form.label.map(|l| sanitize::sanitize_text(&l)).unwrap_or(default_label);

    let res = sqlx::query_as::<_, PaymentMethod>(
        r#"
        INSERT INTO payment_methods (
            user_id, method_type, processor_type, processor_token,
            brand, last_four, holder_name, routing_number, bank_country,
            bank_system, label, is_default, status
        )
        VALUES ($1, 'bank', 'manual', $2, $3, $4, $5, $6, $7, $8, $9, false, 'active')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(processor_token)
    .bind(&bank_name)
    .bind(&last_four)
    .bind(holder_name)
    .bind(form.routing_code)
    .bind(form.bank_country)
    .bind(form.bank_system)
    .bind(&final_label)
    .fetch_one(pool)
    .await?;

    Ok(res)
}

pub async fn delete_payment_method(
    pool: &PgPool,
    user_id: &Uuid,
    method_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM payment_methods WHERE id = $1 AND user_id = $2")
        .bind(method_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_default_payment_method(
    pool: &PgPool,
    user_id: &Uuid,
    method_id: Uuid,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Determine method_type first so we unset default for THAT type only
    let mt: String = sqlx::query_scalar(
        "SELECT method_type FROM payment_methods WHERE id = $1 AND user_id = $2",
    )
    .bind(method_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE payment_methods SET is_default = false WHERE user_id = $1 AND method_type = $2",
    )
    .bind(user_id)
    .bind(&mt)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE payment_methods SET is_default = true WHERE id = $1 AND user_id = $2")
        .bind(method_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
