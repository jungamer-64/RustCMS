use axum::{
    response::{IntoResponse, Response},
    Json, http::StatusCode,
};
use serde_json::json;
use std::fmt;
use validator::ValidationErrors;

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
    BadRequest(String),
    #[cfg(feature = "auth")]
    Argon2(argon2::Error),
    #[cfg(feature = "auth")]
    Jwt(jsonwebtoken::errors::Error),
    Biscuit(String),
    Search(String),
    Media(String),
    Config(String),
    IO(std::io::Error),
    Serde(serde_json::Error),
    #[cfg(feature = "search")]
    Tantivy(tantivy::TantivyError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "database")]
            AppError::Database(err) => write!(f, "Database error: {}", err),
            #[cfg(feature = "cache")]
            AppError::Redis(err) => write!(f, "Cache error: {}", err),
            AppError::Validation(err) => write!(f, "Validation error: {}", err),
            AppError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            AppError::Authorization(msg) => write!(f, "Authorization error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::RateLimit(msg) => write!(f, "Rate limit exceeded: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            #[cfg(feature = "auth")]
            AppError::Argon2(err) => write!(f, "Argon2 error: {}", err),
            #[cfg(feature = "auth")]
            AppError::Jwt(err) => write!(f, "JWT error: {}", err),
            AppError::Biscuit(msg) => write!(f, "Biscuit auth error: {}", msg),
            AppError::Search(msg) => write!(f, "Search error: {}", msg),
            AppError::Media(msg) => write!(f, "Media error: {}", msg),
            AppError::Config(msg) => write!(f, "Configuration error: {}", msg),
            AppError::IO(err) => write!(f, "IO error: {}", err),
            AppError::Serde(err) => write!(f, "Serialization error: {}", err),
            #[cfg(feature = "search")]
            AppError::Tantivy(err) => write!(f, "Search error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(feature = "database")]
impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => AppError::NotFound("Resource not found".to_string()),
            _ => AppError::Database(err),
        }
    }
}

#[cfg(feature = "cache")]
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Redis(err)
    }
}

#[cfg(feature = "auth")]
impl From<argon2::Error> for AppError {
    fn from(err: argon2::Error) -> Self {
        AppError::Argon2(err)
    }
}

#[cfg(feature = "auth")]
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Jwt(err)
    }
}

impl From<ValidationErrors> for AppError {
    fn from(err: ValidationErrors) -> Self {
        AppError::Validation(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IO(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serde(err)
    }
}

#[cfg(feature = "search")]
impl From<tantivy::TantivyError> for AppError {
    fn from(err: tantivy::TantivyError) -> Self {
        AppError::Tantivy(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            #[cfg(feature = "database")]
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error occurred",
            ),
            #[cfg(feature = "cache")]
            AppError::Redis(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cache error occurred",
            ),
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, "Invalid input data"),
            AppError::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg.as_str()),
            AppError::Authorization(msg) => (StatusCode::FORBIDDEN, msg.as_str()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.as_str()),
            AppError::RateLimit(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.as_str()),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.as_str(),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            #[cfg(feature = "auth")]
            AppError::Argon2(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password hashing error",
            ),
            #[cfg(feature = "auth")]
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid authentication token",
            ),
            AppError::Biscuit(msg) => (StatusCode::UNAUTHORIZED, msg.as_str()),
            AppError::Search(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.as_str(),
            ),
            AppError::Media(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            AppError::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.as_str(),
            ),
            AppError::IO(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "IO error occurred",
            ),
            AppError::Serde(_) => (
                StatusCode::BAD_REQUEST,
                "Serialization error",
            ),
            #[cfg(feature = "search")]
            AppError::Tantivy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Search service error",
            ),
        };

        let body = Json(json!({
            "success": false,
            "error": error_message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
