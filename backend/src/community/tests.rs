use super::validation;

#[test]
fn test_comment_validation_length() {
    assert!(validation::validate_comment_length("short comment").is_ok());
    
    let ok_long = "a".repeat(100);
    assert!(validation::validate_comment_length(&ok_long).is_ok());
    
    let too_long = "a".repeat(4000);
    assert!(validation::validate_comment_length(&too_long).is_err(), "Should return error for >3000 chars");
    
    let too_short = "  "; // Trims to empty
    assert!(validation::validate_comment_length(too_short).is_err(), "Should catch effectively empty comments");
}

#[test]
fn test_ammonia_sanitization() {
    let dirty = "<script>alert('xss')</script><b>Hello</b>";
    let clean = validation::sanitize_html_basic(dirty);
    assert_eq!(clean, "<b>Hello</b>");
    
    let dirty2 = "<a href='javascript:alert(1)'>Click</a>";
    let clean2 = validation::sanitize_html_basic(dirty2);
    assert_eq!(clean2, "<a rel=\"noopener noreferrer\">Click</a>");
}
