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

// Phase 2: PostRepository
#[cfg(feature = "restructure_domain")]
use crate::domain::post::{Post, PostId};

// Phase 2: CommentRepository
#[cfg(feature = "restructure_domain")]
use crate::domain::comment::{Comment, CommentId};

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
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepositoryError>;
}

// ============================================================================
// Post Repository Port (Phase 2 で実装完了)
// ============================================================================

#[cfg(feature = "restructure_domain")]
#[async_trait]
pub trait PostRepository: Send + Sync {
    /// 投稿を保存（作成または更新）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn save(&self, post: Post) -> Result<(), RepositoryError>;

    /// IDで投稿を検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_id(&self, id: PostId) -> Result<Option<Post>, RepositoryError>;

    /// スラッグで投稿を検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, RepositoryError>;

    /// 投稿を削除
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn delete(&self, id: PostId) -> Result<(), RepositoryError>;

    /// 全投稿を取得（ページネーション対応）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Post>, RepositoryError>;

    /// 著者IDで投稿を検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_author(
        &self,
        author_id: UserId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Post>, RepositoryError>;
}

// ============================================================================
// Comment Repository Port
// ============================================================================

/// コメントリポジトリ（Port）
///
/// コメント管理のための抽象インターフェース
#[async_trait]
pub trait CommentRepository: Send + Sync {
    /// コメントを作成または更新
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn save(&self, comment: Comment) -> Result<(), RepositoryError>;

    /// IDでコメントを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_id(&self, id: CommentId) -> Result<Option<Comment>, RepositoryError>;

    /// 投稿に属するコメントを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_post(
        &self,
        post_id: PostId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Comment>, RepositoryError>;

    /// 著者のコメントを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_author(
        &self,
        author_id: UserId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Comment>, RepositoryError>;

    /// コメントを削除
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn delete(&self, id: CommentId) -> Result<(), RepositoryError>;

    /// 全コメントを取得（ページネーション対応）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Comment>, RepositoryError>;
}

// ============================================================================
// Tag Repository Port (Phase 2 で実装)
// ============================================================================

#[cfg(feature = "restructure_domain")]
use crate::domain::tag::{Tag, TagId, TagName};

/// タグリポジトリ（Port/Interface）
///
/// タグのデータベースアクセスを抽象化します。
/// Infrastructure層で具体的な実装を提供します。
#[cfg(feature = "restructure_domain")]
#[async_trait]
pub trait TagRepository: Send + Sync {
    /// タグを保存（作成または更新）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn save(&self, tag: Tag) -> Result<(), RepositoryError>;

    /// IDでタグを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_id(&self, id: TagId) -> Result<Option<Tag>, RepositoryError>;

    /// 名前でタグを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_name(&self, name: &TagName) -> Result<Option<Tag>, RepositoryError>;

    /// タグを削除
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn delete(&self, id: TagId) -> Result<(), RepositoryError>;

    /// 全タグを取得（ページネーション対応）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Tag>, RepositoryError>;

    /// 使用中のタグを取得
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_in_use(&self, limit: i64, offset: i64) -> Result<Vec<Tag>, RepositoryError>;
}

// ============================================================================
// Category Repository Port (Phase 2 で実装)
// ============================================================================

#[cfg(feature = "restructure_domain")]
use crate::domain::category::{Category, CategoryId, CategorySlug};

/// カテゴリリポジトリ（Port/Interface）
///
/// カテゴリのデータベースアクセスを抽象化します。
/// Infrastructure層で具体的な実装を提供します。
#[cfg(feature = "restructure_domain")]
#[async_trait]
pub trait CategoryRepository: Send + Sync {
    /// カテゴリを保存（作成または更新）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn save(&self, category: Category) -> Result<(), RepositoryError>;

    /// IDでカテゴリを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_id(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError>;

    /// スラッグでカテゴリを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_slug(&self, slug: &CategorySlug) -> Result<Option<Category>, RepositoryError>;

    /// カテゴリを削除
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn delete(&self, id: CategoryId) -> Result<(), RepositoryError>;

    /// 全カテゴリを取得（ページネーション対応）
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_all(&self, limit: i64, offset: i64) -> Result<Vec<Category>, RepositoryError>;

    /// アクティブなカテゴリのみ取得
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn list_active(&self, limit: i64, offset: i64) -> Result<Vec<Category>, RepositoryError>;
}

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
