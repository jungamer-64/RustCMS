//! Phase 6-D: Legacy handler (disabled with restructure_domain)
#![cfg(not(feature = "restructure_domain"))]
//! User Handlers - CQRS統合版
//!
//! User Commands/Queriesを呼び出す薄い層（Phase 4）

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::app::AppState;
use crate::application::ports::repositories::UserRepository;
use crate::application::user::{
    CreateUserRequest, GetUserById, ListUsers, RegisterUser, SuspendUser, UpdateUser,
    UpdateUserRequest, UserDto,
};
use crate::domain::user::UserId;
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

/// ユーザー一覧レスポンス
#[derive(Debug, Serialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserDto>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

// ============================================================================
// Handlers（薄い層 - Use Cases呼び出しのみ）
// ============================================================================

/// ユーザー登録ハンドラ
///
/// POST /api/v2/users
#[cfg(feature = "restructure_domain")]
pub async fn register_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<UserDto>, AppError> {
    let repo = state.user_repository();
    let use_case = RegisterUser::new(repo);

    let user_dto = use_case
        .execute(request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(user_dto))
}

/// ユーザー取得ハンドラ
///
/// GET /api/v2/users/:id
#[cfg(feature = "restructure_domain")]
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UserDto>, AppError> {
    let user_id = UserId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid user ID: {}", e)))?;

    let repo = state.user_repository();
    let use_case = GetUserById::new(repo);

    let user_dto = use_case
        .execute(user_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(Json(user_dto))
}

/// ユーザー更新ハンドラ
///
/// PUT /api/v2/users/:id
#[cfg(feature = "restructure_domain")]
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserDto>, AppError> {
    let user_id = UserId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid user ID: {}", e)))?;

    let repo = state.user_repository();
    let use_case = UpdateUser::new(repo);

    let user_dto = use_case
        .execute(user_id, request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(user_dto))
}

/// ユーザー停止ハンドラ
///
/// POST /api/v2/users/:id/suspend
#[cfg(feature = "restructure_domain")]
pub async fn suspend_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let user_id = UserId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid user ID: {}", e)))?;

    let repo = state.user_repository();
    let use_case = SuspendUser::new(repo);

    use_case
        .execute(user_id)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// ユーザー一覧ハンドラ
///
/// GET /api/v2/users
#[cfg(feature = "restructure_domain")]
pub async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ListUsersResponse>, AppError> {
    let repo = state.user_repository();
    let use_case = ListUsers::new(repo);

    let (users, total) = use_case
        .execute(params.page, params.per_page)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(ListUsersResponse {
        users,
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

    #[test]
    fn test_pagination_custom() {
        let params: PaginationParams =
            serde_json::from_str(r#"{"page": 2, "per_page": 50}"#).expect("Failed to deserialize");
        assert_eq!(params.page, 2);
        assert_eq!(params.per_page, 50);
    }
}
