// src/application/ports/repositories.rs
//! Repository Ports (インターフェース定義)
//!
//! Port/Adapter パターンのPort定義です。
//! Infrastructure層がこれらのtraitを実装します。

use async_trait::async_trait;

use crate::domain::comment::{Comment, CommentId};
use crate::domain::post::PostId;
use crate::domain::user::UserId;

#[cfg(feature = "restructure_domain")]
use crate::domain::category::{Category, CategoryId, CategorySlug};
#[cfg(feature = "restructure_domain")]
use crate::domain::post::Post;
#[cfg(feature = "restructure_domain")]
use crate::domain::tag::{Tag, TagId, TagName};
#[cfg(feature = "restructure_domain")]
use crate::domain::user::{Email, User, Username};

// ============================================================================
// Type Aliases for cleaner signatures
// ============================================================================

type RepoResult<T> = Result<T, RepositoryError>;

// ============================================================================
// User Repository Port
// ============================================================================

#[cfg(feature = "restructure_domain")]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: User) -> RepoResult<()>;
    async fn find_by_id(&self, id: UserId) -> RepoResult<Option<User>>;
    async fn find_by_email(&self, email: &Email) -> RepoResult<Option<User>>;
    async fn find_by_username(&self, username: &Username) -> RepoResult<Option<User>>;
    async fn delete(&self, id: UserId) -> RepoResult<()>;
    async fn list_all(&self, limit: i64, offset: i64) -> RepoResult<Vec<User>>;
    // Phase 9 additions: update password hash and last_login
    async fn update_password_hash(&self, user_id: UserId, password_hash: String) -> RepoResult<()>;
    async fn update_last_login(&self, user_id: UserId) -> RepoResult<()>;
}

// ============================================================================
// Post Repository Port
// ============================================================================

#[cfg(feature = "restructure_domain")]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn save(&self, post: Post) -> RepoResult<()>;
    async fn find_by_id(&self, id: PostId) -> RepoResult<Option<Post>>;
    async fn find_by_slug(&self, slug: &str) -> RepoResult<Option<Post>>;
    async fn delete(&self, id: PostId) -> RepoResult<()>;
    async fn list_all(&self, limit: i64, offset: i64) -> RepoResult<Vec<Post>>;
    async fn find_by_author(
        &self,
        author_id: UserId,
        limit: i64,
        offset: i64,
    ) -> RepoResult<Vec<Post>>;
}

// ============================================================================
// Comment Repository Port
// ============================================================================

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CommentRepository: Send + Sync {
    async fn save(&self, comment: Comment) -> RepoResult<()>;
    async fn find_by_id(&self, id: CommentId) -> RepoResult<Option<Comment>>;
    async fn find_by_post(
        &self,
        post_id: PostId,
        limit: i64,
        offset: i64,
    ) -> RepoResult<Vec<Comment>>;
    async fn find_by_author(
        &self,
        author_id: UserId,
        limit: i64,
        offset: i64,
    ) -> RepoResult<Vec<Comment>>;
    async fn delete(&self, id: CommentId) -> RepoResult<()>;
    async fn list_all(&self, limit: i64, offset: i64) -> RepoResult<Vec<Comment>>;
}

// ============================================================================
// Tag Repository Port
// ============================================================================

#[cfg(feature = "restructure_domain")]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TagRepository: Send + Sync {
    async fn save(&self, tag: Tag) -> RepoResult<()>;
    async fn find_by_id(&self, id: TagId) -> RepoResult<Option<Tag>>;
    async fn find_by_name(&self, name: &TagName) -> RepoResult<Option<Tag>>;
    async fn delete(&self, id: TagId) -> RepoResult<()>;
    async fn list_all(&self, limit: i64, offset: i64) -> RepoResult<Vec<Tag>>;
    async fn list_in_use(&self, limit: i64, offset: i64) -> RepoResult<Vec<Tag>>;
}

// ============================================================================
// Category Repository Port
// ============================================================================

#[cfg(feature = "restructure_domain")]
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn save(&self, category: Category) -> RepoResult<()>;
    async fn find_by_id(&self, id: CategoryId) -> RepoResult<Option<Category>>;
    async fn find_by_slug(&self, slug: &CategorySlug) -> RepoResult<Option<Category>>;
    async fn delete(&self, id: CategoryId) -> RepoResult<()>;
    async fn list_all(&self, limit: i64, offset: i64) -> RepoResult<Vec<Category>>;
    async fn list_active(&self, limit: i64, offset: i64) -> RepoResult<Vec<Category>>;
}

// ============================================================================
// Repository Error
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Diesel Error からの変換
#[cfg(feature = "database")]
impl From<diesel::result::Error> for RepositoryError {
    fn from(err: diesel::result::Error) -> Self {
        use diesel::result::Error as DieselError;
        match err {
            DieselError::NotFound => Self::NotFound("Record not found".into()),
            DieselError::DatabaseError(kind, _) => {
                Self::DatabaseError(format!("Database error: {kind:?}"))
            }
            DieselError::QueryBuilderError(msg) => {
                Self::DatabaseError(format!("Query builder error: {msg}"))
            }
            DieselError::DeserializationError(e) => {
                Self::ConversionError(format!("Deserialization error: {e}"))
            }
            DieselError::SerializationError(e) => {
                Self::ConversionError(format!("Serialization error: {e}"))
            }
            _ => Self::Unknown(err.to_string()),
        }
    }
}

#[cfg(feature = "database")]
impl From<diesel::r2d2::PoolError> for RepositoryError {
    fn from(err: diesel::r2d2::PoolError) -> Self {
        Self::ConnectionError(format!("Connection pool error: {err}"))
    }
}

// ============================================================================
// Helper Extensions
// ============================================================================

pub trait RepositoryResultExt<T> {
    fn not_found(entity: impl std::fmt::Display) -> RepoResult<T>;
}

impl<T> RepositoryResultExt<T> for RepoResult<T> {
    fn not_found(entity: impl std::fmt::Display) -> RepoResult<T> {
        Err(RepositoryError::NotFound(entity.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_error_display() {
        let errors = [
            RepositoryError::NotFound("User 123".into()),
            RepositoryError::Duplicate("email@example.com".into()),
            RepositoryError::DatabaseError("Connection failed".into()),
            RepositoryError::ValidationError("Invalid email".into()),
            RepositoryError::Unknown("Something went wrong".into()),
        ];

        for error in &errors {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn test_not_found_helper() {
        let result: RepoResult<()> = RepoResult::not_found("User 123");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }
}
