// src/application/dto/user.rs
//! User DTOs
//!
//! User エンティティと HTTP レスポンス/リクエストの間のデータ変換を担当します。

use crate::domain::user::{Email, User, Username};
use serde::{Deserialize, Serialize};

/// ユーザーレスポンス DTO（完全版）
#[derive(Debug, Clone, Serialize)]
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

/// ユーザー一覧用 DTO（最小限フィールド）
#[derive(Debug, Clone, Serialize)]
pub struct UserListDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

impl From<User> for UserListDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id().to_string(),
            username: user.username().to_string(),
            email: user.email().to_string(),
            is_active: user.is_active(),
        }
    }
}

/// ユーザー登録リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// ユーザー更新リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// ユーザーフィルター（Query 用）
#[derive(Debug, Clone, Default)]
pub struct UserFilter {
    pub username: Option<String>,
    pub email: Option<String>,
    pub is_active: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_dto_from_user() {
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        let dto = UserDto::from(user.clone());

        assert_eq!(dto.id, user.id().to_string());
        assert_eq!(dto.username, "testuser");
        assert_eq!(dto.email, "test@example.com");
        assert_eq!(dto.is_active, true);
    }

    #[test]
    fn test_user_list_dto_from_user() {
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        let dto = UserListDto::from(user);

        assert_eq!(dto.username, "testuser");
        assert_eq!(dto.email, "test@example.com");
    }

    #[test]
    fn test_create_user_request_deserialization() {
        let json = r#"{
            "username": "newuser",
            "email": "new@example.com",
            "password": "password123"
        }"#;

        let request: CreateUserRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.username, "newuser");
        assert_eq!(request.email, "new@example.com");
        assert_eq!(request.password, "password123");
    }

    #[test]
    fn test_update_user_request_partial() {
        let json = r#"{
            "username": "updateduser"
        }"#;

        let request: UpdateUserRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.username, Some("updateduser".to_string()));
        assert_eq!(request.email, None);
        assert_eq!(request.password, None);
    }
}
