/// Input validation utilities.
///
/// Centralized validation prevents scattered, inconsistent checks across routes.
use crate::error::AppError;

/// Validates an email address format.
pub fn validate_email(email: &str) -> Result<(), AppError> {
    let email = email.trim();

    if email.is_empty() {
        return Err(AppError::BadRequest("Email is required.".to_string()));
    }

    if email.len() > 255 {
        return Err(AppError::BadRequest(
            "Email address is too long.".to_string(),
        ));
    }

    // Basic email format check: must contain exactly one @ with content on both sides
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(AppError::BadRequest(
            "Please enter a valid email address.".to_string(),
        ));
    }

    // Domain must contain at least one dot
    if !parts[1].contains('.') {
        return Err(AppError::BadRequest(
            "Please enter a valid email address.".to_string(),
        ));
    }

    Ok(())
}

/// Validates password strength.
/// Requirements: at least 8 characters, at least one uppercase, one lowercase, one digit.
pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters long.".to_string(),
        ));
    }

    if password.len() > 128 {
        return Err(AppError::BadRequest(
            "Password is too long (max 128 characters).".to_string(),
        ));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if !has_uppercase || !has_lowercase || !has_digit {
        return Err(AppError::BadRequest(
            "Password must contain at least one uppercase letter, one lowercase letter, and one number.".to_string(),
        ));
    }

    Ok(())
}
