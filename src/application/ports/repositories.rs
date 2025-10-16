//! Repository Ports (インターフェース定義)
//!
//! Port/Adapter パターンのPort定義です。
//! Infrastructure層がこれらのtraitを実装します。
//!
//! ## 設計原則
//! - 複数のRepository traitを単一ファイルに統合（監査推奨）
//! - async_traitを使用した非同期メソッド定義
//! - Send + Sync制約でスレッド安全性を保証

use async_trait::async_trait;

#[cfg(feature = "restructure_domain")]
use crate::domain::user::{Email, User, UserId};

// Phase 3 で実装予定: PostRepository
// use crate::domain::post::{Post, PostId};

// ============================================================================
// User Repository Port
// ============================================================================

/// ユーザーリポジトリ（Port/Interface）
///
/// データベースへのアクセスを抽象化します。
/// Infrastructure層で具体的な実装（DieselUserRepository等）を提供します。
#[cfg(feature = "restructure_domain")]
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// ユーザーを保存（作成または更新）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn save(&self, user: User) -> Result<(), RepositoryError>;

    /// IDでユーザーを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;

    /// メールアドレスでユーザーを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;

    /// ユーザーを削除
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn delete(&self, id: UserId) -> Result<(), RepositoryError>;

    /// 全ユーザーを取得（ページネーション対応）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_all(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, RepositoryError>;
}

// ============================================================================
// Post Repository Port (Phase 3 で実装予定)
// ============================================================================

// #[cfg(feature = "restructure_domain")]
// #[async_trait]
// pub trait PostRepository: Send + Sync {
//     async fn save(&self, post: Post) -> Result<(), RepositoryError>;
//     async fn find_by_id(&self, id: PostId) -> Result<Option<Post>, RepositoryError>;
//     async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError>;
//     async fn delete(&self, id: PostId) -> Result<(), RepositoryError>;
// }

// ============================================================================
// Repository Error
// ============================================================================

/// リポジトリエラー
///
/// データベース操作で発生するエラーを表現します。
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_error_display() {
        let error = RepositoryError::NotFound("User".to_string());
        assert_eq!(format!("{error}"), "Entity not found: User");
    }

    #[test]
    fn test_repository_error_duplicate() {
        let error = RepositoryError::Duplicate("email@example.com".to_string());
        assert!(format!("{error}").contains("Duplicate"));
    }
}
