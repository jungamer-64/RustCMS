//! Phase 6-D: Legacy handler (disabled with restructure_domain)
#![cfg(not(feature = "restructure_domain"))]
//! Post Handlers - CQRS統合版
//!
//! Post Commands/Queriesを呼び出す薄い層（Phase 4）

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::app::AppState;
use crate::application::ports::repositories::PostRepository;
use crate::application::post::{
    ArchivePost, CreatePost, CreatePostRequest, GetPostById, ListPosts, PostDto, PublishPost,
    UpdatePost, UpdatePostRequest,
};
use crate::domain::post::PostId;
use crate::error::AppError;

// ============================================================================
// Request/Response Types
// ============================================================================

/// ページネーションパラメータ
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

/// 投稿一覧レスポンス
#[derive(Debug, Serialize)]
pub struct ListPostsResponse {
    pub posts: Vec<PostDto>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

// ============================================================================
// Handlers（薄い層 - Use Cases呼び出しのみ）
// ============================================================================

/// 投稿作成ハンドラ
///
/// POST /api/v2/posts
#[cfg(feature = "restructure_domain")]
pub async fn create_post(
    State(state): State<AppState>,
    Json(request): Json<CreatePostRequest>,
) -> Result<Json<PostDto>, AppError> {
    let repo = state.post_repository();
    let use_case = CreatePost::new(repo);

    let post_dto = use_case
        .execute(request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(post_dto))
}

/// 投稿取得ハンドラ
///
/// GET /api/v2/posts/:id
#[cfg(feature = "restructure_domain")]
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PostDto>, AppError> {
    let post_id = PostId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid post ID: {}", e)))?;

    let repo = state.post_repository();
    let use_case = GetPostById::new(repo);

    let post_dto = use_case
        .execute(post_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(Json(post_dto))
}

/// 投稿更新ハンドラ
///
/// PUT /api/v2/posts/:id
#[cfg(feature = "restructure_domain")]
pub async fn update_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePostRequest>,
) -> Result<Json<PostDto>, AppError> {
    let post_id = PostId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid post ID: {}", e)))?;

    let repo = state.post_repository();
    let use_case = UpdatePost::new(repo);

    let post_dto = use_case
        .execute(post_id, request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(post_dto))
}

/// 投稿公開ハンドラ
///
/// POST /api/v2/posts/:id/publish
#[cfg(feature = "restructure_domain")]
pub async fn publish_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PostDto>, AppError> {
    let post_id = PostId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid post ID: {}", e)))?;

    let repo = state.post_repository();
    let use_case = PublishPost::new(repo);

    let post_dto = use_case
        .execute(post_id)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(post_dto))
}

/// 投稿アーカイブハンドラ
///
/// POST /api/v2/posts/:id/archive
#[cfg(feature = "restructure_domain")]
pub async fn archive_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let post_id = PostId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid post ID: {}", e)))?;

    let repo = state.post_repository();
    let use_case = ArchivePost::new(repo);

    use_case
        .execute(post_id)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// 投稿一覧ハンドラ
///
/// GET /api/v2/posts
#[cfg(feature = "restructure_domain")]
pub async fn list_posts(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ListPostsResponse>, AppError> {
    let repo = state.post_repository();
    let use_case = ListPosts::new(repo);

    let (posts, total) = use_case
        .execute(params.page, params.per_page)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(ListPostsResponse {
        posts,
        total,
        page: params.page,
        per_page: params.per_page,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults() {
        let params: PaginationParams = serde_json::from_str("{}").expect("Failed to deserialize");
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
    }
}
