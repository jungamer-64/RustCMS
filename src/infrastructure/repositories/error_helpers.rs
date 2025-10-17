// src/infrastructure/repositories/error_helpers.rs
//! エラーハンドリングヘルパー trait（Phase 3 Refactoring）
//!
//! Repository 実装全体で統一されたエラー変換パターンを提供します。
//! これにより、以下の3つの異なるパターンを統一できます：
//!
//! **Before（3つの異なるパターン）**:
//! ```ignore
//! // Pattern 1: format! with context
//! .map_err(|e| RepositoryError::DatabaseError(format!("Failed to query: {}", e)))?
//!
//! // Pattern 2: to_string() conversion
//! .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?
//!
//! // Pattern 3: ConversionError with manual format
//! .map_err(|e| RepositoryError::ConversionError(format!("Invalid value: {}", e)))?
//! ```
//!
//! **After（統一されたパターン）**:
//! ```ignore
//! use crate::infrastructure::repositories::error_helpers::ToRepositoryError;
//!
//! // Database operation errors
//! .map_err(|e| e.to_db_error())?
//!
//! // Validation/conversion errors with context
//! .map_err(|e| e.to_conversion_error("Invalid tag name"))?
//! ```

#![cfg(feature = "restructure_domain")]

use crate::application::ports::repositories::RepositoryError;

/// Repository エラーへの変換を提供する trait
///
/// あらゆる `Display` 実装型に対して統一されたエラー変換メソッドを提供します。
/// これにより、Repository 実装全体で一貫したエラーハンドリングが可能になります。
///
/// # 使用例
///
/// ```ignore
/// use crate::infrastructure::repositories::error_helpers::ToRepositoryError;
///
/// // データベース操作エラー
/// let result = database_query()
///     .map_err(|e| e.to_db_error())?;
///
/// // バリデーションエラー（コンテキスト付き）
/// let name = TagName::new(value)
///     .map_err(|e| e.to_conversion_error("Tag name validation"))?;
///
/// // コンテキスト情報を動的に生成
/// let email = Email::new(input)
///     .map_err(|e| e.to_conversion_error(&format!("Email validation for user {}", user_id)))?;
/// ```
pub trait ToRepositoryError {
    /// エラーを `RepositoryError::DatabaseError` に変換
    ///
    /// データベース操作（クエリ、挿入、更新、削除など）で発生したエラーに使用します。
    ///
    /// # 戻り値
    /// `RepositoryError::DatabaseError` バリアント（エラーメッセージは `to_string()` で生成）
    ///
    /// # 例
    /// ```ignore
    /// self.db.create_tag(name, description)
    ///     .map_err(|e| e.to_db_error())?;
    /// ```
    fn to_db_error(self) -> RepositoryError;

    /// エラーを `RepositoryError::ConversionError` に変換（コンテキスト付き）
    ///
    /// Value Object のバリデーションや型変換で発生したエラーに使用します。
    /// `context` パラメータでエラーの発生箇所や原因を明示できます。
    ///
    /// # 引数
    /// - `context`: エラーのコンテキスト情報（例: "Invalid tag name", "Email validation"）
    ///
    /// # 戻り値
    /// `RepositoryError::ConversionError` バリアント（`context: error_message` 形式）
    ///
    /// # 例
    /// ```ignore
    /// let tag_name = TagName::new(name)
    ///     .map_err(|e| e.to_conversion_error("Invalid tag name"))?;
    ///
    /// let email = Email::new(email_str)
    ///     .map_err(|e| e.to_conversion_error(&format!("User {} email", user_id)))?;
    /// ```
    fn to_conversion_error(self, context: &str) -> RepositoryError;
}

/// あらゆる `Display` 型に対する `ToRepositoryError` 実装
///
/// この blanket implementation により、`Display` トレイトを実装している型
/// （`DomainError`, `diesel::result::Error`, `std::io::Error` など）は
/// 自動的に `ToRepositoryError` メソッドを利用できます。
impl<E: std::fmt::Display> ToRepositoryError for E {
    fn to_db_error(self) -> RepositoryError {
        RepositoryError::DatabaseError(self.to_string())
    }

