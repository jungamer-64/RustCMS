//! Application-wide error type and mapping to HTTP responses.
//!
//! Keeps the API error shape consistent across handlers. This file intentionally
//! avoids exporting internal error details to clients while preserving them in
//! logs for operators.

use crate::telemetry::TelemetryError;
use crate::utils::api_types::{ApiResponse, ValidationError};
use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use std::fmt;
use tracing::{debug, error};
use validator::ValidationErrors;

/// The application's unified error type with enhanced context information.
#[derive(Debug)]
pub enum AppError {
    #[cfg(feature = "database")]
    Database(diesel::result::Error),
    #[cfg(feature = "cache")]
    Redis(redis::RedisError),
    Validation(ValidationErrors),
    Authentication(String),
    Authorization(String),
    NotFound(String),
    Conflict(String),
    RateLimit(String),
    Internal(String),
    NotImplemented(String),
    BadRequest(String),
    #[cfg(feature = "auth")]
    Argon2(argon2::Error),
    Biscuit(String),
    Search(String),
    Media(String),
    Config(String),
    ConfigLoad(config::ConfigError),
    ConfigValueMissing(String),
    ConfigValidationError(String),
    Telemetry(TelemetryError),
    IO(std::io::Error),
    Serde(serde_json::Error),
    #[cfg(feature = "search")]
    Tantivy(tantivy::TantivyError),
    /// Generic parsing error with context
    ParseError {
        message: String,
        context: String,
    },
    /// File operation error with path context
    FileError {
        operation: String,
        path: String,
        source: std::io::Error,
    },
    /// Network error with endpoint context
    NetworkError {
        endpoint: String,
        source: String,
    },
}

//--- Trait Implementations ---//

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error_message())
    }
}

impl AppError {
    /// Get the formatted error message for display.
    fn error_message(&self) -> String {
        match self {
            #[cfg(feature = "database")]
            Self::Database(err) => format!("Database error: {err}"),
            #[cfg(feature = "cache")]
            Self::Redis(err) => format!("Cache error: {err}"),
            Self::Validation(err) => format!("Validation error: {err}"),
            Self::Authentication(msg) => format!("Authentication error: {msg}"),
            Self::Authorization(msg) => format!("Authorization error: {msg}"),
            Self::NotFound(msg) => format!("Not found: {msg}"),
            Self::Conflict(msg) => format!("Conflict: {msg}"),
            Self::RateLimit(msg) => format!("Rate limit exceeded: {msg}"),
            Self::Internal(msg) => format!("Internal error: {msg}"),
            Self::NotImplemented(msg) => format!("Not implemented: {msg}"),
            Self::BadRequest(msg) => format!("Bad request: {msg}"),
            #[cfg(feature = "auth")]
            Self::Argon2(err) => format!("Password hashing error: {err}"),
            Self::Biscuit(msg) => format!("Biscuit auth error: {msg}"),
            Self::Search(msg) => format!("Search error: {msg}"),
            Self::Media(msg) => format!("Media error: {msg}"),
            Self::Config(msg) => format!("Configuration error: {msg}"),
            Self::ConfigLoad(err) => format!("Configuration loading error: {err}"),
            Self::ConfigValueMissing(key) => format!("Configuration value missing for key: {key}"),
            Self::ConfigValidationError(msg) => format!("Configuration validation error: {msg}"),
            Self::Telemetry(err) => format!("Telemetry error: {err}"),
            Self::IO(err) => format!("IO error: {err}"),
            Self::Serde(err) => format!("Serialization error: {err}"),
            #[cfg(feature = "search")]
            Self::Tantivy(err) => format!("Tantivy search error: {err}"),
            Self::ParseError { message, context } => {
                format!("Parse error: {message} (context: {context})")
            }
            Self::FileError {
                operation,
                path,
                source,
            } => {
                format!("File error during {operation} on {path}: {source}")
            }
            Self::NetworkError { endpoint, source } => {
                format!("Network error at {endpoint}: {source}")
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "database")]
            Self::Database(err) => Some(err),
            #[cfg(feature = "cache")]
            Self::Redis(err) => Some(err),
            Self::ConfigLoad(err) => Some(err),
            Self::IO(err) => Some(err),
            Self::Serde(err) => Some(err),
            Self::Telemetry(err) => Some(err),
            #[cfg(feature = "search")]
            Self::Tantivy(err) => Some(err),
            Self::FileError { source, .. } => Some(source),
            // Errors wrapping only a String or simple type don't have a source
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let summary = self.summary();
        error!(summary = %summary, "Converting error into HTTP response");
        debug!(error.details = ?self, "Full error details");

        // Delegate classification and validation extraction to helpers to keep
        // this function small and easier for clippy to analyze.
        let (status, message, validation_details) = self.classify_and_validation();

        let body = match validation_details {
            Some(details) => Json(ApiResponse::error_with_validation(message, details)),
            None => Json(ApiResponse::error(message)),
        };

        (status, body).into_response()
    }
}

