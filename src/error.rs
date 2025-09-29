//! Application-wide error type and mapping to HTTP responses.
//!
//! Keeps the API error shape consistent across handlers. This file intentionally
//! avoids exporting internal error details to clients while preserving them in
//! logs for operators.

use crate::utils::api_types::{ApiResponse, ValidationError};
use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use std::fmt;
use tracing::{debug, error};
use validator::ValidationErrors;

/// The application's unified error type.
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
    IO(std::io::Error),
    Serde(serde_json::Error),
    #[cfg(feature = "search")]
    Tantivy(tantivy::TantivyError),
}

//--- Trait Implementations ---//

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "database")]
            Self::Database(err) => write!(f, "Database error: {err}"),
            #[cfg(feature = "cache")]
            Self::Redis(err) => write!(f, "Cache error: {err}"),
            Self::Validation(err) => write!(f, "Validation error: {err}"),
            Self::Authentication(msg) => write!(f, "Authentication error: {msg}"),
            Self::Authorization(msg) => write!(f, "Authorization error: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::Conflict(msg) => write!(f, "Conflict: {msg}"),
            Self::RateLimit(msg) => write!(f, "Rate limit exceeded: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
            Self::NotImplemented(msg) => write!(f, "Not implemented: {msg}"),
            Self::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            #[cfg(feature = "auth")]
            Self::Argon2(err) => write!(f, "Password hashing error: {err}"),
            Self::Biscuit(msg) => write!(f, "Biscuit auth error: {msg}"),
            Self::Search(msg) => write!(f, "Search error: {msg}"),
            Self::Media(msg) => write!(f, "Media error: {msg}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
            Self::ConfigLoad(err) => write!(f, "Configuration loading error: {err}"),
            Self::ConfigValueMissing(key) => {
                write!(f, "Configuration value missing for key: {key}")
            }
            Self::ConfigValidationError(msg) => write!(f, "Configuration validation error: {msg}"),
            Self::IO(err) => write!(f, "IO error: {err}"),
            Self::Serde(err) => write!(f, "Serialization error: {err}"),
            #[cfg(feature = "search")]
            Self::Tantivy(err) => write!(f, "Tantivy search error: {err}"),
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
            #[cfg(feature = "search")]
            Self::Tantivy(err) => Some(err),
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
    /// Keeping the heavy match logic separate reduces the cognitive complexity of
    /// `into_response` and makes the mapping easier to unit test if desired.
    fn classify_and_validation(&self) -> (StatusCode, String, Option<Vec<ValidationError>>) {
        match self {
            #[cfg(feature = "database")]
            Self::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error occurred".to_string(),
                None,
            ),
            #[cfg(feature = "cache")]
            Self::Redis(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A cache error occurred".to_string(),
                None,
            ),
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occurred".to_string(),
                None,
            ),
            #[cfg(feature = "auth")]
            Self::Argon2(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A password hashing error occurred".to_string(),
                None,
            ),
            Self::Search(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                s.as_str().to_string(),
                None,
            ),
            Self::Config(_)
            | Self::ConfigLoad(_)
            | Self::ConfigValueMissing(_)
            | Self::ConfigValidationError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A server configuration error occurred".to_string(),
                None,
            ),
            Self::IO(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An I/O error occurred".to_string(),
                None,
            ),
            #[cfg(feature = "search")]
            Self::Tantivy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A search service error occurred".to_string(),
                None,
            ),

            Self::Validation(ve) => {
                let details: Vec<ValidationError> = ve
                    .field_errors()
                    .into_iter()
                    .flat_map(|(field, errors)| {
                        errors.iter().map(move |e| ValidationError {
                            field: field.to_string(),
                            message: e
                                .message
                                .as_ref()
                                .map_or_else(|| "Invalid value".to_string(), |s| s.to_string()),
                        })
                    })
                    .collect();
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Invalid input".to_string(),
                    Some(details),
                )
            }

            Self::Authentication(s) => (StatusCode::UNAUTHORIZED, s.as_str().to_string(), None),
            Self::Authorization(s) => (StatusCode::FORBIDDEN, s.as_str().to_string(), None),
            Self::NotFound(s) => (StatusCode::NOT_FOUND, s.as_str().to_string(), None),
            Self::Conflict(s) => (StatusCode::CONFLICT, s.as_str().to_string(), None),
            Self::RateLimit(s) => (StatusCode::TOO_MANY_REQUESTS, s.as_str().to_string(), None),
            Self::BadRequest(s) => (StatusCode::BAD_REQUEST, s.as_str().to_string(), None),
            Self::Biscuit(s) => (StatusCode::UNAUTHORIZED, s.as_str().to_string(), None),
            Self::Media(s) => (StatusCode::BAD_REQUEST, s.as_str().to_string(), None),
            Self::Serde(_) => (
                StatusCode::BAD_REQUEST,
                "Failed to process request body".to_string(),
                None,
            ),
            Self::NotImplemented(s) => (StatusCode::NOT_IMPLEMENTED, s.as_str().to_string(), None),
        }
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
            Self::NotImplemented(_) => "not implemented",
            #[cfg(feature = "search")]
            Self::Tantivy(_) => "search backend error",
        }
    }
}

/// A module-local `Result` alias.
pub type Result<T> = std::result::Result<T, AppError>;
