use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use crate::utils::api_types::{ValidationError, ValidationErrorResponse};
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
    let (status, error_message, validation_details): (StatusCode, &str, Option<Vec<ValidationError>>) = match &self {
            #[cfg(feature = "database")]
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred", None),
            #[cfg(feature = "cache")]
            AppError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Cache error occurred", None),
            AppError::Validation(ve) => {
                let mut list = Vec::new();
                for (field, errs) in ve.field_errors().iter() {
                    for e in errs.iter() {
                        let message = e.message.clone().unwrap_or_else(|| std::borrow::Cow::from("validation error"));
                        list.push(ValidationError { field: field.to_string(), message: message.to_string() });
                    }
                }
                (StatusCode::BAD_REQUEST, "Invalid input data", Some(list))
            },
            AppError::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg.as_str(), None),
            AppError::Authorization(msg) => (StatusCode::FORBIDDEN, msg.as_str(), None),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str(), None),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.as_str(), None),
            AppError::RateLimit(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.as_str(), None),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            #[cfg(feature = "auth")]
            AppError::Argon2(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Password hashing error", None),
            AppError::Biscuit(msg) => (StatusCode::UNAUTHORIZED, msg.as_str(), None),
            AppError::Search(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            AppError::Media(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            AppError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            AppError::IO(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO error occurred", None),
            AppError::Serde(_) => (StatusCode::BAD_REQUEST, "Serialization error", None),
            #[cfg(feature = "search")]
            AppError::Tantivy(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Search service error", None),
        };
        let body = ValidationErrorResponse {
            success: false,
            error: error_message.to_string(),
            validation_errors: validation_details.unwrap_or_default(),
        };
        let body = Json(json!({
            "success": body.success,
            "error": body.error,
            "validation_errors": body.validation_errors,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
