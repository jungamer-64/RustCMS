// src/application/dto/post.rs
//! Post DTOs
//!
//! Post エンティティと HTTP レスポンス/リクエストの間のデータ変換を担当します。

use crate::domain::post::{Content, Post, PostStatus, Slug, Title};
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 投稿レスポンス DTO（完全版）
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PostDto {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub status: String,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Post> for PostDto {
    fn from(post: Post) -> Self {
        Self {
            id: post.id().to_string(),
            slug: post.slug().to_string(),
            title: post.title().as_str().to_string(),
            content: post.content().as_str().to_string(),
            author_id: post.author_id().to_string(),
            status: post.status().to_string(),
            published_at: post.published_at(),
            created_at: post.created_at(),
            updated_at: post.updated_at(),
        }
    }
}

/// 投稿一覧用 DTO（最小限フィールド）
#[derive(Debug, Clone, Serialize)]
pub struct PostListDto {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub author_id: String,
    pub status: String,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<Post> for PostListDto {
    fn from(post: Post) -> Self {
        Self {
            id: post.id().to_string(),
            slug: post.slug().to_string(),
            title: post.title().as_str().to_string(),
            author_id: post.author_id().to_string(),
            status: post.status().to_string(),
            published_at: post.published_at(),
        }
    }
}

/// 投稿作成リクエスト
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,

    #[validate(length(min = 10, max = 100000))]
    pub content: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 3, max = 50))]
    pub slug: Option<String>, // Optional: 未指定の場合はtitleから自動生成
}

/// 投稿更新リクエスト
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1, max = 200))]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 10, max = 100000))]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 3, max = 50))]
    pub slug: Option<String>,
}

// Phase 6-C: Type aliases for handler compatibility
pub type CreatePostDto = CreatePostRequest;
pub type UpdatePostDto = UpdatePostRequest;

/// 投稿公開リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct PublishPostRequest {
    pub post_id: String,
}

/// 投稿フィルター（Query 用）
#[derive(Debug, Clone, Default)]
pub struct PostFilter {
    pub status: Option<PostStatus>,
    pub author_id: Option<UserId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_dto_from_post() {
        let author_id = UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content =
            Content::new("This is test content that is long enough.".to_string()).unwrap();

        let post = Post::new(author_id, title, slug, content);

        let dto = PostDto::from(post.clone());

        assert_eq!(dto.id, post.id().to_string());
        assert_eq!(dto.slug, "test-post");
        assert_eq!(dto.title, "Test Post");
        assert_eq!(dto.status, "draft");
    }

    #[test]
    fn test_post_list_dto_from_post() {
        let author_id = UserId::new();
        let title = Title::new("Test Post".to_string()).unwrap();
        let slug = Slug::new("test-post".to_string()).unwrap();
        let content =
            Content::new("This is test content that is long enough.".to_string()).unwrap();

        let post = Post::new(author_id, title, slug, content);

        let dto = PostListDto::from(post);

        assert_eq!(dto.slug, "test-post");
        assert_eq!(dto.title, "Test Post");
        assert_eq!(dto.status, "draft");
    }

    #[test]
    fn test_create_post_request_deserialization() {
        let json = r#"{
            "title": "New Post",
            "content": "Post content here that is long enough for validation",
            "slug": "new-post"
        }"#;

        let request: CreatePostRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.title, "New Post");
        assert_eq!(
            request.content,
            "Post content here that is long enough for validation"
        );
        assert_eq!(request.slug, Some("new-post".to_string()));
    }

    #[test]
    fn test_update_post_request_partial() {
        let json = r#"{
            "title": "Updated Title"
        }"#;

        let request: UpdatePostRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.title, Some("Updated Title".to_string()));
        assert_eq!(request.content, None);
    }
}
