// src/application/use_cases/post/create_post.rs
//! CreatePostUseCase - 投稿作成 Use Case
//!
//! Phase 3 Week 8-9: Use Case 実装

use crate::application::dto::post::{CreatePostRequest, PostDto};
use crate::application::ports::repositories::PostRepository;
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::post::{Content, Post, Slug, Title};
use crate::domain::user::UserId;
use crate::infrastructure::events::bus::{AppEvent, PostEventData};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// 投稿作成 Use Case
///
/// # 責務
/// - CreatePostRequest の検証
/// - Domain Entity の生成
/// - Repository への保存
/// - Domain Event の発行
/// - PostDto の返却
pub struct CreatePostUseCase {
    post_repository: Arc<dyn PostRepository>,
    event_bus: broadcast::Sender<AppEvent>,
}

impl CreatePostUseCase {
    /// Use Case の生成
    pub fn new(
        post_repository: Arc<dyn PostRepository>,
        event_bus: broadcast::Sender<AppEvent>,
    ) -> Self {
        Self {
            post_repository,
            event_bus,
        }
    }

    /// 投稿作成を実行
    ///
    /// # Arguments
    /// - `author_id_str` - 著者のユーザーID
    /// - `request` - 投稿作成リクエスト
    ///
    /// # Returns
    /// - `Ok(PostDto)` - 作成された投稿
    /// - `Err(ApplicationError)` - 検証エラーまたは保存エラー
    ///
    /// # Errors
    /// - `ValidationError` - リクエストの検証失敗
    /// - `DomainError` - Domain層でのビジネスルール違反
    /// - `RepositoryError` - データベース保存失敗
    pub async fn execute(
        &self,
        author_id_str: &str,
        request: CreatePostRequest,
    ) -> ApplicationResult<PostDto> {
        // 1. Parse author UUID
        let author_uuid = Uuid::parse_str(author_id_str)
            .map_err(|_| ApplicationError::ValidationError("Invalid UUID format".to_string()))?;
        let author_id = UserId::from_uuid(author_uuid);

        // 2. Request → Domain Value Objects 変換
        let title = Title::new(request.title)
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        let content = Content::new(request.content)
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        let slug = if let Some(slug_str) = request.slug {
            Slug::new(slug_str).map_err(|e| ApplicationError::ValidationError(e.to_string()))?
        } else {
            // スラッグが指定されていない場合、タイトルから自動生成
            Slug::from_title(&title)
                .map_err(|e| ApplicationError::ValidationError(e.to_string()))?
        };

        // 3. スラッグ重複チェック（Repository）
        if let Some(_existing) = self.post_repository.find_by_slug(slug.as_str()).await? {
            return Err(ApplicationError::ValidationError(
                "Slug already exists".to_string(),
            ));
        }

        // 4. Domain Entity 生成
        let post = Post::new(author_id, title, slug, content);

        // 5. Domain → DTO 変換（save前にクローン）
        let post_dto = PostDto::from(post.clone());

        // 6. Repository への保存（所有権を渡す）
        self.post_repository.save(post).await?;

        // 7. AppEvent 発行（Fire-and-Forget）
        let event_data = PostEventData {
            id: Uuid::parse_str(&post_dto.id).unwrap_or_default(),
            title: post_dto.title.clone(),
            slug: post_dto.slug.clone(),
            author_id: Uuid::parse_str(&post_dto.author_id).unwrap_or_default(),
            published: post_dto.published_at.is_some(),
        };
        let _ = self.event_bus.send(AppEvent::PostCreated(event_data));

        // 8. DTO を返却
        Ok(post_dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockPostRepository;

    #[tokio::test]
    async fn test_create_post_success() {
        // Arrange: Mock Repository
        let mut mock_repo = MockPostRepository::new();

        // スラッグ重複チェック: なし
        mock_repo
            .expect_find_by_slug()
            .times(1)
            .returning(|_| Ok(None));

        // 保存成功
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = CreatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 投稿作成実行
        let author_id = UserId::new();
        let request = CreatePostRequest {
            title: "Test Post".to_string(),
            content: "This is a test post content.".to_string(),
            slug: Some("test-post".to_string()),
        };

        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert: 成功
        assert!(result.is_ok());
        let post_dto = result.unwrap();
        assert_eq!(post_dto.title, "Test Post");
        assert_eq!(post_dto.slug, "test-post");
        assert!(post_dto.published_at.is_none()); // 下書き状態
    }

    #[tokio::test]
    async fn test_create_post_slug_already_exists() {
        // Arrange: Mock Repository
        let mut mock_repo = MockPostRepository::new();

        // スラッグ重複チェック: 既存投稿あり
        let existing_title = Title::new("Existing Post".to_string()).unwrap();
        let existing_content = Content::new("Existing content".to_string()).unwrap();
        let existing_slug = Slug::new("test-post".to_string()).unwrap();
        let existing_author = UserId::new();
        let existing_post = Post::new(existing_author, existing_title, existing_slug, existing_content);

        mock_repo
            .expect_find_by_slug()
            .times(1)
            .returning(move |_| Ok(Some(existing_post.clone())));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = CreatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 投稿作成実行
        let author_id = UserId::new();
        let request = CreatePostRequest {
            title: "Test Post".to_string(),
            content: "This is a test post content.".to_string(),
            slug: Some("test-post".to_string()),
        };

        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert: ValidationError
        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("Slug already exists"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }

    #[tokio::test]
    async fn test_create_post_auto_generate_slug() {
        // Arrange: Mock Repository
        let mut mock_repo = MockPostRepository::new();

        // スラッグ重複チェック: なし
        mock_repo
            .expect_find_by_slug()
            .times(1)
            .returning(|_| Ok(None));

        // 保存成功
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = CreatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: スラッグなしで投稿作成実行
        let author_id = UserId::new();
        let request = CreatePostRequest {
            title: "My Awesome Post Title".to_string(),
            content: "This is a test post content.".to_string(),
            slug: None, // スラッグ未指定
        };

        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert: 成功、スラッグが自動生成されている
        assert!(result.is_ok());
        let post_dto = result.unwrap();
        assert_eq!(post_dto.title, "My Awesome Post Title");
        assert_eq!(post_dto.slug, "my-awesome-post-title"); // タイトルから生成
    }

    #[tokio::test]
    async fn test_create_post_invalid_author_id() {
        // Arrange: Mock Repository（呼び出されない）
        let mock_repo = MockPostRepository::new();

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = CreatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 不正なUUIDで投稿作成実行
        let request = CreatePostRequest {
            title: "Test Post".to_string(),
            content: "This is a test post content.".to_string(),
            slug: Some("test-post".to_string()),
        };

        let result = use_case.execute("invalid-uuid", request).await;

        // Assert: ValidationError
        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("Invalid UUID format"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }
}
