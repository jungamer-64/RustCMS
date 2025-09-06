//! Post Handlers
//!
//! Handles CRUD operations for blog posts and content management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::utils::{cache_key::CacheKeyBuilder};
use crate::utils::response_ext::ApiOk;
use crate::{
    models::{CreatePostRequest, Post, UpdatePostRequest},
    AppState, Result,
};
use crate::models::pagination::{normalize_page_limit, Paginated};

/// Post query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PostQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub status: Option<String>,
    pub author: Option<Uuid>,
    pub tag: Option<String>,
    pub sort: Option<String>,
}

/// Post response for API
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
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

// Posts list now directly returns Paginated<PostResponse> instead of wrapper

/// Create a new post
#[utoipa::path(
    post,
    path = "/api/v1/posts",
    tag = "Posts",
    security(("BearerAuth" = [])),
    request_body = CreatePostRequest,
    responses(
    (status=201, body=crate::utils::api_types::ApiResponse<PostResponse>, description="Post created",
        examples((
            "Created" = (
                summary = "新規作成成功",
                value = json!({
                    "success": true,
                    "data": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "title": "Hello World",
                        "content": "First post body...",
                        "author_id": "1d2e3f40-1111-2222-3333-444455556666",
                        "status": "draft",
                        "tags": ["intro"],
                        "created_at": "2025-09-05T12:00:00Z",
                        "updated_at": "2025-09-05T12:00:00Z"
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))
    ),
    (status=400, description="Validation error", body=crate::utils::api_types::ApiResponse<serde_json::Value>),
        (status=401, description="Unauthorized"),
        (status=500, description="Server error")
    )
)]
pub async fn create_post(
    State(state): State<AppState>,
    Json(request): Json<CreatePostRequest>,
) -> Result<impl IntoResponse> {
    let post = state.db_create_post(request).await?;
    #[cfg(feature = "cache")]
    state.cache_invalidate_prefix("posts:*").await;
    #[cfg(feature = "search")]
    state.search_index_post_safe(&post).await;
    Ok((StatusCode::CREATED, ApiOk(PostResponse::from(&post))))
}

/// Get post by ID
#[utoipa::path(
    get,
    path = "/api/v1/posts/{id}",
    tag = "Posts",
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<PostResponse>,
        examples((
            "Found" = (
                summary = "取得成功",
                value = json!({
                    "success": true,
                    "data": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "title": "Hello World",
                        "content": "First post body...",
                        "author_id": "1d2e3f40-1111-2222-3333-444455556666",
                        "status": "published",
                        "tags": ["intro"],
                        "created_at": "2025-09-05T12:00:00Z",
                        "updated_at": "2025-09-05T12:05:00Z"
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))
    ),
        (status=404, description="Post not found"),
        (status=500, description="Server error")
    )
)]
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let cache_key = CacheKeyBuilder::new("post").kv("id", id).build();
    #[cfg(feature = "cache")]
    {
        let response = state.cache_get_or_set::<PostResponse, _, _>(&cache_key, Duration::from_secs(300), || async {
            let post = state.db_get_post_by_id(id).await?;
            Ok(PostResponse::from(&post))
        }).await?;
    return Ok(ApiOk(response));
    }
    #[cfg(not(feature = "cache"))]
    {
        let post = state.db_get_post_by_id(id).await?;
    return Ok(ApiOk(PostResponse::from(&post)));
    }
}

/// Get all posts with pagination and filtering
#[utoipa::path(
    get,
    path = "/api/v1/posts",
    tag = "Posts",
    params(PostQuery),
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<Paginated<PostResponse>>,
        examples((
            "List" = (
                summary = "ページ付き一覧",
                value = json!({
                    "success": true,
                    "data": {
                        "items": [
                            {
                                "id": "550e8400-e29b-41d4-a716-446655440000",
                                "title": "Hello World",
                                "content": "First post body...",
                                "author_id": "1d2e3f40-1111-2222-3333-444455556666",
                                "status": "published",
                                "tags": ["intro"],
                                "created_at": "2025-09-05T12:00:00Z",
                                "updated_at": "2025-09-05T12:05:00Z"
                            }
                        ],
                        "page": 1,
                        "per_page": 20,
                        "total": 1,
                        "total_pages": 1
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))
    ),
        (status=500, description="Server error")
    )
)]
pub async fn get_posts(
    State(state): State<AppState>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let (page, limit) = normalize_page_limit(query.page, query.limit);
    let cache_key = CacheKeyBuilder::new("posts")
        .kv("page", page)
        .kv("limit", limit)
        .kv_opt("status", query.status.clone())
        .kv_opt("author", query.author)
        .kv_opt("tag", query.tag.clone())
        .kv_opt("sort", query.sort.clone())
        .build();
    #[cfg(feature = "cache")]
    {
        let response = state.cache_get_or_set::<Paginated<PostResponse>, _, _>(&cache_key, Duration::from_secs(300), || async {
            let posts = state
                .db_get_posts(page, limit, query.status.clone(), query.author, query.tag.clone(), query.sort.clone())
                .await?;
            let total = state
                .db_count_posts_filtered(query.status.clone(), query.author, query.tag.clone())
                .await?;
            let paginated = Paginated::new(posts.iter().map(PostResponse::from).collect(), total, page, limit);
            Ok(paginated)
        }).await?;
    return Ok(ApiOk(response));
    }
    #[cfg(not(feature = "cache"))]
    {
    let posts = state
            .db_get_posts(page, limit, query.status, query.author, query.tag, query.sort)
            .await?;
        let total = state
            .db_count_posts_filtered(query.status.clone(), query.author, query.tag.clone())
            .await?;
    let response = Paginated::new(posts.iter().map(PostResponse::from).collect(), total, page, limit);
    return Ok(ApiOk(response));
    }
}

