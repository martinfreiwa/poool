//! HTML sanitization utilities for preventing stored XSS attacks.
//!
//! All user-supplied text fields that will be rendered in HTML templates
//! must be sanitized before storage. This module provides a `sanitize_html()`
//! function that strips dangerous HTML tags and attributes while preserving
//! safe content like plain text and basic formatting.

/// Strip HTML tags from a string, returning only the text content.
/// This is the most conservative approach — no HTML is allowed.
///
/// # Examples
/// ```
/// assert_eq!(strip_tags("<script>alert('xss')</script>Hello"), "alert('xss')Hello");
/// assert_eq!(strip_tags("Normal text"), "Normal text");
/// assert_eq!(strip_tags("<b>Bold</b> and <i>italic</i>"), "Bold and italic");
/// ```
pub fn strip_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;

    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result
}

/// Sanitize a user-supplied text field for safe storage.
///
/// - Strips all HTML tags
/// - Trims leading/trailing whitespace  
/// - Collapses multiple consecutive whitespace into single spaces
/// - Removes null bytes
pub fn sanitize_text(input: &str) -> String {
    let stripped = strip_tags(input);
    stripped
        .replace('\0', "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Sanitize a multi-line text field (descriptions, notes).
///
/// Like `sanitize_text` but preserves intentional newlines.
/// - Strips all HTML tags
/// - Removes null bytes
/// - Trims each line
/// - Removes excessive blank lines (max 2 consecutive)
pub fn sanitize_multiline(input: &str) -> String {
    let stripped = strip_tags(input);
    let cleaned = stripped.replace('\0', "");
    
    let mut result = String::with_capacity(cleaned.len());
    let mut consecutive_blanks = 0;

    for line in cleaned.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            consecutive_blanks += 1;
            if consecutive_blanks <= 2 {
                result.push('\n');
            }
        } else {
            consecutive_blanks = 0;
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(trimmed);
        }
    }

    result.trim().to_string()
}

/// Sanitize a rich-text (HTML) field using the `ammonia` library.
///
/// This is used for Quill editor content (support replies, admin notes).
/// - Allows: b, i, u, s, p, br, blockquote, code, ol, ul, li, div, h1, h2, h3, span
/// - Attributes: style (limited), href, target
/// - Strips all scripts, event handlers, and dangerous attributes.
pub fn sanitize_html(input: &str) -> String {
    let mut cleaner = ammonia::Builder::new();
    
    // Configure safe tags
    cleaner.tags(maplit::hashset![
        "b", "i", "u", "s", "p", "br", "blockquote", "code", "pre",
        "ol", "ul", "li", "div", "h1", "h2", "h3", "span", "a", "strong", "em"
    ]);

    // Allowed attributes - only on specific tags
    cleaner.tag_attributes(maplit::hashmap![
        "a" => maplit::hashset!["href", "target", "rel"],
        "span" => maplit::hashset!["style"],
        "div" => maplit::hashset!["style"]
    ]);

    // Add safe rel to links
    cleaner.link_rel(Some("noopener noreferrer"));

    cleaner.clean(input).to_string()
}

/// Sanitize a URL field — only allow http/https/mailto schemes.
/// Returns None if the URL uses a dangerous scheme (javascript:, data:, etc.)
pub fn sanitize_url(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();
    
    // Allow http, https, mailto, and protocol-relative URLs
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("//")
    {
        Some(trimmed.to_string())
    } else if !lower.contains("://") && !lower.starts_with("javascript:") && !lower.starts_with("data:") {
        // Relative URL or bare domain — allow
        Some(trimmed.to_string())
    } else {
        tracing::warn!("Rejected unsafe URL scheme: {}", &trimmed[..trimmed.len().min(100)]);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tags_basic() {
        assert_eq!(strip_tags("Hello World"), "Hello World");
        assert_eq!(strip_tags("<b>Bold</b>"), "Bold");
        assert_eq!(strip_tags("<script>alert('xss')</script>"), "alert('xss')");
    }

    #[test]
    fn test_strip_tags_nested() {
        assert_eq!(strip_tags("<div><p>Hello</p></div>"), "Hello");
        assert_eq!(strip_tags("A <b>B <i>C</i> D</b> E"), "A B C D E");
    }

    #[test]
    fn test_strip_tags_attributes() {
        assert_eq!(
            strip_tags(r#"<img src="x" onerror="alert(1)">"#),
            ""
        );
        assert_eq!(
            strip_tags(r#"<a href="javascript:alert(1)">Click</a>"#),
            "Click"
        );
    }

    #[test]
    fn test_sanitize_text() {
        assert_eq!(sanitize_text("  Hello   World  "), "Hello World");
        assert_eq!(sanitize_text("<script>xss</script>Clean"), "xssClean");
        assert_eq!(sanitize_text("No\0null\0bytes"), "Nonullbytes");
    }

    #[test]
    fn test_sanitize_multiline() {
        assert_eq!(
            sanitize_multiline("Line 1\nLine 2\n\n\n\n\nLine 3"),
            "Line 1\nLine 2\n\nLine 3"
        );
        assert_eq!(
            sanitize_multiline("<p>Para 1</p>\n<p>Para 2</p>"),
            "Para 1\nPara 2"
        );
    }

    #[test]
    fn test_sanitize_url_safe() {
        assert_eq!(sanitize_url("https://example.com"), Some("https://example.com".into()));
        assert_eq!(sanitize_url("http://example.com"), Some("http://example.com".into()));
        assert_eq!(sanitize_url("mailto:user@example.com"), Some("mailto:user@example.com".into()));
        assert_eq!(sanitize_url("/relative/path"), Some("/relative/path".into()));
    }

    #[test]
    fn test_sanitize_url_dangerous() {
        assert_eq!(sanitize_url("javascript:alert(1)"), None);
        assert_eq!(sanitize_url("data:text/html,<script>alert(1)</script>"), None);
        assert_eq!(sanitize_url("  "), None);
    }
}
