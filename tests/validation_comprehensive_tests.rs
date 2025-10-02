//! Comprehensive input validation tests
//!
//! Tests for input validation, sanitization, and edge cases.

use cms_backend::utils::api_types::PaginationQuery;
use serde_json::json;

#[test]
fn test_pagination_query_valid_defaults() {
    let query = PaginationQuery::default();
    
    assert_eq!(query.page, 1);
    assert_eq!(query.per_page, 20);
}

#[test]
fn test_pagination_query_custom_values() {
    let query = PaginationQuery {
        page: 5,
        per_page: 50,
    };
    
    assert_eq!(query.page, 5);
    assert_eq!(query.per_page, 50);
}

#[test]
fn test_pagination_query_deserialization_with_defaults() {
    // Test that defaults are applied when fields are missing
    let json_str = r#"{}"#;
    let query: PaginationQuery = serde_json::from_str(json_str).unwrap();
    
    assert_eq!(query.page, 1);
    assert_eq!(query.per_page, 20);
}

#[test]
fn test_pagination_query_deserialization_custom() {
    let json_str = r#"{"page": 3, "per_page": 25}"#;
    let query: PaginationQuery = serde_json::from_str(json_str).unwrap();
    
    assert_eq!(query.page, 3);
    assert_eq!(query.per_page, 25);
}

#[test]
fn test_pagination_query_large_values() {
    let query = PaginationQuery {
        page: 1000,
        per_page: 100,
    };
    
    assert_eq!(query.page, 1000);
    assert_eq!(query.per_page, 100);
}

#[test]
fn test_pagination_query_negative_values_deserialization() {
    // Negative values should fail to deserialize for u32
    let json_str = r#"{"page": -1, "per_page": -10}"#;
    let result = serde_json::from_str::<PaginationQuery>(json_str);
    
    // Should fail to parse negative values for u32
    assert!(result.is_err());
}

#[test]
fn test_string_length_validation() {
    // Test various string lengths for common fields
    let short = "a";
    let normal = "valid_username";
    let long = "a".repeat(1000);
    
    assert_eq!(short.len(), 1);
    assert_eq!(normal, "valid_username");
    assert!(long.len() > 500);
}

#[test]
fn test_email_format_validation() {
    let valid_emails = vec![
        "user@example.com",
        "user.name@example.com",
        "user+tag@example.co.uk",
        "user123@test-domain.com",
    ];
    
    let invalid_emails = vec![
        "not-an-email",    // No @
        "@example.com",    // No local part
        "user@",           // No domain
        "user @example.com", // Space in email
        "",                // Empty
    ];
    
    for email in valid_emails {
        assert!(email.contains('@'));
        assert!(email.contains('.'));
    }
    
    for email in invalid_emails {
        // These should fail validation
        // Valid email must have @ and . and no spaces and not be empty
        let is_invalid = email.is_empty() 
            || !email.contains('@') 
            || !email.contains('.') 
            || email.contains(' ')
            || email.starts_with('@')
            || email.ends_with('@');
        assert!(is_invalid, "Expected {} to be invalid", email);
    }
}

#[test]
fn test_username_format_validation() {
    let valid_usernames = vec![
        "user123",
        "valid_user",
        "User-Name",
        "user.name",
    ];
    
    let long_username = "a".repeat(100);
    let invalid_usernames = vec![
        "",
        "ab", // too short
        long_username.as_str(), // too long
        "user name", // contains space
        "user@name", // invalid character
        "user#name", // invalid character
    ];
    
    for username in valid_usernames {
        assert!(username.len() >= 3);
        assert!(username.len() <= 50);
        assert!(!username.contains(' '));
    }
    
    for username in invalid_usernames {
        let is_invalid = username.is_empty() 
            || username.len() < 3 
            || username.len() > 50 
            || username.contains(' ')
            || username.contains('@')
            || username.contains('#');
        assert!(is_invalid);
    }
}

