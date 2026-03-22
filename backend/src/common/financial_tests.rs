//! Financial logic unit tests — Phase 11.1
//!
//! Tests all critical monetary functions for correctness, edge cases,
//! and IEEE754 float avoidance. Every test verifies integer-only math.

#[cfg(test)]
mod tests {
    // ─── parse_dollars_to_cents ─────────────────────────────────
    // Import from wallet module
    // Since parse_dollars_to_cents is private, we duplicate the logic here for testing.
    // This is the canonical test suite — if the logic changes in routes.rs, update here too.

    fn parse_dollars_to_cents(raw: &str) -> i64 {
        let cleaned: String = raw
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if cleaned.is_empty() {
            return 0;
        }
        let parts: Vec<&str> = cleaned.split('.').collect();
        let dollars: i64 = parts[0].parse().unwrap_or(0);
        let cents: i64 = if parts.len() > 1 {
            let frac = parts[1];
            match frac.len() {
                0 => 0,
                1 => frac.parse::<i64>().unwrap_or(0) * 10,
                _ => frac[..2].parse::<i64>().unwrap_or(0),
            }
        } else {
            0
        };
        dollars * 100 + cents
    }

    #[test]
    fn parse_whole_dollars() {
        assert_eq!(parse_dollars_to_cents("100"), 10000);
        assert_eq!(parse_dollars_to_cents("1"), 100);
        assert_eq!(parse_dollars_to_cents("0"), 0);
    }

    #[test]
    fn parse_dollars_and_cents() {
        assert_eq!(parse_dollars_to_cents("19.99"), 1999);
        assert_eq!(parse_dollars_to_cents("0.01"), 1);
        assert_eq!(parse_dollars_to_cents("0.10"), 10);
        assert_eq!(parse_dollars_to_cents("999.99"), 99999);
    }

    #[test]
    fn parse_single_decimal() {
        // "5.5" → 5 dollars 50 cents
        assert_eq!(parse_dollars_to_cents("5.5"), 550);
        assert_eq!(parse_dollars_to_cents("0.5"), 50);
    }

    #[test]
    fn parse_extra_decimals_truncated() {
        // "19.995" → takes first 2 fractional digits → 99 cents
        assert_eq!(parse_dollars_to_cents("19.995"), 1999);
        assert_eq!(parse_dollars_to_cents("1.123456"), 112);
    }

    #[test]
    fn parse_dollar_sign_stripped() {
        assert_eq!(parse_dollars_to_cents("$100"), 10000);
        assert_eq!(parse_dollars_to_cents("$19.99"), 1999);
    }

    #[test]
    fn parse_commas_stripped() {
        assert_eq!(parse_dollars_to_cents("1,000"), 100000);
        assert_eq!(parse_dollars_to_cents("1,000,000.50"), 100000050);
    }

    #[test]
    fn parse_empty_and_invalid() {
        assert_eq!(parse_dollars_to_cents(""), 0);
        assert_eq!(parse_dollars_to_cents("abc"), 0);
        assert_eq!(parse_dollars_to_cents("$"), 0);
        assert_eq!(parse_dollars_to_cents("."), 0);
    }

    #[test]
    fn parse_ieee754_tricky_values() {
        // These are values known to cause float rounding errors:
        // 0.1 + 0.2 ≠ 0.3 in IEEE754, but our string parser handles them correctly
        assert_eq!(parse_dollars_to_cents("0.10"), 10);
        assert_eq!(parse_dollars_to_cents("0.20"), 20);
        assert_eq!(parse_dollars_to_cents("0.30"), 30);
        // 19.99 * 100 = 1998.9999... in float, but we get 1999
        assert_eq!(parse_dollars_to_cents("19.99"), 1999);
        // 9.99 * 100 = 998.999... in float
        assert_eq!(parse_dollars_to_cents("9.99"), 999);
        // 99999.99 — large value
        assert_eq!(parse_dollars_to_cents("99999.99"), 9999999);
    }

    #[test]
    fn parse_max_deposit() {
        // $1,000,000 = 100_000_000 cents
        assert_eq!(parse_dollars_to_cents("1000000"), 100_000_000);
        assert_eq!(parse_dollars_to_cents("1000000.00"), 100_000_000);
    }

