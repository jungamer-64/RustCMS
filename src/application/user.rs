// src/application/user.rs
//! User Application Layer - CQRS統合
//!
//! Commands + Queries + DTOs を単一ファイルに統合

use serde::{Deserialize, Serialize};

#[cfg(feature = "restructure_domain")]
use crate::domain::user::{Email, User, UserId, Username};

#[cfg(feature = "restructure_domain")]
use crate::application::ports::repositories::{RepositoryError, UserRepository};

// ============================================================================
// DTOs - Data Transfer Objects
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

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

#[cfg(feature = "restructure_domain")]
pub struct RegisterUser<'a> {
    repo: &'a dyn UserRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> RegisterUser<'a> {
    pub const fn new(repo: &'a dyn UserRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, request: CreateUserRequest) -> Result<UserDto, RepositoryError> {
        // Value Objects 作成
        let username = Username::new(request.username)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
        let email = Email::new(request.email)
            .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

        // メール重複チェック
        if self.repo.find_by_email(&email).await?.is_some() {
            return Err(RepositoryError::Duplicate(format!(
                "Email '{}' is already in use",
                email
            )));
        }

        // ドメインエンティティ作成
        let user = User::new(username, email);

        // 永続化
        self.repo.save(user.clone()).await?;

        Ok(UserDto::from(user))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct UpdateUser<'a> {
    repo: &'a dyn UserRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> UpdateUser<'a> {
    pub const fn new(repo: &'a dyn UserRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        request: UpdateUserRequest,
    ) -> Result<UserDto, RepositoryError> {
        // ユーザー取得
        let mut user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("User {user_id}")))?;

        // メールアドレス変更
        if let Some(new_email) = request.email {
            let email = Email::new(new_email)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;

            // 重複チェック（他のユーザーが使用していないか）
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

        // ユーザー名変更
        if let Some(new_username) = request.username {
            let username = Username::new(new_username)
                .map_err(|e| RepositoryError::ValidationError(e.to_string()))?;
            user.change_username(username);
        }

        // 永続化
        self.repo.save(user.clone()).await?;

        Ok(UserDto::from(user))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct SuspendUser<'a> {
    repo: &'a dyn UserRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> SuspendUser<'a> {
    pub const fn new(repo: &'a dyn UserRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<UserDto, RepositoryError> {
        let mut user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| RepositoryError::NotFound(format!("User {user_id}")))?;

        user.deactivate();
        self.repo.save(user.clone()).await?;

        Ok(UserDto::from(user))
    }
}

// ============================================================================
// Queries - 読み取り操作
// ============================================================================

#[cfg(feature = "restructure_domain")]
pub struct GetUserById<'a> {
    repo: &'a dyn UserRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> GetUserById<'a> {
    pub const fn new(repo: &'a dyn UserRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<Option<UserDto>, RepositoryError> {
        Ok(self.repo.find_by_id(user_id).await?.map(UserDto::from))
    }
}

#[cfg(feature = "restructure_domain")]
pub struct ListUsers<'a> {
    repo: &'a dyn UserRepository,
}

#[cfg(feature = "restructure_domain")]
impl<'a> ListUsers<'a> {
    pub const fn new(repo: &'a dyn UserRepository) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, limit: i64, offset: i64) -> Result<Vec<UserDto>, RepositoryError> {
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

        mock_repo.expect_find_by_email().returning(|_| Ok(None));
        mock_repo.expect_save().returning(|_| Ok(()));

        let use_case = RegisterUser::new(&mock_repo);

        let request = CreateUserRequest {
            username: "testuser".into(),
            email: "test@example.com".into(),
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

        let existing = User::new(
            Username::new("existing".into()).unwrap(),
            Email::new("test@example.com".into()).unwrap(),
        );

        mock_repo
            .expect_find_by_email()
            .returning(move |_| Ok(Some(existing.clone())));

        let use_case = RegisterUser::new(&mock_repo);

        let request = CreateUserRequest {
            username: "newuser".into(),
            email: "test@example.com".into(),
        };

        let result = use_case.execute(request).await;
        assert!(matches!(result, Err(RepositoryError::Duplicate(_))));
    }

    #[tokio::test]
    async fn test_update_user_email() {
        let mut mock_repo = MockUserRepository::new();

        let user = User::new(
            Username::new("testuser".into()).unwrap(),
            Email::new("old@example.com".into()).unwrap(),
        );
        let user_id = user.id();

        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(user.clone())));
        mock_repo.expect_find_by_email().returning(|_| Ok(None));
        mock_repo.expect_save().returning(|_| Ok(()));

        let use_case = UpdateUser::new(&mock_repo);

        let request = UpdateUserRequest {
            username: None,
            email: Some("new@example.com".into()),
        };

        let result = use_case.execute(user_id, request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().email, "new@example.com");
    }
}
