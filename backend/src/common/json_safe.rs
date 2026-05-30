//! Safe JSON serialisation for embedding inside HTML `<script>` islands.
//!
//! Bundle F11 (CDDRP Phase 3.2): when JSON is shipped via Jinja's `| safe`
//! filter into `<script type="application/json">…</script>`, raw
//! `serde_json::to_string` output can break out of the script tag if any
//! string field contains `</script>` (or U+2028 / U+2029, which are legacy
//! JS line terminators that some older engines treat as statement breaks).
//!
//! This helper escapes those sequences while keeping the output valid JSON
//! (any `\/` inside a JSON string is unescaped correctly by `JSON.parse`).

/// Serialise a value to JSON and escape sequences that would break out of
/// an HTML `<script type="application/json">` island.
///
/// - `</` → `<\/` (escapes `</script>` and any other `</tag>`)
/// - U+2028 → ` `
/// - U+2029 → ` `
///
/// On serialisation failure, returns the JSON literal `null` so the
/// downstream `JSON.parse` call still succeeds.
pub fn to_safe_json_script<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v)
        .unwrap_or_else(|_| "null".to_string())
        .replace("</", "<\\/")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_closing_script_tag() {
        let payload = serde_json::json!({
            "title": "evil</script><script>alert(1)</script>"
        });
        let out = to_safe_json_script(&payload);
        assert!(!out.contains("</script>"), "raw </script> leaked: {out}");
        assert!(out.contains("<\\/script>"), "expected escaped form: {out}");
        // Still valid JSON, and JSON.parse will recover the original string.
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            parsed["title"].as_str().unwrap(),
            "evil</script><script>alert(1)</script>"
        );
    }

    #[test]
    fn escapes_line_terminators() {
        let payload = serde_json::json!({ "s": "a\u{2028}b\u{2029}c" });
        let out = to_safe_json_script(&payload);
        assert!(!out.contains('\u{2028}'));
        assert!(!out.contains('\u{2029}'));
        assert!(out.contains("\\u2028"));
        assert!(out.contains("\\u2029"));
    }

    #[test]
    fn fallback_on_serialise_failure() {
        // f64 NaN is not representable in JSON and makes serde_json error.
        let nan = f64::NAN;
        assert_eq!(to_safe_json_script(&nan), "null");
    }
}
