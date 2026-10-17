//! Input validation rules for links, users, and passwords.

/// Validate that a link target is a well-formed `http://` or `https://` URL.
///
/// # Errors
/// Returns a human-readable validation error if the target is empty, uses an disallowed scheme
/// (such as `javascript:`), or contains whitespace.
pub fn validate_target_url(target: &str) -> Result<(), String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err("Target URL cannot be empty".to_string());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err("Target URL must not contain whitespace".to_string());
    }
    let rest = if let Some(r) = trimmed.strip_prefix("https://") {
        r
    } else if let Some(r) = trimmed.strip_prefix("http://") {
        r
    } else {
        return Err("Target URL must start with http:// or https://".to_string());
    };
    if rest.is_empty() || rest.starts_with('/') {
        return Err("Target URL must include a valid host".to_string());
    }
    Ok(())
}

/// Validate a short link code.
///
/// # Errors
/// Returns an error if the code is `"app"` (which conflicts with the `/app/` admin UI mount),
/// contains slashes or disallowed characters, or exceeds 64 characters.
pub fn validate_link_code(code: &str, allow_empty: bool) -> Result<(), String> {
    if code.is_empty() {
        return if allow_empty {
            Ok(())
        } else {
            Err("Link code cannot be empty".to_string())
        };
    }
    if code.len() > 64 {
        return Err("Link code cannot exceed 64 characters".to_string());
    }
    if code.eq_ignore_ascii_case("app") {
        return Err("The code 'app' is reserved for the administration interface".to_string());
    }
    if !code
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(
            "Link code may only contain ASCII letters, digits, '-', '_', or '.'".to_string(),
        );
    }
    Ok(())
}

/// Validate a user password.
///
/// # Errors
/// Returns an error if the password is shorter than 4 characters or longer than 256 characters.
pub fn validate_password(password: &str) -> Result<(), String> {
    let char_count = password.chars().count();
    if char_count < 4 {
        return Err("Password must be at least 4 characters long".to_string());
    }
    if char_count > 256 {
        return Err("Password cannot exceed 256 characters".to_string());
    }
    Ok(())
}

/// Validate a username.
///
/// # Errors
/// Returns an error if the username is empty or exceeds 64 characters.
pub fn validate_username(username: &str) -> Result<(), String> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if trimmed.len() > 64 {
        return Err("Username cannot exceed 64 characters".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_target_url() {
        assert!(validate_target_url("https://example.com/path?x=1").is_ok());
        assert!(validate_target_url("http://localhost:8080/test").is_ok());
        assert!(validate_target_url("").is_err());
        assert!(validate_target_url("javascript:alert(1)").is_err());
        assert!(validate_target_url("https://").is_err());
        assert!(validate_target_url("https:///path").is_err());
        assert!(validate_target_url("https://example.com/with space").is_err());
    }

    #[test]
    fn test_validate_link_code() {
        assert!(validate_link_code("my-link_1.0", false).is_ok());
        assert!(validate_link_code("", true).is_ok());
        assert!(validate_link_code("", false).is_err());
        assert!(validate_link_code("app", false).is_err());
        assert!(validate_link_code("APP", false).is_err());
        assert!(validate_link_code("bad/code", false).is_err());
        assert!(validate_link_code("bad code", false).is_err());
        assert!(validate_link_code(&"a".repeat(65), false).is_err());
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("demo").is_ok());
        assert!(validate_password(&"a".repeat(64)).is_ok());
        assert!(validate_password("abc").is_err());
        assert!(validate_password(&"a".repeat(257)).is_err());
    }
}