/// Update post
#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}",
    tag = "Posts",
    request_body = UpdatePostRequest,
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<PostResponse>,
        examples((
            "Updated" = (
                summary = "更新成功",
                value = json!({
                    "success": true,
                    "data": {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "title": "Hello World (edited)",
                        "content": "Updated post body...",
                        "author_id": "1d2e3f40-1111-2222-3333-444455556666",
                        "status": "published",
                        "tags": ["intro"],
                        "created_at": "2025-09-05T12:00:00Z",
                        "updated_at": "2025-09-05T12:10:00Z"
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))
    ),
    (status=400, description="Validation error", body=crate::utils::api_types::ApiResponse<serde_json::Value>),
        (status=404, description="Post not found"),
        (status=500, description="Server error")
    )
)]
pub async fn update_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePostRequest>,
) -> Result<impl IntoResponse> {
    let post = state.db_update_post(id, request).await?;
    #[cfg(feature = "search")]
    state.search_index_post_safe(&post).await;
    #[cfg(feature = "cache")]
    state.invalidate_post_caches(id).await;
    Ok(ApiOk(PostResponse::from(&post)))
}

/// Delete post
#[utoipa::path(
    delete,
    path = "/api/v1/posts/{id}",
    tag = "Posts",
    security(("BearerAuth" = [])),
    responses(
        (status=200, description="Post deleted", examples((
            "Deleted" = (
                summary="削除成功",
                value = json!({
                    "success": true,
                    "data": {"message": "Post deleted successfully"},
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=404, description="Post not found"),
        (status=500, description="Server error")
    )
)]
pub async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.db_delete_post(id).await?;
    #[cfg(feature = "search")]
    state.search_remove_post_safe(id).await;
    #[cfg(feature = "cache")]
    state.invalidate_post_caches(id).await;
    Ok(ApiOk(json!({
        "success": true,
        "message": "Post deleted successfully"
    })))
}

/// Get posts by tag
#[utoipa::path(
    get,
    path = "/api/v1/posts/tag/{tag}",
    tag = "Posts",
    params(PostQuery),
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=Paginated<PostResponse>),
        (status=500, description="Server error")
    )
)]
pub async fn get_posts_by_tag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let (page, limit) = normalize_page_limit(query.page, query.limit);
    // Avoid moving fields twice
    let status = query.status.clone();
    let author = query.author;
    let sort = query.sort.clone();
    let posts = state
        .db_get_posts(page, limit, status.clone(), author, Some(tag.clone()), sort)
        .await?;
    let total = state
        .db_count_posts_filtered(status, author, Some(tag.clone()))
        .await?;
    let response = Paginated::new(posts.iter().map(PostResponse::from).collect(), total, page, limit);
    Ok(ApiOk(response))
}

/// Publish post
#[utoipa::path(
    post,
    path = "/api/v1/posts/{id}/publish",
    tag = "Posts",
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<PostResponse>, examples((
        "Published" = (
            summary="公開成功",
            value = json!({
                "success": true,
                "data": {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "title": "Hello World",
                    "content": "First post body...",
                    "author_id": "1d2e3f40-1111-2222-3333-444455556666",
                    "status": "published",
                    "tags": ["intro"],
                    "created_at": "2025-09-05T12:00:00Z",
                    "updated_at": "2025-09-05T12:05:00Z"
                },
                "message": null,
                "error": null,
                "validation_errors": null
            })
        )
    ))),
        (status=404, description="Post not found"),
        (status=500, description="Server error")
    )
)]
pub async fn publish_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let update_request = UpdatePostRequest::empty().publish_now();
    let post = state.db_update_post(id, update_request).await?;
    #[cfg(feature = "search")]
    state.search_index_post_safe(&post).await;
    Ok(ApiOk(PostResponse::from(&post)))
}
 



