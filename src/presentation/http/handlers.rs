//! HTTP Handlers 実装（Phase 4.9 Simplified）
//!
//! 薄いハンドラー実装: HTTP リクエスト/レスポンス変換のみ
//! ビジネスロジックはアプリケーション層に委譲
//!
//! 参考: RESTRUCTURE_EXAMPLES.md

#[cfg(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
))]
#[allow(clippy::module_inception)]
#[allow(clippy::unnecessary_cast)]
pub mod handlers {
    use crate::application::category::CategoryDto;
    use crate::application::comment::CommentDto;
    use crate::application::post::{CreatePostRequest, PostDto, UpdatePostRequest};
    use crate::application::tag::TagDto;
    use crate::application::user::{CreateUserRequest, UpdateUserRequest, UserDto};
    use crate::common::types::ApplicationError;
    use crate::presentation::http::responses::HttpErrorResponse;
    use axum::Json;
    use axum::extract::Path;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use uuid::Uuid;

    // ========================================================================
    // Error Response Helper
    // ========================================================================

    /// ApplicationError を HTTP Response に変換
    pub fn error_to_response(error: ApplicationError) -> Response {
        let response: HttpErrorResponse = error.into();
        (
            StatusCode::from_u16(response.status as u16)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(response),
        )
            .into_response()
    }

    // ========================================================================
    // User Handlers (Phase 4.1)
    // ========================================================================

    /// ユーザー登録
    pub async fn register_user(
        Json(request): Json<CreateUserRequest>,
    ) -> Result<(StatusCode, Json<UserDto>), Response> {
        // Phase 4.9+1: Application層と接続
        let user = UserDto {
            id: Uuid::new_v4(),
            username: request.username,
            email: request.email,
            is_active: true,
        };
        Ok((StatusCode::CREATED, Json(user)))
    }

    /// ユーザー取得
    pub async fn get_user(
        Path(user_id): Path<Uuid>,
    ) -> Result<(StatusCode, Json<UserDto>), Response> {
        // Phase 4.9+1: ユーザーをリポジトリから取得
        Err(error_to_response(ApplicationError::NotFound(format!(
            "User {} not found",
            user_id
        ))))
    }

    /// ユーザー更新
    pub async fn update_user(
        Path(user_id): Path<Uuid>,
    Json(_request): Json<UpdateUserRequest>,
    ) -> Result<(StatusCode, Json<UserDto>), Response> {
        // Phase 4.9+1: ユーザー更新ロジック
        Err(error_to_response(ApplicationError::NotFound(format!(
            "User {} not found",
            user_id
        ))))
    }

    /// ユーザー削除
    pub async fn delete_user(Path(_user_id): Path<Uuid>) -> Result<StatusCode, Response> {
        // Phase 4.9+1: ユーザー削除ロジック
        Err(error_to_response(ApplicationError::NotFound(
            "User not found".to_string(),
        )))
    }

    // ========================================================================
    // Post Handlers (Phase 4.2)
    // ========================================================================

    /// 投稿作成
    pub async fn create_post(
        Json(request): Json<CreatePostRequest>,
    ) -> Result<(StatusCode, Json<PostDto>), Response> {
        // Phase 4.9+1: Application層と接続
        let post = PostDto {
            id: Uuid::new_v4(),
            title: request.title,
            slug: "placeholder".to_string(), // TODO: slug生成ロジック
            content: request.content,
            author_id: request.author_id,
            is_published: false,
            created_at: chrono::Utc::now(),
        };
        Ok((StatusCode::CREATED, Json(post)))
    }

    /// 投稿取得
    pub async fn get_post(
        Path(_post_id): Path<Uuid>,
    ) -> Result<(StatusCode, Json<PostDto>), Response> {
        Err(error_to_response(ApplicationError::NotFound(
            "Post not found".to_string(),
        )))
    }

    /// 投稿更新
    pub async fn update_post(
        Path(_post_id): Path<Uuid>,
        Json(_request): Json<UpdatePostRequest>,
    ) -> Result<(StatusCode, Json<PostDto>), Response> {
        Err(error_to_response(ApplicationError::NotFound(
            "Post not found".to_string(),
        )))
    }

    // ========================================================================
    // Comment Handlers (Phase 4.3)
    // ========================================================================

    /// コメント作成
    pub async fn create_comment(
        Path(_post_id): Path<Uuid>,
        Json(_request): Json<serde_json::Value>,
    ) -> Result<(StatusCode, Json<CommentDto>), Response> {
        Err(error_to_response(ApplicationError::NotFound(
            "Post not found".to_string(),
        )))
    }

    /// コメント一覧取得
    pub async fn list_comments(
        Path(_post_id): Path<Uuid>,
    ) -> Result<(StatusCode, Json<Vec<CommentDto>>), Response> {
        Err(error_to_response(ApplicationError::NotFound(
            "Post not found".to_string(),
        )))
    }

    // ========================================================================
    // Tag Handlers (Phase 4.4)
    // ========================================================================

    /// タグ作成
    pub async fn create_tag(
        Json(_request): Json<serde_json::Value>,
    ) -> Result<(StatusCode, Json<TagDto>), Response> {
        Err(error_to_response(ApplicationError::Unknown(
            "Not implemented".to_string(),
        )))
    }

    /// タグ取得
    pub async fn get_tag(
        Path(_tag_id): Path<Uuid>,
    ) -> Result<(StatusCode, Json<TagDto>), Response> {
        Err(error_to_response(ApplicationError::NotFound(
            "Tag not found".to_string(),
        )))
    }

    // ========================================================================
    // Category Handlers (Phase 4.5)
    // ========================================================================

    /// カテゴリ作成
    pub async fn create_category(
        Json(_request): Json<serde_json::Value>,
    ) -> Result<(StatusCode, Json<CategoryDto>), Response> {
        Err(error_to_response(ApplicationError::Unknown(
            "Not implemented".to_string(),
        )))
    }

    /// カテゴリ取得
    pub async fn get_category(
        Path(_category_id): Path<Uuid>,
    ) -> Result<(StatusCode, Json<CategoryDto>), Response> {
        Err(error_to_response(ApplicationError::NotFound(
            "Category not found".to_string(),
        )))
    }
}

#[cfg(all(
    feature = "restructure_presentation",
    feature = "restructure_application"
))]
pub use handlers::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(
        feature = "restructure_presentation",
        feature = "restructure_application"
    ))]
    fn test_error_to_response() {
        use crate::common::types::ApplicationError;
        let error = ApplicationError::NotFound("Test".to_string());
        let response = handlers::error_to_response(error);
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
