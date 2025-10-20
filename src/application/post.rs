// src/application/post.rs
//! Post Application Layer - CQRS統合
//!
//! Phase 6-D: Legacy application layer (disabled with restructure_domain)
//! Commands + Queries + DTOs を単一ファイルに統合（監査推奨パターン）
#![cfg(not(feature = "restructure_domain"))]

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::post::{Content, Post, PostId, PostStatus, Slug, Title};
#[cfg(feature = "restructure_domain")]
use crate::domain::user::UserId;

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{PostRepository, RepositoryError};

// ============================================================================
// DTOs
// ============================================================================

/// 投稿作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub author_id: String, // UUID文字列
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    // Note: excerpt removed - not in current Post domain model
}

/// 投稿更新リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePostRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // Note: excerpt removed - not in current Post domain model
}

/// 投稿レスポンス
#[derive(Debug, Clone, Serialize)]
pub struct PostDto {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub author_id: String,
    pub status: String,
    // Note: created_at/updated_at/published_at removed - not in current Post domain model
}

#[cfg(feature = "restructure_domain")]
impl From<Post> for PostDto {
    fn from(post: Post) -> Self {
        Self {
            id: post.id().to_string(),
            title: post.title().to_string(),
            slug: post.slug().to_string(),
            content: post.content().to_string(),
            author_id: post.author_id().to_string(),
            status: format!("{:?}", post.status()).to_lowercase(),
        }
    }
}

// ============================================================================
// Commands
// ============================================================================

/// 投稿作成コマンド
#[cfg(feature = "restructure_domain")]
pub struct CreatePost {
    repo: Arc<dyn PostRepository>,
}

#[cfg(feature = "restructure_domain")]
impl CreatePost {
    pub fn new(repo: Arc<dyn PostRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, request: CreatePostRequest) -> Result<PostDto, RepositoryError> {
        // 1. Author ID 変換
        let author_id = UserId::from_string(&request.author_id)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 2. Value Objects 作成
        let title = Title::new(request.title)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let slug = if let Some(slug_str) = request.slug {
            Slug::new(slug_str).map_err(|e| RepositoryError::ValidationError(e.to_string()))?
        } else {
            Slug::from_title(title.as_str())
        };

        let content = Content::new(request.content)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 3. スラッグ重複チェック
        if self.repo.find_by_slug(&slug).await?.is_some() {
            return Err(RepositoryError::Duplicate(format!(
                "Slug '{}' already exists",
                slug
            )));
        }

        // 4. ドメインエンティティ作成
        let post = Post::new(author_id, title, slug, content);

        // 5. 永続化
        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

/// 投稿公開コマンド
#[cfg(feature = "restructure_domain")]
pub struct PublishPost {
    repo: Arc<dyn PostRepository>,
}

#[cfg(feature = "restructure_domain")]
impl PublishPost {
    pub fn new(repo: Arc<dyn PostRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<PostDto, RepositoryError> {
        let mut post = self
            .repo
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Post {}", post_id)))?;

        post.publish()
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

/// 投稿更新コマンド
#[cfg(feature = "restructure_domain")]
pub struct UpdatePost {
    repo: Arc<dyn PostRepository>,
}

#[cfg(feature = "restructure_domain")]
impl UpdatePost {
    pub fn new(repo: Arc<dyn PostRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        post_id: PostId,
        request: UpdatePostRequest,
    ) -> Result<PostDto, RepositoryError> {
        let mut post = self
            .repo
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Post {}", post_id)))?;

        if let Some(new_title) = request.title {
            let title = Title::new(new_title)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            post.update_title(title)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
        }

        if let Some(new_content) = request.content {
            post.update_content(new_content)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
        }

        if let Some(new_excerpt) = request.excerpt {
            post.update_excerpt(Some(new_excerpt));
        }

        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

/// 投稿アーカイブコマンド
#[cfg(feature = "restructure_domain")]
pub struct ArchivePost {
    repo: Arc<dyn PostRepository>,
}

#[cfg(feature = "restructure_domain")]
impl ArchivePost {
    pub fn new(repo: Arc<dyn PostRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<PostDto, RepositoryError> {
        let mut post = self
            .repo
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Post {}", post_id)))?;

        post.archive()
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

// ============================================================================
// Queries
// ============================================================================

/// 投稿取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct GetPostById {
    repo: Arc<dyn PostRepository>,
}

#[cfg(feature = "restructure_domain")]
impl GetPostById {
    pub fn new(repo: Arc<dyn PostRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<Option<PostDto>, RepositoryError> {
        let post = self.repo.find_by_id(post_id).await?;
        Ok(post.map(PostDto::from))
    }
}

/// 投稿一覧取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct ListPosts {
    repo: Arc<dyn PostRepository>,
}

#[cfg(feature = "restructure_domain")]
impl ListPosts {
    pub fn new(repo: Arc<dyn PostRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PostDto>, RepositoryError> {
        let posts = self.repo.list_all(limit, offset).await?;
        Ok(posts.into_iter().map(PostDto::from).collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "restructure_domain"))]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockPostRepository;
    use crate::domain::user::UserId;

    #[tokio::test]
    async fn test_create_post_success() {
        let mut mock_repo = MockPostRepository::new();

        // スラッグ重複チェック: None
        mock_repo
            .expect_find_by_slug()
            .returning(|_| Box::pin(async { Ok(None) }));

        // 保存成功
        mock_repo
            .expect_save()
            .returning(|_| Box::pin(async { Ok(()) }));

        let use_case = CreatePost::new(Arc::new(mock_repo));

        let request = CreatePostRequest {
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            author_id: UserId::new().to_string(),
            slug: Some("test-post".to_string()),
            excerpt: None,
        };

        let result = use_case.execute(request).await;
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert_eq!(dto.title, "Test Post");
        assert_eq!(dto.slug, "test-post");
    }

    #[tokio::test]
    async fn test_publish_post_success() {
        let mut mock_repo = MockPostRepository::new();

        let post = Post::new(
            UserId::new(),
            Title::new("Test".to_string()).unwrap(),
            Slug::new("test".to_string()).unwrap(),
            Content::new("Content".to_string()).unwrap(),
        );

        let post_id = post.id();

        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(post.clone())));

        mock_repo
            .expect_save()
            .returning(|_| Ok(()));

        let use_case = PublishPost::new(Arc::new(mock_repo));

        let result = use_case.execute(post_id).await;
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert_eq!(dto.status, "published");
    }
}
