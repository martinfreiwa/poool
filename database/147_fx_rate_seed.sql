-- 147 — Villa-Returns B2: placeholder IDR→USD FX rate.
--
-- Seeds a single row in fx_rates_daily so the NAV snapshot job has a value to
-- freeze. Real-world rate today: 1 USD ≈ 15,500 IDR → 1 IDR ≈ 0.0000645 USD.
--
-- Scale convention (documented per the migration 140 design note):
-- rate_bps for IDR→USD is interpreted by the application as the conversion
-- coefficient when divided by 10000; the resulting USD-cent figure for typical
-- IDR-cent inputs will be approximate at integer scale. A higher-precision
-- column (e.g. micro-bps) is a separate slice (B2-X) if more accuracy is
-- needed before production use.

INSERT INTO fx_rates_daily (snapshot_date, base_currency, quote_currency, rate_bps, source)
VALUES (CURRENT_DATE, 'IDR', 'USD', 645, 'manual_dev_seed')
ON CONFLICT (snapshot_date, base_currency, quote_currency) DO NOTHING;
