//! Phase 6-D: Legacy handler (disabled with restructure_domain)
#![cfg(not(feature = "restructure_domain"))]
//! Comment Handlers - CQRS統合版
//!
//! Comment Commands/Queriesを呼び出す薄い層（Phase 4）

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::app::AppState;
use crate::application::comment::{
    CommentDto, CreateComment, CreateCommentRequest, GetCommentById, ListCommentsByPost,
    PublishComment,
};
use crate::application::ports::repositories::CommentRepository;
use crate::domain::comment::CommentId;
use crate::domain::post::PostId;
use crate::error::AppError;

// ============================================================================
// Request/Response Types
// ============================================================================

/// コメント一覧レスポンス
#[derive(Debug, Serialize)]
pub struct ListCommentsResponse {
    pub comments: Vec<CommentDto>,
    pub total: usize,
}

// ============================================================================
// Handlers（薄い層 - Use Cases呼び出しのみ）
// ============================================================================

/// コメント作成ハンドラ
///
/// POST /api/v2/comments
#[cfg(feature = "restructure_domain")]
pub async fn create_comment(
    State(state): State<AppState>,
    Json(request): Json<CreateCommentRequest>,
) -> Result<Json<CommentDto>, AppError> {
    let repo = state.comment_repository();
    let use_case = CreateComment::new(repo);

    let comment_dto = use_case
        .execute(request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(comment_dto))
}

/// コメント取得ハンドラ
///
/// GET /api/v2/comments/:id
#[cfg(feature = "restructure_domain")]
pub async fn get_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CommentDto>, AppError> {
    let comment_id = CommentId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid comment ID: {}", e)))?;

    let repo = state.comment_repository();
    let use_case = GetCommentById::new(repo);

    let comment_dto = use_case
        .execute(comment_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(Json(comment_dto))
}

/// コメント公開ハンドラ
///
/// POST /api/v2/comments/:id/publish
#[cfg(feature = "restructure_domain")]
pub async fn publish_comment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CommentDto>, AppError> {
    let comment_id = CommentId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid comment ID: {}", e)))?;

    let repo = state.comment_repository();
    let use_case = PublishComment::new(repo);

    let comment_dto = use_case
        .execute(comment_id)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(comment_dto))
}

/// 投稿のコメント一覧ハンドラ
///
/// GET /api/v2/posts/:post_id/comments
#[cfg(feature = "restructure_domain")]
pub async fn list_comments_by_post(
    State(state): State<AppState>,
    Path(post_id): Path<String>,
) -> Result<Json<ListCommentsResponse>, AppError> {
    let post_id = PostId::from_string(&post_id)
        .map_err(|e| AppError::BadRequest(format!("Invalid post ID: {}", e)))?;

    let repo = state.comment_repository();
    let use_case = ListCommentsByPost::new(repo);

    let comments = use_case
        .execute(post_id)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let total = comments.len();

    Ok(Json(ListCommentsResponse { comments, total }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_comments_response() {
        let response = ListCommentsResponse {
            comments: vec![],
            total: 0,
        };
        assert_eq!(response.total, 0);
    }
}