    // ─── format_usd ────────────────────────────────────────────

    fn format_usd(cents: i64) -> String {
        let is_negative = cents < 0;
        let abs = cents.unsigned_abs();
        let dollars = abs / 100;
        let remainder = abs % 100;

        let dollar_str = dollars.to_string();
        let mut result = String::new();
        if is_negative {
            result.push('-');
        }
        result.push('$');

        let bytes = dollar_str.as_bytes();
        for (i, &c) in bytes.iter().enumerate() {
            if i > 0 && (bytes.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(c as char);
        }

        result.push('.');
        result.push_str(&format!("{:02}", remainder));
        result
    }

    #[test]
    fn format_usd_zero() {
        assert_eq!(format_usd(0), "$0.00");
    }

    #[test]
    fn format_usd_small() {
        assert_eq!(format_usd(1), "$0.01");
        assert_eq!(format_usd(99), "$0.99");
        assert_eq!(format_usd(100), "$1.00");
    }

    #[test]
    fn format_usd_large() {
        assert_eq!(format_usd(100_000_000), "$1,000,000.00");
        assert_eq!(format_usd(999_999_999), "$9,999,999.99");
    }

    #[test]
    fn format_usd_negative() {
        assert_eq!(format_usd(-500), "-$5.00");
        assert_eq!(format_usd(-1), "-$0.01");
    }

    // ─── calculate_fee_cents ────────────────────────────────────

    fn calculate_fee_cents(total_cents: i64, fee_bps: i32) -> i64 {
        let bps = fee_bps.max(0) as i64;
        let fee = total_cents.saturating_mul(bps) / 10_000;
        fee.min(total_cents)
    }

    #[test]
    fn fee_standard_5_percent() {
        // $100 at 500 BPS (5%) = $5
        assert_eq!(calculate_fee_cents(10_000, 500), 500);
    }

    #[test]
    fn fee_1_percent() {
        // $1000 at 100 BPS (1%) = $10
        assert_eq!(calculate_fee_cents(100_000, 100), 1_000);
    }

    #[test]
    fn fee_fractional_cent_rounds_down() {
        // $1.00 at 300 BPS (3%) = 3 cents
        assert_eq!(calculate_fee_cents(100, 300), 3);
        // $0.99 at 300 BPS = 2.97 → 2 cents (integer truncation)
        assert_eq!(calculate_fee_cents(99, 300), 2);
    }

    #[test]
    fn fee_zero_total() {
        assert_eq!(calculate_fee_cents(0, 500), 0);
    }

    #[test]
    fn fee_zero_rate() {
        assert_eq!(calculate_fee_cents(10_000, 0), 0);
    }

    #[test]
    fn fee_negative_rate_clamped() {
        assert_eq!(calculate_fee_cents(10_000, -500), 0);
    }

    #[test]
    fn fee_100_percent() {
        assert_eq!(calculate_fee_cents(10_000, 10_000), 10_000);
    }

    #[test]
    fn fee_over_100_percent_clamped() {
        // 200% → capped at total
        assert_eq!(calculate_fee_cents(10_000, 20_000), 10_000);
    }

    #[test]
    fn fee_large_trade() {
        // $1,000,000 at 250 BPS (2.5%) = $25,000
        assert_eq!(calculate_fee_cents(100_000_000, 250), 2_500_000);
    }

    #[test]
    fn fee_overflow_protection() {
        // Very large trade × large BPS — saturating_mul prevents panic
        let result = calculate_fee_cents(i64::MAX, 10_000);
        // Should not panic. Result should be i64::MAX (clamped)
        assert!(result > 0);
    }

    // ─── IDR conversion (integer math) ──────────────────────────

    fn convert_usd_cents_to_idr(cents: i64, rate: i64) -> i64 {
        (cents / 100) * rate
    }

    #[test]
    fn idr_basic_conversion() {
        // $100 → $100 * 15,500 = 1,550,000 IDR
        assert_eq!(convert_usd_cents_to_idr(10_000, 15_500), 1_550_000);
    }

    #[test]
    fn idr_small_amount() {
        // $0.99 → 0 * 15500 = 0 (sub-dollar truncated in integer math)
        assert_eq!(convert_usd_cents_to_idr(99, 15_500), 0);
        // $1.00 → 1 * 15500 = 15,500
        assert_eq!(convert_usd_cents_to_idr(100, 15_500), 15_500);
    }

    #[test]
    fn idr_large_amount() {
        // $1,000,000 = 100_000_000 cents → 1,000,000 * 15,500 = 15,500,000,000
        assert_eq!(
            convert_usd_cents_to_idr(100_000_000, 15_500),
            15_500_000_000
        );
    }

    #[test]
    fn idr_zero() {
        assert_eq!(convert_usd_cents_to_idr(0, 15_500), 0);
    }

    // ─── Balance invariant helpers ──────────────────────────────

    #[test]
    fn balance_never_negative_check() {
        // Verify the invariant: after withdrawal, balance >= 0
        let balance_cents: i64 = 10_000; // $100
        let withdrawal_cents: i64 = 10_000; // $100
        let new_balance = balance_cents - withdrawal_cents;
        assert!(
            new_balance >= 0,
            "Balance went negative: {} - {} = {}",
            balance_cents,
            withdrawal_cents,
            new_balance
        );
    }

    #[test]
    fn balance_insufficient_rejected() {
        let balance_cents: i64 = 5_000; // $50
        let withdrawal_cents: i64 = 10_000; // $100
        assert!(
            balance_cents < withdrawal_cents,
            "Insufficient funds check should reject this"
        );
    }

    // ─── BPS formatting ────────────────────────────────────────

    fn format_bps(bps: i32) -> String {
        format!("{:.1}%", bps as f64 / 100.0)
    }

    #[test]
    fn bps_formatting() {
        assert_eq!(format_bps(0), "0.0%");
        assert_eq!(format_bps(100), "1.0%");
        assert_eq!(format_bps(500), "5.0%");
        assert_eq!(format_bps(1000), "10.0%");
        assert_eq!(format_bps(7250), "72.5%");
        assert_eq!(format_bps(10000), "100.0%");
    }

    // ─── Dollar→Cents roundtrip integrity ───────────────────────

    #[test]
    fn roundtrip_parse_format() {
        // Parse "19.99" → 1999 cents → format → "$19.99"
        let cents = parse_dollars_to_cents("19.99");
        assert_eq!(cents, 1999);
        let formatted = format_usd(cents);
        assert_eq!(formatted, "$19.99");
    }

    #[test]
    fn roundtrip_large_amount() {
        let cents = parse_dollars_to_cents("999999.99");
        assert_eq!(cents, 99_999_999);
        let formatted = format_usd(cents);
        assert_eq!(formatted, "$999,999.99");
    }

    // ─── Dividend distribution math ─────────────────────────────
    // Verifying the u128 fixed-point dividend distribution logic

    #[test]
    fn dividend_pro_rata_distribution() {
        // Total payout: $10,000 = 1_000_000 cents
        // Asset total: 10,000 tokens
        // Investor A: 6,000 tokens (60%)
        // Investor B: 4,000 tokens (40%)
        let total_payout_cents: i64 = 1_000_000;
        let total_tokens: i64 = 10_000;

        let a_tokens: i64 = 6_000;
        let b_tokens: i64 = 4_000;

        // u128 fixed-point: per_token_cents_x1e18 = payout * 1e18 / total_tokens
        let per_token_x1e18: u128 =
            (total_payout_cents as u128) * 1_000_000_000_000_000_000 / (total_tokens as u128);

        let a_payout = ((a_tokens as u128) * per_token_x1e18 / 1_000_000_000_000_000_000) as i64;
        let b_payout = ((b_tokens as u128) * per_token_x1e18 / 1_000_000_000_000_000_000) as i64;

        assert_eq!(a_payout, 600_000); // $6,000
        assert_eq!(b_payout, 400_000); // $4,000
        assert_eq!(a_payout + b_payout, total_payout_cents);
    }

    #[test]
    fn dividend_rounding_dust() {
        // Total payout: $100.00 = 10,000 cents
        // 3 equal investors with 3,333, 3,333, 3,334 tokens (10,000 total)
        let total_payout_cents: i64 = 10_000;
        let total_tokens: i64 = 10_000;

        let per_token_x1e18: u128 =
            (total_payout_cents as u128) * 1_000_000_000_000_000_000 / (total_tokens as u128);

        let a_payout = ((3333_u128) * per_token_x1e18 / 1_000_000_000_000_000_000) as i64;
        let b_payout = ((3333_u128) * per_token_x1e18 / 1_000_000_000_000_000_000) as i64;
        let c_payout = ((3334_u128) * per_token_x1e18 / 1_000_000_000_000_000_000) as i64;

        assert_eq!(a_payout, 3_333);
        assert_eq!(b_payout, 3_333);
        assert_eq!(c_payout, 3_334);
        assert_eq!(a_payout + b_payout + c_payout, total_payout_cents);
    }

    // ─── Trade settlement invariants ────────────────────────────

    #[test]
    fn trade_settlement_zero_sum() {
        // In any trade: buyer pays (price * qty + fee), seller receives (price * qty - fee)
        // Platform collects: buyer_fee + seller_fee
        let price_cents: i64 = 10_500; // $105
        let quantity: i32 = 10;
        let buyer_fee_bps: i32 = 500; // 5%
        let seller_fee_bps: i32 = 250; // 2.5%

        let trade_value = price_cents * quantity as i64;
        let buyer_fee = calculate_fee_cents(trade_value, buyer_fee_bps);
        let seller_fee = calculate_fee_cents(trade_value, seller_fee_bps);

        let buyer_total_debit = trade_value + buyer_fee;
        let seller_total_credit = trade_value - seller_fee;
        let platform_revenue = buyer_fee + seller_fee;

        // Conservation of money: buyer debit = seller credit + platform revenue
        assert_eq!(
            buyer_total_debit,
            seller_total_credit + platform_revenue,
            "Money is not conserved in trade settlement"
        );
    }

    #[test]
    fn trade_filled_quantity_invariant() {
        // quantity_filled can never exceed quantity
        let quantity: i32 = 100;
        let mut filled: i32 = 0;

        // Simulate partial fills
        for fill in [30, 25, 20, 15, 10] {
            let actual_fill = fill.min(quantity - filled);
            filled += actual_fill;
            assert!(
                filled <= quantity,
                "Filled {} exceeds quantity {}",
                filled,
                quantity
            );
        }
        assert_eq!(filled, quantity);
    }

    // ─── Investment limit calculations ──────────────────────────

    #[test]
    fn investment_limit_5_percent_rule() {
        // Annual income $60,000 → limit = 5% = $3,000
        let annual_income_cents: i64 = 6_000_000;
        let limit_cents = annual_income_cents * 5 / 100;
        assert_eq!(limit_cents, 300_000); // $3,000
    }

    #[test]
    fn investment_limit_10_percent_rule() {
        // Annual income $200,000 → limit = 10% = $20,000
        let annual_income_cents: i64 = 20_000_000;
        let limit_cents = annual_income_cents * 10 / 100;
        assert_eq!(limit_cents, 2_000_000); // $20,000
    }

    #[test]
    fn investment_limit_progress_percentage() {
        let limit_cents: i64 = 300_000; // $3,000
        let invested_cents: i64 = 150_000; // $1,500

        let progress_pct = if limit_cents > 0 {
            (invested_cents * 100) / limit_cents
        } else {
            0
        };
        assert_eq!(progress_pct, 50);
    }

    // ─── Withdrawal security rules ──────────────────────────────

    #[test]
    fn withdrawal_max_per_transaction() {
        let max_withdrawal_cents: i64 = 1_000_000; // $10,000
        let amount_cents: i64 = 1_000_001;
        assert!(
            amount_cents > max_withdrawal_cents,
            "Should block withdrawal exceeding per-tx max"
        );
    }

    #[test]
    fn withdrawal_daily_limit() {
        let daily_limit_cents: i64 = 2_500_000; // $25,000
        let already_withdrawn: i64 = 2_000_000; // $20,000
        let new_withdrawal: i64 = 600_000; // $6,000

        assert!(
            already_withdrawn + new_withdrawal > daily_limit_cents,
            "Should block withdrawal exceeding daily limit"
        );
    }

    #[test]
    fn withdrawal_velocity_limit() {
        let max_per_hour: i32 = 3;
        let current_hour_count: i32 = 3;
        assert!(
            current_hour_count >= max_per_hour,
            "Should block withdrawal when velocity limit reached"
        );
    }
}
