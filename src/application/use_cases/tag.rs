// src/application/tag.rs
//! タグ管理アプリケーション層（Phase 3 Step 8）
//!
//! CQRS統合パターン

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::types::ApplicationError;

// ============================================================================
// DTOs
// ============================================================================

/// タグ作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

/// タグ レスポンス DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDto {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub usage_count: i32,
}

// ============================================================================
// Commands (書き込み操作)
// ============================================================================

/// タグ作成コマンド（Phase 3.8 実装予定）
pub struct CreateTagCommand {
    pub request: CreateTagRequest,
}

impl CreateTagCommand {
    pub fn new(request: CreateTagRequest) -> Self {
        Self { request }
    }

    pub fn execute(&self) -> Result<TagDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "CreateTagCommand.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Queries (読み取り操作)
// ============================================================================

/// タグ Slug で取得クエリ（Phase 3.8 実装予定）
pub struct GetTagBySlugQuery {
    pub slug: String,
}

impl GetTagBySlugQuery {
    pub fn new(slug: String) -> Self {
        Self { slug }
    }

    pub fn execute(&self) -> Result<TagDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "GetTagBySlugQuery.execute() Phase 3.8 で実装予定".to_string(),
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
    fn test_tag_dto_creation() {
        let dto = TagDto {
            id: Uuid::new_v4(),
            name: "Rust".to_string(),
            slug: "rust".to_string(),
            usage_count: 5,
        };
        assert_eq!(dto.name, "Rust");
        assert_eq!(dto.usage_count, 5);
    }

    #[test]
    fn test_create_tag_command_not_implemented() {
        let request = CreateTagRequest {
            name: "Tag".to_string(),
        };
        let command = CreateTagCommand::new(request);
        let result = command.execute();
        assert!(matches!(result, Err(ApplicationError::RepositoryError(_))));
    }
}
