// src/application/user.rs
//! ユーザー管理アプリケーション層（Phase 3 Step 7）
//!
//! CQRS統合パターン: Commands + Queries + DTOs を単一ファイルに統合
//! 参考: RESTRUCTURE_EXAMPLES.md

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "restructure_domain")]
use crate::domain::user::{User};

use crate::common::types::ApplicationError;

// ============================================================================
// DTOs (Data Transfer Objects)
// ============================================================================

/// ユーザー作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    // TODO: Phase 3.7 - password フィールド追加（ハッシュ化して保存）
}

/// ユーザー更新リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

/// ユーザーレスポンス DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

impl UserDto {
    /// Domain Entity から DTO に変換
    #[cfg(feature = "restructure_domain")]
    pub fn from_domain(user: User) -> Self {
        Self {
            id: user.id().into(),
            username: user.username().to_string(),
            email: user.email().to_string(),
            is_active: user.is_active(),
        }
    }

    /// DTO を Domain Entity に変換
    #[cfg(feature = "restructure_domain")]
    pub fn to_domain(self) -> Result<User, ApplicationError> {
        // TODO: Phase 3.7 - Domain エンティティ構築ロジック実装
        // Username と Email の検証、Entity の生成等
        Err(ApplicationError::RepositoryError(
            "to_domain() Phase 3.7 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Commands (書き込み操作)
// ============================================================================

/// ユーザー登録コマンド（Phase 3.7 実装予定）
pub struct RegisterUserCommand {
    pub request: CreateUserRequest,
}

impl RegisterUserCommand {
    pub fn new(request: CreateUserRequest) -> Self {
        Self { request }
    }

    /// TODO: Phase 3.7 - ハンドラー実装
    /// 処理フロー:
    /// 1. Username/Email のバリデーション
    /// 2. 既存ユーザーの確認（重複チェック）
    /// 3. Domain User エンティティ作成
    /// 4. Repository.save() で永続化
    /// 5. Domain Event 発行（UserCreated）
    /// 6. DTO に変換してレスポンス
    pub fn execute(&self) -> Result<UserDto, ApplicationError> {
        // TODO: Phase 3.7 で実装予定
        Err(ApplicationError::RepositoryError(
            "RegisterUserCommand.execute() Phase 3.7 で実装予定".to_string(),
        ))
    }
}

// ============================================================================
// Queries (読み取り操作)
// ============================================================================

/// ユーザー ID で取得クエリ（Phase 3.7 実装予定）
pub struct GetUserByIdQuery {
    pub id: Uuid,
}

impl GetUserByIdQuery {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    /// TODO: Phase 3.7 - ハンドラー実装
    /// 処理フロー:
    /// 1. Repository.find_by_id() で取得
    /// 2. 見つからなかったら NotFound エラー
    /// 3. DTO に変換してレスポンス
    pub fn execute(&self) -> Result<UserDto, ApplicationError> {
        // TODO: Phase 3.7 で実装予定
        Err(ApplicationError::RepositoryError(
            "GetUserByIdQuery.execute() Phase 3.7 で実装予定".to_string(),
        ))
    }
}

/// ユーザーメールで取得クエリ（Phase 3.7 実装予定）
pub struct GetUserByEmailQuery {
    pub email: String,
}

impl GetUserByEmailQuery {
    pub fn new(email: String) -> Self {
        Self { email }
    }

    /// TODO: Phase 3.7 - ハンドラー実装
    pub fn execute(&self) -> Result<UserDto, ApplicationError> {
        // TODO: Phase 3.7 で実装予定
        Err(ApplicationError::RepositoryError(
            "GetUserByEmailQuery.execute() Phase 3.7 で実装予定".to_string(),
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
    fn test_user_dto_creation() {
        let dto = UserDto {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            is_active: true,
        };
        assert_eq!(dto.username, "testuser");
        assert_eq!(dto.email, "test@example.com");
        assert!(dto.is_active);
    }

    #[test]
    fn test_register_user_command_creation() {
        let request = CreateUserRequest {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
        };
        let command = RegisterUserCommand::new(request);
        assert_eq!(command.request.username, "newuser");
    }

    #[test]
    fn test_get_user_by_id_query_not_implemented() {
        let id = Uuid::new_v4();
        let query = GetUserByIdQuery::new(id);
        let result = query.execute();
        assert!(matches!(result, Err(ApplicationError::RepositoryError(_))));
    }

    #[test]
    fn test_register_user_command_not_implemented() {
        let request = CreateUserRequest {
            username: "user".to_string(),
            email: "user@example.com".to_string(),
        };
        let command = RegisterUserCommand::new(request);
        let result = command.execute();
        assert!(matches!(result, Err(ApplicationError::RepositoryError(_))));
    }
}
