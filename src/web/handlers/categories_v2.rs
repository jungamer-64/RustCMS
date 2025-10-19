//! Category Handlers - CQRS統合版
//!
//! Phase 6-D: Legacy category handlers (disabled with restructure_domain)
//! Category Commands/Queriesを呼び出す薄い層（Phase 4）
#![cfg(not(feature = "restructure_domain"))]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::app::AppState;
use crate::application::category::{
    CategoryDto, CreateCategory, CreateCategoryRequest, DeactivateCategory, GetCategoryById,
    ListCategories, UpdateCategory, UpdateCategoryRequest,
};
use crate::application::ports::repositories::CategoryRepository;
use crate::domain::category::CategoryId;
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

/// カテゴリ一覧レスポンス
#[derive(Debug, Serialize)]
pub struct ListCategoriesResponse {
    pub categories: Vec<CategoryDto>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

// ============================================================================
// Handlers（薄い層 - Use Cases呼び出しのみ）
// ============================================================================

/// カテゴリ作成ハンドラ
///
/// POST /api/v2/categories
#[cfg(feature = "restructure_domain")]
pub async fn create_category(
    State(state): State<AppState>,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Json<CategoryDto>, AppError> {
    let repo = state.category_repository();
    let use_case = CreateCategory::new(repo);

    let category_dto = use_case
        .execute(request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(category_dto))
}

/// カテゴリ取得ハンドラ
///
/// GET /api/v2/categories/:id
#[cfg(feature = "restructure_domain")]
pub async fn get_category(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CategoryDto>, AppError> {
    let category_id = CategoryId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid category ID: {}", e)))?;

    let repo = state.category_repository();
    let use_case = GetCategoryById::new(repo);

    let category_dto = use_case
        .execute(category_id)
        .await
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    Ok(Json(category_dto))
}

/// カテゴリ更新ハンドラ
///
/// PUT /api/v2/categories/:id
#[cfg(feature = "restructure_domain")]
pub async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryDto>, AppError> {
    let category_id = CategoryId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid category ID: {}", e)))?;

    let repo = state.category_repository();
    let use_case = UpdateCategory::new(repo);

    let category_dto = use_case
        .execute(category_id, request)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(Json(category_dto))
}

/// カテゴリ無効化ハンドラ
///
/// POST /api/v2/categories/:id/deactivate
#[cfg(feature = "restructure_domain")]
pub async fn deactivate_category(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let category_id = CategoryId::from_string(&id)
        .map_err(|e| AppError::BadRequest(format!("Invalid category ID: {}", e)))?;

    let repo = state.category_repository();
    let use_case = DeactivateCategory::new(repo);

    use_case
        .execute(category_id)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// カテゴリ一覧ハンドラ
///
/// GET /api/v2/categories
#[cfg(feature = "restructure_domain")]
pub async fn list_categories(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ListCategoriesResponse>, AppError> {
    let repo = state.category_repository();
    let use_case = ListCategories::new(repo);

    let (categories, total) = use_case
        .execute(params.page, params.per_page)
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    Ok(Json(ListCategoriesResponse {
        categories,
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
        let params: PaginationParams =
            serde_json::from_str("{}").expect("Failed to deserialize");
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
    }
}