    fn to_conversion_error(self, context: &str) -> RepositoryError {
        RepositoryError::ConversionError(format!("{}: {}", context, self))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test error type that implements Display
    #[derive(Debug)]
    struct TestError {
        message: String,
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn test_to_db_error_converts_display_to_database_error() {
        let test_error = TestError {
            message: "Connection failed".to_string(),
        };

        let repo_error = test_error.to_db_error();

        match repo_error {
            RepositoryError::DatabaseError(msg) => {
                assert_eq!(msg, "Connection failed");
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[test]
    fn test_to_conversion_error_adds_context() {
        let test_error = TestError {
            message: "Value too long".to_string(),
        };

        let repo_error = test_error.to_conversion_error("Tag name validation");

        match repo_error {
            RepositoryError::ConversionError(msg) => {
                assert_eq!(msg, "Tag name validation: Value too long");
            }
            _ => panic!("Expected ConversionError variant"),
        }
    }

    #[test]
    fn test_to_conversion_error_with_dynamic_context() {
        let user_id = 42;
        let test_error = TestError {
            message: "Invalid format".to_string(),
        };

        let context = format!("Email validation for user {}", user_id);
        let repo_error = test_error.to_conversion_error(&context);

        match repo_error {
            RepositoryError::ConversionError(msg) => {
                assert_eq!(msg, "Email validation for user 42: Invalid format");
            }
            _ => panic!("Expected ConversionError variant"),
        }
    }

    #[test]
    fn test_works_with_string_errors() {
        let error_msg = "Database connection timeout".to_string();
        let repo_error = error_msg.to_db_error();

        match repo_error {
            RepositoryError::DatabaseError(msg) => {
                assert_eq!(msg, "Database connection timeout");
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[test]
    fn test_works_with_str_errors() {
        let error_msg = "Query execution failed";
        let repo_error = error_msg.to_conversion_error("Database query");

        match repo_error {
            RepositoryError::ConversionError(msg) => {
                assert_eq!(msg, "Database query: Query execution failed");
            }
            _ => panic!("Expected ConversionError variant"),
        }
    }

    #[test]
    fn test_preserves_multiline_error_messages() {
        let test_error = TestError {
            message: "Error on line 1\nError on line 2".to_string(),
        };

        let repo_error = test_error.to_db_error();

        match repo_error {
            RepositoryError::DatabaseError(msg) => {
                assert_eq!(msg, "Error on line 1\nError on line 2");
            }
            _ => panic!("Expected DatabaseError variant"),
        }
    }

    #[test]
    fn test_empty_context_creates_valid_error() {
        let test_error = TestError {
            message: "Error occurred".to_string(),
        };

        let repo_error = test_error.to_conversion_error("");

        match repo_error {
            RepositoryError::ConversionError(msg) => {
                assert_eq!(msg, ": Error occurred");
            }
            _ => panic!("Expected ConversionError variant"),
        }
    }

    #[test]
    fn test_unicode_in_error_messages() {
        let test_error = TestError {
            message: "タグ名が無効です".to_string(),
        };

        let repo_error = test_error.to_conversion_error("日本語バリデーション");

        match repo_error {
            RepositoryError::ConversionError(msg) => {
                assert_eq!(msg, "日本語バリデーション: タグ名が無効です");
            }
            _ => panic!("Expected ConversionError variant"),
        }
    }

    #[test]
    fn test_chaining_with_result() {
        fn might_fail() -> Result<i32, TestError> {
            Err(TestError {
                message: "Operation failed".to_string(),
            })
        }

        let result: Result<i32, RepositoryError> =
            might_fail().map_err(|e| e.to_db_error());

        assert!(result.is_err());
        match result {
            Err(RepositoryError::DatabaseError(msg)) => {
                assert_eq!(msg, "Operation failed");
            }
            _ => panic!("Expected DatabaseError"),
        }
    }
}
