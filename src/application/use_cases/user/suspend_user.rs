// src/application/use_cases/user/suspend_user.rs
// src/application/use_cases/user/suspend_user.rs
//! SuspendUserUseCase - ユーザー停止 Use Case
//!
//! Phase 3 Week 8-9: Use Case 実装

use crate::application::dto::user::UserDto;
use crate::application::ports::repositories::UserRepository;
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::user::{UserId, UserRole};
use crate::infrastructure::events::bus::{AppEvent, UserEventData};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// ユーザー停止 Use Case
///
/// # 責務
/// - 既存ユーザーの取得
/// - Domain Entity の deactivate 実行
/// - Repository への保存
/// - Domain Event の発行
/// - UserDto の返却
pub struct SuspendUserUseCase {
    user_repository: Arc<dyn UserRepository>,
    event_bus: broadcast::Sender<AppEvent>,
}

impl SuspendUserUseCase {
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

    /// ユーザー停止を実行
    ///
    /// # Arguments
    /// - `user_id_str` - 停止対象ユーザーID
    ///
    /// # Returns
    /// - `Ok(UserDto)` - 停止されたユーザー
    /// - `Err(ApplicationError)` - エラー
    ///
    /// # Errors
    /// - `NotFound` - ユーザーが存在しない
    /// - `ValidationError` - UUID フォーマットが不正
    /// - `DomainError` - Domain層でのビジネスルール違反
    /// - `RepositoryError` - データベース保存失敗
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

        // 4. Domain → DTO 変換（save前にクローン）
        let user_dto = UserDto::from(user.clone());

        // 5. Repository への保存（所有権を渡す）
        self.user_repository.save(user).await?;

        // 6. AppEvent 発行（Fire-and-Forget）
        let _ = self
            .event_bus
            .send(AppEvent::UserDeleted(Uuid::parse_str(&user_dto.id).unwrap_or_default()));

        // 7. DTO を返却
        Ok(user_dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockUserRepository;
    use crate::domain::user::{Email, User, Username};

    #[tokio::test]
    async fn test_suspend_user_success() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // アクティブなユーザーを返す
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let mut user = User::new(username, email);
        let _ = user.activate(); // アクティベート
        let user_id = user.id();

        let mock_user_for_find = user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        // 保存成功
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = SuspendUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: ユーザー停止実行
        let result = use_case.execute(&user_id.to_string()).await;

        // Assert: 成功
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "testuser");
        assert!(!user_dto.is_active); // 非アクティブになっている
    }

    #[tokio::test]
    async fn test_suspend_user_not_found() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // ユーザーが見つからない
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(|_| Ok(None));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = SuspendUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 存在しないユーザーの停止を試行
        let result = use_case
            .execute("123e4567-e89b-12d3-a456-426614174000")
            .await;

        // Assert: NotFound
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
        // Arrange: Mock Repository（呼び出されない）
        let mock_repo = MockUserRepository::new();

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = SuspendUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 不正なUUIDで停止を試行
        let result = use_case.execute("invalid-uuid").await;

        // Assert: ValidationError
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
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // 既に非アクティブなユーザーを返す
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user_id = UserId::new();
        let now = chrono::Utc::now();
        let user = User::restore(
            user_id,
            username,
            email,
            Some("hashed_password".to_string()),
            UserRole::Subscriber, // 購読者ロール
            false,                // 非アクティブで復元
            now,
            now,
        );

        let mock_user_for_find = user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        // save() は呼ばれない（ValidationError で早期リターン）

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = SuspendUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 既に非アクティブなユーザーの停止を試行
        let result = use_case.execute(&user_id.to_string()).await;

        // Assert: ValidationError（既に非アクティブ）
        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("already inactive"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }
}
