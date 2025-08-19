use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::utils::api_types::{ApiResponse, ValidationError, ValidationErrorResponse};

/// アプリケーションエラー型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    #[error("Authorization failed: {0}")]
    Authorization(String),
    
    #[error("Validation failed: {0}")]
    Validation(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Resource conflict: {0}")]
    Conflict(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Auth error: {0}")]
    Auth(String),
    
    #[error("Biscuit token error: {0}")]
    BiscuitToken(String),
    
    #[error("User not found")]
    UserNotFound,
    
    #[error("User already exists")]
    UserAlreadyExists,
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalServer(String),
    
    #[error("URL encoding error: {0}")]
    UrlEncoding(String),
    
    #[error("Invalid URL parameter: {0}")]
    InvalidUrlParam(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("WebAuthn error: {0}")]
    WebAuthn(String),

    #[error("Biscuit error: {0}")]
    Biscuit(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("JSON error: {0}")]
    Json(String),
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("Validation errors")]
    ValidationErrors(Vec<ValidationError>),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Authentication(_) | AppError::Auth(_) | AppError::Unauthorized(_) | AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::Authorization(_) | AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Validation(_) | AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) | AppError::UserNotFound => StatusCode::NOT_FOUND,
            AppError::Conflict(_) | AppError::UserAlreadyExists => StatusCode::CONFLICT,
            AppError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            AppError::ValidationErrors(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::UrlEncoding(_) | AppError::InvalidUrlParam(_) | AppError::UrlParse(_) => StatusCode::BAD_REQUEST,
            AppError::WebAuthn(_) => StatusCode::BAD_REQUEST,
            AppError::Biscuit(_) | AppError::BiscuitToken(_) => StatusCode::UNAUTHORIZED,
            AppError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Database(_) | AppError::InternalServer(_) | AppError::Serialization(_) | AppError::Internal(_) | AppError::Json(_) | AppError::Io(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    pub fn error_message(&self) -> String {
        match self {
            AppError::Config(msg) => format!("Configuration error: {}", msg),
            AppError::Authentication(_) => "Authentication failed".to_string(),
            AppError::Authorization(_) => "Access denied".to_string(),
            AppError::Validation(msg) | AppError::BadRequest(msg) => msg.clone(),
            AppError::NotFound(resource) => format!("{} not found", resource),
            AppError::Conflict(msg) => format!("Resource conflict: {}", msg),
            AppError::Unauthorized(msg) => format!("Unauthorized: {}", msg),
            AppError::Forbidden(msg) => format!("Access forbidden: {}", msg),
            AppError::Auth(msg) => format!("Authentication error: {}", msg),
            AppError::BiscuitToken(msg) => format!("Biscuit token error: {}", msg),
            AppError::UserNotFound => "User not found".to_string(),
            AppError::UserAlreadyExists => "User already exists".to_string(),
            AppError::InvalidCredentials => "Invalid credentials".to_string(),
            AppError::RateLimit => "Rate limit exceeded".to_string(),
            AppError::ValidationErrors(_) => "Validation failed".to_string(),
            AppError::Database(_) => "Database operation failed".to_string(),
            AppError::InternalServer(_) => "Internal server error".to_string(),
            AppError::Serialization(_) => "Data serialization failed".to_string(),
            AppError::UrlEncoding(msg) => format!("URL encoding error: {}", msg),
            AppError::InvalidUrlParam(msg) => format!("Invalid URL parameter: {}", msg),
            AppError::WebAuthn(msg) => format!("WebAuthn error: {}", msg),
            AppError::Biscuit(msg) => format!("Authorization token error: {}", msg),
            AppError::Redis(msg) => format!("Redis error: {}", msg),
            AppError::UrlParse(_) => "Invalid URL format".to_string(),
            AppError::Internal(msg) => format!("Internal error: {}", msg),
            AppError::Json(msg) => format!("JSON error: {}", msg),
            AppError::Io(msg) => format!("IO error: {}", msg),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        
        let body = match self {
            AppError::ValidationErrors(errors) => {
                Json(ValidationErrorResponse {
                    success: false,
                    error: "Validation failed".to_string(),
                    validation_errors: errors,
                })
                .into_response()
            }
            _ => {
                Json(ApiResponse::<()>::error(self.error_message()))
                    .into_response()
            }
        };

        (status, body).into_response()
    }
}

/// 結果型のエイリアス
pub type AppResult<T> = Result<T, AppError>;

#[cfg(feature = "auth")]
impl From<biscuit_auth::error::Token> for AppError {
    fn from(err: biscuit_auth::error::Token) -> Self {
        AppError::BiscuitToken(err.to_string())
    }
}
