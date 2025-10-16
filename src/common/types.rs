//! Common Types (共通型定義)
//!
//! ドメイン層、アプリケーション層、インフラストラクチャ層で共有される型を定義します。
//! 既存の error.rs と互換性を保ちながら、新しい型階層を導入します。

use std::fmt;
use thiserror::Error;

// ============================================================================
// Domain Layer Errors
// ============================================================================

/// ドメイン層のエラー
///
/// ビジネスルール違反や不正な状態遷移を表現します。
#[derive(Debug, Error, Clone)]
pub enum DomainError {
    #[error("Invalid user ID: {0}")]
    InvalidUserId(String),
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
    #[error("Invalid slug: {0}")]
    InvalidSlug(String),
    #[error("Invalid title: {0}")]
    InvalidTitle(String),
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    #[error("Invalid published at: {0}")]
    InvalidPublishedAt(String),
    #[error("Invalid comment text: {0}")]
    InvalidCommentText(String),
    #[error("Invalid comment author: {0}")]
    InvalidCommentAuthor(String),
    #[error("Invalid comment post: {0}")]
    InvalidCommentPost(String),
    #[error("Invalid comment status: {0}")]
    InvalidCommentStatus(String),
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Invalid tag name: {0}")]
    InvalidTagName(String),
    #[error("Invalid tag description: {0}")]
    InvalidTagDescription(String),
    #[error("Invalid tag status: {0}")]
    InvalidTagStatus(String),
    #[error("Invalid category name: {0}")]
    InvalidCategoryName(String),
    #[error("Invalid category slug: {0}")]
    InvalidCategorySlug(String),
    #[error("Invalid category description: {0}")]
    InvalidCategoryDescription(String),
    #[error("Invalid category status: {0}")]
    InvalidCategoryStatus(String),
    #[error("Unknown domain error: {0}")]
    Unknown(String),
}

// ============================================================================
// Application Layer Errors
// ============================================================================

/// アプリケーション層のエラー
///
/// ユースケース実行時のエラーを表現します。
#[derive(Debug, Error, Clone)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    DomainError(#[from] DomainError),

    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("Use case validation failed: {0}")]
    ValidationError(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Authorization failed: {0}")]
    Unauthorized(String),

    #[error("Unknown application error: {0}")]
    Unknown(String),
}

// ============================================================================
// Infrastructure Layer Errors
// ============================================================================

/// インフラストラクチャ層のエラー
///
/// 技術的な実装詳細に関するエラーを表現します。
#[derive(Debug, Error, Clone)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Search index error: {0}")]
    SearchError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unknown infrastructure error: {0}")]
    Unknown(String),
}

// ============================================================================
// Unified Error Type (既存のAppErrorとの互換性)
// ============================================================================

/// 統一エラー型（既存の AppError と互換性を保つ）
///
/// 全レイヤーでの使用を想定し、詳細なエラー情報を保持します。
#[derive(Debug, Clone)]
pub enum AppError {
    Domain(DomainError),
    Application(ApplicationError),
    Infrastructure(InfrastructureError),
    Internal(String),
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    Unauthorized(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Domain(e) => write!(f, "Domain error: {e}"),
            AppError::Application(e) => write!(f, "Application error: {e}"),
            AppError::Infrastructure(e) => write!(f, "Infrastructure error: {e}"),
            AppError::Internal(e) => write!(f, "Internal error: {e}"),
            AppError::NotFound(e) => write!(f, "Not found: {e}"),
            AppError::Conflict(e) => write!(f, "Conflict: {e}"),
            AppError::BadRequest(e) => write!(f, "Bad request: {e}"),
            AppError::Unauthorized(e) => write!(f, "Unauthorized: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

// Conversions from layer-specific errors to unified AppError
impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self {
        AppError::Domain(e)
    }
}

impl From<ApplicationError> for AppError {
    fn from(e: ApplicationError) -> Self {
        AppError::Application(e)
    }
}

impl From<InfrastructureError> for AppError {
    fn from(e: InfrastructureError) -> Self {
        AppError::Infrastructure(e)
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// ドメイン層のResult型
pub type DomainResult<T> = std::result::Result<T, DomainError>;

/// アプリケーション層のResult型
pub type ApplicationResult<T> = std::result::Result<T, ApplicationError>;

/// インフラストラクチャ層のResult型
pub type InfrastructureResult<T> = std::result::Result<T, InfrastructureError>;

/// 統一Result型
pub type Result<T> = std::result::Result<T, AppError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error_display() {
        let error = DomainError::InvalidEmail("test".to_string());
        assert!(format!("{error}").contains("Invalid email"));
    }

    #[test]
    fn test_application_error_from_domain() {
        let domain_err = DomainError::InvalidEmail("test".to_string());
        let app_err: ApplicationError = domain_err.into();
        assert!(format!("{app_err}").contains("Domain error"));
    }

    #[test]
    fn test_app_error_from_domain() {
        let domain_err = DomainError::InvalidEmail("test".to_string());
        let app_err: AppError = domain_err.into();
        assert!(matches!(app_err, AppError::Domain(_)));
    }

    #[test]
    fn test_app_error_display() {
        let error = AppError::NotFound("User 123".to_string());
        assert_eq!(format!("{error}"), "Not found: User 123");
    }

    #[test]
    fn test_infrastructure_error_display() {
        let error = InfrastructureError::DatabaseError("Connection pool exhausted".to_string());
        assert!(format!("{error}").contains("Database error"));
    }
}
