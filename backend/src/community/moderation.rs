use ammonia::Builder;
use regex::Regex;

/// Result of a moderation check
pub struct ModerationResult {
    /// True if the content should be flagged for manual review
    pub is_flagged: bool,
    /// Reason for flagging, if any
    pub flag_reason: Option<String>,
    /// True if the content contains investment discussion and needs a disclaimer
    pub needs_disclaimer: bool,
    /// The sanitized HTML content
    pub sanitized_content: String,
}

/// Validates and sanitizes user-generated content for community posts and comments
pub fn moderate_content(content: &str, is_high_level_user: bool) -> ModerationResult {
    let lower_content = content.to_lowercase();
    let mut is_flagged = false;
    let mut flag_reason = None;
    
    // 1. Check for spam/scam keywords (Pump & Dump, forbidden promises)
    let forbidden = [
        "guaranteed returns", "risk-free", "risk free", "100% safe", "guaranteed profit", "guaranteed 28% returns",
    ];
    for keyword in &forbidden {
        if lower_content.contains(keyword) {
            is_flagged = true;
            flag_reason = Some(format!("Contains forbidden phrase: {}", keyword));
            break;
        }
    }
    
    // 2. URL Filter / New-User Sandbox (under Level 2 cannot post URLs)
    if !is_flagged && !is_high_level_user {
        if let Ok(re) = Regex::new(r"(https?://\S+)") {
            if re.is_match(content) {
                is_flagged = true;
                flag_reason = Some("New user posted a URL".to_string());
            }
        }
    }
    
    // 3. Check if we need to append an investment disclaimer
    let mut needs_disclaimer = false;
    let investment_keywords = ["invest", "return ", "yield", "profit", "dividend", "roi", "price target", "buy now", "sell now"];
    for keyword in &investment_keywords {
        if lower_content.contains(keyword) {
            needs_disclaimer = true;
            break;
        }
    }
    
    // 4. Sanitize HTML
    let sanitized_content = Builder::default()
        .clean(content)
        .to_string();

    ModerationResult {
        is_flagged,
        flag_reason,
        needs_disclaimer,
        sanitized_content,
    }
}
