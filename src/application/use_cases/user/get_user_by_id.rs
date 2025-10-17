// src/application/use_cases/user/get_user_by_id.rs
//! GetUserByIdUseCase - ユーザー詳細取得 Use Case
//!
//! Phase 3 Week 8-9: Use Case 実装

use crate::application::dto::user::UserDto;
use crate::application::ports::repositories::UserRepository;
use crate::common::types::{ApplicationError, ApplicationResult};
use crate::domain::user::UserId;
use std::sync::Arc;
use uuid::Uuid;

/// ユーザー詳細取得 Use Case
///
/// # 責務
/// - UserId の解析
/// - Repository からの取得
/// - UserDto への変換
pub struct GetUserByIdUseCase {
    user_repository: Arc<dyn UserRepository>,
}

impl GetUserByIdUseCase {
    /// Use Case の生成
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }

    /// ユーザー詳細取得を実行
    ///
    /// # Arguments
    /// - `user_id_str` - ユーザーID（UUID文字列）
    ///
    /// # Returns
    /// - `Ok(UserDto)` - ユーザー詳細
    /// - `Err(ApplicationError)` - ユーザーが見つからない、または不正なID
    ///
    /// # Errors
    /// - `ValidationFailed` - UUID形式が不正
    /// - `NotFound` - ユーザーが存在しない
    /// - `RepositoryError` - データベースエラー
    pub async fn execute(&self, user_id_str: &str) -> ApplicationResult<UserDto> {
        // 1. UUID 文字列 → UserId 変換
        let uuid = Uuid::parse_str(user_id_str)
            .map_err(|_| ApplicationError::ValidationError("Invalid UUID format".to_string()))?;

        let user_id = UserId::from_uuid(uuid);

        // 2. Repository から取得
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("User not found: {}", user_id)))?;

        // 3. Domain → DTO 変換
        Ok(UserDto::from(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::MockUserRepository;
    use crate::domain::user::{Email, User, Username};
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_get_user_by_id_success() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);
        let user_id = user.id();

        mock_repo
            .expect_find_by_id()
            .with(eq(user_id))
            .times(1)
            .returning(move |_| Ok(Some(user.clone())));

        // Use Case 生成
        let use_case = GetUserByIdUseCase::new(Arc::new(mock_repo));

        // Act: ユーザー取得実行
        let result = use_case.execute(&user_id.to_string()).await;

        // Assert: 成功
        assert!(result.is_ok());
        let user_dto = result.unwrap();
        assert_eq!(user_dto.username, "testuser");
        assert_eq!(user_dto.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_get_user_by_id_not_found() {
        // Arrange: Mock Repository
        let mut mock_repo = MockUserRepository::new();

        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(|_| Ok(None));

        // Use Case 生成
        let use_case = GetUserByIdUseCase::new(Arc::new(mock_repo));

        // Act: 存在しないユーザーを取得
        let result = use_case.execute(&Uuid::new_v4().to_string()).await;

        // Assert: NotFound
        assert!(result.is_err());
        match result {
            Err(ApplicationError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_user_by_id_invalid_uuid() {
        // Arrange: Mock Repository（呼び出されない）
        let mock_repo = MockUserRepository::new();

        // Use Case 生成
        let use_case = GetUserByIdUseCase::new(Arc::new(mock_repo));

        // Act: 不正なUUIDで取得
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
}
