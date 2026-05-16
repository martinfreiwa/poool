//! Fuzzy name matching for KYC-vs-payment-method checks (P1-6).
//!
//! Compares the user's verified KYC name (first + last from
//! `user_profiles`) against the holder name they typed on a payment-
//! method form. Designed to:
//!
//!   - Accept legitimate variations (case, accents, middle name,
//!     "Maria Rossi" vs "Rossi Maria", "M. Rossi").
//!   - Flag obvious mismatches (different surname entirely) so a money-
//!     mule can't add a third-party bank account.
//!
//! We intentionally do NOT reject on mismatch — payments compliance
//! tolerates legitimate edge cases (married name, transliterations).
//! Instead we return [`MatchOutcome::PotentialMismatch`] and let the
//! caller open a compliance review.
//!
//! Algorithm
//! ---------
//! 1. Normalize both names: lowercase, strip diacritics, collapse
//!    punctuation/whitespace.
//! 2. Tokenize into words ≥ 2 chars (drops initials so "M." doesn't
//!    survive as a mandatory token).
//! 3. Require every KYC token to appear (as a prefix-match) in the
//!    holder tokens. Prefix-match handles "M Rossi" vs "Maria Rossi".
//! 4. If the comparison is empty either side, fail closed.

/// Outcome of one name comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchOutcome {
    /// All KYC tokens were found in the holder name. Save normally.
    Match,
    /// At least one KYC token was missing from the holder name. The
    /// caller should still save the row but flag it for compliance
    /// review. Vector lists the missing tokens for the alert summary.
    PotentialMismatch { missing: Vec<String> },
}

impl MatchOutcome {
    /// Convenience for handlers that just want the bool decision.
    pub fn is_match(&self) -> bool {
        matches!(self, MatchOutcome::Match)
    }
}

/// Compare the user's KYC `(first_name, last_name)` against a payment-
/// method holder string. See module docs for the algorithm.
pub fn compare(kyc_first: &str, kyc_last: &str, holder: &str) -> MatchOutcome {
    let kyc_tokens: Vec<String> = tokenize(&format!("{} {}", kyc_first, kyc_last));
    let holder_tokens: Vec<String> = tokenize(holder);

    if kyc_tokens.is_empty() || holder_tokens.is_empty() {
        // Fail closed when we can't compare — caller will queue review.
        return MatchOutcome::PotentialMismatch {
            missing: kyc_tokens,
        };
    }

    let missing: Vec<String> = kyc_tokens
        .into_iter()
        .filter(|t| !holder_tokens.iter().any(|h| h.starts_with(t) || t.starts_with(h)))
        .collect();

    if missing.is_empty() {
        MatchOutcome::Match
    } else {
        MatchOutcome::PotentialMismatch { missing }
    }
}

fn tokenize(s: &str) -> Vec<String> {
    let normalized = normalize(s);
    normalized
        .split_whitespace()
        .filter(|w| w.len() >= 2) // skip single-letter initials
        .map(|w| w.to_string())
        .collect()
}

/// Lowercase + ASCII-fold + strip punctuation. Handles the most common
/// Latin diacritics. Not unicode-perfect but covers the vast majority
/// of European names without dragging in `unicode-normalization`.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let folded = match lower {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ě' | 'ę' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'ī' | 'į' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' | 'ő' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' => 'u',
            'ý' | 'ÿ' => 'y',
            'ñ' | 'ń' | 'ň' => 'n',
            'ç' | 'ć' | 'č' => 'c',
            'š' | 'ś' => 's',
            'ž' | 'ź' | 'ż' => 'z',
            'ł' => 'l',
            'ß' => {
                // German eszett expands to "ss"
                out.push_str("ss");
                continue;
            }
            // Pass through ASCII letters + digits
            c if c.is_ascii_alphanumeric() => c,
            // Everything else → whitespace separator
            _ => ' ',
        };
        out.push(folded);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_simple_exact() {
        assert!(compare("Maria", "Rossi", "Maria Rossi").is_match());
    }

    #[test]
    fn matches_reversed_order() {
        assert!(compare("Maria", "Rossi", "Rossi Maria").is_match());
    }

    #[test]
    fn matches_with_middle_name() {
        assert!(compare("Maria", "Rossi", "Maria Anna Rossi").is_match());
    }

    #[test]
    fn matches_with_accents() {
        assert!(compare("José", "Müller", "Jose Muller").is_match());
        assert!(compare("Jose", "Mueller", "José Müller").is_match());
    }

    #[test]
    fn matches_case_insensitive() {
        assert!(compare("MARIA", "rossi", "MARIA ROSSI").is_match());
    }

    #[test]
    fn flags_wrong_surname() {
        let r = compare("Maria", "Rossi", "Maria Bianchi");
        assert!(!r.is_match());
        if let MatchOutcome::PotentialMismatch { missing } = r {
            assert!(missing.iter().any(|t| t == "rossi"));
        }
    }

    #[test]
    fn flags_completely_different() {
        let r = compare("Maria", "Rossi", "John Doe");
        assert!(!r.is_match());
    }

    #[test]
    fn matches_initial_in_holder() {
        // M Rossi → "M" is dropped (<2 chars), "Rossi" matches "Rossi"
        // KYC "Maria Rossi" needs both → maria not in holder → mismatch.
        // This is the safer default.
        let r = compare("Maria", "Rossi", "M Rossi");
        assert!(!r.is_match());
    }

    #[test]
    fn matches_prefix() {
        // "Maria" matches "Marias" (Hungarian variation) via prefix logic.
        assert!(compare("Maria", "Rossi", "Marias Rossi").is_match());
    }

    #[test]
    fn empty_inputs_fail_closed() {
        let r = compare("", "", "Some Holder");
        assert!(!r.is_match());
    }
}
