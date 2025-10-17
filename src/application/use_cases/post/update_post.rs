//! 投稿更新ユースケース（UpdatePostUseCase）
//!
//! # 責務
//! - 投稿のタイトル、コンテンツ、スラッグを更新
//! - Repository を使って永続化
//! - AppEvent::PostUpdated イベントを発行
//!
//! # ビジネスルール
//! - スラッグ変更時は重複チェックを実施
//! - 少なくとも1つのフィールドが指定される必要がある
//!
//! # 使用例
//! ```ignore
//! let request = UpdatePostRequest {
//!     title: Some("New Title".to_string()),
//!     content: None,
//!     slug: None,
//! };
//! let use_case = UpdatePostUseCase::new(post_repo, event_bus);
//! let dto = use_case.execute("post-uuid", request).await?;
//! ```

use crate::application::dto::post::{PostDto, UpdatePostRequest};
use crate::application::ports::repositories::PostRepository;
use crate::common::error_types::{ApplicationError, ApplicationResult};
use crate::domain::post::{Content, PostId, Slug, Title};
use crate::infrastructure::events::bus::{AppEvent, PostEventData};
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

/// 投稿更新ユースケース
#[derive(Clone)]
pub struct UpdatePostUseCase {
    post_repository: Arc<dyn PostRepository>,
    event_bus: Sender<AppEvent>,
}

impl UpdatePostUseCase {
    /// 新しい UpdatePostUseCase を作成
    pub fn new(post_repository: Arc<dyn PostRepository>, event_bus: Sender<AppEvent>) -> Self {
        Self {
            post_repository,
            event_bus,
        }
    }

    /// 投稿を更新する
    ///
    /// # Arguments
    /// * `post_id_str` - 更新する投稿のUUID文字列
    /// * `request` - 更新リクエスト（title, content, slug のいずれか）
    ///
    /// # Returns
    /// 更新された投稿の DTO
    ///
    /// # Errors
    /// - 投稿が見つからない場合
    /// - UUID が無効な形式の場合
    /// - スラッグが既に使用されている場合
    /// - 更新するフィールドが1つも指定されていない場合
    pub async fn execute(
        &self,
        post_id_str: &str,
        request: UpdatePostRequest,
    ) -> ApplicationResult<PostDto> {
        // 1. Parse post UUID
        let post_uuid = Uuid::parse_str(post_id_str)
            .map_err(|_| ApplicationError::ValidationError("Invalid UUID format".to_string()))?;
        let post_id = PostId::from_uuid(post_uuid);

        // 2. 少なくとも1つのフィールドが指定されているか確認
        if request.title.is_none() && request.content.is_none() && request.slug.is_none() {
            return Err(ApplicationError::ValidationError(
                "At least one field must be specified".to_string(),
            ));
        }

        // 3. Repository から投稿取得
        let mut post = self
            .post_repository
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound("Post not found".to_string()))?;

        // 4. タイトル更新（指定された場合）
        if let Some(title_str) = request.title {
            let title = Title::new(title_str)
                .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;
            post.change_title(title);
        }

        // 5. コンテンツ更新（指定された場合）
        if let Some(content_str) = request.content {
            let content = Content::new(content_str)
                .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;
            post.change_content(content);
        }

        // 6. スラッグ更新（指定された場合）
        if let Some(slug_str) = request.slug {
            let new_slug = Slug::new(slug_str)
                .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

            // スラッグが変更される場合のみ重複チェック
            if new_slug != *post.slug() {
                if let Some(_existing) = self.post_repository.find_by_slug(new_slug.as_str()).await?
                {
                    return Err(ApplicationError::ValidationError(
                        "Slug already exists".to_string(),
                    ));
                }
                post.change_slug(new_slug);
            }
        }

        // 7. Repository に保存（永続化）
        self.post_repository.save(post.clone()).await?;

        // 8. Domain → DTO 変換
        let post_dto = PostDto::from(post.clone());

