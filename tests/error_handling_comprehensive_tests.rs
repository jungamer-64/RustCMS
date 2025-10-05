//! Comprehensive error handling tests
//!
//! Tests for all error types, error conversions, and HTTP response mapping.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use cms_backend::error::AppError;

#[test]
fn test_authentication_error_display() {
    let err = AppError::Authentication("Invalid credentials".to_string());
    assert_eq!(err.summary(), "authentication failure");
    let display = format!("{err}");
    assert!(display.contains("Invalid credentials"));
}

#[test]
fn test_authorization_error_display() {
    let err = AppError::Authorization("Insufficient permissions".to_string());
    assert_eq!(err.summary(), "authorization failure");
    let display = format!("{err}");
    assert!(display.contains("Insufficient permissions"));
}

#[test]
fn test_not_found_error_display() {
    let err = AppError::NotFound("Resource not found".to_string());
    assert_eq!(err.summary(), "resource not found");
}

#[test]
fn test_conflict_error_display() {
    let err = AppError::Conflict("Resource already exists".to_string());
    assert_eq!(err.summary(), "conflict");
}

#[test]
fn test_rate_limit_error_display() {
    let err = AppError::RateLimit("Too many requests".to_string());
    assert_eq!(err.summary(), "rate limited");
}

#[test]
fn test_bad_request_error_display() {
    let err = AppError::BadRequest("Invalid input".to_string());
    assert_eq!(err.summary(), "bad request");
}

#[test]
fn test_internal_error_display() {
    let err = AppError::Internal("Something went wrong".to_string());
    assert_eq!(err.summary(), "internal server error");
}

#[test]
fn test_not_implemented_error_display() {
    let err = AppError::NotImplemented("Feature not implemented".to_string());
    assert_eq!(err.summary(), "not implemented");
}

#[test]
fn test_parse_error_display() {
    let err = AppError::ParseError {
        message: "Invalid format".to_string(),
        context: "parsing user input".to_string(),
    };
    assert_eq!(err.summary(), "parse error");
    let display = format!("{err}");
    assert!(display.contains("Invalid format"));
    assert!(display.contains("parsing user input"));
}

#[test]
fn test_file_error_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = AppError::FileError {
        operation: "read".to_string(),
        path: "/tmp/test.txt".to_string(),
        source: io_err,
    };
    assert_eq!(err.summary(), "file operation error");
    let display = format!("{err}");
    assert!(display.contains("read"));
    assert!(display.contains("/tmp/test.txt"));
}

#[test]
fn test_network_error_display() {
    let err = AppError::NetworkError {
        endpoint: "https://api.example.com".to_string(),
        source: "Connection timeout".to_string(),
    };
    assert_eq!(err.summary(), "network error");
    let display = format!("{err}");
    assert!(display.contains("api.example.com"));
    assert!(display.contains("timeout"));
}

#[test]
fn test_authentication_error_response_status() {
    let err = AppError::Authentication("Invalid token".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_authorization_error_response_status() {
    let err = AppError::Authorization("Access denied".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_not_found_error_response_status() {
    let err = AppError::NotFound("User not found".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_conflict_error_response_status() {
    let err = AppError::Conflict("Username already exists".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_rate_limit_error_response_status() {
    let err = AppError::RateLimit("Rate limit exceeded".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn test_bad_request_error_response_status() {
    let err = AppError::BadRequest("Invalid parameters".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_internal_error_response_status() {
    let err = AppError::Internal("Internal server error".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_not_implemented_error_response_status() {
    let err = AppError::NotImplemented("Feature not available".to_string());
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[test]
fn test_parse_error_response_status() {
    let err = AppError::ParseError {
        message: "Invalid JSON".to_string(),
        context: "request body".to_string(),
    };
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_file_error_response_status() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let err = AppError::FileError {
        operation: "write".to_string(),
        path: "/etc/protected".to_string(),
        source: io_err,
    };
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_network_error_response_status() {
    let err = AppError::NetworkError {
        endpoint: "https://external-api.com".to_string(),
        source: "502 Bad Gateway".to_string(),
    };
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[test]
fn test_io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
    let app_err: AppError = io_err.into();
    assert_eq!(app_err.summary(), "io error");
}

#[test]
fn test_serde_error_conversion() {
    let json_str = "{invalid json}";
    let serde_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
    let app_err: AppError = serde_err.into();
    assert_eq!(app_err.summary(), "serde error");
}

#[test]
fn test_config_error_conversion() {
    use config::ConfigError;
    let config_err = ConfigError::NotFound("test".to_string());
    let app_err: AppError = config_err.into();
    assert_eq!(app_err.summary(), "configuration error");
}

#[cfg(feature = "database")]
#[test]
fn test_database_not_found_conversion() {
    use diesel::result::Error as DieselError;
    let db_err = DieselError::NotFound;
    let app_err: AppError = db_err.into();
    // NotFound should be mapped to AppError::NotFound
    assert_eq!(app_err.summary(), "resource not found");
}

#[cfg(feature = "database")]
#[test]
fn test_database_other_error_conversion() {
    use diesel::result::Error as DieselError;
    let db_err = DieselError::RollbackTransaction;
    let app_err: AppError = db_err.into();
    assert_eq!(app_err.summary(), "database error");
}

#[test]
fn test_error_source_chain() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "root cause");
    let app_err = AppError::FileError {
        operation: "read".to_string(),
        path: "/test".to_string(),
        source: io_err,
    };

    // Check that we can access the source
    assert!(std::error::Error::source(&app_err).is_some());
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AppError>();
}

#[test]
fn test_multiple_error_types_display() {
    let errors = vec![
        AppError::Authentication("auth error".to_string()),
        AppError::Authorization("authz error".to_string()),
        AppError::NotFound("not found".to_string()),
        AppError::Conflict("conflict".to_string()),
        AppError::RateLimit("rate limit".to_string()),
    ];

    for err in errors {
        let display = format!("{err}");
        assert!(!display.is_empty());
        assert!(!err.summary().is_empty());
    }
}
