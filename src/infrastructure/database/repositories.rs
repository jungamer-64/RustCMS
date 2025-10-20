// infrastructure/database/repositories.rs
// Repository実装（監査済み構造: 単一ファイル統合パターン）
//
// Pattern: Multiple Repository implementations in a single file
// Rationale: 
// - High cohesion (all Diesel implementations in one place)
// - Reduced import statements
// - < 1000 lines total is acceptable for combined repositories
//
// Phase 9: New implementation following audited structure (RESTRUCTURE_EXAMPLES.md)

#![cfg(feature = "database")]

use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::sync::Arc;

use crate::application::ports::repositories::{
    RepositoryError, UserRepository, PostRepository, CommentRepository,
};
use crate::domain::user::{User, UserId, Email, Username};
use crate::domain::post::{Post, PostId, Slug, Content};
use crate::domain::comment::{Comment, CommentId, CommentText, CommentStatus};
use crate::infrastructure::database::models::{DbUser, NewDbUser, DbPost, NewDbPost, DbComment, NewDbComment};
use crate::infrastructure::database::schema;

// ============================================================================
// Type Aliases
// ============================================================================

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

// ============================================================================
// DieselUserRepository
// ============================================================================

/// Diesel implementation of UserRepository (Phase 9 - New Structure)
pub struct DieselUserRepository {
    pool: Arc<PgPool>,
}

