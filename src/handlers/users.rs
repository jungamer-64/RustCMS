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
use uuid::Uuid;

use crate::utils::{common_types::UserInfo, cache_key::{entity_id_key, ListCacheKey}};
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

// Shared helper to fetch paginated users with caching and filters
pub(crate) async fn paginate_users(
    state: AppState,
    page: u32,
    limit: u32,
    role: Option<String>,
    active: Option<bool>,
    sort: Option<String>,
    cache_key: String,
) -> Result<Paginated<UserInfo>> {
    let filters = Arc::new((role.clone(), active, sort.clone()));
    let response: Paginated<UserInfo> = crate::utils::paginate::fetch_paginated_cached_with_filters(
        state.clone(),
        cache_key,
        crate::utils::cache_ttl::CACHE_TTL_DEFAULT,
        page,
        limit,
        filters,
        |f| {
            let state = state.clone();
            move || async move {
                let (role, active, sort) = (*f).clone();
                let users = state.db_get_users(page, limit, role, active, sort).await?;
                Ok(users.iter().map(UserInfo::from).collect())
            }
        },
        |f| {
            let state = state.clone();
            move || async move {
                let (role, active, _) = (*f).clone();
                state.db_count_users_filtered(role, active).await
            }
        },
    ).await?;
    Ok(response)
}

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
    // Build cache key (use helper to keep parity with posts)
    let cache_key = ListCacheKey::Users { page, limit, role: &query.role, active: query.active, sort: &query.sort }.to_cache_key();

    let resp = paginate_users(
        state.clone(),
        page,
        limit,
        query.role.clone(),
        query.active,
        query.sort.clone(),
        cache_key,
    )
    .await?;
    Ok(ApiOk(resp))
}

// Helper to build consistent cache key for users listing
// Deprecated: replaced by ListCacheKey::Users; kept temporarily if other modules reference it.
pub(crate) fn build_users_cache_key(page: u32, limit: u32, role: &Option<String>, active: Option<bool>, sort: &Option<String>) -> String {
    ListCacheKey::Users { page, limit, role, active, sort }.to_cache_key()
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
    let cache_key = entity_id_key("user", id);
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
    let fut = async {
        let _user = state.db_update_user(id, update_request).await?;
        #[cfg(feature = "search")]
        state.search_remove_user_safe(id).await;
        Ok::<(), crate::AppError>(())
    };
    crate::utils::response_ext::delete_with(fut, "User deactivated successfully").await
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
    let tag_opt = query.tag.clone();
    let cache_key = crate::handlers::posts::build_posts_cache_key(
        "user_posts:user",
        page,
        limit,
        &query.status,
        &Some(id),
        &tag_opt,
        &query.sort,
    );
    let resp = crate::handlers::posts::paginate_posts(
        state.clone(),
        page,
        limit,
        query.status.clone(),
        Some(id),
        tag_opt,
        query.sort.clone(),
        cache_key,
    ).await?;
    Ok(ApiOk(resp))
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
