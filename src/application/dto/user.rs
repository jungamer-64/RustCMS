//! User DTOs
//!
//! User エンティティと HTTP レスポンス/リクエストの間のデータ変換

use crate::common::types::DomainError;
use crate::domain::user::{Email, User, Username};
use serde::{Deserialize, Serialize};

// ============================================================================
// Response DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

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

impl From<&User> for UserDto {
    fn from(user: &User) -> Self {
        Self {
            id: user.id().to_string(),
            username: user.username().to_string(),
            email: user.email().to_string(),
            is_active: user.is_active(),
        }
    }
}

/// 一覧用 DTO（将来的に詳細と分離する可能性あり）
pub type UserListDto = UserDto;

// ============================================================================
// Request DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

pub type UpdateUserDto = UpdateUserRequest;

// ============================================================================
// Domain Conversion
// ============================================================================

/// ユーザーと認証情報をセットで扱う DTO
#[derive(Debug, Clone)]
pub struct CreateUserWithPassword {
    pub user: User,
    pub password: String,
}

impl TryFrom<CreateUserRequest> for CreateUserWithPassword {
    type Error = DomainError;

    fn try_from(req: CreateUserRequest) -> Result<Self, Self::Error> {
        let username = Username::new(req.username)?;
        let email = Email::new(req.email)?;
        Ok(Self {
            user: User::new(username, email),
            password: req.password,
        })
    }
}

// ============================================================================
// Query Filter
// ============================================================================

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserFilter {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user() -> User {
        User::new(
            Username::new("testuser".into()).unwrap(),
            Email::new("test@example.com".into()).unwrap(),
        )
    }

    #[test]
    fn test_user_dto_from_user() {
        let user = create_test_user();
        let dto = UserDto::from(user.clone());

        assert_eq!(dto.id, user.id().to_string());
        assert_eq!(dto.username, "testuser");
        assert_eq!(dto.email, "test@example.com");
        assert!(dto.is_active);
    }

    #[test]
    fn test_user_dto_from_reference() {
        let user = create_test_user();
        let dto = UserDto::from(&user);

        assert_eq!(dto.id, user.id().to_string());
        assert_eq!(dto.username, "testuser");
    }

    #[test]
    fn test_create_user_with_password_try_from() {
        let request = CreateUserRequest {
            username: "testuser".into(),
            email: "test@example.com".into(),
            password: "password123".into(),
        };

        let result = CreateUserWithPassword::try_from(request);
        assert!(result.is_ok());

        let converted = result.unwrap();
        assert_eq!(converted.user.username().to_string(), "testuser");
        assert_eq!(converted.password, "password123");
    }

    #[test]
    fn test_create_user_with_password_invalid_email() {
        let request = CreateUserRequest {
            username: "testuser".into(),
            email: "invalid-email".into(),
            password: "password123".into(),
        };

        let result = CreateUserWithPassword::try_from(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_user_request_partial() {
        let json = r#"{"username": "updateduser"}"#;
        let request: UpdateUserRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.username.as_deref(), Some("updateduser"));
        assert!(request.email.is_none());
        assert!(request.password.is_none());
    }

    #[test]
    fn test_user_filter_default() {
        let filter = UserFilter::default();
        assert!(filter.username.is_none());
        assert!(filter.email.is_none());
        assert!(filter.is_active.is_none());
    }
}
