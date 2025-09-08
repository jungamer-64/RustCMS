//! User Handlers
//!
//!
//! Handles user management operations

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use utoipa::ToSchema;
use serde_json::json;
use uuid::Uuid;

use crate::utils::{common_types::UserInfo, cache_key::CacheKeyBuilder};
use std::sync::Arc;
use crate::utils::response_ext::ApiOk;
use crate::{models::UpdateUserRequest, AppState, Result};
use crate::models::pagination::{normalize_page_limit, Paginated};

/// User query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct UserQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub role: Option<String>,
    pub active: Option<bool>,
    pub sort: Option<String>,
}

// Users list now returns Paginated<UserInfo> directly

/// Get all users with pagination
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "Users",
    params(UserQuery),
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=Paginated<UserInfo>, examples((
        "UserList" = (
            summary="ユーザ一覧",
            value = json!({
                "success": true,
                "data": {
                    "items": [{
                        "id": "1d2e3f40-1111-2222-3333-444455556666",
                        "username": "alice",
                        "email": "alice@example.com",
                        "role": "subscriber"
                    }],
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
    ))),
        (status=500, description="Server error")
    )
)]
pub async fn get_users(
    State(state): State<AppState>,
    Query(query): Query<UserQuery>,
) -> Result<impl IntoResponse> {
    let (page, limit) = normalize_page_limit(query.page, query.limit);

    // Build cache key
    let cache_key = crate::utils::cache_key::build_list_cache_key(
        "users",
        page,
        limit,
        &[
            ("role", query.role.clone()),
            ("active", query.active.map(|b| b.to_string())),
            ("sort", query.sort.clone()),
        ],
    );

    let filters = Arc::new((query.role.clone(), query.active, query.sort.clone()));
    let f1 = filters.clone();
    let f2 = filters.clone();

    let state1 = state.clone();
    let state2 = state.clone();

    let response = crate::utils::paginate::fetch_paginated_cached(
        state.clone(),
        cache_key,
        crate::utils::cache_ttl::CACHE_TTL_DEFAULT,
        page,
        limit,
        move || async move {
            let (role1, active1, sort1) = (*f1).clone();
            let users = state1
                .db_get_users(page, limit, role1, active1, sort1)
                .await?;
            Ok(users.iter().map(UserInfo::from).collect())
        },
        move || async move {
            let (role2, active2, _) = (*f2).clone();
            state2.db_count_users_filtered(role2, active2).await
        },
    ).await?;
    Ok(ApiOk(response))
}

