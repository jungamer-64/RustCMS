// src/application/use_cases/user/update_user.rs
//! UpdateUserUseCase - ユーザー情報更新 Use Case
//!
//! Phase 3 Week 8-9: Use Case 実装

use crate::application::dto::user::{UpdateUserRequest, UserDto};
use crate::application::ports::repositories::UserRepository;
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::user::{Email, User, UserId, Username};
use crate::infrastructure::events::bus::{AppEvent, UserEventData};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// ユーザー情報更新 Use Case
///
/// # 責務
/// - UpdateUserRequest の検証
/// - 既存ユーザーの取得
/// - Domain Entity の更新
/// - Repository への保存
/// - Domain Event の発行
/// - UserDto の返却
pub struct UpdateUserUseCase {
    user_repository: Arc<dyn UserRepository>,
    event_bus: broadcast::Sender<AppEvent>,
}

impl UpdateUserUseCase {
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

    /// ユーザー情報更新を実行
    ///
    /// # Arguments
    /// - `user_id_str` - 更新対象ユーザーID
    /// - `request` - ユーザー更新リクエスト
    ///
    /// # Returns
    /// - `Ok(UserDto)` - 更新されたユーザー
    /// - `Err(ApplicationError)` - 検証エラーまたは保存エラー
    ///
    /// # Errors
    /// - `NotFound` - ユーザーが存在しない
    /// - `ValidationError` - リクエストの検証失敗
    /// - `DomainError` - Domain層でのビジネスルール違反
    /// - `RepositoryError` - データベース保存失敗
    pub async fn execute(
        &self,
        user_id_str: &str,
        request: UpdateUserRequest,
    ) -> ApplicationResult<UserDto> {
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

        // 3. Username 更新（指定されている場合）
        if let Some(new_username) = request.username {
            let username = Username::new(new_username)
                .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;
            user.change_username(username);
        }

        // 4. Email 更新（指定されている場合）
        if let Some(new_email) = request.email {
            let email = Email::new(new_email)
                .map_err(|e| ApplicationError::ValidationError(e.to_string()))?;

            // Email 重複チェック
            if let Some(existing) = self.user_repository.find_by_email(&email).await? {
                if existing.id() != user.id() {
                    return Err(ApplicationError::ValidationError(
                        "Email already in use by another user".to_string(),
                    ));
                }
            }

            user.change_email(email);
        }

        // 5. Domain → DTO 変換（save前にクローン）
        let user_dto = UserDto::from(user.clone());

        // 6. Repository への保存（所有権を渡す）
        self.user_repository.save(user).await?;

        // 7. AppEvent 発行（Fire-and-Forget）
        let event_data = UserEventData {
            id: Uuid::parse_str(&user_dto.id).unwrap_or_default(),
            username: user_dto.username.clone(),
            email: user_dto.email.clone(),
            role: "user".to_string(), // デフォルトロール
        };
        let _ = self.event_bus.send(AppEvent::UserUpdated(event_data));

        // 8. DTO を返却
        Ok(user_dto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockUserRepository;

    #[tokio::test]
    async fn test_update_user_success() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // 既存ユーザーを返す
        let existing_username = Username::new("olduser".to_string()).unwrap();
        let existing_email = Email::new("old@example.com".to_string()).unwrap();
        let existing_user = User::new(existing_username, existing_email);
        let user_id = existing_user.id();

        let mock_user_for_find = existing_user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        // Email 重複チェック: なし
        mock_repo
            .expect_find_by_email()
            .times(1)
            .returning(|_| Ok(None));

        // 保存成功
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = UpdateUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: ユーザー更新実行
        let request = UpdateUserRequest {
            username: Some("newuser".to_string()),
            email: Some("new@example.com".to_string()),
            password: None,
        };

        let result = use_case.execute(&user_id.to_string(), request).await;

        // Assert: 成功
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "newuser");
        assert_eq!(user_dto.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_update_user_not_found() {
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
        let use_case = UpdateUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 存在しないユーザーの更新を試行
        let request = UpdateUserRequest {
            username: Some("newuser".to_string()),
            email: None,
            password: None,
        };

        let result = use_case
            .execute("123e4567-e89b-12d3-a456-426614174000", request)
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
    async fn test_update_user_email_already_in_use() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // 既存ユーザーを返す
        let existing_username = Username::new("user1".to_string()).unwrap();
        let existing_email = Email::new("user1@example.com".to_string()).unwrap();
        let existing_user = User::new(existing_username, existing_email);
        let user_id = existing_user.id();

        let mock_user_for_find = existing_user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        // Email 重複チェック: 別のユーザーが使用中
        let other_username = Username::new("user2".to_string()).unwrap();
        let other_email = Email::new("user2@example.com".to_string()).unwrap();
        let other_user = User::new(other_username, other_email);

        mock_repo
            .expect_find_by_email()
            .times(1)
            .returning(move |_| Ok(Some(other_user.clone())));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = UpdateUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: 他のユーザーのEmailに更新を試行
        let request = UpdateUserRequest {
            username: None,
            email: Some("user2@example.com".to_string()),
            password: None,
        };

        let result = use_case.execute(&user_id.to_string(), request).await;

        // Assert: ValidationError
        assert!(result.is_err());
        match result {
            Err(ApplicationError::ValidationError(msg)) => {
                assert!(msg.contains("Email already in use"));
            }
            _ => panic!("Expected ValidationError error"),
        }
    }

    #[tokio::test]
    async fn test_update_user_username_only() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        // 既存ユーザーを返す
        let existing_username = Username::new("olduser".to_string()).unwrap();
        let existing_email = Email::new("user@example.com".to_string()).unwrap();
        let existing_user = User::new(existing_username, existing_email);
        let user_id = existing_user.id();

        let mock_user_for_find = existing_user.clone();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(mock_user_for_find.clone())));

        // 保存成功
        mock_repo.expect_save().times(1).returning(|_| Ok(()));

        // Event Bus 作成
        let (event_bus, _rx) = broadcast::channel(10);

        // Use Case 生成
        let use_case = UpdateUserUseCase::new(Arc::new(mock_repo), event_bus);

        // Act: Username のみ更新
        let request = UpdateUserRequest {
            username: Some("newuser".to_string()),
            email: None,
            password: None,
        };

        let result = use_case.execute(&user_id.to_string(), request).await;

        // Assert: 成功
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "newuser");
        assert_eq!(user_dto.email, "user@example.com"); // Email は変更なし
    }
}
