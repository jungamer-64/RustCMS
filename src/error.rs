//! アプリケーション共通エラーとHTTPレスポンスへのマッピング
//!
//! 本モジュールはドメイン横断のエラー型 `AppError` を定義し、`IntoResponse` 実装で
//! HTTPステータス/エラーメッセージ/バリデーション詳細へマッピングします。各層からは
//! `?` 演算子で `AppError` に合流させることで、ルータ/ハンドラから一貫した姿で返却できます。
//! Feature に応じて DB/Cache/Search 等の依存エラーを包含します。
use crate::utils::api_types::{ApiResponse, ValidationError};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
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
            Self::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            #[cfg(feature = "auth")]
            Self::Argon2(err) => write!(f, "Argon2 error: {err}"),
            Self::Biscuit(msg) => write!(f, "Biscuit auth error: {msg}"),
            Self::Search(msg) => write!(f, "Search error: {msg}"),
            Self::Media(msg) => write!(f, "Media error: {msg}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
            Self::IO(err) => write!(f, "IO error: {err}"),
            Self::Serde(err) => write!(f, "Serialization error: {err}"),
            #[cfg(feature = "search")]
            Self::Tantivy(err) => write!(f, "Search error: {err}"),
        }
    }
}

impl std::error::Error for AppError {}

#[cfg(feature = "database")]
impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => Self::NotFound("Resource not found".to_string()),
            _ => Self::Database(err),
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

#[cfg(feature = "search")]
impl From<tantivy::TantivyError> for AppError {
    fn from(err: tantivy::TantivyError) -> Self {
        Self::Tantivy(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, validation_details): (
            StatusCode,
            &str,
            Option<Vec<ValidationError>>,
        ) = match &self {
            #[cfg(feature = "database")]
            Self::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error occurred",
                None,
            ),
            #[cfg(feature = "cache")]
            Self::Redis(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cache error occurred",
                None,
            ),
            Self::Validation(ve) => {
                let mut list = Vec::new();
                for (field, errs) in ve.field_errors() {
                    for e in errs {
                        let message = e
                            .message
                            .clone()
                            .unwrap_or_else(|| std::borrow::Cow::from("validation error"));
                        list.push(ValidationError {
                            field: field.to_string(),
                            message: message.to_string(),
                        });
                    }
                }
                (StatusCode::BAD_REQUEST, "Invalid input data", Some(list))
            }
            Self::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg.as_str(), None),
            Self::Authorization(msg) => (StatusCode::FORBIDDEN, msg.as_str(), None),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str(), None),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg.as_str(), None),
            Self::RateLimit(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.as_str(), None),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            #[cfg(feature = "auth")]
            Self::Argon2(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password hashing error",
                None,
            ),
            Self::Biscuit(msg) => (StatusCode::UNAUTHORIZED, msg.as_str(), None),
            Self::Search(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            Self::Media(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            Self::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            Self::IO(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO error occurred", None),
            Self::Serde(_) => (StatusCode::BAD_REQUEST, "Serialization error", None),
            #[cfg(feature = "search")]
            Self::Tantivy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Search service error",
                None,
            ),
        };
        let body = validation_details.map_or_else(
            || Json(ApiResponse::error(error_message.to_string())),
            |details| {
                Json(ApiResponse::error_with_validation(
                    error_message.to_string(),
                    details,
                ))
            },
        );
        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
