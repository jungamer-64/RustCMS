//! Post Handlers
//! 
//! Handles CRUD operations for blog posts and content management

use axum::{
    response::{IntoResponse, Json},
    extract::{State, Path, Query},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    AppState, Result,
    models::{Post, CreatePostRequest, UpdatePostRequest},
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
    // Create post in database with validated data
    let post = state.database.create_post(request).await?;
    
    // Index post for search (optional feature)
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_post(&post).await {
        // Log error but don't fail the creation
        eprintln!("Failed to index post for search: {}", e);
    }

    // Clear relevant cache entries (optional feature)
    #[cfg(feature = "cache")]
    if let Err(e) = state.cache.delete("posts:*").await {
        eprintln!("Failed to clear post cache: {}", e);
    }

    Ok((StatusCode::CREATED, Json(PostResponse::from(&post))))
}

/// Get post by ID
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Try cache first
    let cache_key = format!("post:{}", id);
    if let Ok(Some(cached)) = state.cache.get::<PostResponse>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Get from database
    let post = state.database.get_post_by_id(id).await?;
    let response = PostResponse::from(&post);
    
    // Cache the result
    if let Err(e) = state.cache.set(cache_key, &response, Some(Duration::from_secs(300))).await {
        eprintln!("Failed to cache post: {}", e);
    }

    Ok(Json(response))
}

/// Get all posts with pagination and filtering
pub async fn get_posts(
    State(state): State<AppState>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    
    // Build cache key based on query parameters
    let cache_key = format!(
        "posts:page:{}:limit:{}:status:{}:author:{}:tag:{}:sort:{}", 
        page,
        limit,
        query.status.as_deref().unwrap_or("all"),
        query.author.map(|a| a.to_string()).unwrap_or_else(|| "all".to_string()),
        query.tag.as_deref().unwrap_or("all"),
        query.sort.as_deref().unwrap_or("created_at")
    );

    // Try cache first
    if let Ok(Some(cached)) = state.cache.get::<PostsResponse>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Get from database
    let posts = state.database.get_posts(
        page,
        limit,
        query.status,
        query.author,
        query.tag,
        query.sort,
    ).await?;
    
    let total = state.database.count_posts(None).await?;
    let total_pages = (total as f32 / limit as f32).ceil() as u32;
    
    let response = PostsResponse {
        posts: posts.iter().map(PostResponse::from).collect(),
        total,
        page,
        limit,
        total_pages,
    };
    
    // Cache the result for 5 minutes
    if let Err(e) = state.cache.set(cache_key, &response, Some(Duration::from_secs(300))).await {
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
    // Update in database
    let post = state.database.update_post(id, request).await?;
    
    // Update search index
    if let Err(e) = state.search.index_post(&post).await {
        eprintln!("Failed to update post in search index: {}", e);
    }

    // Clear cache
    let cache_key = format!("post:{}", id);
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear post cache: {}", e);
    }
    
    // Clear posts list cache
    if let Err(e) = state.cache.delete("posts:*").await {
        eprintln!("Failed to clear posts cache: {}", e);
    }

    Ok(Json(PostResponse::from(&post)))
}

/// Delete post
pub async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Delete from database
    state.database.delete_post(id).await?;
    
    // Remove from search index
    if let Err(e) = state.search.remove_document(&id.to_string()).await {
        eprintln!("Failed to remove post from search index: {}", e);
    }

    // Clear cache
    let cache_key = format!("post:{}", id);
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear post cache: {}", e);
    }
    
    // Clear posts list cache
    if let Err(e) = state.cache.delete("posts:*").await {
        eprintln!("Failed to clear posts cache: {}", e);
    }

    Ok(Json(json!({
        "success": true,
        "message": "Post deleted successfully"
    })))
}

/// Get posts by tag
pub async fn get_posts_by_tag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    
    let posts = state.database.get_posts(
        page,
        limit,
        query.status,
        query.author,
        Some(tag.clone()),
        query.sort,
    ).await?;
    
    let total = state.database.count_posts(Some(&tag)).await?;
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

    let post = state.database.update_post(id, update_request).await?;
    
    // Update search index
    if let Err(e) = state.search.index_post(&post).await {
        eprintln!("Failed to update post in search index: {}", e);
    }

    Ok(Json(PostResponse::from(&post)))
}
