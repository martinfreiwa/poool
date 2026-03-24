use crate::error::AppError;

/// Validates comment length as per M1 requirements (1-2000 chars)
pub fn validate_comment_length(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::BadRequest("Comment cannot be empty".to_string()));
    }
    if content.chars().count() > 2000 {
        return Err(AppError::BadRequest(
            "Comment exceeds maximum length of 2000 characters".to_string(),
        ));
    }
    Ok(())
}

/// Sanitizes HTML to prevent XSS. For M1, we use basic Ammonia cleaning.
/// In M2, this will be expanded to a full moderation pipeline.
pub fn sanitize_html_basic(html: &str) -> String {
    ammonia::clean(html)
}

/// Automod: Profanity and link blocker (M6-BE.1)
pub fn check_automod(content: &str) -> Option<String> {
    let lower = content.to_lowercase();

    // Simplistic profanity filter
    let banned_words = [
        "fuck", "shit", "bitch", "asshole", "cunt", "nigger", "faggot", "dyke", "tranny",
    ];
    for word in banned_words.iter() {
        if lower.contains(word) {
            return Some(format!("Profanity detected: {}", word));
        }
    }

    // Simplistic link filter
    let banned_links = ["pornhub.com", "xvideos.com", "onlyfans.com", "scam.com"];
    for link in banned_links.iter() {
        if lower.contains(link) {
            return Some(format!("Banned link detected: {}", link));
        }
    }

    None
}
