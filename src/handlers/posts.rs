//! Post Handlers
//!
//! Handles CRUD operations for blog posts and content management

//! Post Handlers
//!
//! Handles CRUD operations for blog posts and content management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::utils::api_types::ApiResponse;
use crate::{
    models::{CreatePostRequest, Post, UpdatePostRequest},
    AppState, Result,
};

/// Post query parameters
#[derive(Debug, Deserialize)]
pub struct PostQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub status: Option<String>,
    pub author: Option<Uuid>,
    pub tag: Option<String>,
    pub sort: Option<String>,
}

/// Post response for API
#[derive(Debug, Serialize, Deserialize)]
pub struct PostResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub status: String,
    pub author_id: Uuid,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&Post> for PostResponse {
    fn from(post: &Post) -> Self {
        Self {
            id: post.id,
            title: post.title.clone(),
            content: post.content.clone(),
            excerpt: post.excerpt.clone(),
            status: post.status.clone(),
            author_id: post.author_id,
            tags: post.tags.clone(),
            metadata: serde_json::json!({
                "meta_title": post.meta_title,
                "meta_description": post.meta_description,
                "categories": post.categories
            }),
            created_at: post.created_at,
            updated_at: post.updated_at,
            published_at: post.published_at,
        }
    }
}

/// Paginated posts response
#[derive(Debug, Serialize, Deserialize)]
pub struct PostsResponse {
    pub posts: Vec<PostResponse>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

/// Create a new post
pub async fn create_post(
    State(state): State<AppState>,
    Json(request): Json<CreatePostRequest>,
) -> Result<impl IntoResponse> {
    let post = state.db_create_post(request).await?;
    #[cfg(feature = "cache")]
    if let Err(e) = state.cache.delete("posts:*").await {
        eprintln!("Failed to clear post cache: {}", e);
    }
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_post(&post).await {
        eprintln!("Failed to index post for search: {}", e);
    }
    Ok((StatusCode::CREATED, Json(ApiResponse::success(PostResponse::from(&post)))))
}

/// Get post by ID
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let cache_key = format!("post:{}", id);
    #[cfg(feature = "cache")]
    if let Ok(Some(cached)) = state.cache.get::<PostResponse>(&cache_key).await {
        return Ok(Json(ApiResponse::success(cached)));
    }
    let post = state.db_get_post_by_id(id).await?;
    let response = PostResponse::from(&post);
    #[cfg(feature = "cache")]
    if let Err(e) = state
        .cache
        .set(cache_key, &response, Some(Duration::from_secs(300)))
        .await
    {
        eprintln!("Failed to cache post: {}", e);
    }
    Ok(Json(ApiResponse::success(response)))
}

/// Get all posts with pagination and filtering
pub async fn get_posts(
    State(state): State<AppState>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let cache_key = format!(
        "posts:page:{}:limit:{}:status:{}:author:{}:tag:{}:sort:{}",
        page,
        limit,
        query.status.as_deref().unwrap_or("all"),
        query
            .author
            .map(|a| a.to_string())
            .unwrap_or_else(|| "all".to_string()),
        query.tag.as_deref().unwrap_or("all"),
        query.sort.as_deref().unwrap_or("created_at")
    );
    #[cfg(feature = "cache")]
    if let Ok(Some(cached)) = state.cache.get::<PostsResponse>(&cache_key).await {
        return Ok(Json(cached));
    }
    let posts = state
        .db_get_posts(page, limit, query.status, query.author, query.tag, query.sort)
        .await?;
    let total = state.db_count_posts(None).await?;
    let total_pages = (total as f32 / limit as f32).ceil() as u32;
    let response = PostsResponse {
        posts: posts.iter().map(PostResponse::from).collect(),
        total,
        page,
        limit,
        total_pages,
    };
    #[cfg(feature = "cache")]
    if let Err(e) = state
        .cache
        .set(cache_key, &response, Some(Duration::from_secs(300)))
        .await
    {
        eprintln!("Failed to cache posts: {}", e);
    }
    Ok(Json(response))
}

/// Update post
pub async fn update_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePostRequest>,
) -> Result<impl IntoResponse> {
    let post = state.db_update_post(id, request).await?;
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_post(&post).await {
        eprintln!("Failed to update post in search index: {}", e);
    }
    let cache_key = format!("post:{}", id);
    #[cfg(feature = "cache")]
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear post cache: {}", e);
    }
    #[cfg(feature = "cache")]
    if let Err(e) = state.cache.delete("posts:*").await {
        eprintln!("Failed to clear posts cache: {}", e);
    }
    Ok(Json(ApiResponse::success(PostResponse::from(&post))))
}

/// Delete post
pub async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.db_delete_post(id).await?;
    #[cfg(feature = "search")]
    if let Err(e) = state.search.remove_document(&id.to_string()).await {
        eprintln!("Failed to remove post from search index: {}", e);
    }
    let cache_key = format!("post:{}", id);
    #[cfg(feature = "cache")]
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear post cache: {}", e);
    }
    #[cfg(feature = "cache")]
    if let Err(e) = state.cache.delete("posts:*").await {
        eprintln!("Failed to clear posts cache: {}", e);
    }
    Ok(Json(ApiResponse::success(json!({
        "success": true,
        "message": "Post deleted successfully"
    }))))
}

/// Get posts by tag
pub async fn get_posts_by_tag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let posts = state
        .db_get_posts(page, limit, query.status, query.author, Some(tag.clone()), query.sort)
        .await?;
    let total = state.db_count_posts(Some(&tag)).await?;
    let total_pages = (total as f32 / limit as f32).ceil() as u32;
    let response = PostsResponse {
        posts: posts.iter().map(PostResponse::from).collect(),
        total,
        page,
        limit,
        total_pages,
    };
    Ok(Json(response))
}

/// Publish post
pub async fn publish_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let update_request = UpdatePostRequest {
        title: None,
        content: None,
        excerpt: None,
        slug: None,
        published: Some(true),
        tags: None,
        category: None,
        featured_image: None,
        meta_title: None,
        meta_description: None,
        status: Some(crate::models::PostStatus::Published),
        published_at: Some(chrono::Utc::now()),
    };
    let post = state.db_update_post(id, update_request).await?;
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_post(&post).await {
        eprintln!("Failed to update post in search index: {}", e);
    }
    Ok(Json(PostResponse::from(&post)))
}
 



