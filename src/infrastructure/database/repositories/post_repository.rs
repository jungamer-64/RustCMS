//! DieselPostRepository - PostRepository trait の Diesel 実装
//!
//! ## 責務
//! - PostRepository Port の具体的実装
//! - Domain Post Entity ↔ Diesel DbPost モデルのマッピング
//! - データベースエラーの RepositoryError への変換
//!
//! ## 設計原則
//! - スラッグの一意性制約をDB層で保証
//! - ステータス文字列の変換（Draft/Published/Archived）
//! - Connection Pool を通じてトランザクション管理

use crate::application::ports::repositories::{PostRepository, RepositoryError};
use crate::database::schema::posts;
use crate::domain::post::{Content, Post, PostId, PostStatus, Slug, Title};
use crate::domain::user::UserId;
use crate::infrastructure::database::models::{DbPost, NewDbPost};
use async_trait::async_trait;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::sync::Arc;
use uuid::Uuid;

/// Diesel ベースの PostRepository 実装
pub struct DieselPostRepository {
    pool: Arc<Pool<ConnectionManager<PgConnection>>>,
}

impl DieselPostRepository {
    /// 新しい DieselPostRepository を作成
    #[must_use]
    pub fn new(pool: Arc<Pool<ConnectionManager<PgConnection>>>) -> Self {
        Self { pool }
    }

    /// DbPost から Domain Post Entity への変換
    fn db_post_to_domain(db_post: DbPost) -> Result<Post, RepositoryError> {
        let post_id = PostId::from_uuid(db_post.id);
        let author_id = UserId::from_uuid(db_post.author_id);
        
        let title = Title::new(db_post.title).map_err(|e| {
            RepositoryError::ConversionError(format!("Invalid title: {}", e))
        })?;
        
        let slug = Slug::new(db_post.slug).map_err(|e| {
            RepositoryError::ConversionError(format!("Invalid slug: {}", e))
        })?;
        
        let content = Content::new(db_post.content).map_err(|e| {
            RepositoryError::ConversionError(format!("Invalid content: {}", e))
        })?;
        
        let status = match db_post.status.as_str() {
            "draft" => PostStatus::Draft,
            "published" => PostStatus::Published,
            "archived" => PostStatus::Archived,
            _ => return Err(RepositoryError::ConversionError(
                format!("Unknown post status: {}", db_post.status)
            )),
        };

        Ok(Post::restore(
            post_id,
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

    /// Domain Post Entity から NewDbPost への変換
    fn domain_post_to_new_db(post: &Post) -> NewDbPost {
        let status_str = match post.status() {
            PostStatus::Draft => "draft",
            PostStatus::Published => "published",
            PostStatus::Archived => "archived",
        };

        NewDbPost {
            id: post.id().into_uuid(),
            title: post.title().as_str().to_string(),
            slug: post.slug().as_str().to_string(),
            content: post.content().as_str().to_string(),
            excerpt: None, // TODO: excerpt フィールドを Post Entity に追加
            author_id: post.author_id().into_uuid(),
            status: status_str.to_string(),
            featured_image_id: None,
            tags: Vec::new(), // TODO: tags を Post Entity に追加
            categories: Vec::new(), // TODO: categories を Post Entity に追加
            meta_title: None,
            meta_description: None,
            published_at: post.published_at(),
            created_at: post.created_at(),
            updated_at: post.updated_at(),
        }
    }
}

#[async_trait]
impl PostRepository for DieselPostRepository {
    /// 投稿を保存（作成または更新）
    async fn save(&self, post: Post) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let new_db_post = Self::domain_post_to_new_db(&post);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            diesel::insert_into(posts::table)
                .values(&new_db_post)
                .on_conflict(posts::id)
                .do_update()
                .set((
                    posts::title.eq(&new_db_post.title),
                    posts::slug.eq(&new_db_post.slug),
                    posts::content.eq(&new_db_post.content),
                    posts::status.eq(&new_db_post.status),
                    posts::published_at.eq(new_db_post.published_at),
                    posts::updated_at.eq(Utc::now()),
                ))
                .execute(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to save post: {}", e))
                })?;

            Ok(())
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// IDで投稿を検索
    async fn find_by_id(&self, id: PostId) -> Result<Option<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let post_uuid = id.into_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_post = posts::table
                .filter(posts::id.eq(post_uuid))
                .first::<DbPost>(&mut conn)
                .optional()
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to find post by id: {}", e))
                })?;

