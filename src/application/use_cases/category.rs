// src/application/category.rs
//! カテゴリー管理アプリケーション層（Phase 3 Step 8）
//!
//! CQRS統合パターン

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::types::ApplicationError;

// ============================================================================
// DTOs
// ============================================================================

/// カテゴリー作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
}

/// カテゴリー レスポンス DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i32,
}

// ============================================================================
// Commands (書き込み操作)
// ============================================================================

/// カテゴリー作成コマンド（Phase 3.8 実装予定）
pub struct CreateCategoryCommand {
    pub request: CreateCategoryRequest,
}

impl CreateCategoryCommand {
    pub fn new(request: CreateCategoryRequest) -> Self {
        Self { request }
    }

    pub fn execute(&self) -> Result<CategoryDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "CreateCategoryCommand.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Queries (読み取り操作)
// ============================================================================

/// カテゴリー Slug で取得クエリ（Phase 3.8 実装予定）
pub struct GetCategoryBySlugQuery {
    pub slug: String,
}

impl GetCategoryBySlugQuery {
    pub fn new(slug: String) -> Self {
        Self { slug }
    }

    pub fn execute(&self) -> Result<CategoryDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "GetCategoryBySlugQuery.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_dto_creation() {
        let dto = CategoryDto {
            id: Uuid::new_v4(),
            name: "Technology".to_string(),
            slug: "technology".to_string(),
            description: Some("Tech articles".to_string()),
            post_count: 10,
        };
        assert_eq!(dto.name, "Technology");
        assert_eq!(dto.post_count, 10);
    }

    #[test]
    fn test_create_category_command_not_implemented() {
        let request = CreateCategoryRequest {
            name: "Category".to_string(),
            description: None,
        };
        let command = CreateCategoryCommand::new(request);
        let result = command.execute();
        assert!(matches!(result, Err(ApplicationError::RepositoryError(_))));
    }
}
