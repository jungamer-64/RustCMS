//! Security validation utilities for input sanitization and validation
//!
//! Provides validation functions to prevent injection attacks and ensure
//! data integrity across all handlers and API endpoints.

use crate::middleware::security::{encode_url_component, escape_html};
use validator::ValidationError;

/// Validate and sanitize user content (posts, comments, etc.)
/// Prevents XSS while preserving safe formatting
pub fn validate_and_sanitize_content(content: &str) -> Result<String, ValidationError> {
    // Length validation
    if content.is_empty() {
        return Err(ValidationError::new("content_empty"));
    }

    if content.len() > 50000 {
        // 50KB limit
        return Err(ValidationError::new("content_too_long"));
    }

    // Basic content sanitization while preserving some formatting
    let sanitized = content
        .lines()
        .map(sanitize_safe_content)
        .collect::<Vec<_>>()
        .join("\n");

    Ok(sanitized)
}

/// Sanitize content while preserving safe markdown-like formatting
fn sanitize_safe_content(content: &str) -> String {
    // Allow basic markdown but escape dangerous HTML
    content
        .replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .replace("javascript:", "")
        .replace("data:", "")
        .replace("vbscript:", "")
        .replace("onload=", "")
        .replace("onerror=", "")
        .replace("onclick=", "")
}

/// Validate user-provided titles and names
pub fn validate_title(title: &str) -> Result<String, ValidationError> {
    let trimmed = title.trim();

    if trimmed.is_empty() {
        return Err(ValidationError::new("title_empty"));
    }

    if trimmed.len() > 200 {
        return Err(ValidationError::new("title_too_long"));
    }

    // Escape HTML but allow basic text
    Ok(escape_html(trimmed))
}

/// Validate and sanitize search queries
pub fn validate_search_query(query: &str) -> Result<String, ValidationError> {
    let trimmed = query.trim();

    if trimmed.is_empty() {
        return Err(ValidationError::new("query_empty"));
    }

    if trimmed.len() > 100 {
        return Err(ValidationError::new("query_too_long"));
    }

    // Remove dangerous characters from search queries
    let sanitized = trimmed
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?-_".contains(*c))
        .collect::<String>();

    if sanitized.is_empty() {
        return Err(ValidationError::new("query_invalid"));
    }

    Ok(sanitized)
}

/// Validate email addresses with additional security checks
pub fn validate_email_secure(email: &str) -> Result<String, ValidationError> {
    let trimmed = email.trim().to_lowercase();

    // Basic format validation
    if !trimmed.contains('@') || !trimmed.contains('.') {
        return Err(ValidationError::new("email_invalid"));
    }

    // Length validation
    if trimmed.len() > 254 {
        // RFC 5321 limit
        return Err(ValidationError::new("email_too_long"));
    }

    // Prevent malicious characters
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(ValidationError::new("email_invalid_chars"));
    }

    Ok(trimmed)
}

/// Validate usernames with security considerations
pub fn validate_username_secure(username: &str) -> Result<String, ValidationError> {
    let trimmed = username.trim();

    if trimmed.is_empty() {
        return Err(ValidationError::new("username_empty"));
    }

    if trimmed.len() < 3 {
        return Err(ValidationError::new("username_too_short"));
    }

    if trimmed.len() > 50 {
        return Err(ValidationError::new("username_too_long"));
    }

    // Only allow safe characters
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || "_-".contains(c))
    {
        return Err(ValidationError::new("username_invalid_chars"));
    }

    // Prevent reserved usernames
    let reserved = ["admin", "root", "system", "null", "undefined", "api", "www"];
    if reserved
        .iter()
        .any(|&reserved| trimmed.eq_ignore_ascii_case(reserved))
    {
        return Err(ValidationError::new("username_reserved"));
    }

    Ok(trimmed.to_string())
}

/// Validate file paths to prevent directory traversal
pub fn validate_file_path(path: &str) -> Result<String, ValidationError> {
    // Prevent directory traversal
    if path.contains("..") || path.contains("//") {
        return Err(ValidationError::new("path_traversal"));
    }

    // Only allow safe characters
    if !path
        .chars()
        .all(|c| c.is_alphanumeric() || "._-/".contains(c))
    {
        return Err(ValidationError::new("path_invalid_chars"));
    }

    Ok(encode_url_component(path))
}

/// Rate limiting: validate API request frequency
pub fn validate_request_rate(requests_per_minute: u32) -> bool {
    // Implement basic rate limiting validation
    requests_per_minute <= 100 // Max 100 requests per minute per IP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_validation() {
        // Valid content
        assert!(validate_and_sanitize_content("Hello world!").is_ok());

        // Empty content
        assert!(validate_and_sanitize_content("").is_err());

        // Content with scripts should be sanitized
        let malicious = "<script>alert('xss')</script>Hello";
        let result = validate_and_sanitize_content(malicious).unwrap();
        assert!(!result.contains("<script"));
    }

    #[test]
    fn test_title_validation() {
        // Valid title
        assert!(validate_title("My Blog Post").is_ok());

        // Empty title
        assert!(validate_title("").is_err());
        assert!(validate_title("   ").is_err());

        // XSS attempt
        let malicious = "<script>alert('xss')</script>";
        let result = validate_title(malicious).unwrap();
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_search_query_validation() {
        // Valid query
        assert!(validate_search_query("rust programming").is_ok());

        // Empty query
        assert!(validate_search_query("").is_err());

        // Query with dangerous characters
        let result = validate_search_query("test<script>alert(1)</script>").unwrap();
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_username_validation() {
        // Valid username
        assert!(validate_username_secure("user123").is_ok());

        // Invalid usernames
        assert!(validate_username_secure("").is_err());
        assert!(validate_username_secure("ab").is_err()); // too short
        assert!(validate_username_secure("admin").is_err()); // reserved
        assert!(validate_username_secure("user@domain").is_err()); // invalid chars
    }

    #[test]
    fn test_path_validation() {
        // Valid path
        assert!(validate_file_path("uploads/image.jpg").is_ok());

        // Directory traversal attempts
        assert!(validate_file_path("../etc/passwd").is_err());
        assert!(validate_file_path("uploads/../config").is_err());
    }
}
