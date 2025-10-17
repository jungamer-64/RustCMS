/// DieselCommentRepository - CommentRepository trait implementation using Diesel ORM
///
/// This implementation provides PostgreSQL persistence for Comment entities using
/// the established Repository pattern. It follows the same async wrapping strategy
/// as UserRepository and PostRepository.
///
/// **Architecture**:
/// - Async operations wrapped via `tokio::task::spawn_blocking`
/// - UPSERT pattern using `on_conflict().do_update()`
/// - Bidirectional mapping between `DbComment` and `Comment` domain entity
/// - CommentStatus conversion: String ↔ Enum
///
/// **Implementation Notes**:
/// - Connection pool: `Arc<Pool<ConnectionManager<PgConnection>>>`
/// - Error chain: DB Error → RepositoryError → ApplicationError
/// - Value Object validation during DB→Domain conversion
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

use crate::application::ports::repositories::{CommentRepository, RepositoryError};
use crate::common::types::DomainError;
use crate::domain::comment::{Comment, CommentId, CommentStatus, CommentText};
use crate::domain::post::PostId;
use crate::domain::user::UserId;
use crate::infrastructure::database::models::{DbComment, NewDbComment};

/// Diesel-based implementation of CommentRepository
///
/// Uses PostgreSQL connection pool for thread-safe concurrent access.
pub struct DieselCommentRepository {
    pool: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl DieselCommentRepository {
    /// Create new repository instance
    pub fn new(pool: Arc<Pool<ConnectionManager<PgConnection>>>) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Private conversion helpers
    // ========================================================================

    /// Convert database model to domain entity
    ///
    /// # Errors
    /// - `RepositoryError::ConversionError` if Value Object validation fails
    /// - `RepositoryError::ConversionError` if status string is invalid
    fn db_comment_to_domain(db_comment: DbComment) -> Result<Comment, RepositoryError> {
        // Convert IDs (no validation needed - UUIDs are always valid)
        let id = CommentId::from_uuid(db_comment.id);
        let post_id = PostId::from_uuid(db_comment.post_id);
        let author_id = UserId::from_uuid(db_comment.author_id);

        // Validate CommentText
        let text = CommentText::new(db_comment.content).map_err(|e| match e {
            DomainError::InvalidCommentText(msg) => RepositoryError::ConversionError(msg),
            _ => RepositoryError::ConversionError(format!("Unexpected error: {e}")),
        })?;

        // Convert status string to enum
        let status = if db_comment.is_approved {
            CommentStatus::Published
        } else {
            CommentStatus::Pending
        };

        // Restore domain entity using factory method
        Ok(Comment::restore(
            id,
            post_id,
            author_id,
            text,
            status,
            db_comment.created_at,
            None, // edited_at not stored in current schema
            db_comment.updated_at,
        ))
    }

    /// Convert domain entity to database model (for insert/update)
    fn domain_comment_to_new_db(comment: &Comment) -> NewDbComment {
        NewDbComment {
            id: *comment.id().as_uuid(),
            post_id: *comment.post_id().as_uuid(),
            author_id: *comment.author_id().as_uuid(),
            content: comment.text().as_str().to_string(),
            parent_id: None, // TODO: Phase 3 - add threading support
            is_approved: matches!(
                comment.status(),
                CommentStatus::Published | CommentStatus::Edited
            ),
            created_at: comment.created_at(),
            updated_at: comment.updated_at(),
        }
    }
}

#[async_trait]
impl CommentRepository for DieselCommentRepository {
    async fn save(&self, comment: Comment) -> Result<(), RepositoryError> {
        use crate::infrastructure::database::schema::comments;

        let pool = Arc::clone(&self.pool);
        let new_db_comment = Self::domain_comment_to_new_db(&comment);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(format!("Connection error: {e}")))?;

            diesel::insert_into(comments::table)
                .values(&new_db_comment)
                .on_conflict(comments::id)
                .do_update()
                .set((
                    comments::content.eq(&new_db_comment.content),
                    comments::is_approved.eq(new_db_comment.is_approved),
                    comments::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| RepositoryError::DatabaseError(format!("Failed to save comment: {e}")))?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Join error: {e}")))?
    }

    async fn find_by_id(&self, id: CommentId) -> Result<Option<Comment>, RepositoryError> {
        use crate::infrastructure::database::schema::comments;

        let pool = Arc::clone(&self.pool);
        let comment_uuid = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(format!("Connection error: {e}")))?;

