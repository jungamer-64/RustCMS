//! Post Handlers
//!
//! Handles CRUD operations for blog posts and content management

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::dto_from_model; // macro
use crate::models::pagination::{Paginated, normalize_page_limit};
use crate::utils::cache_key::{ListCacheKey, entity_id_key};
use crate::utils::crud;
use crate::utils::response_ext::ApiOk;
use crate::utils::response_ext::delete_with;
use crate::{
    AppState, Result,
    models::{CreatePostRequest, Post, UpdatePostRequest},
};

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

/// Post DTO for API (single post payload)
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct PostDto {
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

dto_from_model!(PostDto, Post, |p| PostDto {
    id: p.id,
    title: p.title.clone(),
    content: p.content.clone(),
    excerpt: p.excerpt.clone(),
    status: p.status.clone(),
    author_id: p.author_id,
    tags: p.tags.clone(),
    metadata: serde_json::json!({
        "meta_title": p.meta_title,
        "meta_description": p.meta_description,
        "categories": p.categories
    }),
    created_at: p.created_at,
    updated_at: p.updated_at,
    published_at: p.published_at,
});

// Posts list now directly returns Paginated<PostDto> instead of wrapper

// Shared helper to fetch paginated posts with caching and filters
#[allow(clippy::too_many_arguments)]
pub(crate) async fn paginate_posts(
    state: AppState,
    page: u32,
    limit: u32,
    status: Option<String>,
    author: Option<Uuid>,
    tag: Option<String>,
    sort: Option<String>,
    // Posts list now directly returns Paginated<PostDto> instead of wrapper
    cache_key: String,
) -> Result<Paginated<PostDto>> {
    use std::sync::Arc;
    let filters = Arc::new((status.clone(), author, tag.clone(), sort.clone()));
    let response: Paginated<PostDto> = crate::utils::paginate::fetch_paginated_cached_mapped(
        state.clone(),
        cache_key,
        crate::utils::cache_ttl::CACHE_TTL_DEFAULT,
        page,
        limit,
        {
            let state = state.clone();
            let f = filters.clone();
            move || async move {
                let (status, author, tag, sort) = (*f).clone();
                state
                    .db_get_posts(page, limit, status, author, tag, sort)
                    .await
            }
        },
        {
            let state = state.clone();
            let f = filters.clone();
            move || async move {
                let (status, author, tag, _) = (*f).clone();
                state.db_count_posts_filtered(status, author, tag).await
            }
        },
    |p| PostDto::from(p),
    )
    .await?;
    Ok(response)
}

/// Create a new post
#[utoipa::path(
    post,
    path = "/api/v1/posts",
    tag = "Posts",
    security(("BearerAuth" = [])),
    request_body = CreatePostRequest,
    responses(
    (status=201, body=crate::utils::api_types::ApiResponse<PostDto>, description="Post created",
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
    // Perform create then (if feature enabled) index.
    #[cfg(feature = "search")]
    let hook = Some(|model: &Post, st: AppState| {
        let m = model.clone();
        async move {
            st.search_index_entity_safe(crate::utils::search_index::SearchEntity::Post(&m))
                .await;
        }
    });
    #[cfg(not(feature = "search"))]
    let hook: Option<fn(&Post, AppState) -> _> = None;

    let (status, api_ok) = crud::create_entity(
        state.clone(),
        request,
        |st, req| async move { st.db_create_post(req).await },
        |m: &Post| PostDto::from(m),
        hook,
    )
    .await?;
    Ok((status, api_ok))
}

/// Get post by ID
///
/// # Errors
/// - 指定 ID の投稿が存在しない場合。
/// - キャッシュまたは DB へのアクセスに失敗した場合。
#[utoipa::path(
    get,
    path = "/api/v1/posts/{id}",
    tag = "Posts",
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<PostDto>,
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
    let cache_key = entity_id_key("post", id);
    let dto: PostDto = crate::utils::cache_helpers::cache_or_compute(
        state.clone(),
        &cache_key,
        crate::utils::cache_ttl::CACHE_TTL_DEFAULT,
        move || async move {
            let post = state.db_get_post_by_id(id).await?;
            Ok(PostDto::from(&post))
        },
    )
    .await?;
    Ok(ApiOk(dto))
}

/// Get all posts with pagination and filtering
///
/// # Errors
/// - クエリ条件に合致する投稿の取得で DB アクセスに失敗した場合。
/// - キャッシュ操作に失敗した場合。
#[utoipa::path(
    get,
    path = "/api/v1/posts",
    tag = "Posts",
    params(PostQuery),
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<Paginated<PostDto>>,
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
    let cache_key = ListCacheKey::Posts {
        page,
        limit,
        status: &query.status,
        author: &query.author,
        tag: &query.tag,
        sort: &query.sort,
    }
    .to_cache_key();
    let resp = paginate_posts(
        state.clone(),
        page,
        limit,
        query.status.clone(),
        query.author,
        query.tag.clone(),
        query.sort.clone(),
        cache_key,
    )
    .await?;
    Ok(ApiOk(resp))
}

/// Update post
///
/// # Errors
/// - 指定 ID の投稿が存在しない場合。
/// - バリデーションまたは DB 更新に失敗した場合。
#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}",
    tag = "Posts",
    request_body = UpdatePostRequest,
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<PostDto>,
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
    let api_ok = crud::update_entity(
        state.clone(),
        id,
        request,
        |st, pid, req| async move { st.db_update_post(pid, req).await },
        |p: &crate::models::post::Post| PostDto::from(p),
        Some(|p: crate::models::post::Post, st: AppState| async move {
            #[cfg(feature = "search")]
            st.search_index_entity_safe(crate::utils::search_index::SearchEntity::Post(&p))
                .await;
        }),
    )
    .await?;
    Ok(api_ok)
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
/// Delete post by ID.
///
/// # Errors
/// - 指定 ID の投稿がない場合。
/// - DB 操作に失敗した場合。
pub async fn delete_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let fut = async {
        state.db_delete_post(id).await?;
        #[cfg(feature = "search")]
        state.search_remove_post_safe(id).await;
        Ok::<(), crate::AppError>(())
    };
    delete_with(fut, "Post deleted successfully").await
}

/// Get posts by tag
#[utoipa::path(
    get,
    path = "/api/v1/posts/tag/{tag}",
    tag = "Posts",
    params(PostQuery),
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<Paginated<PostDto>>),
        (status=500, description="Server error")
    )
)]
pub async fn get_posts_by_tag(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(query): Query<PostQuery>,
) -> Result<impl IntoResponse> {
    let (page, limit) = normalize_page_limit(query.page, query.limit);
    let tag_opt = Some(tag.clone());
    let cache_key = crate::utils::cache_key::build_list_cache_key(
        "posts:tag",
        page,
        limit,
        &[
            ("status", query.status.clone()),
            ("author", query.author.map(|u| u.to_string())),
            ("tag", tag_opt.clone()),
            ("sort", query.sort.clone()),
        ],
    );
    let resp = paginate_posts(
        state.clone(),
        page,
        limit,
        query.status.clone(),
        query.author,
        Some(tag.clone()),
        query.sort.clone(),
        cache_key,
    )
    .await?;
    Ok(ApiOk(resp))
}

/// Publish post
#[utoipa::path(
    post,
    path = "/api/v1/posts/{id}/publish",
    tag = "Posts",
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<PostDto>, examples((
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
    Ok(ApiOk(PostDto::from(&post)))
}
