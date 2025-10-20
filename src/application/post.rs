//! Post Application Layer - CQRS統合
//!
//! Commands + Queries + DTOs を単一ファイルに統合

use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::post::{Content, Post, PostId, Slug, Title};
#[cfg(feature = "restructure_domain")]
use crate::domain::user::UserId;

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{PostRepository, RepositoryError};

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub author_id: String,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePostRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostDto {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub author_id: String,
    pub status: String,
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

#[cfg(feature = "restructure_domain")]
pub struct CreatePost<'a> {
    repo: &'a dyn PostRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> CreatePost<'a> {
    pub const fn new(repo: &'a dyn PostRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, request: CreatePostRequest) -> Result<PostDto, RepositoryError> {
        let author_id = UserId::from_string(&request.author_id)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let title = Title::new(request.title)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        let slug = if let Some(slug_str) = request.slug {
            Slug::new(slug_str).map_err(|e| RepositoryError::ValidationError(e.to_string()))?
        } else {
            Slug::from_title(&title).map_err(|e| RepositoryError::ValidationError(e.to_string()))?
        };

        let content = Content::new(request.content)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // スラッグ重複チェック
        if self.repo.find_by_slug(slug.as_str()).await?.is_some() {
            return Err(RepositoryError::Duplicate(format!(
                "Slug '{}' already exists",
                slug
            )));
        }

        let post = Post::new(author_id, title, slug, content);
        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct PublishPost<'a> {
    repo: &'a dyn PostRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> PublishPost<'a> {
    pub const fn new(repo: &'a dyn PostRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<PostDto, RepositoryError> {
        let mut post = self
            .repo
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Post {post_id}")))?;

        post.publish()
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct UpdatePost<'a> {
    repo: &'a dyn PostRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> UpdatePost<'a> {
    pub const fn new(repo: &'a dyn PostRepository) -> Self {
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
            .ok_or_else(|| RepositoryError::NotFound(format!("Post {post_id}")))?;

        if let Some(new_title) = request.title {
            let title = Title::new(new_title)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            // update_title is a void method (per domain) so call directly after VO validation
            post.update_title(title);
        }

        if let Some(new_content) = request.content {
            let content = Content::new(new_content)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            post.update_content(content);
        }

        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct ArchivePost<'a> {
    repo: &'a dyn PostRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> ArchivePost<'a> {
    pub const fn new(repo: &'a dyn PostRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<PostDto, RepositoryError> {
        let mut post = self
            .repo
            .find_by_id(post_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("Post {post_id}")))?;

        post.archive()
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        self.repo.save(post.clone()).await?;

        Ok(PostDto::from(post))
    }
}

// ============================================================================
// Queries
// ============================================================================

#[cfg(feature = "restructure_domain")]
pub struct GetPostById<'a> {
    repo: &'a dyn PostRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> GetPostById<'a> {
    pub const fn new(repo: &'a dyn PostRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, post_id: PostId) -> Result<Option<PostDto>, RepositoryError> {
        Ok(self.repo.find_by_id(post_id).await?.map(PostDto::from))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct ListPosts<'a> {
    repo: &'a dyn PostRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> ListPosts<'a> {
    pub const fn new(repo: &'a dyn PostRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, limit: i64, offset: i64) -> Result<Vec<PostDto>, RepositoryError> {
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

    #[tokio::test]
    async fn test_create_post_success() {
        let mut mock_repo = MockPostRepository::new();

        mock_repo.expect_find_by_slug().returning(|_| Ok(None));
        mock_repo.expect_save().returning(|_| Ok(()));

        let use_case = CreatePost::new(&mock_repo);

        let request = CreatePostRequest {
            title: "Test Post".into(),
            content: "Test content".into(),
            author_id: UserId::new().to_string(),
            slug: Some("test-post".into()),
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
            Title::new("Test".into()).unwrap(),
            Slug::new("test".into()).unwrap(),
            Content::new("Content body".into()).unwrap(),
        );

        let post_id = post.id();

        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(post.clone())));
        mock_repo.expect_save().returning(|_| Ok(()));

        let use_case = PublishPost::new(&mock_repo);

        let result = use_case.execute(post_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, "published");
    }
}
