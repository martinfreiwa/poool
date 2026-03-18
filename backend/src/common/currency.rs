//! Shared currency formatting utilities.
//!
//! RULE: All monetary values are BIGINT cents — never floats.
//! These functions format cents as human-readable strings using
//! integer-only arithmetic.

/// Format cents as USD string: "$1,234.56"
#[allow(dead_code)]
pub fn format_usd(cents: i64) -> String {
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
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c as char);
    }

    result.push('.');
    result.push_str(&format!("{:02}", remainder));
    result
}

/// Format cents as IDR string using a fixed conversion rate.
/// Uses dot as thousands separator per Indonesian convention.
///
/// `rate` is the USD→IDR rate (e.g., 15_500).
#[allow(dead_code)]
pub fn format_idr(cents: i64, rate: i64) -> String {
    // Integer math: cents → dollars → IDR (no float rounding)
    let idr_val = (cents / 100) * rate;
    let is_negative = idr_val < 0;
    let val = idr_val.abs().to_string();

    let mut result = String::new();
    if is_negative {
        result.push('-');
    }

    let bytes = val.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push('.');
        }
        result.push(c as char);
    }

    format!("Rp {}", result)
}

/// Format cents as a simple display string: "$X.XX" for USD, "Rp X" for IDR.
/// This is the version used in JSON API responses.
#[allow(dead_code)]
pub fn format_amount_display(cents: i64, currency: &str) -> String {
    match currency {
        "IDR" => format_idr(cents, crate::config::DEFAULT_USD_TO_IDR_RATE_I64),
        _ => {
            let abs = (cents as i128).unsigned_abs();
            let sign = if cents < 0 { "-" } else { "" };
            format!("{}${}.{:02}", sign, abs / 100, abs % 100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_usd() {
        assert_eq!(format_usd(0), "$0.00");
        assert_eq!(format_usd(100), "$1.00");
        assert_eq!(format_usd(19999), "$199.99");
        assert_eq!(format_usd(100_000_000), "$1,000,000.00");
        assert_eq!(format_usd(-500), "-$5.00");
    }

    #[test]
    fn test_format_idr() {
        assert_eq!(format_idr(10000, 15_500), "Rp 1.550.000"); // $100 → 1,550,000 IDR
        assert_eq!(format_idr(0, 15_500), "Rp 0");
    }

    #[test]
    fn test_format_amount_display() {
        assert_eq!(format_amount_display(19999, "USD"), "$199.99");
        assert_eq!(format_amount_display(-500, "USD"), "-$5.00");
    }
}
