/// Candlestick Chart API — OHLCV aggregation from trade_history.
///
/// Provides `GET /api/marketplace/:asset_id/candles?interval=1h&from=&to=`
///
/// Since we don't have TimescaleDB continuous aggregates (tasks 2.9/2.10 blocked),
/// this implementation uses raw SQL aggregation over trade_history.
/// When TimescaleDB is available, swap the queries to use the materialized views.
///
/// Supported intervals: 1m, 5m, 15m, 1h, 4h, 1d, 1w
///
/// All prices are in cents (i64). No floats.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── DTOs ──────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Query parameters for the candles endpoint.
#[derive(Debug, Deserialize)]
pub struct CandleQuery {
    /// Interval: "1m", "5m", "15m", "1h", "4h", "1d", "1w"
    pub interval: Option<String>,
    /// Start time (ISO 8601). Default: 24h ago.
    pub from: Option<DateTime<Utc>>,
    /// End time (ISO 8601). Default: now.
    pub to: Option<DateTime<Utc>>,
    /// Max candles to return. Default: 500, max: 2000.
    pub limit: Option<i32>,
}

/// A single OHLCV candlestick.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Candle {
    /// Start of the bucket (rounded to interval).
    pub timestamp: DateTime<Utc>,
    /// Opening price (first trade in bucket) in cents.
    pub open_cents: i64,
    /// Highest price in bucket in cents.
    pub high_cents: i64,
    /// Lowest price in bucket in cents.
    pub low_cents: i64,
    /// Closing price (last trade in bucket) in cents.
    pub close_cents: i64,
    /// Total volume (tokens traded) in bucket.
    pub volume: i64,
    /// Number of trades in this bucket.
    pub trade_count: i64,
}

