// src/application/post.rs
//! ブログ記事アプリケーション層（Phase 3 Step 8）
//!
//! CQRS統合パターン: Commands + Queries + DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::types::ApplicationError;

// ============================================================================
// DTOs
// ============================================================================

/// ブログ記事作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub author_id: Uuid,
}

/// ブログ記事更新リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<String>,
}

/// ブログ記事レスポンス DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDto {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub author_id: Uuid,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Commands (書き込み操作)
// ============================================================================

/// ブログ記事作成コマンド（Phase 3.8 実装予定）
pub struct CreatePostCommand {
    pub request: CreatePostRequest,
}

impl CreatePostCommand {
    pub fn new(request: CreatePostRequest) -> Self {
        Self { request }
    }

    /// TODO: Phase 3.8 - ハンドラー実装
    pub fn execute(&self) -> Result<PostDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "CreatePostCommand.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

/// ブログ記事公開コマンド（Phase 3.8 実装予定）
pub struct PublishPostCommand {
    pub post_id: Uuid,
}

impl PublishPostCommand {
    pub fn new(post_id: Uuid) -> Self {
        Self { post_id }
    }

    /// TODO: Phase 3.8 - ハンドラー実装
    pub fn execute(&self) -> Result<PostDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "PublishPostCommand.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Queries (読み取り操作)
// ============================================================================

/// ブログ記事 ID で取得クエリ（Phase 3.8 実装予定）
pub struct GetPostByIdQuery {
    pub id: Uuid,
}

impl GetPostByIdQuery {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn execute(&self) -> Result<PostDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "GetPostByIdQuery.execute() Phase 3.8 で実装予定".to_string(),
        ))
    }
}

/// ブログ記事 Slug で取得クエリ（Phase 3.8 実装予定）
pub struct GetPostBySlugQuery {
    pub slug: String,
}

impl GetPostBySlugQuery {
    pub fn new(slug: String) -> Self {
        Self { slug }
    }

    pub fn execute(&self) -> Result<PostDto, ApplicationError> {
        Err(ApplicationError::RepositoryError(
            "GetPostBySlugQuery.execute() Phase 3.8 で実装予定".to_string(),
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
    fn test_post_dto_creation() {
        let dto = PostDto {
            id: Uuid::new_v4(),
            title: "Test Post".to_string(),
            slug: "test-post".to_string(),
            content: "Content".to_string(),
            author_id: Uuid::new_v4(),
            is_published: false,
            created_at: Utc::now(),
        };
        assert_eq!(dto.title, "Test Post");
    }

    #[test]
    fn test_create_post_command_not_implemented() {
        let request = CreatePostRequest {
            title: "Title".to_string(),
            content: "Content".to_string(),
            author_id: Uuid::new_v4(),
        };
        let command = CreatePostCommand::new(request);
        let result = command.execute();
        assert!(matches!(result, Err(ApplicationError::RepositoryError(_))));
    }
}