impl AppError {
    /// Helper to classify an error into (status, message, optional validation details).
    ///
    /// Delegates to specialized helpers to keep this function manageable.
    fn classify_and_validation(&self) -> (StatusCode, String, Option<Vec<ValidationError>>) {
        // Handle validation errors specially as they have details
        if let Self::Validation(ve) = self {
            return Self::handle_validation_error(ve);
        }

        // Classify other errors by category
        if let Some(result) = Self::classify_internal_errors(self) {
            return result;
        }

        Self::classify_client_errors(self)
    }

    /// Handle validation errors with detailed field information
    fn handle_validation_error(
        ve: &ValidationErrors,
    ) -> (StatusCode, String, Option<Vec<ValidationError>>) {
        let details: Vec<ValidationError> = ve
            .field_errors()
            .into_iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| ValidationError {
                    field: field.to_string(),
                    message: e
                        .message
                        .as_ref()
                        .map_or_else(|| "Invalid value".to_string(), ToString::to_string),
                })
            })
            .collect();
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid input".to_string(),
            Some(details),
        )
    }

    /// Classify internal server errors (5xx status codes)
    fn classify_internal_errors(
        error: &Self,
    ) -> Option<(StatusCode, String, Option<Vec<ValidationError>>)> {
        let result = match error {
            #[cfg(feature = "database")]
            Self::Database(_) => (
                "A database error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            #[cfg(feature = "cache")]
            Self::Redis(_) => ("A cache error occurred", StatusCode::INTERNAL_SERVER_ERROR),
            Self::Internal(_) => (
                "An internal server error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            #[cfg(feature = "auth")]
            Self::Argon2(_) => (
                "A password hashing error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Self::Search(s) => return Some((StatusCode::INTERNAL_SERVER_ERROR, s.clone(), None)),
            Self::Config(_)
            | Self::ConfigLoad(_)
            | Self::ConfigValueMissing(_)
            | Self::ConfigValidationError(_) => (
                "A server configuration error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Self::Telemetry(_) => (
                "A telemetry subsystem error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Self::IO(_) => ("An I/O error occurred", StatusCode::INTERNAL_SERVER_ERROR),
            #[cfg(feature = "search")]
            Self::Tantivy(_) => (
                "A search service error occurred",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            Self::FileError {
                operation, path, ..
            } => {
                return Some((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("File operation failed: {operation} on {path}"),
                    None,
                ));
            }
            _ => return None,
        };

        Some((result.1, result.0.to_string(), None))
    }

    /// Classify client errors (4xx status codes)
    fn classify_client_errors(error: &Self) -> (StatusCode, String, Option<Vec<ValidationError>>) {
        match error {
            Self::Authentication(s) | Self::Biscuit(s) => {
                (StatusCode::UNAUTHORIZED, s.clone(), None)
            }
            Self::Authorization(s) => (StatusCode::FORBIDDEN, s.clone(), None),
            Self::NotFound(s) => (StatusCode::NOT_FOUND, s.clone(), None),
            Self::Conflict(s) => (StatusCode::CONFLICT, s.clone(), None),
            Self::RateLimit(s) => (StatusCode::TOO_MANY_REQUESTS, s.clone(), None),
            Self::BadRequest(s) | Self::Media(s) => (StatusCode::BAD_REQUEST, s.clone(), None),
            Self::Serde(_) => (
                StatusCode::BAD_REQUEST,
                "Failed to process request body".to_string(),
                None,
            ),
            Self::ParseError { message, .. } => (StatusCode::BAD_REQUEST, message.clone(), None),
            Self::NotImplemented(s) => (StatusCode::NOT_IMPLEMENTED, s.clone(), None),
            Self::NetworkError { endpoint, .. } => (
                StatusCode::BAD_GATEWAY,
                format!("Network error communicating with: {endpoint}"),
                None,
            ),
            // Fallback for any unhandled cases
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred".to_string(),
                None,
            ),
        }
    }
}

impl From<TelemetryError> for AppError {
    fn from(err: TelemetryError) -> Self {
        Self::Telemetry(err)
    }
}

//--- Conversions from external error types ---//

#[cfg(feature = "database")]
impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => Self::NotFound("Resource not found".into()),
            other => Self::Database(other),
        }
    }
}