/// Full candle response including metadata.
#[derive(Debug, Serialize)]
pub struct CandleResponse {
    pub asset_id: Uuid,
    pub interval: String,
    pub candles: Vec<Candle>,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════
// ── INTERVAL PARSING ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Parse an interval string into a PostgreSQL interval expression.
/// Returns (pg_interval, max_default_lookback_hours).
fn parse_interval(s: &str) -> Result<(&'static str, i64), AppError> {
    match s {
        "1m" => Ok(("1 minute", 6)),     // 6h of 1-min candles = 360 max
        "5m" => Ok(("5 minutes", 24)),   // 24h of 5-min = 288 max
        "15m" => Ok(("15 minutes", 72)), // 3d of 15-min = 288 max
        "1h" => Ok(("1 hour", 168)),     // 7d of 1h = 168
        "4h" => Ok(("4 hours", 720)),    // 30d of 4h = 180
        "1d" => Ok(("1 day", 2160)),     // 90d of 1d
        "1w" => Ok(("1 week", 8760)),    // 1y of 1w
        _ => Err(AppError::BadRequest(format!(
            "Invalid interval '{}'. Supported: 1m, 5m, 15m, 1h, 4h, 1d, 1w",
            s
        ))),
    }
}

// ═══════════════════════════════════════════════════════════════
// ── FETCH CANDLES ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Fetch OHLCV candles from trade_history using date_trunc aggregation.
///
/// This is the "no TimescaleDB" fallback. It works but will be slower
/// for large datasets compared to continuous aggregates.
pub async fn get_candles(
    pool: &PgPool,
    asset_id: Uuid,
    query: CandleQuery,
) -> Result<CandleResponse, AppError> {
    let interval_str = query.interval.as_deref().unwrap_or("1h");
    let (_pg_interval, default_lookback) = parse_interval(interval_str)?;
    let limit = query.limit.unwrap_or(500).min(2000).max(1) as i64;

    let now = Utc::now();
    let from = query
        .from
        .unwrap_or_else(|| now - chrono::Duration::hours(default_lookback));
    let to = query.to.unwrap_or(now);

    // Use date_trunc for 1d and 1w, time_bucket-style floor for sub-day
    // Since we don't have TimescaleDB, we use date_trunc + floor arithmetic.
    //
    // For standard intervals (1h, 1d, 1w), date_trunc works directly.
    // For non-standard (5m, 15m, 4h), we use epoch-based bucketing.
    let candles = match interval_str {
        "1h" | "1d" | "1w" => {
            let trunc_unit = match interval_str {
                "1h" => "hour",
                "1d" => "day",
                "1w" => "week",
                _ => "hour",
            };

            let query_str = format!(
                r#"SELECT
                    date_trunc('{}', executed_at) AS timestamp,
                    (array_agg(price_cents ORDER BY executed_at ASC))[1] AS open_cents,
                    MAX(price_cents) AS high_cents,
                    MIN(price_cents) AS low_cents,
                    (array_agg(price_cents ORDER BY executed_at DESC))[1] AS close_cents,
                    COALESCE(SUM(quantity::BIGINT), 0) AS volume,
                    COUNT(*) AS trade_count
                FROM trade_history
                WHERE asset_id = $1 AND executed_at >= $2 AND executed_at <= $3
                GROUP BY date_trunc('{}', executed_at)
                ORDER BY timestamp ASC
                LIMIT $4"#,
                trunc_unit, trunc_unit
            );

            sqlx::query_as::<_, Candle>(&query_str)
                .bind(asset_id)
                .bind(from)
                .bind(to)
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(AppError::Database)?
        }

        _ => {
            // For non-standard intervals (1m, 5m, 15m, 4h),
            // use epoch-based floor bucketing.
            let interval_seconds: i64 = match interval_str {
                "1m" => 60,
                "5m" => 300,
                "15m" => 900,
                "4h" => 14400,
                _ => 3600,
            };

            let query_str = format!(
                r#"SELECT
                    to_timestamp(
                        FLOOR(EXTRACT(EPOCH FROM executed_at) / {0}) * {0}
                    ) AT TIME ZONE 'UTC' AS timestamp,
                    (array_agg(price_cents ORDER BY executed_at ASC))[1] AS open_cents,
                    MAX(price_cents) AS high_cents,
                    MIN(price_cents) AS low_cents,
                    (array_agg(price_cents ORDER BY executed_at DESC))[1] AS close_cents,
                    COALESCE(SUM(quantity::BIGINT), 0) AS volume,
                    COUNT(*) AS trade_count
                FROM trade_history
                WHERE asset_id = $1 AND executed_at >= $2 AND executed_at <= $3
                GROUP BY FLOOR(EXTRACT(EPOCH FROM executed_at) / {0})
                ORDER BY timestamp ASC
                LIMIT $4"#,
                interval_seconds
            );

            sqlx::query_as::<_, Candle>(&query_str)
                .bind(asset_id)
                .bind(from)
                .bind(to)
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(AppError::Database)?
        }
    };

    Ok(CandleResponse {
        asset_id,
        interval: interval_str.to_string(),
        candles,
        from,
        to,
    })
}

/// Get the latest price and 24h stats for a candle chart overlay.
#[derive(Debug, Serialize)]
pub struct ChartSummary {
    pub asset_id: Uuid,
    pub last_price_cents: Option<i64>,
    pub high_24h_cents: Option<i64>,
    pub low_24h_cents: Option<i64>,
    pub volume_24h: Option<i64>,
    pub change_24h_cents: Option<i64>,
    pub change_24h_pct: Option<f64>,
}

/// Fetch a compact 24h summary for chart header display.
pub async fn get_chart_summary(pool: &PgPool, asset_id: Uuid) -> Result<ChartSummary, AppError> {
    #[derive(sqlx::FromRow)]
    struct RawSummary {
        last_price: Option<i64>,
        high_24h: Option<i64>,
        low_24h: Option<i64>,
        volume_24h: Option<i64>,
        first_price: Option<i64>,
    }

    let summary = sqlx::query_as::<_, RawSummary>(
        r#"SELECT
            (SELECT price_cents FROM trade_history
             WHERE asset_id = $1 ORDER BY executed_at DESC LIMIT 1) AS last_price,
            (SELECT MAX(price_cents) FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours') AS high_24h,
            (SELECT MIN(price_cents) FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours') AS low_24h,
            (SELECT COALESCE(SUM(quantity::BIGINT), 0)::BIGINT FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours') AS volume_24h,
            (SELECT price_cents FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours'
             ORDER BY executed_at ASC LIMIT 1) AS first_price"#,
    )
    .bind(asset_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let change_cents = match (summary.last_price, summary.first_price) {
        (Some(last), Some(first)) if first > 0 => Some(last - first),
        _ => None,
    };

    let change_pct = match (summary.last_price, summary.first_price) {
        (Some(last), Some(first)) if first > 0 => {
            Some(((last - first) as f64 / first as f64) * 100.0)
        }
        _ => None,
    };

    Ok(ChartSummary {
        asset_id,
        last_price_cents: summary.last_price,
        high_24h_cents: summary.high_24h,
        low_24h_cents: summary.low_24h,
        volume_24h: summary.volume_24h,
        change_24h_cents: change_cents,
        change_24h_pct: change_pct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interval_valid() {
        assert!(parse_interval("1m").is_ok());
        assert!(parse_interval("5m").is_ok());
        assert!(parse_interval("15m").is_ok());
        assert!(parse_interval("1h").is_ok());
        assert!(parse_interval("4h").is_ok());
        assert!(parse_interval("1d").is_ok());
        assert!(parse_interval("1w").is_ok());
    }

    #[test]
    fn test_parse_interval_invalid() {
        assert!(parse_interval("2m").is_err());
        assert!(parse_interval("3h").is_err());
        assert!(parse_interval("1y").is_err());
        assert!(parse_interval("").is_err());
    }

    #[test]
    fn test_parse_interval_returns_correct_pg() {
        let (pg, _) = parse_interval("1h").unwrap();
        assert_eq!(pg, "1 hour");

        let (pg, _) = parse_interval("1d").unwrap();
        assert_eq!(pg, "1 day");

        let (pg, _) = parse_interval("5m").unwrap();
        assert_eq!(pg, "5 minutes");
    }

    #[test]
    fn test_limit_clamping() {
        fn clamp_limit(requested_limit: Option<i32>) -> i32 {
            requested_limit.unwrap_or(500).clamp(1, 2000)
        }

        let limit = clamp_limit(Some(5000));
        assert_eq!(limit, 2000);

        let limit = clamp_limit(Some(0));
        assert_eq!(limit, 1);

        let limit = clamp_limit(None);
        assert_eq!(limit, 500);
    }

    #[test]
    fn test_change_pct_calculation() {
        let last = 15000i64; // $150
        let first = 10000i64; // $100
        let pct = ((last - first) as f64 / first as f64) * 100.0;
        assert!((pct - 50.0).abs() < 0.001); // 50% increase

        // Negative
        let last2 = 8000i64;
        let first2 = 10000i64;
        let pct2 = ((last2 - first2) as f64 / first2 as f64) * 100.0;
        assert!((pct2 - (-20.0)).abs() < 0.001); // 20% decrease
    }

    #[test]
    fn test_default_lookback_per_interval() {
        let (_, lookback_1m) = parse_interval("1m").unwrap();
        let (_, lookback_1h) = parse_interval("1h").unwrap();
        let (_, lookback_1d) = parse_interval("1d").unwrap();

        assert!(lookback_1m < lookback_1h);
        assert!(lookback_1h < lookback_1d);
    }
}