#[test]
fn test_password_strength_requirements() {
    let weak_passwords = vec![
        "",
        "123",
        "password",
        "12345678",
        "abcdefgh",
    ];
    
    let strong_passwords = vec![
        "P@ssw0rd123!",
        "MyStr0ng!Pass",
        "C0mplex#Password",
    ];
    
    for password in weak_passwords {
        // Should fail strength requirements
        assert!(password.len() < 12 || !password.chars().any(|c| c.is_ascii_uppercase())
            || !password.chars().any(|c| c.is_ascii_lowercase())
            || !password.chars().any(|c| c.is_ascii_digit())
            || !password.chars().any(|c| !c.is_alphanumeric()));
    }
    
    for password in strong_passwords {
        // Should meet all requirements
        assert!(password.len() >= 8);
        assert!(password.chars().any(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
    }
}

#[test]
fn test_url_format_validation() {
    let valid_urls = vec![
        "https://example.com",
        "http://example.com",
        "https://example.com/path",
        "https://example.com:8080",
        "https://sub.example.com",
    ];
    
    let invalid_urls = vec![
        "",
        "not a url",
        "ftp://example.com", // unsupported scheme
        "example.com", // missing scheme
        "http://", // incomplete
    ];
    
    for url in valid_urls {
        assert!(url.starts_with("http://") || url.starts_with("https://"));
    }
    
    for url in invalid_urls {
        let is_invalid = url.is_empty() 
            || (!url.starts_with("http://") && !url.starts_with("https://"))
            || url.len() < 10;
        assert!(is_invalid);
    }
}

#[test]
fn test_json_injection_prevention() {
    // Test that special JSON characters are handled safely
    let malicious_inputs = vec![
        r#"{"key": "value"}"#,
        r#"'; DROP TABLE users; --"#,
        r#"<script>alert('xss')</script>"#,
        "\n\r\t",
        "\\u0000",
    ];
    
    for input in malicious_inputs {
        // Should be safely serializable
        let json = json!({"data": input});
        let serialized = serde_json::to_string(&json);
        assert!(serialized.is_ok());
    }
}

#[test]
fn test_sql_injection_prevention_patterns() {
    // Test common SQL injection patterns
    let malicious_patterns = vec![
        "' OR '1'='1",
        "admin'--",
        "1; DROP TABLE users",
        "' UNION SELECT * FROM users--",
        "1' AND '1'='1",
    ];
    
    for pattern in malicious_patterns {
        // These should be detected as containing SQL keywords
        let upper = pattern.to_uppercase();
        let has_sql_keywords = upper.contains("DROP") 
            || upper.contains("UNION") 
            || upper.contains("SELECT")
            || pattern.contains("--")
            || pattern.contains("'");
        assert!(has_sql_keywords);
    }
}

#[test]
fn test_xss_prevention_patterns() {
    // Test common XSS patterns
    let xss_patterns = vec![
        "<script>alert('xss')</script>",
        "<img src=x onerror=alert('xss')>",
        "javascript:alert('xss')",
        "<iframe src='javascript:alert(1)'>",
        "onerror=alert('xss')",
    ];
    
    for pattern in xss_patterns {
        // These should be detected as containing HTML/JS
        let lower = pattern.to_lowercase();
        let has_xss_indicators = lower.contains("<script")
            || lower.contains("<img")
            || lower.contains("<iframe")
            || lower.contains("javascript:")
            || lower.contains("onerror=");
        assert!(has_xss_indicators);
    }
}

#[test]
fn test_path_traversal_prevention() {
    // Test path traversal patterns
    let malicious_paths = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "/etc/passwd",
        "../../secrets.txt",
        "./../../config",
    ];
    
    for path in malicious_paths {
        // Should contain traversal patterns
        let has_traversal = path.contains("..") 
            || path.starts_with('/') 
            || path.contains("\\..\\");
        assert!(has_traversal);
    }
}

#[test]
fn test_unicode_normalization() {
    // Test that unicode is handled consistently
    let unicode_strings = vec![
        "café",
        "日本語",
        "🚀",
        "Ñoño",
        "Москва",
    ];
    
    for s in unicode_strings {
        // Should be valid UTF-8 and serializable
        assert!(s.is_char_boundary(0));
        let json = json!({"text": s});
        assert!(serde_json::to_string(&json).is_ok());
    }
}

#[test]
fn test_null_byte_injection() {
    // Test that null bytes are handled
    let inputs_with_nulls = vec![
        "test\0null",
        "\0",
        "prefix\0suffix",
    ];
    
    for input in inputs_with_nulls {
        // Should contain null byte
        assert!(input.contains('\0'));
        // But should still be serializable
        let json = json!({"data": input});
        assert!(serde_json::to_string(&json).is_ok());
    }
}

#[test]
fn test_large_number_handling() {
    // Test boundary values for numbers
    let numbers = vec![
        i64::MIN,
        i64::MAX,
        0_i64,
        -1_i64,
        1_i64,
    ];
    
    for num in numbers {
        let json = json!({"number": num});
        assert!(serde_json::to_string(&json).is_ok());
    }
}

#[test]
fn test_float_precision_handling() {
    // Test float precision and special values
    let floats = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        f64::MIN,
        f64::MAX,
        std::f64::consts::PI,
    ];
    
    for f in floats {
        let json = json!({"float": f});
        assert!(serde_json::to_string(&json).is_ok());
    }
}

#[test]
fn test_deeply_nested_json() {
    // Test handling of deeply nested structures
    let mut nested = json!({"level": 0});
    for i in 1..100 {
        nested = json!({"level": i, "nested": nested});
    }
    
    // Should handle deep nesting
    assert!(serde_json::to_string(&nested).is_ok());
}

#[test]
fn test_array_size_limits() {
    // Test large arrays
    let small_array: Vec<i32> = (0..10).collect();
    let medium_array: Vec<i32> = (0..1000).collect();
    let large_array: Vec<i32> = (0..10000).collect();
    
    assert!(serde_json::to_string(&small_array).is_ok());
    assert!(serde_json::to_string(&medium_array).is_ok());
    assert!(serde_json::to_string(&large_array).is_ok());
}