#[cfg(feature = "cache")]
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        Self::Redis(err)
    }
}

#[cfg(feature = "auth")]
impl From<argon2::Error> for AppError {
    fn from(err: argon2::Error) -> Self {
        Self::Argon2(err)
    }
}

impl From<ValidationErrors> for AppError {
    fn from(err: ValidationErrors) -> Self {
        Self::Validation(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        Self::ConfigLoad(err)
    }
}

#[cfg(feature = "search")]
impl From<tantivy::TantivyError> for AppError {
    fn from(err: tantivy::TantivyError) -> Self {
        Self::Tantivy(err)
    }
}

#[cfg(feature = "restructure_domain")]
impl From<crate::application::ports::repositories::RepositoryError> for AppError {
    fn from(err: crate::application::ports::repositories::RepositoryError) -> Self {
        use crate::application::ports::repositories::RepositoryError as RE;
        match err {
            RE::NotFound(msg) => Self::NotFound(msg),
            RE::Duplicate(msg) => Self::Conflict(msg),
            RE::ValidationError(msg) => Self::BadRequest(msg),
            RE::ConversionError(msg) => Self::BadRequest(format!("Conversion error: {}", msg)),
            RE::ConnectionError(msg) => Self::Internal(format!("Database connection error: {}", msg)),
            RE::DatabaseError(msg) | RE::Unknown(msg) => Self::Internal(msg),
        }
    }
}

//--- AppError inherent methods ---//

impl AppError {
    /// Short, non-sensitive summary used in logs.
    #[must_use]
    pub const fn summary(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation error",
            Self::Authentication(_) => "authentication failure",
            Self::Authorization(_) => "authorization failure",
            Self::NotFound(_) => "resource not found",
            Self::Conflict(_) => "conflict",
            Self::RateLimit(_) => "rate limited",
            Self::BadRequest(_) => "bad request",
            Self::Internal(_) => "internal server error",
            Self::IO(_) => "io error",
            Self::Serde(_) => "serde error",
            #[cfg(feature = "database")]
            Self::Database(_) => "database error",
            #[cfg(feature = "cache")]
            Self::Redis(_) => "cache error",
            #[cfg(feature = "auth")]
            Self::Argon2(_) => "password hashing error",
            Self::Biscuit(_) => "biscuit auth error",
            Self::Search(_) => "search error",
            Self::Media(_) => "media error",
            Self::Config(_)
            | Self::ConfigLoad(_)
            | Self::ConfigValueMissing(_)
            | Self::ConfigValidationError(_) => "configuration error",
            Self::Telemetry(_) => "telemetry error",
            Self::NotImplemented(_) => "not implemented",
            #[cfg(feature = "search")]
            Self::Tantivy(_) => "search backend error",
            Self::ParseError { .. } => "parse error",
            Self::FileError { .. } => "file operation error",
            Self::NetworkError { .. } => "network error",
        }
    }
}

/// A module-local `Result` alias.
pub type Result<T> = std::result::Result<T, AppError>;
