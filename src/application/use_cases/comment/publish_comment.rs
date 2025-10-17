// src/application/use_cases/comment/publish_comment.rs
//! Publish Comment Use Case
//!
//! コメント公開のユースケース実装。
//! Pending状態のコメントをPublished状態に遷移させます。

use crate::application::dto::comment::CommentDto;
use crate::application::ports::repositories::CommentRepository;
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::comment::CommentId;
use crate::infrastructure::events::bus::AppEvent;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

/// コメント公開ユースケース
///
/// # 責務
/// - UUIDのパースと検証
/// - コメントの取得
/// - ドメインメソッド（publish）の呼び出し
/// - リポジトリへの永続化
/// - AppEvent::CommentPublished イベントの発行
pub struct PublishCommentUseCase<C>
where
    C: CommentRepository,
{
    comment_repository: Arc<C>,
    event_bus: Sender<AppEvent>,
}

impl<C> PublishCommentUseCase<C>
where
    C: CommentRepository,
{
    /// 新しいコメント公開ユースケースを構築
    pub fn new(comment_repository: Arc<C>, event_bus: Sender<AppEvent>) -> Self {
        Self {
            comment_repository,
            event_bus,
        }
    }

    /// コメント公開を実行
    ///
    /// # Arguments
    ///
    /// * `comment_id_str` - コメントID（UUID文字列）
    ///
    /// # Returns
    ///
    /// * `ApplicationResult<CommentDto>` - 公開されたコメントのDTO、またはエラー
    ///
    /// # Errors
    ///
    /// - `ApplicationError::InvalidUuid` - UUID形式が不正
    /// - `ApplicationError::NotFound` - コメントが存在しない
    /// - `ApplicationError::ValidationError` - 状態遷移が不正（既に公開済み等）
    /// - `ApplicationError::Repository` - リポジトリ操作エラー
    pub async fn execute(&self, comment_id_str: &str) -> ApplicationResult<CommentDto> {
        // 1. Parse comment UUID
        let comment_uuid = Uuid::parse_str(comment_id_str)
            .map_err(|_| ApplicationError::InvalidUuid(comment_id_str.to_string()))?;
        let comment_id = CommentId::from_uuid(comment_uuid);

        // 2. Retrieve comment
        let mut comment = self
            .comment_repository
            .find_by_id(comment_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound("Comment not found".to_string()))?;

        // 3. Domain publish method (validates state)
        comment
            .publish()
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        // 4. Save to repository
        self.comment_repository.save(comment.clone()).await?;

        // 5. Publish AppEvent::CommentPublished
        let _ = self.event_bus.send(AppEvent::CommentPublished {
            comment_id: comment.id().to_string(),
            post_id: comment.post_id().to_string(),
            author_id: comment.author_id().to_string(),
        });

        // 6. Return DTO
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
    use crate::domain::comment::{Comment, CommentStatus, CommentText};
    use crate::domain::post::PostId;
    use crate::domain::user::UserId;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;
    use tokio::sync::broadcast;

    // Mock implementation
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

    #[tokio::test]
    async fn test_publish_comment_success() {
        // Arrange
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();
        let comment = Comment::new(post_id, author_id, text).unwrap();
        let comment_id = comment.id();

        let mut mock_repo = MockCommentRepo::new();

        // find_by_id expectation
        let comment_clone_find = comment.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(comment_id))
            .times(1)
            .returning(move |_| Ok(Some(comment_clone_find.clone())));

        // save expectation
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = PublishCommentUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&comment_id.to_string()).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.id, comment_id.to_string());
        assert_eq!(dto.status, "Published");
    }

    #[tokio::test]
    async fn test_publish_comment_not_found() {
        // Arrange
        let comment_id = CommentId::new();

        let mut mock_repo = MockCommentRepo::new();
        mock_repo
            .expect_find_by_id()
            .with(eq(comment_id))
            .times(1)
            .returning(|_| Ok(None));

        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = PublishCommentUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&comment_id.to_string()).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApplicationError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_publish_comment_already_published() {
        // Arrange
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();
        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        let comment_id = comment.id();

        // Publish comment first
        comment.publish().unwrap();

        let mut mock_repo = MockCommentRepo::new();
        let comment_clone = comment.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(comment_id))
            .times(1)
            .returning(move |_| Ok(Some(comment_clone.clone())));

        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = PublishCommentUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&comment_id.to_string()).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::ValidationError(_)
        ));
    }

    #[tokio::test]
    async fn test_publish_comment_invalid_uuid() {
        // Arrange
        let mock_repo = MockCommentRepo::new();
        let (event_bus, _rx) = broadcast::channel(16);

        let use_case = PublishCommentUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute("invalid-uuid").await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::InvalidUuid(_)
        ));
    }
}
