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
pub use domain::common::{DomainError, DomainResult};

// ============================================================================
// Application Layer Errors
// ============================================================================

pub use application::common::types::{ApplicationError, ApplicationResult};

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

    // =========================================================================
    // DomainError Tests (基本, 表示, バリアント)
    // =========================================================================

    #[test]
    fn test_domain_error_display() {
        let error = DomainError::InvalidEmail("test".to_string());
        assert!(format!("{error}").contains("Invalid email"));
    }

    #[test]
    fn test_domain_error_all_variants() {
        let variants = vec![
            DomainError::InvalidUserId("".to_string()),
            DomainError::InvalidEmail("".to_string()),
            DomainError::InvalidUsername("".to_string()),
            DomainError::InvalidSlug("".to_string()),
            DomainError::InvalidTitle("".to_string()),
            DomainError::InvalidContent("".to_string()),
            DomainError::InvalidPublishedAt("".to_string()),
            DomainError::InvalidCommentText("".to_string()),
            DomainError::InvalidCommentAuthor("".to_string()),
            DomainError::InvalidCommentPost("".to_string()),
            DomainError::InvalidCommentStatus("".to_string()),
            DomainError::InvalidStateTransition("".to_string()),
            DomainError::InvalidTagName("".to_string()),
            DomainError::InvalidTagDescription("".to_string()),
            DomainError::InvalidTagStatus("".to_string()),
            DomainError::InvalidCategoryName("".to_string()),
            DomainError::InvalidCategorySlug("".to_string()),
            DomainError::InvalidCategoryDescription("".to_string()),
            DomainError::InvalidCategoryStatus("".to_string()),
            DomainError::Unknown("".to_string()),
        ];

        for variant in variants {
            let display = format!("{variant}");
            assert!(
                !display.is_empty(),
                "DomainError display should not be empty"
            );
        }
    }

    #[test]
    fn test_domain_error_is_clone() {
        let error = DomainError::InvalidEmail("original".to_string());
        let cloned = error.clone();
        assert_eq!(format!("{error}"), format!("{cloned}"));
    }

    #[test]
    fn test_domain_error_debug() {
        let error = DomainError::InvalidEmail("test@example.com".to_string());
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("InvalidEmail"));
    }

    // =========================================================================
    // ApplicationError Tests (変換, 表示, エラー伝播)
    // =========================================================================

    #[test]
    fn test_application_error_from_domain() {
        let domain_err = DomainError::InvalidEmail("test".to_string());
        let app_err: ApplicationError = domain_err.into();
        assert!(format!("{app_err}").contains("Domain error"));
    }

    #[test]
    fn test_application_error_all_variants() {
        let domain_error = DomainError::InvalidUsername("invalid".to_string());
        let variants = vec![
            ApplicationError::DomainError(domain_error.clone()),
            ApplicationError::RepositoryError("DB connection failed".to_string()),
            ApplicationError::ValidationError("Invalid input".to_string()),
            ApplicationError::Conflict("Resource already exists".to_string()),
            ApplicationError::NotFound("Resource not found".to_string()),
            ApplicationError::Unauthorized("Access denied".to_string()),
            ApplicationError::Unknown("Unknown error".to_string()),
        ];

        for variant in variants {
            let display = format!("{variant}");
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_application_error_direct_construction() {
        let err = ApplicationError::ValidationError("email is required".to_string());
        assert!(format!("{err}").contains("email is required"));
    }

    // =========================================================================
    // InfrastructureError Tests (DB, Cache, Search, Config, Connection)
    // =========================================================================

    #[test]
    fn test_infrastructure_error_display() {
        let error = InfrastructureError::DatabaseError("Connection pool exhausted".to_string());
        assert!(format!("{error}").contains("Database error"));
    }

    #[test]
    fn test_infrastructure_error_all_variants() {
        let variants = vec![
            InfrastructureError::DatabaseError("Connection timeout".to_string()),
            InfrastructureError::CacheError("Redis unavailable".to_string()),
            InfrastructureError::SearchError("Tantivy index corrupted".to_string()),
            InfrastructureError::ConfigError("Missing required config".to_string()),
            InfrastructureError::ConnectionError("Connection refused".to_string()),
            InfrastructureError::Timeout("Operation timed out".to_string()),
            InfrastructureError::Unknown("Unknown infrastructure error".to_string()),
        ];

        for variant in variants {
            let display = format!("{variant}");
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_infrastructure_error_is_clone() {
        let error = InfrastructureError::CacheError("Redis down".to_string());
        let cloned = error.clone();
        assert_eq!(format!("{error}"), format!("{cloned}"));
    }

    // =========================================================================
    // AppError Tests (全バリアント, 表示, 変換)
    // =========================================================================

    #[test]
    fn test_app_error_from_domain() {
        let domain_err = DomainError::InvalidEmail("test".to_string());
        let app_err: AppError = domain_err.into();
        assert!(matches!(app_err, AppError::Domain(_)));
    }

    #[test]
    fn test_app_error_from_application() {
        let app_err_inner = ApplicationError::NotFound("User 123".to_string());
        let app_err: AppError = app_err_inner.into();
        assert!(matches!(app_err, AppError::Application(_)));
    }

    #[test]
    fn test_app_error_from_infrastructure() {
        let infra_err = InfrastructureError::DatabaseError("Connection error".to_string());
        let app_err: AppError = infra_err.into();
        assert!(matches!(app_err, AppError::Infrastructure(_)));
    }

    #[test]
    fn test_app_error_display() {
        let error = AppError::NotFound("User 123".to_string());
        assert_eq!(format!("{error}"), "Not found: User 123");
    }

    #[test]
    fn test_app_error_all_variants_display() {
        let variants = vec![
            (
                AppError::Domain(DomainError::InvalidEmail("".to_string())),
                "Domain error:",
            ),
            (
                AppError::Application(ApplicationError::NotFound("".to_string())),
                "Application error:",
            ),
            (
                AppError::Infrastructure(InfrastructureError::DatabaseError("".to_string())),
                "Infrastructure error:",
            ),
            (
                AppError::Internal("Internal server error".to_string()),
                "Internal error:",
            ),
            (
                AppError::NotFound("Resource not found".to_string()),
                "Not found:",
            ),
            (
                AppError::Conflict("Duplicate entry".to_string()),
                "Conflict:",
            ),
            (
                AppError::BadRequest("Invalid data".to_string()),
                "Bad request:",
            ),
            (
                AppError::Unauthorized("No auth".to_string()),
                "Unauthorized:",
            ),
        ];

        for (error, expected_prefix) in variants {
            let display = format!("{error}");
            assert!(display.contains(expected_prefix), "Failed for: {error}");
        }
    }

    #[test]
    fn test_app_error_is_error_trait() {
        use std::error::Error;
        let error: Box<dyn Error> = Box::new(AppError::NotFound("test".to_string()));
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn test_app_error_is_clone() {
        let error = AppError::Conflict("Duplicate email".to_string());
        let cloned = error.clone();
        assert_eq!(format!("{error}"), format!("{cloned}"));
    }

    // =========================================================================
    // Result Type Tests (型安全性, 使用可能性)
    // =========================================================================

    #[test]
    fn test_domain_result_success() {
        let result: DomainResult<i32> = Ok(42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_domain_result_error() {
        let result: DomainResult<i32> = Err(DomainError::InvalidEmail("test".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_application_result_success() {
        let result: ApplicationResult<String> = Ok("success".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_application_result_error() {
        let result: ApplicationResult<String> =
            Err(ApplicationError::NotFound("Item not found".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_infrastructure_result_success() {
        let result: InfrastructureResult<bool> = Ok(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_infrastructure_result_error() {
        let result: InfrastructureResult<bool> = Err(InfrastructureError::DatabaseError(
            "Connection failed".to_string(),
        ));
        assert!(result.is_err());
    }

    #[test]
    fn test_unified_result_success() {
        let result: Result<&str> = Ok("success");
        assert!(result.is_ok());
    }

    #[test]
    fn test_unified_result_error() {
        let result: Result<&str> = Err(AppError::NotFound("Not found".to_string()));
        assert!(result.is_err());
    }

    // =========================================================================
    // Error Propagation Tests (層間変換, チェーン)
    // =========================================================================

    #[test]
    fn test_error_propagation_domain_to_app() {
        fn domain_operation() -> DomainResult<()> {
            Err(DomainError::InvalidEmail("bad email".to_string()))
        }

        fn app_operation() -> ApplicationResult<()> {
            domain_operation().map_err(|e| ApplicationError::DomainError(e))
        }

        let result = app_operation();
        assert!(result.is_err());
        if let Err(ApplicationError::DomainError(de)) = result {
            assert!(format!("{de}").contains("Invalid email"));
        } else {
            panic!("Expected DomainError variant");
        }
    }

    #[test]
    fn test_error_propagation_domain_to_unified() {
        let domain_err = DomainError::InvalidUsername("too_short".to_string());
        let unified_err: AppError = domain_err.into();
        assert!(matches!(unified_err, AppError::Domain(_)));
    }

    #[test]
    fn test_error_propagation_app_to_unified() {
        let app_err = ApplicationError::Conflict("Duplicate email".to_string());
        let unified_err: AppError = app_err.into();
        assert!(matches!(unified_err, AppError::Application(_)));
    }

    #[test]
    fn test_error_propagation_infra_to_unified() {
        let infra_err = InfrastructureError::Timeout("Request timeout".to_string());
        let unified_err: AppError = infra_err.into();
        assert!(matches!(unified_err, AppError::Infrastructure(_)));
    }

    // =========================================================================
    // Edge Cases & Boundary Tests (空文字列, 長い文字列, 特殊文字)
    // =========================================================================

    #[test]
    fn test_error_with_empty_message() {
        let error = DomainError::Unknown("".to_string());
        let display = format!("{error}");
        assert!(!display.is_empty());
    }

    #[test]
    fn test_error_with_long_message() {
        let long_msg = "x".repeat(1000);
        let error = DomainError::InvalidContent(long_msg.clone());
        let display = format!("{error}");
        assert!(display.contains(&long_msg));
    }

    #[test]
    fn test_error_with_special_characters() {
        let special_msg = "Error: <tag> & \"quote\" 'apostrophe' \n newline";
        let error = ApplicationError::ValidationError(special_msg.to_string());
        let display = format!("{error}");
        assert!(display.contains(special_msg));
    }

    #[test]
    fn test_error_with_unicode_message() {
        let unicode_msg = "エラー: 無効なメール 🚀";
        let error = DomainError::InvalidEmail(unicode_msg.to_string());
        let display = format!("{error}");
        assert!(display.contains(unicode_msg));
    }

    // =========================================================================
    // State Transition Tests (複数のエラー型変換)
    // =========================================================================

    #[test]
    fn test_multi_step_error_transformation() {
        let original_error = DomainError::InvalidSlug("bad-slug!!".to_string());

        let as_app_error: ApplicationError = original_error.clone().into();
        assert!(matches!(as_app_error, ApplicationError::DomainError(_)));

        let as_unified: AppError = as_app_error.into();
        assert!(matches!(as_unified, AppError::Application(_)));
    }

    #[test]
    fn test_error_recovery_patterns() {
        let result: DomainResult<i32> = Err(DomainError::InvalidUsername("bad".to_string()));

        let recovered: DomainResult<i32> = result.or_else(|_| Ok(0));
        assert_eq!(recovered, Ok(0));
    }

    #[test]
    fn test_error_mapping() {
        let result: ApplicationResult<i32> =
            Err(ApplicationError::NotFound("User 123".to_string()));

        let mapped = result.map_err(|_| AppError::NotFound("Mapped error".to_string()));
        assert!(mapped.is_err());
        if let Err(AppError::NotFound(msg)) = mapped {
            assert_eq!(msg, "Mapped error");
        } else {
            panic!("Expected NotFound variant");
        }
    }

    // =========================================================================
    // Compatibility Tests (既存のappError実装との互換性)
    // =========================================================================

    #[test]
    fn test_app_error_variants_consistency() {
        let internal = AppError::Internal("Server error".to_string());
        let not_found = AppError::NotFound("Resource not found".to_string());
        let conflict = AppError::Conflict("Already exists".to_string());
        let bad_request = AppError::BadRequest("Invalid input".to_string());
        let unauthorized = AppError::Unauthorized("No permission".to_string());

        let variants = vec![internal, not_found, conflict, bad_request, unauthorized];
        for error in variants {
            let _msg = format!("{error}");
            // Should all be displayable
        }
    }

    #[test]
    fn test_domain_error_semantics() {
        // DomainError should represent business rule violations
        let state_error = DomainError::InvalidStateTransition("Cannot publish draft".to_string());
        assert!(format!("{state_error}").contains("Invalid state transition"));

        let validation_error = DomainError::InvalidEmail("no-at-sign".to_string());
        assert!(format!("{validation_error}").contains("Invalid email"));
    }
}
