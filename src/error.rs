//! アプリケーション共通エラーとHTTPレスポンスへのマッピング
//!
//! 本モジュールはドメイン横断のエラー型 `AppError` を定義し、`IntoResponse` 実装で
//! HTTPステータス/エラーメッセージ/バリデーション詳細へマッピングします。各層からは
//! `?` 演算子で `AppError` に合流させることで、ルータ/ハンドラから一貫した姿で返却できます。
//! Feature に応じて DB/Cache/Search 等の依存エラーを包含します。
use crate::utils::api_types::{ApiResponse, ValidationError};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::fmt;
use tracing::{debug, error};
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
    // Back-compat string-based config error
    Config(String),
    // Rich config load/deserialize error from `config` crate
    ConfigLoad(config::ConfigError),
    // Specific missing configuration value (e.g., required env var)
    ConfigValueMissing(String),
    // Configuration validation failed after loading
    ConfigValidationError(String),
    IO(std::io::Error),
    Serde(serde_json::Error),
    #[cfg(feature = "search")]
    Tantivy(tantivy::TantivyError),
}

// Displayの実装を少しRustらしく簡潔にしました。
// 元のコードでも機能は同じですが、こちらの方が一般的です。
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
            Self::Argon2(err) => write!(f, "Password hashing error: {err}"),
            Self::Biscuit(msg) => write!(f, "Biscuit auth error: {msg}"),
            Self::Search(msg) => write!(f, "Search error: {msg}"),
            Self::Media(msg) => write!(f, "Media error: {msg}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
            Self::ConfigLoad(err) => write!(f, "Configuration loading error: {err}"),
            Self::ConfigValueMissing(key) => write!(f, "Configuration value missing for key: {key}"),
            Self::ConfigValidationError(msg) => write!(f, "Configuration validation error: {msg}"),
            Self::IO(err) => write!(f, "IO error: {err}"),
            Self::Serde(err) => write!(f, "Serialization error: {err}"),
            #[cfg(feature = "search")]
            Self::Tantivy(err) => write!(f, "Tantivy search error: {err}"),
        }
    }
}

/// Provide source chaining for wrapped errors so callers can traverse
/// the underlying cause (diesel, redis, io, serde, config, tantivy, ...).
impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(feature = "database")]
            Self::Database(err) => Some(err as &(dyn std::error::Error + 'static)),
            #[cfg(feature = "cache")]
            Self::Redis(err) => Some(err as &(dyn std::error::Error + 'static)),
            // argon2::Error does not currently implement std::error::Error
            // in all versions; skip exposing it as a source to avoid type errors.
            #[cfg(feature = "auth")]
            Self::Argon2(_) => None,
            Self::ConfigLoad(err) => Some(err as &(dyn std::error::Error + 'static)),
            Self::IO(err) => Some(err as &(dyn std::error::Error + 'static)),
            Self::Serde(err) => Some(err as &(dyn std::error::Error + 'static)),
            #[cfg(feature = "search")]
            Self::Tantivy(err) => Some(err as &(dyn std::error::Error + 'static)),
            // Variants that hold Cow/strings or ValidationErrors do not have
            // a boxed underlying `Error` to expose here.
            _ => None,
        }
    }
}

#[cfg(feature = "database")]
impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => Self::NotFound("Resource not found".into()),
            // その他のDBエラーはすべて内包する
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

impl IntoResponse for AppError {
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn into_response(self) -> Response {
        // Delegate classification to AppError::summary
        let summary = self.summary();
        error!(summary = %summary, "Converting error into HTTP response");
        debug!(error.details = ?self, "Full error details");

        let (status, error_message, validation_details) = match &self {
            // 5xx系のエラー。クライアントには汎用的なメッセージを返し、詳細はログで確認する。
            #[cfg(feature = "database")]
            Self::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error occurred",
                None,
            ),
            #[cfg(feature = "cache")]
            Self::Redis(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A cache error occurred",
                None,
            ),
            // For internal server errors we avoid returning internal message
            // verbatim to clients to prevent accidental leakage of sensitive
            // implementation details. The log above contains the full details
            // for operators.
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occurred",
                None,
            ),
            #[cfg(feature = "auth")]
            Self::Argon2(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A password hashing error occurred",
                None,
            ),
            Self::Search(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str(), None),
            Self::Config(_) | Self::ConfigLoad(_) | Self::ConfigValueMissing(_) | Self::ConfigValidationError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A server configuration error occurred",
                None
            ),
            Self::IO(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An I/O error occurred",
                None,
            ),
            #[cfg(feature = "search")]
            Self::Tantivy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A search service error occurred",
                None,
            ),

            // 4xx系のエラー。クライアントに原因が分かるようなメッセージを返す。
            Self::Validation(ve) => {
                let details = ve
                    .field_errors()
                    .into_iter()
                    .flat_map(|(field, errors)| {
                        errors.iter().map(move |e| ValidationError {
                            field: field.to_string(),
                            message: e.message.as_ref().map_or_else(
                                || "Invalid value".to_string(),
                                |s| s.to_string(),
                            ),
                        })
                    })
                    .collect();
                (
            StatusCode::UNPROCESSABLE_ENTITY, // 400 Bad Requestより具体的
                "Invalid input",
                    Some(details),
                )
            }
            Self::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg.as_str(), None),
            Self::Authorization(msg) => (StatusCode::FORBIDDEN, msg.as_str(), None),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.as_str(), None),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg.as_str(), None),
            Self::RateLimit(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.as_str(), None),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            Self::Biscuit(msg) => (StatusCode::UNAUTHORIZED, msg.as_str(), None),
            Self::Media(msg) => (StatusCode::BAD_REQUEST, msg.as_str(), None),
            Self::Serde(_) => (
                StatusCode::BAD_REQUEST,
                "Failed to process request body",
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

impl AppError {
    /// Return a short, non-sensitive summary suitable for logs or client-facing
    /// short messages (not the detailed error text).
    pub fn summary(&self) -> &'static str {
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
            Self::Config(_) | Self::ConfigLoad(_) | Self::ConfigValueMissing(_) | Self::ConfigValidationError(_) => "configuration error",
            #[cfg(feature = "search")]
            Self::Tantivy(_) => "search backend error",
        }
    }
}

// Result型エイリアスはモジュールの最後に定義するのが一般的
pub type Result<T> = std::result::Result<T, AppError>;
