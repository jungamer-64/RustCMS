// src/application/use_cases/user/suspend_user.rs
//! SuspendUserUseCase - ユーザー停止 Use Case
//!
//! ユーザーを一時停止し、必要なイベントを発行します。

use crate::common::types::{ApplicationError, ApplicationResult};
use crate::dto::user::UserDto;
use crate::ports::events::{DomainEvent, EventPublisher};
use crate::ports::repositories::UserRepository;
use chrono::Utc;
use domain::user::{UserId, UserRole};
use std::sync::Arc;
use uuid::Uuid;

/// ユーザー停止 Use Case
pub struct SuspendUserUseCase {
    user_repository: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl SuspendUserUseCase {
    /// Use Case の生成
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            user_repository,
            event_publisher,
        }
    }

    /// ユーザー停止を実行
    ///
    /// # Errors
    /// - `NotFound` ユーザーが存在しない
    /// - `ValidationError` UUID フォーマットが不正、または既に非アクティブ
    /// - `DomainError` ドメイン層のビジネスルール違反
    /// - `RepositoryError` リポジトリ操作に失敗
    /// - `EventPublishError` イベント発行に失敗
    pub async fn execute(&self, user_id_str: &str) -> ApplicationResult<UserDto> {
        // 1. Parse UUID
        let uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| ApplicationError::ValidationError("Invalid UUID format".to_string()))?;
        let user_id = UserId::from_uuid(uuid);

        // 2. 既存ユーザーを取得
        let mut user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("User not found: {}", user_id)))?;

        // 3. ビジネスルールチェック: 既に非アクティブかどうか
        if !user.is_active() {
            return Err(ApplicationError::ValidationError(
                "User is already inactive".to_string(),
            ));
        }

        // 4. ユーザーを停止
        user.deactivate();

        // 5. Domain → DTO 変換（save前にクローン）
        let user_dto = UserDto::from(user.clone());

        // 6. Repository への保存（所有権を渡す）
        self.user_repository.save(user).await?;

        // 7. ドメインイベント発行
        self.event_publisher
            .publish(DomainEvent::UserDeactivated {
                user_id,
                reason: None,
                timestamp: Utc::now(),
            })
            .await?;

        // 8. DTO を返却
        Ok(user_dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::events::{DomainEvent, EventError, EventPublisher};
    use crate::ports::repositories::MockUserRepository;
    use async_trait::async_trait;
    use domain::user::{Email, User, Username};

    struct NoopEventPublisher;

    #[async_trait]
    impl EventPublisher for NoopEventPublisher {
        async fn publish(&self, _event: DomainEvent) -> Result<(), EventError> {
            Ok(())
        }

        async fn publish_batch(&self, _events: Vec<DomainEvent>) -> Result<(), EventError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_suspend_user_success() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // アクティブなユーザーを返す
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let mut user = User::new(username, email);
        let _ = user.activate();
        let user_id = user.id();

        let mock_user_for_find = user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        // 保存成功
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        // Use Case 生成
        let use_case = SuspendUserUseCase::new(
            Arc::new(mock_repo),
            Arc::new(NoopEventPublisher),
        );

        // Act
        let result = use_case.execute(&user_id.to_string()).await;

        // Assert
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "testuser");
        assert!(!user_dto.is_active);
    }

    #[tokio::test]
    async fn test_suspend_user_not_found() {
        let mut mock_repo = MockUserRepository::new();

        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(|_| Ok(None));

        let use_case = SuspendUserUseCase::new(
            Arc::new(mock_repo),
            Arc::new(NoopEventPublisher),
        );

        let result = use_case
            .execute("123e4567-e89b-12d3-a456-426614174000")
            .await;

        assert!(result.is_err());
        match result {
            Err(ApplicationError::NotFound(msg)) => {
                assert!(msg.contains("User not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_suspend_user_invalid_uuid() {
        let mock_repo = MockUserRepository::new();

        let use_case = SuspendUserUseCase::new(
            Arc::new(mock_repo),
            Arc::new(NoopEventPublisher),
        );

        let result = use_case.execute("invalid-uuid").await;

        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("Invalid UUID format"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }

    #[tokio::test]
    async fn test_suspend_user_already_inactive() {
        let mut mock_repo = MockUserRepository::new();

        // 既に非アクティブなユーザー
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user_id = UserId::new();
        let now = chrono::Utc::now();
        let user = User::restore(
            user_id,
            username,
            email,
            Some("hashed_password".to_string()),
            UserRole::Subscriber,
            false,
            now,
            now,
        );

        let mock_user_for_find = user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        let use_case = SuspendUserUseCase::new(
            Arc::new(mock_repo),
            Arc::new(NoopEventPublisher),
        );

        let result = use_case.execute(&user_id.to_string()).await;

        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("already inactive"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }
}
