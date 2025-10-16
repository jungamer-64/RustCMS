// tests/error_handling_tests.rs
//! エラーハンドリング 統合テスト（Phase 4 Step 9）
//!
//! ApplicationError → HTTP Status Code + JSON Response マッピング検証
//! 参考: RESTRUCTURE_EXAMPLES.md (Error Responses パターン)
//!
//! # 実行方法
//! ```bash
//! cargo test --test error_handling_tests --features restructure_application -- --nocapture
//! ```

#![cfg(feature = "restructure_application")]

#[cfg(test)]
mod tests {
    use cms_server::common::types::ApplicationError;
    use cms_server::presentation::http::responses::HttpErrorResponse;

    // ========== 400 Bad Request テスト ==========

    /// ValidationError のマッピング確認
    #[test]
    fn test_validation_error_maps_to_400() {
        let app_error = ApplicationError::ValidationError("Invalid email format".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 400);
        assert_eq!(http_error.error_type, "VALIDATION_ERROR");
        assert_eq!(http_error.message, "Invalid email format");
    }

    /// 複数バリデーションエラー
    #[test]
    fn test_multiple_validation_errors() {
        let errors = vec![
            ApplicationError::ValidationError("Email is required".into()),
            ApplicationError::ValidationError("Username must be 3+ characters".into()),
        ];

        for app_error in errors {
            let http_error = HttpErrorResponse::from(app_error);
            assert_eq!(http_error.status, 400);
        }
    }

    // ========== 404 Not Found テスト ==========

    /// UserNotFound のマッピング確認
    #[test]
    fn test_user_not_found_maps_to_404() {
        let app_error = ApplicationError::UserNotFound("user-uuid-123".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.error_type, "NOT_FOUND");
        assert!(http_error.message.contains("user-uuid-123"));
    }

    /// PostNotFound のマッピング確認
    #[test]
    fn test_post_not_found_maps_to_404() {
        let app_error = ApplicationError::PostNotFound("missing-post-slug".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.error_type, "NOT_FOUND");
    }

    /// CommentNotFound のマッピング確認
    #[test]
    fn test_comment_not_found_maps_to_404() {
        let app_error = ApplicationError::CommentNotFound("comment-id".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.error_type, "NOT_FOUND");
    }

    /// TagNotFound のマッピング確認
    #[test]
    fn test_tag_not_found_maps_to_404() {
        let app_error = ApplicationError::TagNotFound("unknown-tag".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.error_type, "NOT_FOUND");
    }

    /// CategoryNotFound のマッピング確認
    #[test]
    fn test_category_not_found_maps_to_404() {
        let app_error = ApplicationError::CategoryNotFound("unknown-category".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 404);
        assert_eq!(http_error.error_type, "NOT_FOUND");
    }

    // ========== 409 Conflict テスト ==========

    /// EmailAlreadyInUse のマッピング確認
    #[test]
    fn test_email_conflict_maps_to_409() {
        let app_error = ApplicationError::EmailAlreadyInUse("john@example.com".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 409);
        assert_eq!(http_error.error_type, "CONFLICT");
        assert!(http_error.message.contains("john@example.com"));
    }

    /// SlugAlreadyInUse のマッピング確認
    #[test]
    fn test_slug_conflict_maps_to_409() {
        let app_error = ApplicationError::SlugAlreadyInUse("my-post-slug".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 409);
        assert_eq!(http_error.error_type, "CONFLICT");
        assert!(http_error.message.contains("my-post-slug"));
    }

    /// BusinessRuleViolation のマッピング確認
    #[test]
    fn test_business_rule_violation_maps_to_409() {
        let app_error =
            ApplicationError::BusinessRuleViolation("Cannot update published post".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 409);
        assert_eq!(http_error.error_type, "CONFLICT");
    }

    // ========== 401 Unauthorized テスト ==========

    /// AuthenticationFailed のマッピング確認
    #[test]
    fn test_authentication_failed_maps_to_401() {
        let app_error = ApplicationError::AuthenticationFailed("Invalid credentials".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 401);
        assert_eq!(http_error.error_type, "UNAUTHORIZED");
    }

    // ========== 403 Forbidden テスト ==========

    /// AuthorizationFailed のマッピング確認
    #[test]
    fn test_authorization_failed_maps_to_403() {
        let app_error = ApplicationError::AuthorizationFailed("Insufficient permissions".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 403);
        assert_eq!(http_error.error_type, "FORBIDDEN");
    }

    // ========== 500 Internal Server Error テスト ==========

    /// RepositoryError のマッピング確認
    #[test]
    fn test_repository_error_maps_to_500() {
        let app_error = ApplicationError::RepositoryError("Database connection timeout".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 500);
        assert_eq!(http_error.error_type, "INTERNAL_SERVER_ERROR");
        assert!(http_error.details.is_some());
    }

    /// CacheError のマッピング確認
    #[test]
    fn test_cache_error_maps_to_500() {
        let app_error = ApplicationError::CacheError("Redis connection failed".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 500);
        assert_eq!(http_error.error_type, "INTERNAL_SERVER_ERROR");
    }

    /// SearchError のマッピング確認
    #[test]
    fn test_search_error_maps_to_500() {
        let app_error = ApplicationError::SearchError("Tantivy index error".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 500);
        assert_eq!(http_error.error_type, "INTERNAL_SERVER_ERROR");
    }

    /// Other エラーのマッピング確認
    #[test]
    fn test_other_error_maps_to_500() {
        let app_error = ApplicationError::Other("Unexpected error".into());
        let http_error = HttpErrorResponse::from(app_error);

        assert_eq!(http_error.status, 500);
        assert_eq!(http_error.error_type, "INTERNAL_SERVER_ERROR");
    }

    // ========== エラーレスポンス JSON シリアライゼーション ==========

    /// エラーレスポンスを JSON にシリアライズ可能
    #[test]
    fn test_error_response_serialization() {
        let app_error = ApplicationError::UserNotFound("user-id".into());
        let http_error = HttpErrorResponse::from(app_error);

        let json_result = serde_json::to_string(&http_error);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("404"));
        assert!(json_str.contains("NOT_FOUND"));
    }

    /// エラーレスポンス Display trait
    #[test]
    fn test_error_response_display() {
        let app_error = ApplicationError::ValidationError("Invalid input".into());
        let http_error = HttpErrorResponse::from(app_error);

        let display_str = format!("{}", http_error);
        assert!(display_str.contains("400"));
        assert!(display_str.contains("VALIDATION_ERROR"));
        assert!(display_str.contains("Invalid input"));
    }
}
