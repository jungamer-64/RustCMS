// src/application/use_cases/user/register_user.rs
//! RegisterUserUseCase - ユーザー登録 Use Case
//!
//! Phase 3 Week 8-9: Use Case 実装

use crate::application::dto::user::{CreateUserRequest, UserDto};
use crate::application::ports::repositories::UserRepository;
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::user::{Email, User, Username};
use crate::infrastructure::events::bus::{AppEvent, UserEventData};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// ユーザー登録 Use Case
///
/// # 責務
/// - CreateUserRequest の検証
/// - Domain Entity の生成
/// - Repository への保存
/// - Domain Event の発行
/// - UserDto の返却
pub struct RegisterUserUseCase {
    user_repository: Arc<dyn UserRepository>,
    event_bus: broadcast::Sender<AppEvent>,
}

impl RegisterUserUseCase {
    /// Use Case の生成
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        event_bus: broadcast::Sender<AppEvent>,
    ) -> Self {
        Self {
            user_repository,
            event_bus,
        }
    }

    /// ユーザー登録を実行
    ///
    /// # Arguments
    /// - `request` - ユーザー作成リクエスト
    ///
    /// # Returns
    /// - `Ok(UserDto)` - 作成されたユーザー
    /// - `Err(ApplicationError)` - 検証エラーまたは保存エラー
    ///
    /// # Errors
    /// - `ValidationFailed` - リクエストの検証失敗
    /// - `DomainError` - Domain層でのビジネスルール違反
    /// - `RepositoryError` - データベース保存失敗
    pub async fn execute(&self, request: CreateUserRequest) -> ApplicationResult<UserDto> {
        // 1. Request → Domain Value Objects 変換
        let username = Username::new(request.username)
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        let email = Email::new(request.email)
            .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

        // 2. Email 重複チェック（Repository）
        if let Some(_existing) = self.user_repository.find_by_email(&email).await? {
            return Err(ApplicationError::ValidationError(
                "Email already exists".to_string(),
            ));
        }

        // 3. Domain Entity 生成
        let user = User::new(username, email);

        // 4. Domain → DTO 変換（save前にクローン）
        let user_dto = UserDto::from(user.clone());

        // 5. Repository への保存（所有権を渡す）
        self.user_repository.save(user).await?;

        // 6. AppEvent 発行（Fire-and-Forget）
        let event_data = UserEventData {
            id: Uuid::parse_str(&user_dto.id).unwrap_or_default(),
            username: user_dto.username.clone(),
            email: user_dto.email.clone(),
            role: "user".to_string(), // デフォルトロール
        };
        let _ = self.event_bus.send(AppEvent::UserCreated(event_data));

        // 7. DTO を返却
        Ok(user_dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockUserRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_register_user_success() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // Email 重複チェック: なし
        mock_repo
            .expect_find_by_email()
            .times(1)
            .returning(|_| Ok(None));

        // 保存成功
        mock_repo
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = RegisterUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: ユーザー登録実行
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
        };

        let result = use_case.execute(request).await;

        // Assert: 成功
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "testuser");
        assert_eq!(user_dto.email, "test@example.com");
        assert!(user_dto.is_active);
    }

    #[tokio::test]
    async fn test_register_user_email_already_exists() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // Email 重複チェック: 既存ユーザーあり
        let existing_username = Username::new("existing".to_string()).unwrap();
        let existing_email = Email::new("test@example.com".to_string()).unwrap();
        let existing_user = User::new(existing_username, existing_email);

        mock_repo
            .expect_find_by_email()
            .times(1)
            .returning(move |_| Ok(Some(existing_user.clone())));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = RegisterUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: ユーザー登録実行
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
        };

        let result = use_case.execute(request).await;

        // Assert: ValidationError
        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("Email already exists"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }

    #[tokio::test]
    async fn test_register_user_invalid_email() {
        // Arrange: Mock Repository（呼び出されない）
        let mock_repo = MockUserRepository::new();

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = RegisterUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 不正なEmailでユーザー登録実行
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "invalid-email".to_string(), // '@'がない
            password: "SecurePass123!".to_string(),
        };

        let result = use_case.execute(request).await;

        // Assert: ValidationError
        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(_)) => {}
            _ => panic!("Expected ValidationError error"),
        }
    }
}
