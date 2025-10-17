//! 投稿アーカイブユースケース（ArchivePostUseCase）
//!
//! # 責務
//! - 投稿を Archived 状態に遷移させる
//! - Repository を使って永続化
//! - AppEvent::PostArchived イベントを発行
//!
//! # ビジネスルール
//! - 既にアーカイブ済みの投稿はエラー（Domain層で検証）
//! - Draft, Published いずれの状態からもアーカイブ可能
//!
//! # 使用例
//! ```ignore
//! let use_case = ArchivePostUseCase::new(post_repo, event_bus);
//! let dto = use_case.execute("post-uuid").await?;
//! ```

use crate::application::dto::post::PostDto;
use crate::application::ports::repositories::PostRepository;
use crate::common::error_types::{ApplicationError, ApplicationResult};
use crate::domain::post::PostId;
use crate::infrastructure::events::bus::{AppEvent, PostEventData};
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

/// 投稿アーカイブユースケース
#[derive(Clone)]
pub struct ArchivePostUseCase {
    post_repository: Arc<dyn PostRepository>,
    event_bus: Sender<AppEvent>,
}

impl ArchivePostUseCase {
    /// 新しい ArchivePostUseCase を作成
    pub fn new(post_repository: Arc<dyn PostRepository>, event_bus: Sender<AppEvent>) -> Self {
        Self {
            post_repository,
            event_bus,
        }
    }

    /// 投稿をアーカイブする
    ///
    /// # Arguments
    /// * `post_id_str` - アーカイブする投稿のUUID文字列
    ///
    /// # Returns
    /// アーカイブされた投稿の DTO
    ///
    /// # Errors
    /// - 投稿が見つからない場合
    /// - UUID が無効な形式の場合
    /// - 投稿が既にアーカイブ済みの場合（Domain層エラー）
    pub async fn execute(&self, post_id_str: &str) -> ApplicationResult<PostDto> {
        // 1. Parse post UUID
        let post_uuid = Uuid::parse_str(post_id_str)
            .map_err(|_| ApplicationError::ValidationError("Invalid UUID format".to_string()))?;
        let post_id = PostId::from_uuid(post_uuid);

        // 2. Repository から投稿取得
        let mut post = self
            .post_repository
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound("Post not found".to_string()))?;

        // 3. Domain Entity の archive メソッド呼び出し（状態検証含む）
        post.archive()
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        // 4. Repository に保存（永続化）
        self.post_repository.save(post.clone()).await?;

        // 5. Domain → DTO 変換
        let post_dto = PostDto::from(post.clone());

        // 6. PostArchived イベント発行
        let event = AppEvent::PostArchived(PostEventData {
            id: *post.id().as_uuid(),
            author_id: *post.author_id().as_uuid(),
            title: post.title().as_str().to_string(),
            slug: post.slug().as_str().to_string(),
            published: false,
        });

        // Fire-and-forget（エラーは無視）
        let _ = self.event_bus.send(event);

        Ok(post_dto)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockPostRepository;
    use crate::domain::post::{Content, Post, Slug, Title};
    use crate::domain::user::UserId;
    use mockall::predicate::*;

    fn create_test_post() -> Post {
        let author_id = UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is a test post content with enough length".to_string())
            .unwrap();

        Post::new(author_id, title, slug, content)
    }

    #[tokio::test]
    async fn test_archive_post_success() {
        // Arrange
        let mut post = create_test_post();
        // 公開しておく（Draft からもアーカイブ可能だが、Published からのケースをテスト）
        post.publish().unwrap();

        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が post を返す
        let post_clone = post.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        // save が成功する
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = ArchivePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.id, post_id_str);
        assert_eq!(dto.status, "archived");
    }

    #[tokio::test]
    async fn test_archive_post_from_draft() {
        // Arrange - Draft 状態からのアーカイブ
        let post = create_test_post();
        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が draft post を返す
        let post_clone = post.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        // save が成功する
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = ArchivePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.status, "archived");
    }

    #[tokio::test]
    async fn test_archive_post_not_found() {
        // Arrange
        let post_id_str = Uuid::new_v4().to_string();
        let post_id = PostId::from_uuid(Uuid::parse_str(&post_id_str).unwrap());

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が None を返す
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(|_| Ok(None));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = ArchivePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str).await;

        // Assert
        assert!(result.is_err());
        matches!(result.unwrap_err(), ApplicationError::NotFound(_));
    }

    #[tokio::test]
    async fn test_archive_post_already_archived() {
        // Arrange
        let mut post = create_test_post();
        // 先にアーカイブしておく
        post.archive().unwrap();

        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が既にアーカイブ済みの post を返す
        let post_clone = post.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = ArchivePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str).await;

        // Assert
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            ApplicationError::ValidationError(_)
        );
    }

    #[tokio::test]
    async fn test_archive_post_invalid_uuid() {
        // Arrange
        let mut mock_repo = MockPostRepository::new();
        mock_repo.expect_find_by_id().times(0); // 呼ばれないはず

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = ArchivePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute("invalid-uuid").await;

        // Assert
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            ApplicationError::ValidationError(_)
        );
    }
}