/// Get user by ID
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "Users",
    security(("BearerAuth" = [])),
    responses(
        (status=200, body=UserInfo, examples((
            "User" = (
                summary="ユーザ取得",
                value = json!({
                    "success": true,
                    "data": {
                        "id": "1d2e3f40-1111-2222-3333-444455556666",
                        "username": "alice",
                        "email": "alice@example.com",
                        "role": "subscriber"
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=404, description="User not found"),
        (status=500, description="Server error")
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Try cache first
    let cache_key = CacheKeyBuilder::new("user").kv("id", id).build();
    let info: UserInfo = crate::utils::cache_helpers::cache_or_compute(
        state.clone(),
        &cache_key,
        crate::utils::cache_ttl::CACHE_TTL_LONG,
        move || async move {
            let user = state.db_get_user_by_id(id).await?;
            Ok(UserInfo::from(&user))
        },
    ).await?;
    Ok(ApiOk(info))
}

/// Update user
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "Users",
    security(("BearerAuth" = [])),
    responses(
        (status=200, body=UserInfo, examples((
            "Updated" = (
                summary="更新成功",
                value = json!({
                    "success": true,
                    "data": {
                        "id": "1d2e3f40-1111-2222-3333-444455556666",
                        "username": "alice",
                        "email": "alice@example.com",
                        "role": "editor"
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
    (status=400, description="Validation error", body=crate::utils::api_types::ApiResponse<serde_json::Value>),
        (status=404, description="User not found"),
        (status=500, description="Server error")
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse> {
    // Update in database (record DB timing)
    let user = state.db_update_user(id, request).await?;

    // Update search index
    #[cfg(feature = "search")]
    state.search_index_user_safe(&user).await;

    // Clear cache
    #[cfg(feature = "cache")]
    state.invalidate_user_caches(id).await;

    Ok(ApiOk(UserInfo::from(&user)))
}

/// Delete user (soft delete by deactivating)
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "Users",
    security(("BearerAuth" = [])),
    responses(
        (status=200, description="User deactivated", examples((
            "Deactivated" = (
                summary="無効化成功",
                value = json!({
                    "success": true,
                    "data": {"message": "User deactivated successfully"},
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=404, description="User not found"),
        (status=500, description="Server error")
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Soft delete by deactivating
    let update_request = UpdateUserRequest::deactivate();

    let _user = state.db_update_user(id, update_request).await?;

    // Remove from search index
    #[cfg(feature = "search")]
    state.search_remove_user_safe(id).await;

    // Clear cache
    #[cfg(feature = "cache")]
    state.invalidate_user_caches(id).await;

    Ok(ApiOk(json!({
        "success": true,
        "message": "User deactivated successfully"
    })))
}

/// Get user's posts
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/posts",
    tag = "Users",
    params(crate::handlers::posts::PostQuery),
    security(("BearerAuth" = [])),
    responses(
    (status=200, body=crate::utils::api_types::ApiResponse<crate::models::pagination::Paginated<crate::handlers::posts::PostDto>>, examples((
        "UserPosts" = (
            summary="ユーザ投稿一覧",
            value = json!({
                "success": true,
                "data": {
                    "items": [{
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "title": "Hello World",
                        "content": "First post body...",
                        "author_id": "1d2e3f40-1111-2222-3333-444455556666",
                        "status": "published",
                        "tags": ["intro"],
                        "created_at": "2025-09-05T12:00:00Z",
                        "updated_at": "2025-09-05T12:05:00Z"
                    }],
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
    ))),
        (status=404, description="User not found"),
        (status=500, description="Server error")
    )
)]
pub async fn get_user_posts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<crate::handlers::posts::PostQuery>,
) -> Result<impl IntoResponse> {
    let (page, limit) = normalize_page_limit(query.page, query.limit);
    let cache_key = crate::utils::cache_key::build_list_cache_key(
        "user_posts:user",
        page,
        limit,
        &[
            ("user", Some(id.to_string())),
            ("status", query.status.clone()),
            ("tag", query.tag.clone()),
            ("sort", query.sort.clone()),
        ],
    );
    let filters = Arc::new((query.status.clone(), query.tag.clone(), query.sort.clone()));
    let f1 = filters.clone();
    let f2 = filters.clone();

    let state1 = state.clone();
    let state2 = state.clone();

    let response = crate::utils::paginate::fetch_paginated_cached(
        state.clone(),
        cache_key,
        crate::utils::cache_ttl::CACHE_TTL_DEFAULT,
        page,
        limit,
        move || async move {
            let (status1, tag1, sort1) = (*f1).clone();
            let posts = state1
                .db_get_posts(page, limit, status1, Some(id), tag1, sort1)
                .await?;
            Ok(posts.iter().map(crate::handlers::posts::PostDto::from).collect())
        },
        move || async move {
            let (status2, tag2, _) = (*f2).clone();
            state2.db_count_posts_filtered(status2, Some(id), tag2).await
        },
    ).await?;
    return Ok(ApiOk(response));
}

/// Change user role (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/role",
    tag = "Users",
    security(("BearerAuth" = [])),
    responses(
        (status=200, body=UserInfo, examples((
            "RoleChanged" = (
                summary="ロール変更成功",
                value = json!({
                    "success": true,
                    "data": {
                        "id": "1d2e3f40-1111-2222-3333-444455556666",
                        "username": "alice",
                        "email": "alice@example.com",
                        "role": "admin"
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=400, description="Invalid role value"),
        (status=404, description="User not found"),
        (status=500, description="Server error")
    )
)]
pub async fn change_user_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<serde_json::Value>,
) -> Result<impl IntoResponse> {
    let new_role = request["role"]
        .as_str()
        .ok_or_else(|| crate::AppError::BadRequest("Missing role field".to_string()))?;

    let role_enum = match new_role {
        "admin" => crate::models::UserRole::Admin,
        "editor" => crate::models::UserRole::Editor,
        "subscriber" => crate::models::UserRole::Subscriber,
        _ => return Err(crate::AppError::BadRequest("Invalid role".to_string())),
    };

    let update_request = UpdateUserRequest::with_role(role_enum);

    let user = state.db_update_user(id, update_request).await?;

    // Clear cache
    #[cfg(feature = "cache")]
    state.invalidate_user_caches(id).await;

    Ok(ApiOk(UserInfo::from(&user)))
}
