//! User Handlers
//! 
//! Handles user management operations

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
    models::{User, UpdateUserRequest},
    handlers::auth::UserInfo,
};

/// User query parameters
#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub role: Option<String>,
    pub active: Option<bool>,
    pub sort: Option<String>,
}

/// Users response
#[derive(Debug, Serialize, Deserialize)]
pub struct UsersResponse {
    pub users: Vec<UserInfo>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

/// Get all users with pagination
pub async fn get_users(
    State(state): State<AppState>,
    Query(query): Query<UserQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    
    // Build cache key
    let cache_key = format!(
        "users:page:{}:limit:{}:role:{}:active:{}:sort:{}", 
        page,
        limit,
        query.role.as_deref().unwrap_or("all"),
        query.active.map(|a| a.to_string()).unwrap_or_else(|| "all".to_string()),
        query.sort.as_deref().unwrap_or("created_at")
    );

    // Try cache first
    if let Ok(Some(cached)) = state.cache.get::<UsersResponse>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Get from database
    let users = state.database.get_users(
        page,
        limit,
        query.role,
        query.active,
        query.sort,
    ).await?;
    
    let total = state.database.count_users().await?;
    let total_pages = (total as f32 / limit as f32).ceil() as u32;
    
    let response = UsersResponse {
        users: users.iter().map(UserInfo::from).collect(),
        total,
        page,
        limit,
        total_pages,
    };
    
    // Cache for 5 minutes
    if let Err(e) = state.cache.set(cache_key, &response, Some(Duration::from_secs(300))).await {
        eprintln!("Failed to cache users: {}", e);
    }

    Ok(Json(response))
}

/// Get user by ID
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Try cache first
    let cache_key = format!("user:{}", id);
    if let Ok(Some(cached)) = state.cache.get::<UserInfo>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Get from database
    let user = state.database.get_user_by_id(id).await?;
    let response = UserInfo::from(&user);
    
    // Cache for 10 minutes
    if let Err(e) = state.cache.set(cache_key, &response, Some(Duration::from_secs(600))).await {
        eprintln!("Failed to cache user: {}", e);
    }

    Ok(Json(response))
}

/// Update user
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse> {
    // Update in database
    let user = state.database.update_user(id, request).await?;
    
    // Update search index
    if let Err(e) = state.search.index_user(&user).await {
        eprintln!("Failed to update user in search index: {}", e);
    }

    // Clear cache
    let cache_key = format!("user:{}", id);
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear user cache: {}", e);
    }
    
    // Clear users list cache
    if let Err(e) = state.cache.delete("users:*").await {
        eprintln!("Failed to clear users cache: {}", e);
    }

    Ok(Json(UserInfo::from(&user)))
}

/// Delete user (soft delete by deactivating)
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Soft delete by deactivating
    let update_request = UpdateUserRequest {
        username: None,
        email: None,
        first_name: None,
        last_name: None,
        role: None,
        is_active: Some(false),
    };

    let user = state.database.update_user(id, update_request).await?;
    
    // Remove from search index
    if let Err(e) = state.search.remove_document(&id.to_string()).await {
        eprintln!("Failed to remove user from search index: {}", e);
    }

    // Clear cache
    let cache_key = format!("user:{}", id);
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear user cache: {}", e);
    }

    Ok(Json(json!({
        "success": true,
        "message": "User deactivated successfully"
    })))
}

/// Get user's posts
pub async fn get_user_posts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<crate::handlers::posts::PostQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    
    // Get posts by author
    let posts = state.database.get_posts(
        page,
        limit,
        query.status,
        Some(id), // author_id
        query.tag,
        query.sort,
    ).await?;
    
    let total = state.database.count_posts_by_author(id).await?;
    let total_pages = (total as f32 / limit as f32).ceil() as u32;
    
    let response = crate::handlers::posts::PostsResponse {
        posts: posts.iter().map(crate::handlers::posts::PostResponse::from).collect(),
        total,
        page,
        limit,
        total_pages,
    };

    Ok(Json(response))
}

/// Change user role (admin only)
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

    let update_request = UpdateUserRequest {
        username: None,
        email: None,
        first_name: None,
        last_name: None,
        role: Some(role_enum),
        is_active: None,
    };

    let user = state.database.update_user(id, update_request).await?;
    
    // Clear cache
    let cache_key = format!("user:{}", id);
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to clear user cache: {}", e);
    }

    Ok(Json(UserInfo::from(&user)))
}
