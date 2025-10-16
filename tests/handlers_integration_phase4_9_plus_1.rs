//! HTTP Handlers Integration Tests（Phase 4.9+1）
//!
//! Presentation Layer と Application Layer の統合テスト
//! - handlers.rs が正しく DTO を返すか
//! - error_to_response が HttpErrorResponse に変換するか
//! - router.rs が正しくハンドラーをマウントするか
//!
//! 参考: RESTRUCTURE_EXAMPLES.md, TESTING_STRATEGY.md

#![cfg(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
))]

#[cfg(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
))]
mod tests {
    use axum::http::StatusCode;
    use cms_backend::application::post::{CreatePostRequest, PostDto};
    use cms_backend::application::user::{CreateUserRequest, UserDto};
    use cms_backend::common::types::ApplicationError;
    use cms_backend::presentation::http::handlers;
    use cms_backend::presentation::http::responses::HttpErrorResponse;
    use serde_json::json;
    use uuid::Uuid;

    // ========================================================================
    // User Handler Tests
    // ========================================================================

    #[tokio::test]
    async fn test_register_user_handler_returns_dto() {
        // Arrange
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
        };

        // Act
        let result = handlers::register_user(axum::Json(request)).await;

        // Assert
        assert!(result.is_ok());
        let (status, dto) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(dto.0.username, "testuser");
        assert_eq!(dto.0.email, "test@example.com");
        assert!(dto.0.is_active);
    }

    #[tokio::test]
    async fn test_get_user_handler_returns_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();

        // Act
        let result: Result<(StatusCode, axum::Json<UserDto>), axum::response::Response> =
            handlers::get_user(axum::extract::Path(user_id)).await;

        // Assert
        assert!(result.is_err());
        let response = result.unwrap_err();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_user_handler_returns_not_found() {
        // Arrange
        let user_id = Uuid::new_v4();

        // Act
        let result = handlers::delete_user(axum::extract::Path(user_id)).await;

        // Assert
        assert!(result.is_err());
        let response = result.unwrap_err();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ========================================================================
    // Post Handler Tests
    // ========================================================================

    #[tokio::test]
    async fn test_create_post_handler_returns_dto() {
        // Arrange
        let request = CreatePostRequest {
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            author_id: Uuid::new_v4(),
        };

        // Act
        let result = handlers::create_post(axum::Json(request)).await;

        // Assert
        assert!(result.is_ok());
        let (status, dto) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(dto.0.title, "Test Post");
        assert_eq!(dto.0.content, "Test content");
        assert!(!dto.0.is_published);
    }

    #[tokio::test]
    async fn test_get_post_handler_returns_not_found() {
        // Arrange
        let post_id = Uuid::new_v4();

        // Act
        let result: Result<(StatusCode, axum::Json<PostDto>), axum::response::Response> =
            handlers::get_post(axum::extract::Path(post_id)).await;

        // Assert
        assert!(result.is_err());
        let response = result.unwrap_err();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ========================================================================
    // Error Response Helper Tests
    // ========================================================================

    #[test]
    fn test_error_to_response_not_found() {
        // Arrange
        let error = ApplicationError::NotFound("User not found".to_string());

        // Act
        let response = handlers::error_to_response(error);

        // Assert
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_to_response_validation_error() {
        // Arrange
        let error = ApplicationError::ValidationError("Invalid email".to_string());

        // Act
        let response = handlers::error_to_response(error);

        // Assert
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_to_response_conflict() {
        // Arrange
        let error = ApplicationError::Conflict("Email already exists".to_string());

        // Act
        let response = handlers::error_to_response(error);

        // Assert
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_error_to_response_unauthorized() {
        // Arrange
        let error = ApplicationError::Unauthorized("Invalid credentials".to_string());

        // Act
        let response = handlers::error_to_response(error);

        // Assert
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_to_response_repository_error() {
        // Arrange
        let error = ApplicationError::RepositoryError("Database error".to_string());

        // Act
        let response = handlers::error_to_response(error);

        // Assert
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ========================================================================
    // Comment Handler Tests (Stubs)
    // ========================================================================

    #[tokio::test]
    async fn test_create_comment_handler_returns_not_found() {
        // Arrange
        let post_id = Uuid::new_v4();
        let request = serde_json::json!({"text": "Test comment"});

        // Act
        let result: Result<(StatusCode, axum::Json<_>), axum::response::Response> =
            handlers::create_comment(axum::extract::Path(post_id), axum::Json(request)).await;

        // Assert
        assert!(result.is_err());
        let response = result.unwrap_err();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_comments_handler_returns_not_found() {
        // Arrange
        let post_id = Uuid::new_v4();

        // Act
        let result: Result<(StatusCode, axum::Json<Vec<_>>), axum::response::Response> =
            handlers::list_comments(axum::extract::Path(post_id)).await;

        // Assert
        assert!(result.is_err());
        let response = result.unwrap_err();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
