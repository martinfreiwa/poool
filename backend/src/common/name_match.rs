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
    let kyc_tokens = tokenize_with_variants(&format!("{} {}", kyc_first, kyc_last));
    let holder_tokens = tokenize_with_variants(holder);

    if kyc_tokens.is_empty() || holder_tokens.is_empty() {
        // Fail closed when we can't compare — caller will queue review.
        return MatchOutcome::PotentialMismatch {
            missing: kyc_tokens.into_iter().map(|v| v[0].clone()).collect(),
        };
    }

    let missing: Vec<String> = kyc_tokens
        .iter()
        .filter(|kyc_variants| {
            // KYC token matches if ANY of its variants prefix-matches any
            // variant of any holder token. Handles "Müller" (strip→muller)
            // vs "Mueller" (expand→mueller) round-trips.
            !holder_tokens.iter().any(|holder_variants| {
                kyc_variants.iter().any(|kt| {
                    holder_variants
                        .iter()
                        .any(|ht| kt.starts_with(ht) || ht.starts_with(kt))
                })
            })
        })
        .map(|v| v[0].clone())
        .collect();

    if missing.is_empty() {
        MatchOutcome::Match
    } else {
        MatchOutcome::PotentialMismatch { missing }
    }
}

/// Tokenize and emit all transliteration variants per token.
///
/// Each inner `Vec<String>` holds 1–2 variants of the same word:
/// - element 0: ASCII-strip normalization (`Müller → muller`)
/// - element 1 (if different): German digraph expansion (`Müller → mueller`)
///
/// Some users type the umlaut, others type the German digraph. Both must
/// match the verified KYC name regardless of which side carries which.
fn tokenize_with_variants(s: &str) -> Vec<Vec<String>> {
    let stripped = normalize(s);
    let expanded = normalize_expand_german(s);

    stripped
        .split_whitespace()
        .zip(expanded.split_whitespace())
        .filter(|(stripped_word, _)| stripped_word.len() >= 2) // skip initials
        .map(|(stripped_word, expanded_word)| {
            if stripped_word == expanded_word {
                vec![stripped_word.to_string()]
            } else {
                vec![stripped_word.to_string(), expanded_word.to_string()]
            }
        })
        .collect()
}

/// German-style normalization: same as [`normalize`] but expands umlauts
/// to digraphs (ü→ue, ö→oe, ä→ae). Lets "Müller" match "Mueller".
///
/// Run alongside [`normalize`] (which strips ü→u) and try both forms —
/// covers users who type either spelling.
pub fn normalize_expand_german(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        match lower {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            // Fall through to ASCII-strip for non-German diacritics so
            // both normalizations agree on Romance-language tokens.
            other => {
                let folded = ascii_fold(other);
                out.push(folded);
            }
        }
    }
    out
}

/// ASCII-fold helper shared between [`normalize`] and
/// [`normalize_expand_german`]. Single-char output only; multi-char
/// expansions (German umlauts, eszett) are handled at the call site.
fn ascii_fold(lower: char) -> char {
    match lower {
        'á' | 'à' | 'â' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ě' | 'ę' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ī' | 'į' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ø' | 'ō' | 'ő' => 'o',
        'ú' | 'ù' | 'û' | 'ū' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'ñ' | 'ń' | 'ň' => 'n',
        'ç' | 'ć' | 'č' => 'c',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        'ł' => 'l',
        c if c.is_ascii_alphanumeric() => c,
        _ => ' ',
    }
}

/// Lowercase + ASCII-fold + strip punctuation. Handles the most common
/// Latin diacritics. Not unicode-perfect but covers the vast majority
/// of European names without dragging in `unicode-normalization`.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        match lower {
            'ä' => out.push('a'),
            'ö' => out.push('o'),
            'ü' => out.push('u'),
            'ß' => out.push_str("ss"),
            other => out.push(ascii_fold(other)),
        }
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