impl DieselUserRepository {
    /// Create a new DieselUserRepository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Get a database connection from the pool
    fn get_conn(&self) -> Result<PgPooledConnection, RepositoryError> {
        self.pool
            .get()
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))
    }

    /// Convert DbUser to Domain User entity
    fn db_to_domain(db_user: DbUser) -> Result<User, RepositoryError> {
        // Parse Value Objects
        let id = UserId::from_uuid(db_user.id);
        let username = Username::new(db_user.username)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid username: {}", e)))?;
        let email = Email::new(db_user.email)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid email: {}", e)))?;

        // Parse role
        let role = crate::domain::user::UserRole::from_str(&db_user.role)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid role: {:?}", e)))?;

        // Restore entity using restore() method
        Ok(User::restore(
            id,
            username,
            email,
            Some(db_user.password_hash.unwrap_or_default()),
            role,
            db_user.is_active,
            db_user.created_at,
            db_user.updated_at,
            db_user.last_login,
        ))
    }

    /// Convert Domain User entity to DbUser
    fn domain_to_db(user: &User) -> NewDbUser {
        NewDbUser {
            id: *user.id().as_uuid(),
            username: user.username().as_str().to_string(),
            email: user.email().as_str().to_string(),
            password_hash: user.password_hash().map(|s| s.as_str().to_string()),
            role: user.role().as_str().to_string(),
            is_active: user.is_active(),
            email_verified: false, // TODO: Add to domain::user::User
            created_at: user.created_at(),
            updated_at: user.updated_at(),
            first_name: None,
            last_name: None,
            last_login: None,
        }
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn save(&self, user: User) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let user_clone = user.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let new_user = Self::domain_to_db(&user_clone);

            // UPSERT: INSERT or UPDATE
            diesel::insert_into(schema::users::table)
                .values(&new_user)
                .on_conflict(schema::users::id)
                .do_update()
                .set((
                    schema::users::username.eq(&new_user.username),
                    schema::users::email.eq(&new_user.email),
                    schema::users::password_hash.eq(&new_user.password_hash),
                    schema::users::role.eq(&new_user.role),
                    schema::users::is_active.eq(&new_user.is_active),
                    schema::users::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let user_id = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_user = schema::users::table
                .find(user_id)
                .select(DbUser::as_select())
                .first::<DbUser>(&mut conn)
                .optional()
                .map_err(RepositoryError::from)?;

            match db_user {
                Some(db) => Self::db_to_domain(db).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let email_str = email.as_str().to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_user = schema::users::table
                .filter(schema::users::email.eq(&email_str))
                .select(DbUser::as_select())
                .first::<DbUser>(&mut conn)
                .optional()
                .map_err(RepositoryError::from)?;

            match db_user {
                Some(db) => Self::db_to_domain(db).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn delete(&self, id: UserId) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let user_id = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            diesel::delete(schema::users::table.find(user_id))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_users = schema::users::table
                .limit(limit)
                .offset(offset)
                .select(DbUser::as_select())
                .load::<DbUser>(&mut conn)
                .map_err(RepositoryError::from)?;

            db_users
                .into_iter()
                .map(Self::db_to_domain)
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let username_str = username.as_str().to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_user = schema::users::table
                .filter(schema::users::username.eq(&username_str))
                .select(DbUser::as_select())
                .first::<DbUser>(&mut conn)
                .optional()
                .map_err(RepositoryError::from)?;

            match db_user {
                Some(db) => Self::db_to_domain(db).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Phase 9: パスワードハッシュを更新
    async fn update_password_hash(
        &self,
        user_id: UserId,
        password_hash: String,
    ) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let id = *user_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            diesel::update(schema::users::table.find(id))
                .set((
                    schema::users::password_hash.eq(Some(password_hash)),
                    schema::users::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Phase 9: 最終ログイン日時を更新
    async fn update_last_login(&self, user_id: UserId) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let id = *user_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            diesel::update(schema::users::table.find(id))
                .set((
                    schema::users::last_login.eq(Some(Utc::now())),
                    schema::users::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }
}

// ============================================================================
// DieselPostRepository
// ============================================================================

/// Diesel implementation of PostRepository (Phase 9 - New Structure)
pub struct DieselPostRepository {
    pool: Arc<PgPool>,
}

impl DieselPostRepository {
    /// Create a new DieselPostRepository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert DbPost to Domain Post entity
    fn db_to_domain(db_post: DbPost) -> Result<Post, RepositoryError> {
        // Parse Value Objects
        let id = PostId::from_uuid(db_post.id);
        let title = crate::domain::post::Title::new(db_post.title)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid title: {}", e)))?;
        let slug = Slug::new(db_post.slug)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid slug: {}", e)))?;
        let author_id = UserId::from_uuid(db_post.author_id);
        let content = Content::new(db_post.content).unwrap();

        // Parse status
        let status = crate::domain::post::PostStatus::from_str(&db_post.status)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid status: {:?}", e)))?;

        // Restore entity
        Ok(Post::restore(
            id,
            author_id,
            title,
            slug,
            content,
            status,
            db_post.created_at,
            db_post.published_at,
            db_post.updated_at,
        ))
    }

    /// Convert Domain Post entity to NewDbPost
    fn domain_to_db(post: &Post) -> NewDbPost {
        NewDbPost {
            id: *post.id().as_uuid(),
            title: post.title().as_str().to_string(),
            slug: post.slug().as_str().to_string(),
            content: post.content().as_str().to_string(),
            excerpt: None, // TODO: Add to domain::post::Post
            author_id: *post.author_id().as_uuid(),
            status: post.status().to_string(),
            featured_image_id: None, // Legacy field
            meta_title: None,
            meta_description: None,
            published_at: post.published_at(),
            created_at: post.created_at(),
            updated_at: post.updated_at(),
            tags: vec![],
            categories: vec![],
        }
    }
}

#[async_trait]
impl PostRepository for DieselPostRepository {
    async fn save(&self, post: Post) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let post_clone = post.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let new_post = Self::domain_to_db(&post_clone);

            diesel::insert_into(schema::posts::table)
                .values(&new_post)
                .on_conflict(schema::posts::id)
                .do_update()
                .set((
                    schema::posts::title.eq(&new_post.title),
                    schema::posts::slug.eq(&new_post.slug),
                    schema::posts::content.eq(&new_post.content),
                    schema::posts::status.eq(&new_post.status),
                    schema::posts::published_at.eq(&new_post.published_at),
                    schema::posts::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_id(&self, id: PostId) -> Result<Option<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let post_id = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_post = schema::posts::table
                .find(post_id)
                .select(DbPost::as_select())
                .first::<DbPost>(&mut conn)
                .optional()
                .map_err(RepositoryError::from)?;

            match db_post {
                Some(db) => Self::db_to_domain(db).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let slug_str = slug.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_post = schema::posts::table
                .filter(schema::posts::slug.eq(&slug_str))
                .select(DbPost::as_select())
                .first::<DbPost>(&mut conn)
                .optional()
                .map_err(RepositoryError::from)?;

            match db_post {
                Some(db) => Self::db_to_domain(db).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn delete(&self, id: PostId) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let post_id = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            diesel::delete(schema::posts::table.find(post_id))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_posts = schema::posts::table
                .limit(limit)
                .offset(offset)
                .select(DbPost::as_select())
                .load::<DbPost>(&mut conn)
                .map_err(RepositoryError::from)?;

            db_posts
                .into_iter()
                .map(Self::db_to_domain)
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_author(&self, author_id: UserId, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let author_uuid = *author_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_posts = schema::posts::table
                .filter(schema::posts::author_id.eq(author_uuid))
                .limit(limit)
                .offset(offset)
                .select(DbPost::as_select())
                .load::<DbPost>(&mut conn)
                .map_err(RepositoryError::from)?;

            db_posts
                .into_iter()
                .map(Self::db_to_domain)
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }
}

// ============================================================================
// DieselCommentRepository
// ============================================================================

/// Diesel implementation of CommentRepository (Phase 9 - New Structure)
pub struct DieselCommentRepository {
    pool: Arc<PgPool>,
}

impl DieselCommentRepository {
    /// Create a new DieselCommentRepository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert DbComment to Domain Comment entity
    fn db_to_domain(db_comment: DbComment) -> Result<Comment, RepositoryError> {
        let id = CommentId::from_uuid(db_comment.id);
        let post_id = PostId::from_uuid(db_comment.post_id);
        let author_id = UserId::from_uuid(db_comment.author_id);
        let content = CommentText::new(db_comment.content)
            .map_err(|e| RepositoryError::ConversionError(format!("Invalid content: {}", e)))?;

        let parent_id = db_comment.parent_id.map(CommentId::from_uuid);
        
        // Convert is_approved to CommentStatus
        let status = if db_comment.is_approved {
            CommentStatus::Published
        } else {
            CommentStatus::Pending
        };

        Ok(Comment::restore(
            id,
            post_id,
            author_id,
            content,
            parent_id,
            status,
            db_comment.created_at,
            None, // edited_at not in DB schema
            db_comment.updated_at,
        ))
    }

    /// Convert Domain Comment entity to NewDbComment
    fn domain_to_db(comment: &Comment) -> NewDbComment {
        NewDbComment {
            id: *comment.id().as_uuid(),
            post_id: *comment.post_id().as_uuid(),
            author_id: *comment.author_id().as_uuid(),
            content: comment.text().as_str().to_string(),
            parent_id: comment.parent_id().map(|id| id.into_uuid()),
            is_approved: true,
            created_at: comment.created_at(),
            updated_at: comment.updated_at(),
        }
    }
}

#[async_trait]
impl CommentRepository for DieselCommentRepository {
    async fn save(&self, comment: Comment) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let comment_clone = comment.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let new_comment = Self::domain_to_db(&comment_clone);

            diesel::insert_into(schema::comments::table)
                .values(&new_comment)
                .on_conflict(schema::comments::id)
                .do_update()
                .set((
                    schema::comments::content.eq(&new_comment.content),
                    schema::comments::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_id(&self, id: CommentId) -> Result<Option<Comment>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let comment_id = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_comment = schema::comments::table
                .find(comment_id)
                .select(DbComment::as_select())
                .first::<DbComment>(&mut conn)
                .optional()
                .map_err(RepositoryError::from)?;

            match db_comment {
                Some(db) => Self::db_to_domain(db).map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn delete(&self, id: CommentId) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let comment_id = *id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            diesel::delete(schema::comments::table.find(comment_id))
                .execute(&mut conn)
                .map_err(RepositoryError::from)?;

            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_comments = schema::comments::table
                .limit(limit)
                .offset(offset)
                .select(DbComment::as_select())
                .load::<DbComment>(&mut conn)
                .map_err(RepositoryError::from)?;

            db_comments
                .into_iter()
                .map(Self::db_to_domain)
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_post(&self, post_id: PostId, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let post_uuid = *post_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_comments = schema::comments::table
                .filter(schema::comments::post_id.eq(post_uuid))
                .limit(limit)
                .offset(offset)
                .select(DbComment::as_select())
                .load::<DbComment>(&mut conn)
                .map_err(RepositoryError::from)?;

            db_comments
                .into_iter()
                .map(Self::db_to_domain)
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }

    async fn find_by_author(&self, author_id: UserId, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let author_uuid = *author_id.as_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

            let db_comments = schema::comments::table
                .filter(schema::comments::author_id.eq(author_uuid))
                .limit(limit)
                .offset(offset)
                .select(DbComment::as_select())
                .load::<DbComment>(&mut conn)
                .map_err(RepositoryError::from)?;

            db_comments
                .into_iter()
                .map(Self::db_to_domain)
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Task join error: {}", e)))?
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_creation() {
        // This test just verifies that the repository structs can be created
        // Full integration tests require a PostgreSQL connection
        // See tests/integration_repositories_phase3.rs for full tests
    }
}