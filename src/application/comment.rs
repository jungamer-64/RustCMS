//! Comment Application Layer - CQRS統合
//!
//! Commands + Queries + DTOs を単一ファイルに統合（監査推奨パターン）

use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::comment::{Comment, CommentId, CommentStatus, CommentText};
#[cfg(feature = "restructure_domain")]
use crate::domain::post::PostId;
#[cfg(feature = "restructure_domain")]
use crate::domain::user::UserId;

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{CommentRepository, RepositoryError};

// ============================================================================
// DTOs
// ============================================================================

/// コメント作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentRequest {
    pub post_id: String, // UUID文字列
    pub author_id: String,
    pub content: String,
    // Note: parent_id removed - not supported in current Comment domain model
}

/// コメントレスポンス
#[derive(Debug, Clone, Serialize)]
pub struct CommentDto {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(feature = "restructure_domain")]
impl From<Comment> for CommentDto {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id().to_string(),
            post_id: comment.post_id().to_string(),
            author_id: comment.author_id().to_string(),
            content: comment.text().to_string(),
            status: format!("{:?}", comment.status()).to_lowercase(),
            created_at: comment.created_at().to_rfc3339(),
            updated_at: comment.updated_at().to_rfc3339(),
        }
    }
}

// ============================================================================
// Commands
// ============================================================================

/// コメント作成コマンド
#[cfg(feature = "restructure_domain")]
pub struct CreateComment {
    repo: Arc<dyn CommentRepository>,
}

#[cfg(feature = "restructure_domain")]
impl CreateComment {
    pub fn new(repo: Arc<dyn CommentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        request: CreateCommentRequest,
    ) -> Result<CommentDto, RepositoryError> {
        // 1. IDs 変換
        let post_id = PostId::from_string(&request.post_id)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let author_id = UserId::from_string(&request.author_id)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 2. CommentText 作成（検証済み）
        let text = CommentText::new(request.content)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 3. ドメインエンティティ作成
        let comment = Comment::new(post_id, author_id, text)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 4. 永続化
        self.repo.save(comment.clone()).await?;

        Ok(CommentDto::from(comment))
    }
}

/// コメント公開コマンド
#[cfg(feature = "restructure_domain")]
pub struct PublishComment {
    repo: Arc<dyn CommentRepository>,
}

#[cfg(feature = "restructure_domain")]
impl PublishComment {
    pub fn new(repo: Arc<dyn CommentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, comment_id: CommentId) -> Result<CommentDto, RepositoryError> {
        let mut comment = self
            .repo
            .find_by_id(comment_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Comment {}", comment_id)))?;

        comment
            .publish()
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(comment.clone()).await?;

        Ok(CommentDto::from(comment))
    }
}

// ============================================================================
// Queries
// ============================================================================

/// コメント取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct GetCommentById {
    repo: Arc<dyn CommentRepository>,
}

#[cfg(feature = "restructure_domain")]
impl GetCommentById {
    pub fn new(repo: Arc<dyn CommentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        comment_id: CommentId,
    ) -> Result<Option<CommentDto>, RepositoryError> {
        let comment = self.repo.find_by_id(comment_id).await?;
        Ok(comment.map(CommentDto::from))
    }
}

/// 投稿のコメント一覧取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct ListCommentsByPost {
    repo: Arc<dyn CommentRepository>,
}

#[cfg(feature = "restructure_domain")]
impl ListCommentsByPost {
    pub fn new(repo: Arc<dyn CommentRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<Vec<CommentDto>, RepositoryError> {
        let comments = self.repo.find_by_post(post_id, 100, 0).await?;
        Ok(comments.into_iter().map(CommentDto::from).collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "restructure_domain"))]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::application::ports::repositories::{MockCommentRepository, RepositoryError};

    #[tokio::test]
    async fn test_create_comment_success() {
        let mut mock_repo = MockCommentRepository::new();

        mock_repo
            .expect_save()
            .returning(|_| Ok(()));

        let use_case = CreateComment::new(Arc::new(mock_repo));

        let request = CreateCommentRequest {
            post_id: PostId::new().to_string(),
            author_id: UserId::new().to_string(),
            content: "Test comment".to_string(),
        };

        let result = use_case.execute(request).await;
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert_eq!(dto.content, "Test comment");
    }

    #[tokio::test]
    async fn test_create_comment_empty_content() {
        let mock_repo = MockCommentRepository::new();
        let use_case = CreateComment::new(Arc::new(mock_repo));

        let request = CreateCommentRequest {
            post_id: PostId::new().to_string(),
            author_id: UserId::new().to_string(),
            content: "".to_string(), // 完全に空
        };

        let result = use_case.execute(request).await;
        assert!(matches!(result, Err(RepositoryError::ValidationError(_))));
    }
}
