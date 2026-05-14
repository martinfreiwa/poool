//! Villa-Returns FX populator — nightly IDR→USD rate fetch.
//!
//! Writes a daily row into `fx_rates_daily` so the operations-publish step and
//! the NAV snapshot job have a fresh rate to freeze. Source: exchangerate.host
//! (keyless; an optional `EXCHANGERATE_HOST_ACCESS_KEY` env var is appended if
//! set, for the access-key tier).
//!
//! Scale convention: `rate_bps` stores the IDR→USD rate × 10^7, matching the
//! migration 147 dev seed (0.0000645 → 645). NOTE the downstream consumer in
//! `villa_nav_snapshot.rs` applies `idr * rate_bps / 10_000`, which assumes a
//! different scale — that mismatch is a pre-existing imprecision flagged in
//! migration 147 as a separate slice (B2-X) and is deliberately NOT addressed
//! here. This populator's only job is to keep `fx_rates_daily` consistent with
//! the seed so behaviour does not change.

use sqlx::PgPool;

/// `rate_bps` = IDR→USD rate × 10^7 (matches the migration 147 seed).
const FX_SCALE: f64 = 10_000_000.0;

/// Fetch the latest IDR→USD rate and upsert today's `fx_rates_daily` row.
pub async fn run_fx_populator(
    pool: &PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut url = "https://api.exchangerate.host/latest?base=IDR&symbols=USD".to_string();
    if let Ok(key) = std::env::var("EXCHANGERATE_HOST_ACCESS_KEY") {
        if !key.trim().is_empty() {
            url.push_str(&format!("&access_key={}", key.trim()));
        }
    }

    let resp = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(format!("exchangerate.host returned HTTP {}", resp.status()).into());
    }

    let body: serde_json::Value = resp.json().await?;
    let rate = body
        .get("rates")
        .and_then(|r| r.get("USD"))
        .and_then(|v| v.as_f64())
        .ok_or("exchangerate.host response missing rates.USD")?;
    if !(rate.is_finite()) || rate <= 0.0 {
        return Err(format!("non-positive or non-finite FX rate: {rate}").into());
    }

    let rate_bps = (rate * FX_SCALE).round() as i64;
    if rate_bps <= 0 {
        return Err(format!("FX rate {rate} rounds to zero at scale {FX_SCALE}").into());
    }

    sqlx::query(
        r#"
        INSERT INTO fx_rates_daily (snapshot_date, base_currency, quote_currency, rate_bps, source)
        VALUES (CURRENT_DATE, 'IDR', 'USD', $1, 'exchangerate.host')
        ON CONFLICT (snapshot_date, base_currency, quote_currency)
        DO UPDATE SET rate_bps = EXCLUDED.rate_bps, source = EXCLUDED.source
        "#,
    )
    .bind(rate_bps)
    .execute(pool)
    .await?;

    tracing::info!("Villa FX populator: IDR->USD rate {rate} -> rate_bps {rate_bps}");
    Ok(())
}
