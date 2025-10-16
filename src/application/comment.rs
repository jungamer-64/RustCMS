// src/application/comment.rs
//! コメント管理アプリケーション層（Phase 3 Step 8）
//!
//! CQRS統合パターン

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::types::ApplicationError;

// ============================================================================
// DTOs
// ============================================================================

/// コメント作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentRequest {
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
}

/// コメント レスポンス DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentDto {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Commands (書き込み操作)
// ============================================================================

/// コメント作成コマンド（Phase 3.8 実装予定）
pub struct CreateCommentCommand {
    pub request: CreateCommentRequest,
}

impl CreateCommentCommand {
    pub fn new(request: CreateCommentRequest) -> Self {
        Self { request }
    }

    pub fn execute(&self) -> Result<CommentDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "CreateCommentCommand.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Queries (読み取り操作)
// ============================================================================

/// ブログ記事のコメント取得クエリ（Phase 3.8 実装予定）
pub struct GetCommentsByPostQuery {
    pub post_id: Uuid,
}

impl GetCommentsByPostQuery {
    pub fn new(post_id: Uuid) -> Self {
        Self { post_id }
    }

    pub fn execute(&self) -> Result<Vec<CommentDto>, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "GetCommentsByPostQuery.execute() Phase 3.8 で実装予定".to_string(),
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
    fn test_comment_dto_creation() {
        let dto = CommentDto {
            id: Uuid::new_v4(),
            post_id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            content: "Great post!".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(dto.content, "Great post!");
    }

    #[test]
    fn test_create_comment_command_not_implemented() {
        let request = CreateCommentRequest {
            post_id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            content: "Comment".to_string(),
        };
        let command = CreateCommentCommand::new(request);
        let result = command.execute();
        assert!(matches!(result, Err(ApplicationError::RepositoryError(_))));
    }
}
