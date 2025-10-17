// src/application/use_cases/comment/create_comment.rs
//! Create Comment Use Case
//!
//! コメント作成のユースケース実装。
//! ドメインロジック（Comment::new）とリポジトリ操作を統合します。

use crate::application::dto::comment::{CommentDto, CreateCommentRequest};
use crate::application::ports::repositories::{CommentRepository, PostRepository};
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::comment::{Comment, CommentText};
use crate::domain::post::PostId;
use crate::domain::user::UserId;
use crate::infrastructure::events::bus::AppEvent;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

/// コメント作成ユースケース
///
/// # 責務
/// - UUIDのパースと検証
/// - 投稿の存在確認
/// - コメントエンティティの作成（Domain層）
/// - リポジトリへの永続化
/// - AppEvent::CommentCreated イベントの発行
pub struct CreateCommentUseCase<C, P>
where
    C: CommentRepository,
    P: PostRepository,
{
    comment_repository: Arc<C>,
    post_repository: Arc<P>,
    event_bus: Sender<AppEvent>,
}

impl<C, P> CreateCommentUseCase<C, P>
where
    C: CommentRepository,
    P: PostRepository,
{
    /// 新しいコメント作成ユースケースを構築
    pub fn new(
        comment_repository: Arc<C>,
        post_repository: Arc<P>,
        event_bus: Sender<AppEvent>,
    ) -> Self {
        Self {
            comment_repository,
            post_repository,
            event_bus,
        }
    }

    /// コメント作成を実行
    ///
    /// # Arguments
    ///
    /// * `author_id_str` - 著者ユーザーID（UUID文字列）
    /// * `request` - コメント作成リクエスト（post_id + text）
    ///
    /// # Returns
    ///
    /// * `ApplicationResult<CommentDto>` - 作成されたコメントのDTO、またはエラー
    ///
    /// # Errors
    ///
    /// - `ApplicationError::InvalidUuid` - UUID形式が不正
    /// - `ApplicationError::NotFound` - 投稿が存在しない
    /// - `ApplicationError::ValidationError` - コメントテキストが不正
    /// - `ApplicationError::Repository` - リポジトリ操作エラー
    pub async fn execute(
        &self,
        author_id_str: &str,
        request: CreateCommentRequest,
    ) -> ApplicationResult<CommentDto> {
        // 1. Parse author UUID
        let author_uuid = Uuid::parse_str(author_id_str)
            .map_err(|_| ApplicationError::InvalidUuid(author_id_str.to_string()))?;
        let author_id = UserId::from_uuid(author_uuid);

        // 2. Parse post UUID
        let post_uuid = Uuid::parse_str(&request.post_id)
            .map_err(|_| ApplicationError::InvalidUuid(request.post_id.clone()))?;
        let post_id = PostId::from_uuid(post_uuid);

        // 3. Verify post exists
        let post = self
            .post_repository
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound("Post not found".to_string()))?;

        // 4. Create comment text Value Object
        let text = CommentText::new(request.text)
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        // 5. Create comment entity (Domain logic)
        let comment = Comment::new(post_id, author_id, text)
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        // 6. Save to repository
        self.comment_repository.save(comment.clone()).await?;

        // 7. Publish AppEvent::CommentCreated
        let _ = self.event_bus.send(AppEvent::CommentCreated {
            comment_id: comment.id().to_string(),
            post_id: post.id().to_string(),
            author_id: author_id.to_string(),
            text: comment.text().as_str().to_string(),
        });

        // 8. Return DTO
        Ok(CommentDto::from(comment))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::RepositoryError;
    use crate::domain::comment::CommentId;
    use crate::domain::post::{Content, Post, PostStatus, Slug, Title};
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use tokio::sync::broadcast;

    // Mock implementations
    mock! {
        pub CommentRepo {}

        #[async_trait]
        impl CommentRepository for CommentRepo {
            async fn save(&self, comment: Comment) -> Result<(), RepositoryError>;
            async fn find_by_id(&self, id: CommentId) -> Result<Option<Comment>, RepositoryError>;
            async fn find_by_post(
                &self,
                post_id: PostId,
                limit: i64,
                offset: i64,
            ) -> Result<Vec<Comment>, RepositoryError>;
            async fn find_by_author(
                &self,
                author_id: UserId,
                limit: i64,
                offset: i64,
            ) -> Result<Vec<Comment>, RepositoryError>;
            async fn delete(&self, id: CommentId) -> Result<(), RepositoryError>;
            async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError>;
        }
    }

    mock! {
        pub PostRepo {}

        #[async_trait]
        impl PostRepository for PostRepo {
            async fn save(&self, post: Post) -> Result<(), RepositoryError>;
            async fn find_by_id(&self, id: PostId) -> Result<Option<Post>, RepositoryError>;
            async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, RepositoryError>;
            async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError>;
            async fn find_by_author(
                &self,
                author_id: UserId,
                limit: i64,
                offset: i64,
            ) -> Result<Vec<Post>, RepositoryError>;
            async fn delete(&self, id: PostId) -> Result<(), RepositoryError>;
        }
    }

    #[tokio::test]
    async fn test_create_comment_success() {
        // Arrange
        let author_id = UserId::new();
        let post_id = PostId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("Test content".to_string()).unwrap();
        let post = Post::new(author_id, title, slug, content);

        let mut mock_comment_repo = MockCommentRepo::new();
        mock_comment_repo
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_post_repo = MockPostRepo::new();
        let post_clone = post.clone();
        mock_post_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = CreateCommentUseCase::new(
            Arc::new(mock_comment_repo),
            Arc::new(mock_post_repo),
            event_bus,
        );

        let request = CreateCommentRequest {
            post_id: post_id.to_string(),
            text: "Great post!".to_string(),
        };

        // Act
        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.post_id, post_id.to_string());
        assert_eq!(dto.author_id, author_id.to_string());
        assert_eq!(dto.text, "Great post!");
        assert_eq!(dto.status, "Pending");
    }

    #[tokio::test]
    async fn test_create_comment_post_not_found() {
        // Arrange
        let author_id = UserId::new();
        let post_id = PostId::new();

        let mock_comment_repo = MockCommentRepo::new();

        let mut mock_post_repo = MockPostRepo::new();
        mock_post_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(|_| Ok(None));

        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = CreateCommentUseCase::new(
            Arc::new(mock_comment_repo),
            Arc::new(mock_post_repo),
            event_bus,
        );

        let request = CreateCommentRequest {
            post_id: post_id.to_string(),
            text: "Great post!".to_string(),
        };

        // Act
        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApplicationError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_create_comment_invalid_author_uuid() {
        // Arrange
        let mock_comment_repo = MockCommentRepo::new();
        let mock_post_repo = MockPostRepo::new();
        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = CreateCommentUseCase::new(
            Arc::new(mock_comment_repo),
            Arc::new(mock_post_repo),
            event_bus,
        );

        let request = CreateCommentRequest {
            post_id: PostId::new().to_string(),
            text: "Great post!".to_string(),
        };

        // Act
        let result = use_case.execute("invalid-uuid", request).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::InvalidUuid(_)
        ));
    }

    #[tokio::test]
    async fn test_create_comment_invalid_post_uuid() {
        // Arrange
        let author_id = UserId::new();
        let mock_comment_repo = MockCommentRepo::new();
        let mock_post_repo = MockPostRepo::new();
        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = CreateCommentUseCase::new(
            Arc::new(mock_comment_repo),
            Arc::new(mock_post_repo),
            event_bus,
        );

        let request = CreateCommentRequest {
            post_id: "invalid-uuid".to_string(),
            text: "Great post!".to_string(),
        };

        // Act
        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::InvalidUuid(_)
        ));
    }

    #[tokio::test]
    async fn test_create_comment_empty_text() {
        // Arrange
        let author_id = UserId::new();
        let post_id = PostId::new();

        let mock_comment_repo = MockCommentRepo::new();

        let mut mock_post_repo = MockPostRepo::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("Test content".to_string()).unwrap();
        let post = Post::new(author_id, title, slug, content);
        let post_clone = post.clone();
        mock_post_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = CreateCommentUseCase::new(
            Arc::new(mock_comment_repo),
            Arc::new(mock_post_repo),
            event_bus,
        );

        let request = CreateCommentRequest {
            post_id: post_id.to_string(),
            text: "".to_string(), // Empty text
        };

        // Act
        let result = use_case.execute(&author_id.to_string(), request).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::ValidationError(_)
        ));
    }
}
