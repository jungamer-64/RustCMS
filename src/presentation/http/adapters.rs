// src/presentation/http/adapters.rs
//! HTTP Request/Response → Application DTO アダプター（Phase 4 Step 10）
//!
//! Presentation層の HTTP ペイロードと Application層の DTOs を相互変換
//! 参考: RESTRUCTURE_EXAMPLES.md (Handler パターン)
//!
//! # 設計パターン
//! - HTTP Request: クライアント入力（JSON）
//! - ApplicationDTO: ビジネスロジック層の型
//! - HTTP Response: API レスポンス（JSON）
//!
//! # ステップ
//! 1. HTTP ペイロード → serde_json::Value で deserialize
//! 2. 入力検証（バリデーション）
//! 3. Application::Command/Query に変換
//! 4. UseCase execute
//! 5. DTO → HTTP Response serialize

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// User Request/Response Adapters
// ============================================================================

/// ユーザー登録リクエスト（HTTP POST /api/v2/users/register）
///
/// # バリデーション
/// - username: 必須、3文字以上、英数字アンダースコアのみ
/// - email: 必須、有効なメールアドレス形式
///
/// # エラーレスポンス
/// - 400 Bad Request: バリデーション失敗
/// - 409 Conflict: メールアドレス重複
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserRequest {
    pub username: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
}

/// ユーザー更新リクエスト（HTTP PUT /api/v2/users/{user_id}）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

/// ユーザーレスポンス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

// ============================================================================
// Post Request/Response Adapters
// ============================================================================

/// ブログ記事作成リクエスト（HTTP POST /api/v2/posts）
///
/// # バリデーション
/// - title: 必須、1文字以上 500文字以下
/// - content: 必須、1文字以上
/// - author_id: 必須、有効なUUID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub author_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// ブログ記事更新リクエスト（HTTP PUT /api/v2/posts/{post_id}）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePostRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_published: Option<bool>,
}

/// ブログ記事レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostResponse {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub author_id: Uuid,
    pub is_published: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

// ============================================================================
// Comment Request/Response Adapters
// ============================================================================

/// コメント作成リクエスト（HTTP POST /api/v2/comments）
///
/// # バリデーション
/// - content: 必須、1文字以上 5000文字以下
/// - post_id: 必須、有効なUUID
/// - author_id: 必須、有効なUUID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub post_id: Uuid,
    pub author_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_comment_id: Option<Uuid>,
}

/// コメントレスポンス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommentResponse {
    pub id: Uuid,
    pub content: String,
    pub post_id: Uuid,
    pub author_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_comment_id: Option<Uuid>,
}

// ============================================================================
// Tag Request/Response Adapters
// ============================================================================

/// タグ作成リクエスト（HTTP POST /api/v2/tags）
///
/// # バリデーション
/// - name: 必須、1文字以上 100文字以下
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// タグレスポンス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub usage_count: u32,
}

// ============================================================================
// Category Request/Response Adapters
// ============================================================================

/// カテゴリー作成リクエスト（HTTP POST /api/v2/categories）
///
/// # バリデーション
/// - name: 必須、1文字以上 100文字以下
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// カテゴリーレスポンス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub post_count: u32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// RegisterUserRequest のシリアライゼーション
    #[test]
    fn test_register_user_request_serialization() {
        let request = RegisterUserRequest {
            username: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("john_doe"));
        assert!(json.contains("john@example.com"));
    }

    /// RegisterUserRequest のデシリアライゼーション
    #[test]
    fn test_register_user_request_deserialization() {
        let json = r#"{"username":"john_doe","email":"john@example.com"}"#;
        let request: RegisterUserRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.username, "john_doe");
        assert_eq!(request.email, "john@example.com");
    }

    /// UserResponse の作成と シリアライゼーション
    #[test]
    fn test_user_response_creation() {
        let user_id = Uuid::new_v4();
        let response = UserResponse {
            id: user_id,
            username: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            is_active: true,
            created_at: Some("2025-01-17T10:00:00Z".to_string()),
            updated_at: None,
        };

        assert_eq!(response.username, "john_doe");
        assert!(response.is_active);
    }

    /// CreatePostRequest のデシリアライゼーション
    #[test]
    fn test_create_post_request_deserialization() {
        let author_id = Uuid::new_v4();
        let json = format!(
            r#"{{"title":"My Post","content":"Content here","author_id":"{}"}}"#,
            author_id
        );
        let request: CreatePostRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.title, "My Post");
        assert_eq!(request.author_id, author_id);
    }

    /// CreateCommentRequest のデシリアライゼーション
    #[test]
    fn test_create_comment_request_deserialization() {
        let post_id = Uuid::new_v4();
        let author_id = Uuid::new_v4();
        let json = format!(
            r#"{{"content":"Nice post!","post_id":"{}","author_id":"{}"}}"#,
            post_id, author_id
        );
        let request: CreateCommentRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.content, "Nice post!");
        assert_eq!(request.post_id, post_id);
    }

    /// CreateTagRequest のデシリアライゼーション
    #[test]
    fn test_create_tag_request_deserialization() {
        let json = r#"{"name":"Rust","description":"Rust programming language"}"#;
        let request: CreateTagRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "Rust");
        assert_eq!(
            request.description,
            Some("Rust programming language".to_string())
        );
    }

    /// CreateCategoryRequest のデシリアライゼーション
    #[test]
    fn test_create_category_request_deserialization() {
        let json = r#"{"name":"Technology"}"#;
        let request: CreateCategoryRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "Technology");
    }

    /// TagResponse のシリアライゼーション（オプションフィールド）
    #[test]
    fn test_tag_response_with_optional_fields() {
        let response = TagResponse {
            id: Uuid::new_v4(),
            name: "Rust".to_string(),
            slug: "rust".to_string(),
            description: Some("Rust language".to_string()),
            usage_count: 42,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Rust"));
        assert!(json.contains("42"));
    }
}
