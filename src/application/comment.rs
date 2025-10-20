//! Comment Application Layer - CQRS統合
//!
//! Commands + Queries + DTOs を単一ファイルに統合

use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::comment::{Comment, CommentId, CommentText};
#[cfg(feature = "restructure_domain")]
use crate::domain::post::PostId;
#[cfg(feature = "restructure_domain")]
use crate::domain::user::UserId;

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{CommentRepository, RepositoryError};

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentRequest {
    pub post_id: String,
    pub author_id: String,
    pub content: String,
}

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

#[cfg(feature = "restructure_domain")]
pub struct CreateComment<'a> {
    repo: &'a dyn CommentRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> CreateComment<'a> {
    pub const fn new(repo: &'a dyn CommentRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        request: CreateCommentRequest,
    ) -> Result<CommentDto, RepositoryError> {
        let post_id = PostId::from_string(&request.post_id)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let author_id = UserId::from_string(&request.author_id)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let text = CommentText::new(request.content)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let comment = Comment::new(post_id, author_id, text)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(comment.clone()).await?;

        Ok(CommentDto::from(comment))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct PublishComment<'a> {
    repo: &'a dyn CommentRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> PublishComment<'a> {
    pub const fn new(repo: &'a dyn CommentRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, comment_id: CommentId) -> Result<CommentDto, RepositoryError> {
        let mut comment = self
            .repo
            .find_by_id(comment_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Comment {comment_id}")))?;

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

#[cfg(feature = "restructure_domain")]
pub struct GetCommentById<'a> {
    repo: &'a dyn CommentRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> GetCommentById<'a> {
    pub const fn new(repo: &'a dyn CommentRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        comment_id: CommentId,
    ) -> Result<Option<CommentDto>, RepositoryError> {
        Ok(self
            .repo
            .find_by_id(comment_id)
            .await?
            .map(CommentDto::from))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct ListCommentsByPost<'a> {
    repo: &'a dyn CommentRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> ListCommentsByPost<'a> {
    pub const fn new(repo: &'a dyn CommentRepository) -> Self {
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
    use crate::application::ports::repositories::MockCommentRepository;

    #[tokio::test]
    async fn test_create_comment_success() {
        let mut mock_repo = MockCommentRepository::new();
        mock_repo.expect_save().returning(|_| Ok(()));

        let use_case = CreateComment::new(&mock_repo);

        let request = CreateCommentRequest {
            post_id: PostId::new().to_string(),
            author_id: UserId::new().to_string(),
            content: "Test comment".into(),
        };

        let result = use_case.execute(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Test comment");
    }

    #[tokio::test]
    async fn test_create_comment_empty_content() {
        let mock_repo = MockCommentRepository::new();
        let use_case = CreateComment::new(&mock_repo);

        let request = CreateCommentRequest {
            post_id: PostId::new().to_string(),
            author_id: UserId::new().to_string(),
            content: String::new(),
        };

        let result = use_case.execute(request).await;
        assert!(matches!(result, Err(RepositoryError::ValidationError(_))));
    }
}