            comments::table
                .filter(comments::id.eq(comment_uuid))
                .first::<DbComment>(&mut conn)
                .optional()
                .map_err(|e| RepositoryError::DatabaseError(format!("Failed to find comment: {e}")))?
                .map(Self::db_comment_to_domain)
                .transpose()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Join error: {e}")))?
    }

    async fn find_by_post(
        &self,
        post_id: PostId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Comment>, RepositoryError> {
        use crate::infrastructure::database::schema::comments;

        let pool = Arc::clone(&self.pool);
        let post_uuid = *post_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(format!("Connection error: {e}")))?;

            comments::table
                .filter(comments::post_id.eq(post_uuid))
                .order(comments::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load::<DbComment>(&mut conn)
                .map_err(|e| RepositoryError::DatabaseError(format!("Failed to find comments: {e}")))?
                .into_iter()
                .map(Self::db_comment_to_domain)
                .collect()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Join error: {e}")))?
    }

    async fn find_by_author(
        &self,
        author_id: UserId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Comment>, RepositoryError> {
        use crate::infrastructure::database::schema::comments;

        let pool = Arc::clone(&self.pool);
        let author_uuid = *author_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(format!("Connection error: {e}")))?;

            comments::table
                .filter(comments::author_id.eq(author_uuid))
                .order(comments::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load::<DbComment>(&mut conn)
                .map_err(|e| RepositoryError::DatabaseError(format!("Failed to find comments: {e}")))?
                .into_iter()
                .map(Self::db_comment_to_domain)
                .collect()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Join error: {e}")))?
    }

    async fn delete(&self, id: CommentId) -> Result<(), RepositoryError> {
        use crate::infrastructure::database::schema::comments;

        let pool = Arc::clone(&self.pool);
        let comment_uuid = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(format!("Connection error: {e}")))?;

            diesel::delete(comments::table.filter(comments::id.eq(comment_uuid)))
                .execute(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to delete comment: {e}"))
                })?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Join error: {e}")))?
    }

    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError> {
        use crate::infrastructure::database::schema::comments;

        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(format!("Connection error: {e}")))?;

            comments::table
                .order(comments::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load::<DbComment>(&mut conn)
                .map_err(|e| RepositoryError::DatabaseError(format!("Failed to list comments: {e}")))?
                .into_iter()
                .map(Self::db_comment_to_domain)
                .collect()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Join error: {e}")))?
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_db_comment_to_domain_success() {
        let db_comment = DbComment {
            id: Uuid::new_v4(),
            post_id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            content: "This is a valid comment".to_string(),
            parent_id: None,
            is_approved: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = DieselCommentRepository::db_comment_to_domain(db_comment);
        assert!(result.is_ok());

        let comment = result.unwrap();
        assert_eq!(comment.status(), CommentStatus::Published);
    }

    #[test]
    fn test_db_comment_to_domain_pending_status() {
        let db_comment = DbComment {
            id: Uuid::new_v4(),
            post_id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            content: "Pending comment".to_string(),
            parent_id: None,
            is_approved: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = DieselCommentRepository::db_comment_to_domain(db_comment);
        assert!(result.is_ok());

        let comment = result.unwrap();
        assert_eq!(comment.status(), CommentStatus::Pending);
    }

    #[test]
    fn test_db_comment_to_domain_empty_content() {
        let db_comment = DbComment {
            id: Uuid::new_v4(),
            post_id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            content: "".to_string(), // Invalid: empty
            parent_id: None,
            is_approved: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = DieselCommentRepository::db_comment_to_domain(db_comment);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepositoryError::ConversionError(_)));
    }

    #[test]
    fn test_domain_comment_to_new_db() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment content".to_string()).unwrap();

        let comment = Comment::new(post_id, author_id, text).unwrap();
        let new_db_comment = DieselCommentRepository::domain_comment_to_new_db(&comment);

        assert_eq!(new_db_comment.id, *comment.id().as_uuid());
        assert_eq!(new_db_comment.post_id, *comment.post_id().as_uuid());
        assert_eq!(new_db_comment.author_id, *comment.author_id().as_uuid());
        assert_eq!(new_db_comment.content, "Test comment content");
        assert!(!new_db_comment.is_approved); // Pending status
    }

    #[test]
    fn test_domain_comment_to_new_db_published() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Published comment".to_string()).unwrap();

        let mut comment = Comment::new(post_id, author_id, text).unwrap();
        comment.publish().unwrap();

        let new_db_comment = DieselCommentRepository::domain_comment_to_new_db(&comment);

        assert!(new_db_comment.is_approved); // Published status
    }
}
