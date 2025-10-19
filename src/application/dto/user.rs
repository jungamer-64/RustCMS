// src/application/dto/user.rs
//! User DTOs
//!
//! User エンティティと HTTP レスポンス/リクエストの間のデータ変換を担当します。

use crate::domain::user::{Email, User, Username};
use crate::common::types::DomainError;
use serde::{Deserialize, Serialize};

/// ユーザーレスポンス DTO
///
/// ユーザー情報を HTTP レスポンスで返すための DTO です。
/// 将来的に詳細情報（profile など）を追加する際は profile フィールドを有効化してください。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    // 将来の拡張用：プロフィール情報
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub profile: Option<UserProfileDto>,
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

/// ユーザー一覧用 DTO
///
/// UserDto と同等のフィールドを持ちます。
/// 将来的に詳細用と一覧用で異なるフィールドが必要になった場合のみ分離してください。
pub type UserListDto = UserDto;

/// ユーザー登録リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// ユーザーと認証情報をセットで扱う DTO
///
/// アプリケーション層でユーザー登録時に使用します。
/// Domain層の User は認証情報を持たないため、この構造体で分離します。
/// これにより「ユーザー生成＋認証登録」の責務が明確になります。
#[derive(Debug, Clone)]
pub struct CreateUserWithPassword {
    pub user: User,
    pub password: String,
}

/// HTTP リクエストから ユーザー＋認証情報への変換
///
/// CreateUserRequest から User と password を分離して初期化します。
/// パスワードはアプリケーション層で別途ハッシング・保存されます。
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

/// ユーザー更新リクエスト
///
/// 一部フィールドのみの更新に対応しています。
/// リクエスト JSON で省略されたフィールドは None になります。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

// Phase 6-C: Type alias for handler compatibility
pub type UpdateUserDto = UpdateUserRequest;

/// ユーザーフィルター（HTTP Query 用）
///
/// ?username=foo&is_active=true のような Query パラメータをデシリアライズするのに使用します。
/// Actix や Axum の Query<UserFilter> で自動的に変換されます。
#[derive(Debug, Clone, Default, Deserialize)]
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

        let dto = UserListDto::from(user.clone());

        assert_eq!(dto.username, "testuser");
        assert_eq!(dto.email, "test@example.com");
        // UserListDto は UserDto のエイリアスなので、is_active も同等性を保つ
        assert_eq!(dto.is_active, user.is_active());
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
    fn test_create_user_request_try_from_user() {
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let result = CreateUserWithPassword::try_from(request).expect("Failed to convert");

        assert_eq!(result.user.username().to_string(), "testuser");
        assert_eq!(result.user.email().to_string(), "test@example.com");
        assert_eq!(result.password, "password123");
    }

    #[test]
    fn test_create_user_with_password_try_from_invalid_email() {
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            email: "invalid-email".to_string(),
            password: "password123".to_string(),
        };

        let result = CreateUserWithPassword::try_from(request);
        assert!(result.is_err(), "Should fail with invalid email");
    }

    #[test]
    fn test_create_user_with_password_try_from_invalid_username() {
        let request = CreateUserRequest {
            username: "".to_string(), // Empty username is invalid
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let result = CreateUserWithPassword::try_from(request);
        assert!(result.is_err(), "Should fail with empty username");
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

    #[test]
    fn test_update_user_request_all_fields() {
        let json = r#"{
            "username": "updateduser",
            "email": "updated@example.com",
            "password": "newpass123"
        }"#;

        let request: UpdateUserRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.username, Some("updateduser".to_string()));
        assert_eq!(request.email, Some("updated@example.com".to_string()));
        assert_eq!(request.password, Some("newpass123".to_string()));
    }

    #[test]
    fn test_user_filter_default() {
        let filter = UserFilter::default();

        assert_eq!(filter.username, None);
        assert_eq!(filter.email, None);
        assert_eq!(filter.is_active, None);
    }

    #[test]
    fn test_user_filter_deserialization() {
        // Simulate query string: ?username=test&is_active=true
        let json = r#"{
            "username": "test",
            "is_active": true
        }"#;

        let filter: UserFilter = serde_json::from_str(json).unwrap();

        assert_eq!(filter.username, Some("test".to_string()));
        assert_eq!(filter.email, None);
        assert_eq!(filter.is_active, Some(true));
    }

    #[test]
    fn test_user_filter_from_query_string() {
        // Test URL-encoded query string デシリアライズ
        // ユースケース: ?username=test&is_active=true
        let query = "username=test&is_active=true";
        let filter: UserFilter = serde_urlencoded::from_str(query).unwrap();

        assert_eq!(filter.username, Some("test".to_string()));
        assert_eq!(filter.email, None);
        assert_eq!(filter.is_active, Some(true));
    }

    #[test]
    fn test_user_filter_from_query_string_partial() {
        // Test partial query string
        let query = "email=test@example.com";
        let filter: UserFilter = serde_urlencoded::from_str(query).unwrap();

        assert_eq!(filter.username, None);
        assert_eq!(filter.email, Some("test@example.com".to_string()));
        assert_eq!(filter.is_active, None);
    }

    #[test]
    fn test_user_list_dto_is_same_as_user_dto() {
        // UserListDto がエイリアスであることを確認
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        let dto = UserDto::from(user.clone());
        let list_dto = UserListDto::from(user);

        // 両者が同等であることを確認
        assert_eq!(dto.id, list_dto.id);
        assert_eq!(dto.username, list_dto.username);
        assert_eq!(dto.email, list_dto.email);
        assert_eq!(dto.is_active, list_dto.is_active);
    }

    #[test]
    fn test_user_dto_serialization() {
        // UserDto が正しく JSON にシリアライズされることを確認
        // これは HTTP レスポンスで使用される際の動作検証です
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        let dto = UserDto::from(user);
        let json = serde_json::to_string(&dto).expect("Failed to serialize UserDto");

        // 必須フィールドが含まれていることを確認
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"email\":\"test@example.com\""));
        assert!(json.contains("\"is_active\":true"));
        assert!(json.contains("\"id\":"));
    }

    #[test]
    fn test_user_dto_serialization_includes_all_fields() {
        // 将来的に profile フィールドを追加する際の破壊的変更を検知
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        let dto = UserDto::from(user);
        let value: serde_json::Value = serde_json::to_value(&dto).unwrap();

        // 全フィールドが存在することを確認
        assert!(value.get("id").is_some(), "id field should exist");
        assert!(value.get("username").is_some(), "username field should exist");
        assert!(value.get("email").is_some(), "email field should exist");
        assert!(value.get("is_active").is_some(), "is_active field should exist");
    }
}