            match db_post {
                Some(db_post) => Ok(Some(Self::db_post_to_domain(db_post)?)),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// スラッグで投稿を検索
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let slug_str = slug.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_post = posts::table
                .filter(posts::slug.eq(&slug_str))
                .first::<DbPost>(&mut conn)
                .optional()
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to find post by slug: {}", e))
                })?;

            match db_post {
                Some(db_post) => Ok(Some(Self::db_post_to_domain(db_post)?)),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// 投稿を削除
    async fn delete(&self, id: PostId) -> Result<(), RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let post_uuid = id.into_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            diesel::delete(posts::table.filter(posts::id.eq(post_uuid)))
                .execute(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to delete post: {}", e))
                })?;

            Ok(())
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// 全投稿を取得（ページネーション対応）
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_posts = posts::table
                .order(posts::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load::<DbPost>(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to list posts: {}", e))
                })?;

            db_posts
                .into_iter()
                .map(Self::db_post_to_domain)
                .collect::<Result<Vec<Post>, RepositoryError>>()
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }

    /// 著者IDで投稿を検索
    async fn find_by_author(
        &self,
        author_id: UserId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Post>, RepositoryError> {
        let pool = Arc::clone(&self.pool);
        let author_uuid = author_id.into_uuid();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                RepositoryError::DatabaseError(format!("Failed to get connection: {}", e))
            })?;

            let db_posts = posts::table
                .filter(posts::author_id.eq(author_uuid))
                .order(posts::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load::<DbPost>(&mut conn)
                .map_err(|e| {
                    RepositoryError::DatabaseError(format!("Failed to find posts by author: {}", e))
                })?;

            db_posts
                .into_iter()
                .map(Self::db_post_to_domain)
                .collect::<Result<Vec<Post>, RepositoryError>>()
        })
        .await
        .map_err(|e| {
            RepositoryError::DatabaseError(format!("Task join error: {}", e))
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::post::{Slug, Title, Content};
    use crate::domain::user::UserId;

    #[test]
    fn test_domain_post_to_new_db_conversion() {
        let author_id = UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content = Content::new("This is a test post content.".to_string()).unwrap();
        
        let post = Post::new(author_id, title, slug, content);
        let new_db_post = DieselPostRepository::domain_post_to_new_db(&post);

        assert_eq!(new_db_post.id, post.id().into_uuid());
        assert_eq!(new_db_post.title, "Test Post");
        assert_eq!(new_db_post.slug, "test-post");
        assert_eq!(new_db_post.content, "This is a test post content.");
        assert_eq!(new_db_post.status, "draft");
        assert_eq!(new_db_post.author_id, author_id.into_uuid());
    }

    #[test]
    fn test_db_post_to_domain_conversion_success() {
        let post_id = Uuid::new_v4();
        let author_id = Uuid::new_v4();
        let now = Utc::now();

        let db_post = DbPost {
            id: post_id,
            title: "Sample Post".to_string(),
            slug: "sample-post".to_string(),
            content: "This is the content of the sample post.".to_string(),
            excerpt: None,
            author_id,
            status: "published".to_string(),
            featured_image_id: None,
            tags: Vec::new(),
            categories: Vec::new(),
            meta_title: None,
            meta_description: None,
            published_at: Some(now),
            created_at: now,
            updated_at: now,
        };

        let post = DieselPostRepository::db_post_to_domain(db_post.clone()).unwrap();

        assert_eq!(post.id().into_uuid(), post_id);
        assert_eq!(post.title().as_str(), "Sample Post");
        assert_eq!(post.slug().as_str(), "sample-post");
        assert_eq!(post.author_id().into_uuid(), author_id);
        assert_eq!(post.status(), PostStatus::Published);
    }

    #[test]
    fn test_db_post_to_domain_conversion_invalid_title() {
        let db_post = DbPost {
            id: Uuid::new_v4(),
            title: "".to_string(), // Empty title
            slug: "test-slug".to_string(),
            content: "Content here".to_string(),
            excerpt: None,
            author_id: Uuid::new_v4(),
            status: "draft".to_string(),
            featured_image_id: None,
            tags: Vec::new(),
            categories: Vec::new(),
            meta_title: None,
            meta_description: None,
            published_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = DieselPostRepository::db_post_to_domain(db_post);
        assert!(result.is_err());
        match result {
            Err(RepositoryError::ConversionError(msg)) => {
                assert!(msg.contains("Invalid title"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_post_status_conversion() {
        let author_id = UserId::new();
        let title = Title::new("Draft Post".to_string()).unwrap();
        let slug = Slug::new("draft-post".to_string()).unwrap();
        let content = Content::new("Draft content".to_string()).unwrap();
        
        let post = Post::new(author_id, title, slug, content);
        let new_db_post = DieselPostRepository::domain_post_to_new_db(&post);
        
        assert_eq!(new_db_post.status, "draft");
    }
}
