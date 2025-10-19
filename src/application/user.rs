//! User Application Layer - CQRS統合
//!
//! Commands + Queries + DTOs を単一ファイルに統合（監査推奨パターン）
//!
//! ## 構造
//! - DTOs: リクエスト/レスポンス型
//! - Commands: 書き込み操作（RegisterUser, UpdateUser等）
//! - Queries: 読み取り操作（GetUserById, ListUsers等）
//!
//! ## 設計原則
//! - 500行未満は単一ファイル推奨（監査ガイドライン）
//! - Use Case は Application Service として実装
//! - Repository Port への依存性注入

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::user::{Email, User, UserId, Username};

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{RepositoryError, UserRepository};

// ============================================================================
// DTOs - Data Transfer Objects
// ============================================================================

/// ユーザー登録リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    // Note: display_name removed - not in current User domain model
}

/// ユーザー更新リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    // Note: display_name removed - not in current User domain model
}

/// ユーザーレスポンス（公開用DTO）
#[derive(Debug, Clone, Serialize)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

#[cfg(feature = "restructure_domain")]
impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id().to_string(),
            username: user.username().to_string(),
            email: user.email().to_string(),
            is_active: user.is_active(),
        }
    }
}

// ============================================================================
// Commands - 書き込み操作
// ============================================================================

/// ユーザー登録コマンド (Use Case)
#[cfg(feature = "restructure_domain")]
pub struct RegisterUser {
    repo: Arc<dyn UserRepository>,
}

#[cfg(feature = "restructure_domain")]
impl RegisterUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    /// ユーザーを登録する
    ///
    /// # Errors
    ///
    /// - バリデーションエラー（無効なメールアドレス等）
    /// - データベースエラー
    pub async fn execute(&self, request: CreateUserRequest) -> Result<UserDto, RepositoryError> {
        // 1. Value Objects 作成（検証済み）
        let username = Username::new(request.username)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
        let email = Email::new(request.email)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // 2. メール重複チェック
        if self.repo.find_by_email(&email).await?.is_some() {
            return Err(RepositoryError::Duplicate(format!(
                "Email '{}' is already in use",
                email
            )));
        }

        // 3. ドメインエンティティ作成
        let user = User::new(username, email);

        // 4. 永続化
        self.repo.save(user.clone()).await?;

        // 5. DTOに変換して返却
        Ok(UserDto::from(user))
    }
}

/// ユーザー更新コマンド
#[cfg(feature = "restructure_domain")]
pub struct UpdateUser {
    repo: Arc<dyn UserRepository>,
}

#[cfg(feature = "restructure_domain")]
impl UpdateUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        request: UpdateUserRequest,
    ) -> Result<UserDto, RepositoryError> {
        // 1. ユーザー取得
        let mut user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("User {}", user_id)))?;

        // 2. メールアドレス変更
        if let Some(new_email) = request.email {
            let email = Email::new(new_email)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

            // 重複チェック
            if let Some(existing) = self.repo.find_by_email(&email).await? {
                if existing.id() != user.id() {
                    return Err(RepositoryError::Duplicate(format!(
                        "Email '{}' is already in use",
                        email
                    )));
                }
            }

            user.change_email(email);
        }

        // 3. ユーザー名変更
        if let Some(new_username) = request.username {
            let username = Username::new(new_username)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            user.change_username(username);
        }

        // 4. 永続化
        self.repo.save(user.clone()).await?;

        Ok(UserDto::from(user))
    }
}

/// ユーザー停止コマンド
#[cfg(feature = "restructure_domain")]
pub struct SuspendUser {
    repo: Arc<dyn UserRepository>,
}

#[cfg(feature = "restructure_domain")]
impl SuspendUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<UserDto, RepositoryError> {
        let mut user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("User {}", user_id)))?;

        user.deactivate();

        self.repo.save(user.clone()).await?;

        Ok(UserDto::from(user))
    }
}

// ============================================================================
// Queries - 読み取り操作
// ============================================================================

/// ユーザー取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct GetUserById {
    repo: Arc<dyn UserRepository>,
}

#[cfg(feature = "restructure_domain")]
impl GetUserById {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<Option<UserDto>, RepositoryError> {
        let user = self.repo.find_by_id(user_id).await?;
        Ok(user.map(UserDto::from))
    }
}

/// ユーザー一覧取得クエリ
#[cfg(feature = "restructure_domain")]
pub struct ListUsers {
    repo: Arc<dyn UserRepository>,
}

#[cfg(feature = "restructure_domain")]
impl ListUsers {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserDto>, RepositoryError> {
        let users = self.repo.list_all(limit, offset).await?;
        Ok(users.into_iter().map(UserDto::from).collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "restructure_domain"))]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockUserRepository;

    #[tokio::test]
    async fn test_register_user_success() {
        let mut mock_repo = MockUserRepository::new();

        // メール重複チェック: None（存在しない）
        mock_repo
            .expect_find_by_email()
            .returning(|_| Box::pin(async { Ok(None) }));

        // 保存成功
        mock_repo
            .expect_save()
            .returning(|_| Box::pin(async { Ok(()) }));

        let use_case = RegisterUser::new(Arc::new(mock_repo));

        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
        };

        let result = use_case.execute(request).await;
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert_eq!(dto.username, "testuser");
        assert_eq!(dto.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_register_user_duplicate_email() {
        let mut mock_repo = MockUserRepository::new();

        // メール重複あり
        let existing_user = User::new(
            Username::new("existing".to_string()).unwrap(),
            Email::new("test@example.com".to_string()).unwrap(),
        );

        mock_repo
            .expect_find_by_email()
            .returning(move |_| Box::pin(async move { Ok(Some(existing_user.clone())) }));

        let use_case = RegisterUser::new(Arc::new(mock_repo));

        let request = CreateUserRequest {
            username: "newuser".to_string(),
            email: "test@example.com".to_string(),
        };

        let result = use_case.execute(request).await;
        assert!(matches!(result, Err(RepositoryError::Duplicate(_))));
    }
}