        // 9. PostUpdated イベント発行
        let event = AppEvent::PostUpdated(PostEventData {
            id: *post.id().as_uuid(),
            author_id: *post.author_id().as_uuid(),
            title: post.title().as_str().to_string(),
            slug: post.slug().as_str().to_string(),
            published: post.is_published(),
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
    use crate::domain::post::Post;
    use crate::domain::user::UserId;
    use mockall::predicate::*;

    fn create_test_post() -> Post {
        let author_id = UserId::new();
        let title = Title::new("Original Title".to_string()).unwrap();
        let slug = Slug::new("original-slug".to_string()).unwrap();
        let content =
            Content::new("This is the original content with sufficient length".to_string())
                .unwrap();

        Post::new(author_id, title, slug, content)
    }

    #[tokio::test]
    async fn test_update_post_title_only() {
        // Arrange
        let post = create_test_post();
        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let request = UpdatePostRequest {
            title: Some("Updated Title".to_string()),
            content: None,
            slug: None,
        };

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
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_update_post_content_only() {
        // Arrange
        let post = create_test_post();
        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let request = UpdatePostRequest {
            title: None,
            content: Some("This is the updated content with sufficient length".to_string()),
            slug: None,
        };

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
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_post_slug_with_duplicate_check() {
        // Arrange
        let post = create_test_post();
        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let request = UpdatePostRequest {
            title: None,
            content: None,
            slug: Some("new-unique-slug".to_string()),
        };

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が post を返す
        let post_clone = post.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        // find_by_slug が None を返す（重複なし）
        mock_repo
            .expect_find_by_slug()
            .with(eq("new-unique-slug"))
            .times(1)
            .returning(|_| Ok(None));

        // save が成功する
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.slug, "new-unique-slug");
    }

    #[tokio::test]
    async fn test_update_post_slug_duplicate() {
        // Arrange
        let post = create_test_post();
        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let request = UpdatePostRequest {
            title: None,
            content: None,
            slug: Some("duplicate-slug".to_string()),
        };

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が post を返す
        let post_clone = post.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        // find_by_slug が既存の投稿を返す（重複あり）
        let existing_post = create_test_post();
        mock_repo
            .expect_find_by_slug()
            .with(eq("duplicate-slug"))
            .times(1)
            .returning(move |_| Ok(Some(existing_post.clone())));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            ApplicationError::ValidationError(_)
        );
    }

    #[tokio::test]
    async fn test_update_post_all_fields() {
        // Arrange
        let post = create_test_post();
        let post_id = post.id();
        let post_id_str = post_id.to_string();

        let request = UpdatePostRequest {
            title: Some("Completely Updated Title".to_string()),
            content: Some("Completely updated content with sufficient length".to_string()),
            slug: Some("completely-updated-slug".to_string()),
        };

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が post を返す
        let post_clone = post.clone();
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(move |_| Ok(Some(post_clone.clone())));

        // find_by_slug が None を返す（重複なし）
        mock_repo
            .expect_find_by_slug()
            .with(eq("completely-updated-slug"))
            .times(1)
            .returning(|_| Ok(None));

        // save が成功する
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.title, "Completely Updated Title");
        assert_eq!(dto.slug, "completely-updated-slug");
    }

    #[tokio::test]
    async fn test_update_post_no_fields_specified() {
        // Arrange
        let post_id_str = Uuid::new_v4().to_string();

        let request = UpdatePostRequest {
            title: None,
            content: None,
            slug: None,
        };

        let mock_repo = MockPostRepository::new();
        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            ApplicationError::ValidationError(_)
        );
    }

    #[tokio::test]
    async fn test_update_post_not_found() {
        // Arrange
        let post_id_str = Uuid::new_v4().to_string();
        let post_id = PostId::from_uuid(Uuid::parse_str(&post_id_str).unwrap());

        let request = UpdatePostRequest {
            title: Some("New Title".to_string()),
            content: None,
            slug: None,
        };

        let mut mock_repo = MockPostRepository::new();

        // find_by_id が None を返す
        mock_repo
            .expect_find_by_id()
            .with(eq(post_id))
            .times(1)
            .returning(|_| Ok(None));

        let (event_bus, _rx) = tokio::sync::broadcast::channel(10);
        let use_case = UpdatePostUseCase::new(Arc::new(mock_repo), event_bus);

        // Act
        let result = use_case.execute(&post_id_str, request).await;

        // Assert
        assert!(result.is_err());
        matches!(result.unwrap_err(), ApplicationError::NotFound(_));
    }
}
