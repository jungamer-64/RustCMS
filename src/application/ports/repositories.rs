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

// Domain entities imported from flattened modules
use crate::domain::comment::{Comment, CommentId};
use crate::domain::post::PostId;
use crate::domain::user::UserId;

// Note: Phase 2 complete - behind restructure_domain feature flag
#[cfg(feature = "restructure_domain")]
use crate::domain::post::Post;
#[cfg(feature = "restructure_domain")]
use crate::domain::user::{Email, User, Username};

// Additional imports needed for Tag and Category repositories
#[cfg(feature = "restructure_domain")]
use crate::domain::category::{Category, CategoryId, CategorySlug};
#[cfg(feature = "restructure_domain")]
use crate::domain::tag::{Tag, TagId, TagName};

// ============================================================================
// User Repository Port
// ============================================================================

/// ユーザーリポジトリ（Port/Interface）
///
/// データベースへのアクセスを抽象化します。
/// Infrastructure層で具体的な実装（DieselUserRepository等）を提供します。
#[cfg(feature = "restructure_domain")]
#[cfg_attr(test, mockall::automock)]
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

    /// ユーザー名でユーザーを検索
    ///
    /// # Errors
    ///
    /// データベースエラーが発生した場合
    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, RepositoryError>;

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
#[cfg_attr(test, mockall::automock)]
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
#[cfg_attr(test, mockall::automock)]
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

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Diesel Error からの変換（Unit of Work で必要）
#[cfg(feature = "database")]
impl From<diesel::result::Error> for RepositoryError {
    fn from(err: diesel::result::Error) -> Self {
        use diesel::result::Error as DieselError;
        match err {
            DieselError::NotFound => RepositoryError::NotFound("Record not found".to_string()),
            DieselError::DatabaseError(kind, _info) => {
                // DatabaseErrorInformation doesn't implement Display, so just use kind
                RepositoryError::DatabaseError(format!("Database error: {kind:?}"))
            }
            DieselError::QueryBuilderError(msg) => {
                RepositoryError::DatabaseError(format!("Query builder error: {msg}"))
            }
            DieselError::DeserializationError(e) => {
                RepositoryError::ConversionError(format!("Deserialization error: {e}"))
            }
            DieselError::SerializationError(e) => {
                RepositoryError::ConversionError(format!("Serialization error: {e}"))
            }
            _ => RepositoryError::Unknown(format!("Diesel error: {err}")),
        }
    }
}

