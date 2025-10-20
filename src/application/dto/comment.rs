// src/application/dto/comment.rs
//! Comment DTOs
//!
//! Comment エンティティと HTTP レスポンス/リクエストの間のデータ変換を担当します。

use crate::domain::comment::{Comment, CommentStatus, CommentText};
use crate::domain::post::PostId;
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// コメントレスポンス DTO（完全版）
#[derive(Debug, Clone, Serialize)]
pub struct CommentDto {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl From<Comment> for CommentDto {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id().to_string(),
            post_id: comment.post_id().to_string(),
            author_id: comment.author_id().to_string(),
            text: comment.text().as_str().to_string(),
            status: format!("{:?}", comment.status()),
            created_at: comment.created_at(),
            edited_at: comment.edited_at(),
            updated_at: comment.updated_at(),
        }
    }
}

/// コメント一覧用 DTO（最小限フィールド）
#[derive(Debug, Clone, Serialize)]
pub struct CommentListDto {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub text: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl From<Comment> for CommentListDto {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id().to_string(),
            post_id: comment.post_id().to_string(),
            author_id: comment.author_id().to_string(),
            text: comment.text().as_str().to_string(),
            status: format!("{:?}", comment.status()),
            created_at: comment.created_at(),
        }
    }
}

/// コメント作成リクエスト
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCommentRequest {
    pub post_id: String,

    #[validate(length(min = 1, max = 2000))]
    pub text: String,
}

/// コメント更新リクエスト
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1, max = 2000))]
    pub text: String,
}

/// コメントフィルター（Query 用）
#[derive(Debug, Clone, Default)]
pub struct CommentFilter {
    pub post_id: Option<PostId>,
    pub author_id: Option<UserId>,
    pub status: Option<CommentStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_dto_from_comment() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let comment = Comment::new(post_id, author_id, text).unwrap();

        let dto = CommentDto::from(comment.clone());

        assert_eq!(dto.id, comment.id().to_string());
        assert_eq!(dto.text, "Test comment");
        assert_eq!(dto.status, "Pending");
        assert_eq!(dto.edited_at, None);
    }

    #[test]
    fn test_comment_list_dto_from_comment() {
        let post_id = PostId::new();
        let author_id = UserId::new();
        let text = CommentText::new("Test comment".to_string()).unwrap();

        let comment = Comment::new(post_id, author_id, text).unwrap();

        let dto = CommentListDto::from(comment);

        assert_eq!(dto.text, "Test comment");
        assert_eq!(dto.status, "Pending");
    }

    #[test]
    fn test_create_comment_request_deserialization() {
        let post_id = PostId::new();
        let json = format!(
            r#"{{
            "post_id": "{}",
            "text": "New comment"
        }}"#,
            post_id
        );

        let request: CreateCommentRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.post_id, post_id.to_string());
        assert_eq!(request.text, "New comment");
    }

    #[test]
    fn test_update_comment_request_deserialization() {
        let json = r#"{
            "text": "Updated comment"
        }"#;

        let request: UpdateCommentRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.text, "Updated comment");
    }
}
