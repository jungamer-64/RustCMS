// src/presentation/http/responses.rs
//! HTTP エラーレスポンス & 共通レスポンス型（Phase 4 Step 3）
//!
//! ApplicationError を HTTP Status Code + JSON Response に変換
//! 参考: RESTRUCTURE_EXAMPLES.md (Error Responses パターン)
//!
//! # 設計原則
//! - ApplicationError の層別エラーを HTTP Status Code にマッピング
//! - 統一されたエラーレスポンスフォーマット
//! - クライアント側で容易にエラー処理可能

use serde::{Deserialize, Serialize};
use std::fmt;

// Note: ApplicationError のインポートは実行時に使用
// テスト時にはモック版を使用する可能性がある
#[cfg(feature = "restructure_application")]
use crate::common::types::ApplicationError;

// ============================================================================
// HTTP レスポンス型
// ============================================================================

/// 成功レスポンス型 T
///
/// # 例
/// ```rust
/// let response: HttpResponse<UserDto> = HttpResponse::ok(user_dto);
/// assert_eq!(response.status, 200);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpResponse<T: Serialize> {
    pub status: u16,
    pub data: T,
    pub message: String,
}

impl<T: Serialize> HttpResponse<T> {
    /// 成功レスポンス（200 OK）
    pub fn ok(data: T) -> Self {
        Self {
            status: 200,
            data,
            message: "OK".to_string(),
        }
    }

    /// 作成成功レスポンス（201 Created）
    pub fn created(data: T) -> Self {
        Self {
            status: 201,
            data,
            message: "Created".to_string(),
        }
    }

    /// カスタムステータスコード
    pub fn with_status(status: u16, data: T, message: String) -> Self {
        Self {
            status,
            data,
            message,
        }
    }
}

/// エラーレスポンス型
///
/// # 例
/// ```rust
/// let error = HttpErrorResponse::from(ApplicationError::UserNotFound("id".into()));
/// assert_eq!(error.status, 404);
/// assert_eq!(error.error_type, "NOT_FOUND");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpErrorResponse {
    pub status: u16,
    pub error_type: String,
    pub message: String,
    pub details: Option<String>,
}

impl HttpErrorResponse {
    /// カスタムエラーレスポンスを作成
    pub fn new(status: u16, error_type: String, message: String, details: Option<String>) -> Self {
        Self {
            status,
            error_type,
            message,
            details,
        }
    }
}

// ============================================================================
// ApplicationError → HTTP Status Code マッピング
// ============================================================================

/// ApplicationError を HTTP Status Code にマッピング
///
/// # エラー階層
/// - DomainError (ビジネスルール違反) → 400 / 409
/// - ApplicationError (ユースケース違反) → 400 / 404 / 409
/// - InfrastructureError (DB/Cache失敗) → 500 / 503
///
/// # 参照
/// RESTRUCTURE_EXAMPLES.md (Error Responses パターン)
#[cfg(feature = "restructure_application")]
impl From<ApplicationError> for HttpErrorResponse {
    fn from(err: ApplicationError) -> Self {
        match err {
            // ========== DomainError を内包している場合 ==========
            ApplicationError::DomainError(domain_err) => Self {
                status: 400, // ほとんどのドメインエラーは 400 Bad Request
                error_type: "DOMAIN_ERROR".to_string(),
                message: domain_err.to_string(),
                details: None,
            },

            // ========== 400 Bad Request（バリデーション失敗）==========
            ApplicationError::ValidationError(msg) => Self {
                status: 400,
                error_type: "VALIDATION_ERROR".to_string(),
                message: msg,
                details: None,
            },

            // ========== 404 Not Found（リソース未検出）==========
            ApplicationError::NotFound(msg) => Self {
                status: 404,
                error_type: "NOT_FOUND".to_string(),
                message: msg,
                details: None,
            },

            // ========== 409 Conflict（ビジネスルール違反）==========
            ApplicationError::Conflict(msg) => Self {
                status: 409,
                error_type: "CONFLICT".to_string(),
                message: msg,
                details: None,
            },

            // ========== 401 Unauthorized（認証失敗）==========
            ApplicationError::Unauthorized(msg) => Self {
                status: 401,
                error_type: "UNAUTHORIZED".to_string(),
                message: msg,
                details: None,
            },

            // ========== 500 Internal Server Error（リポジトリエラー）==========
            ApplicationError::RepositoryError(msg) => Self {
                status: 500,
                error_type: "INTERNAL_SERVER_ERROR".to_string(),
                message: "Database operation failed".to_string(),
                details: Some(msg),
            },

            // ========== 500 Unknown Error ==========
            ApplicationError::Unknown(msg) => Self {
                status: 500,
                error_type: "INTERNAL_SERVER_ERROR".to_string(),
                message: msg,
                details: None,
            },
        }
    }
}

impl fmt::Display for HttpErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HttpError [{}]: {} ({})",
            self.status, self.error_type, self.message
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

// TODO: Phase 4.9 - AppContainer 定義後に有効化する
/*
#[cfg(all(test, feature = "restructure_application"))]
mod tests {
    use super::*;

    /// HttpResponse<T> の作成テスト
    #[test]
    fn test_http_response_ok() {
        #[derive(Serialize)]
        struct TestData {
            id: u32,
            name: String,
        }

        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };
        let response = HttpResponse::ok(data);

        assert_eq!(response.status, 200);
        assert_eq!(response.message, "OK");
    }

    /// HttpResponse<T> created の作成テスト
    #[test]
    fn test_http_response_created() {
        #[derive(Serialize)]
        struct TestData {
            id: u32,
        }

        let data = TestData { id: 1 };
        let response = HttpResponse::created(data);

        assert_eq!(response.status, 201);
        assert_eq!(response.message, "Created");
    }

    /// ApplicationError::ValidationError のマッピングテスト
    #[test]
    fn test_validation_error_mapping() {
        let app_error = ApplicationError::ValidationError("Invalid email format".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 400);
        assert_eq!(http_error.error_type, "VALIDATION_ERROR");
    }

    /// ApplicationError::UserNotFound のマッピングテスト
    #[test]
    fn test_user_not_found_mapping() {
        let app_error = ApplicationError::UserNotFound("user-123".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.error_type, "NOT_FOUND");
        assert!(http_error.message.contains("user-123"));
    }

    /// ApplicationError::EmailAlreadyInUse のマッピングテスト
    #[test]
    fn test_email_conflict_mapping() {
        let app_error = ApplicationError::EmailAlreadyInUse("test@example.com".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 409);
        assert_eq!(http_error.error_type, "CONFLICT");
    }

    /// ApplicationError::RepositoryError のマッピングテスト
    #[test]
    fn test_repository_error_mapping() {
        let app_error = ApplicationError::RepositoryError("Database connection failed".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 500);
        assert_eq!(http_error.error_type, "INTERNAL_SERVER_ERROR");
        assert!(http_error.details.is_some());
    }

    /// HttpErrorResponse の Display trait テスト
    #[test]
    fn test_error_response_display() {
        let error = HttpErrorResponse::new(
            400,
            "VALIDATION_ERROR".to_string(),
            "Invalid input".to_string(),
            None,
        );

        let display_str = format!("{}", error);
        assert!(display_str.contains("400"));
        assert!(display_str.contains("VALIDATION_ERROR"));
    }
}
*/