// r2d2::Error からの変換（コネクションプールエラー）
#[cfg(feature = "database")]
impl From<diesel::r2d2::PoolError> for RepositoryError {
    fn from(err: diesel::r2d2::PoolError) -> Self {
        RepositoryError::ConnectionError(format!("Connection pool error: {err}"))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // RepositoryError Tests (全バリアント, 表示, シリアライズ)
    // =========================================================================

    #[test]
    fn test_repository_error_not_found() {
        let error = RepositoryError::NotFound("User 123".to_string());
        assert_eq!(format!("{error}"), "Entity not found: User 123");
    }

    #[test]
    fn test_repository_error_duplicate() {
        let error = RepositoryError::Duplicate("email@example.com".to_string());
        assert!(format!("{error}").contains("Duplicate"));
    }

    #[test]
    fn test_repository_error_database_error() {
        let error = RepositoryError::DatabaseError("Connection pool exhausted".to_string());
        assert!(format!("{error}").contains("Database error"));
    }

    #[test]
    fn test_repository_error_validation_error() {
        let error = RepositoryError::ValidationError("Invalid email format".to_string());
        assert!(format!("{error}").contains("Validation error"));
    }

    #[test]
    fn test_repository_error_unknown() {
        let error = RepositoryError::Unknown("Something went wrong".to_string());
        assert!(format!("{error}").contains("Unknown error"));
    }

    #[test]
    fn test_repository_error_all_variants() {
        let variants = vec![
            RepositoryError::NotFound("Entity".to_string()),
            RepositoryError::Duplicate("Key".to_string()),
            RepositoryError::DatabaseError("DB error".to_string()),
            RepositoryError::ValidationError("Validation failed".to_string()),
            RepositoryError::Unknown("Unknown".to_string()),
        ];

        for variant in variants {
            let display = format!("{variant}");
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_repository_error_debug() {
        let error = RepositoryError::NotFound("User 123".to_string());
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn test_repository_error_display_special_characters() {
        let error = RepositoryError::ValidationError("Email contains <script> tag".to_string());
        let display = format!("{error}");
        assert!(display.contains("<script>"));
    }

    #[test]
    fn test_repository_error_display_unicode() {
        let error = RepositoryError::Duplicate("ユーザー@例え.jp".to_string());
        let display = format!("{error}");
        assert!(display.contains("ユーザー"));
    }

    // =========================================================================
    // RepositoryError semantic tests (エラー意味論)
    // =========================================================================

    #[test]
    fn test_repository_error_semantics_not_found_vs_error() {
        let not_found = RepositoryError::NotFound("User".to_string());
        let db_error = RepositoryError::DatabaseError("Connection error".to_string());

        // Different error types have different meanings
        assert_ne!(format!("{not_found}"), format!("{db_error}"));
    }

    #[test]
    fn test_repository_error_semantics_duplicate_vs_validation() {
        let duplicate = RepositoryError::Duplicate("Email already exists".to_string());
        let validation = RepositoryError::ValidationError("Email is invalid".to_string());

        // Both relate to email but represent different concerns
        assert_ne!(format!("{duplicate}"), format!("{validation}"));
    }

    // =========================================================================
    // RepositoryError edge cases
    // =========================================================================

    #[test]
    fn test_repository_error_with_empty_message() {
        let error = RepositoryError::Unknown("".to_string());
        let display = format!("{error}");
        assert!(!display.is_empty());
    }

    #[test]
    fn test_repository_error_with_long_message() {
        let long_msg = "x".repeat(1000);
        let error = RepositoryError::DatabaseError(long_msg.clone());
        let display = format!("{error}");
        assert!(display.contains(&long_msg));
    }

    #[test]
    fn test_repository_error_multiple_instantiation() {
        let errors: Vec<_> = (0..10)
            .map(|i| RepositoryError::NotFound(format!("Entity {i}")))
            .collect();

        assert_eq!(errors.len(), 10);
        for (i, error) in errors.iter().enumerate() {
            let msg = format!("{error}");
            assert!(msg.contains(&format!("Entity {i}")));
        }
    }

    // =========================================================================
    // Port definitions compile-time validation
    // =========================================================================

    #[test]
    fn test_repository_port_trait_bounds() {
        // Verify that repository traits would have expected bounds
        // This is a compile-time check via trait signature

        // Traits should be Send + Sync
        #[allow(dead_code)]
        fn assert_send_sync<T: Send + Sync>() {}

        // If we could instantiate, this would hold:
        // assert_send_sync::<dyn UserRepository>();
        // assert_send_sync::<dyn PostRepository>();
        // assert_send_sync::<dyn CommentRepository>();
        // assert_send_sync::<dyn TagRepository>();
        // assert_send_sync::<dyn CategoryRepository>();
    }

    // =========================================================================
    // Repository error recovery patterns
    // =========================================================================

    #[test]
    fn test_repository_error_as_result_ok() {
        let result: Result<&str, RepositoryError> = Ok("success");
        assert!(result.is_ok());
    }

    #[test]
    fn test_repository_error_as_result_err() {
        let result: Result<&str, RepositoryError> =
            Err(RepositoryError::NotFound("Item".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_repository_error_result_mapping() {
        let result: Result<i32, RepositoryError> = Err(RepositoryError::DatabaseError(
            "Connection failed".to_string(),
        ));

        let mapped = result.map_err(|_| RepositoryError::Unknown("Mapped".to_string()));
        assert!(mapped.is_err());
        if let Err(RepositoryError::Unknown(msg)) = mapped {
            assert_eq!(msg, "Mapped");
        } else {
            panic!("Expected Unknown error");
        }
    }

    #[test]
    fn test_repository_error_result_recovery() {
        let result: Result<i32, RepositoryError> =
            Err(RepositoryError::NotFound("Item".to_string()));

        let recovered: Result<i32, RepositoryError> = result.or(Ok(0));
        assert!(recovered.is_ok());
        assert_eq!(recovered.unwrap(), 0);
    }

    // =========================================================================
    // RepositoryError consistency tests
    // =========================================================================

    #[test]
    fn test_repository_error_consistency_across_display_calls() {
        let error = RepositoryError::NotFound("User".to_string());
        let display1 = format!("{error}");
        let display2 = format!("{error}");
        assert_eq!(display1, display2);
    }

    #[test]
    fn test_repository_error_distinct_from_domain_errors() {
        let repo_err = RepositoryError::ValidationError("Invalid data".to_string());
        let display = format!("{repo_err}");

        // Should clearly indicate it's a repository error, not a domain error
        assert!(display.contains("Validation error"));
    }
}
