//! Diesel データベースモデル ↔ Domain Entity マッピング層（Phase 3 プレースホルダー）
//!
//! このモジュールは、データベーススキーマ（Diesel モデル）と Domain層の Entity 型の間の
//! 型安全な変換インターフェースを提供します。
//!
//! # 設計原則
//!
//! - **単方向性**: DB層は Domain層に依存するが、Domain層はDB層に依存しない（依存性の逆転）
//! - **変換の局所化**: すべての変換をこのモジュールに集約
//! - **エラーハンドリング**: 型変換エラーは `InfrastructureError` でラップ
//!
//! # Phase 3 実装状況
//!
//! - ✅ Diesel モデル構造体定義（5 entities）
//! - ⏳ Domain Entity への変換（プレースホルダー）

use crate::common::types::InfrastructureError;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ============================================================================
// User モデルマッピング
// ============================================================================

/// Diesel User モデル（DB スキーマから自動生成）
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::users)]
pub struct DbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel User のための新規挿入用構造体
#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::users)]
pub struct NewDbUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbUser {
    /// Domain層の User Entity に変換
    ///
    /// # Note
    ///
    /// Phase 3 では、Domain層に `new_from_db()` ファクトリメソッドを追加し、
    /// DB値から User を再構築するロジックを実装します。
    /// 現在はプレースホルダー実装です。
    pub fn into_domain(self) -> Result<(), InfrastructureError> {
        // TODO: Phase 3 拡張
        // Step 1: UserId, Email, Username などの値オブジェクトを検証
        // Step 2: User::new_from_db(...) でエンティティ再構築
        // Step 3: 既存 User Entity と互換性確認
        Err(InfrastructureError::DatabaseError(
            "DbUser::into_domain() not yet implemented (Phase 3)".to_string(),
        ))
    }
}

// TODO: Phase 3 拡張で From<&User> 実装を追加（Domain層の整備後）

// ============================================================================
// Post モデルマッピング
// ============================================================================

/// Diesel Post モデル（DB スキーマ）
#[derive(diesel::Queryable, diesel::Selectable, diesel::Identifiable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::posts)]
pub struct DbPost {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub author_id: Uuid,
    pub status: String,
    pub featured_image_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel Post 挿入モデル
#[derive(diesel::Insertable, Debug, Clone)]
#[diesel(table_name = crate::database::schema::posts)]
pub struct NewDbPost {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub author_id: Uuid,
    pub status: String,
    pub featured_image_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbPost {
    /// Diesel モデルを Domain Entity に変換（Phase 3 プレースホルダー）
    pub fn into_domain(self) -> Result<(), InfrastructureError> {
        // TODO: Phase 3 拡張で Post::new_from_db() を呼び出し
        Err(InfrastructureError::DatabaseError(
            "DbPost::into_domain() - Phase 3 拡張で実装予定".to_string(),
        ))
    }
}

// TODO: Phase 3 拡張で From<&Post> 実装を追加

// ============================================================================
// Category モデルマッピング
// ============================================================================

/// Diesel Category モデル
pub struct DbCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub post_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel Category のための新規挿入用構造体
pub struct NewDbCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub post_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbCategory {
    /// Diesel モデルを Domain Entity に変換（Phase 3 プレースホルダー）
    pub fn into_domain(self) -> Result<(), InfrastructureError> {
        // TODO: Phase 3 拡張で Category::new_from_db() を呼び出し
        Err(InfrastructureError::DatabaseError(
            "DbCategory::into_domain() - Phase 3 拡張で実装予定".to_string(),
        ))
    }
}

// TODO: Phase 3 拡張で From<&Category> 実装を追加

// ============================================================================
// Tag モデルマッピング
// ============================================================================

/// Diesel Tag モデル
pub struct DbTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel Tag のための新規挿入用構造体
pub struct NewDbTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub usage_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbTag {
    /// Diesel モデルを Domain Entity に変換（Phase 3 プレースホルダー）
    pub fn into_domain(self) -> Result<(), InfrastructureError> {
        // TODO: Phase 3 拡張で Tag::new_from_db() を呼び出し
        Err(InfrastructureError::DatabaseError(
            "DbTag::into_domain() - Phase 3 拡張で実装予定".to_string(),
        ))
    }
}

// TODO: Phase 3 拡張で From<&Tag> 実装を追加

// ============================================================================
// Comment モデルマッピング
// ============================================================================

/// Diesel Comment モデル
pub struct DbComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Diesel Comment のための新規挿入用構造体
pub struct NewDbComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbComment {
    /// Diesel モデルを Domain Entity に変換（Phase 3 プレースホルダー）
    pub fn into_domain(self) -> Result<(), InfrastructureError> {
        // TODO: Phase 3 拡張で Comment::new_from_db() を呼び出し
        Err(InfrastructureError::DatabaseError(
            "DbComment::into_domain() - Phase 3 拡張で実装予定".to_string(),
        ))
    }
}

// TODO: Phase 3 拡張で From<&Comment> 実装を追加

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_user_struct_creation() {
        let db_user = DbUser {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: Some("hash".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            role: "admin".to_string(),
            is_active: true,
            email_verified: true,
            last_login: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(db_user.username, "testuser");
        assert_eq!(db_user.role, "admin");
    }

    #[test]
    fn test_db_post_struct_creation() {
        let db_post = DbPost {
            id: Uuid::new_v4(),
            title: "Test Post".to_string(),
            slug: "test-post".to_string(),
            content: "Content here".to_string(),
            excerpt: Some("Short excerpt".to_string()),
            author_id: Uuid::new_v4(),
            status: "published".to_string(),
            featured_image_id: None,
            published_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(db_post.title, "Test Post");
        assert_eq!(db_post.status, "published");
    }

    #[test]
    fn test_db_category_struct_creation() {
        let db_category = DbCategory {
            id: Uuid::new_v4(),
            name: "Technology".to_string(),
            slug: "technology".to_string(),
            description: Some("Tech articles".to_string()),
            is_active: true,
            post_count: 5,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(db_category.name, "Technology");
        assert_eq!(db_category.post_count, 5);
    }

    #[test]
    fn test_db_tag_struct_creation() {
        let db_tag = DbTag {
            id: Uuid::new_v4(),
            name: "rust".to_string(),
            slug: "rust".to_string(),
            usage_count: 10,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(db_tag.name, "rust");
        assert_eq!(db_tag.usage_count, 10);
    }

    #[test]
    fn test_db_comment_struct_creation() {
        let db_comment = DbComment {
            id: Uuid::new_v4(),
            post_id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            content: "Great post!".to_string(),
            parent_id: None,
            is_approved: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(db_comment.content, "Great post!");
        assert!(db_comment.is_approved);
    }
}
