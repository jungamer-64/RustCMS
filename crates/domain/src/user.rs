//! ユーザードメインモデル (User Domain Model)
//!
//! Entity + Value Objects 統合パターン（監査推奨）
//!
//! このファイルには以下が含まれます：
//! - Value Objects: UserId, Email, Username
//! - Entity: User
//! - ビジネスルール実装

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Value Objects
// ============================================================================

/// ユーザーID（NewType Pattern）
///
/// # 不変条件
/// - 内部のUUIDは常に有効
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(Uuid);

impl UserId {
    /// 新しいユーザーIDを生成
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 既存のUUIDからユーザーIDを作成
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// 内部のUUIDへの参照を取得
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// UUIDを消費して取得
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<UserId> for Uuid {
    fn from(id: UserId) -> Self {
        id.0
    }
}

impl UserId {
    /// Phase 6-C: Parse UserId from string
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidUserId` if the string is not a valid UUID
    pub fn from_string(s: &str) -> Result<Self, crate::common::types::DomainError> {
        use crate::common::types::DomainError;
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| DomainError::InvalidUserId(format!("Invalid UUID string: {}", s)))
    }
}

/// Email（検証済み）
///
/// # 不変条件
/// - 空でない
/// - '@'を含む
/// - 長さが254文字以内（RFC 5321）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    /// メールアドレスを検証して作成
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - 空の文字列
    /// - '@'を含まない
    /// - 長さが254文字を超える
    pub fn new(email: String) -> Result<Self, EmailError> {
        if email.is_empty() {
            return Err(EmailError::Empty);
        }
        if !email.contains('@') {
            return Err(EmailError::MissingAtSign);
        }
        if email.len() > 254 {
            return Err(EmailError::TooLong);
        }
        Ok(Self(email))
    }

    /// 内部の文字列への参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 文字列を消費して取得
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Email検証エラー
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmailError {
    #[error("Email cannot be empty")]
    Empty,
    #[error("Email must contain '@' sign")]
    MissingAtSign,
    #[error("Email is too long (max 254 characters)")]
    TooLong,
}

/// Username（検証済み）
///
/// # 不変条件
/// - 3〜30文字
/// - ASCII英数字とアンダースコアのみ
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Username(String);

impl Username {
    /// ユーザー名を検証して作成
    ///
    /// # Errors
    ///
    /// 以下の場合にエラーを返します：
    /// - 3文字未満
    /// - 30文字を超える
    /// - 無効な文字を含む
    pub fn new(username: String) -> Result<Self, UsernameError> {
        if username.len() < 3 {
            return Err(UsernameError::TooShort);
        }
        if username.len() > 30 {
            return Err(UsernameError::TooLong);
        }
        if !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(UsernameError::InvalidCharacters);
        }
        Ok(Self(username))
    }

    /// 内部の文字列への参照を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 文字列を消費して取得
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Username検証エラー
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UsernameError {
    #[error("Username is too short (minimum 3 characters)")]
    TooShort,
    #[error("Username is too long (maximum 30 characters)")]
    TooLong,
    #[error("Username contains invalid characters (only alphanumeric and underscore allowed)")]
    InvalidCharacters,
}

/// ユーザーロール（権限レベル）
///
/// システム内でのユーザーの権限レベルを表します。
///
/// # 不変条件
/// - 各ロールは明確に定義された権限セットを持つ
/// - ロール変更はビジネスルールに従って行われる
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// 管理者 - 全ての操作が可能
    Admin,
    /// 編集者 - コンテンツの管理が可能
    Editor,
    /// 著者 - 自分のコンテンツの作成・編集が可能
    Author,
    /// 購読者 - 閲覧のみ可能
    Subscriber,
}

impl UserRole {
    /// 文字列からUserRoleをパース
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::user::UserRole;
    ///
    /// let role = UserRole::from_str("admin").unwrap();
    /// assert_eq!(role, UserRole::Admin);
    /// ```
    ///
    /// # Errors
    ///
    /// 無効なロール名の場合、`UserRoleError::InvalidRole` を返します。
    pub fn from_str(s: &str) -> Result<Self, UserRoleError> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "editor" => Ok(Self::Editor),
            "author" => Ok(Self::Author),
            "subscriber" => Ok(Self::Subscriber),
            _ => Err(UserRoleError::InvalidRole(s.to_string())),
        }
    }

    /// UserRoleを文字列に変換
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Author => "author",
            Self::Subscriber => "subscriber",
        }
    }

    /// デフォルトロール（新規ユーザー用）
    #[must_use]
    pub const fn default_role() -> Self {
        Self::Subscriber
    }

    /// 管理者かどうかを判定
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }

    /// 編集者以上かどうかを判定
    #[must_use]
    pub const fn can_edit(&self) -> bool {
        matches!(self, Self::Admin | Self::Editor)
    }

    /// 著者以上かどうかを判定
    #[must_use]
    pub const fn can_author(&self) -> bool {
        matches!(self, Self::Admin | Self::Editor | Self::Author)
    }
}

impl Default for UserRole {
    fn default() -> Self {
        Self::default_role()
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for UserRole {
    type Err = UserRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

/// UserRole変換エラー
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UserRoleError {
    #[error("Invalid role: {0}")]
    InvalidRole(String),
}

// ============================================================================
// Entity
// ============================================================================

/// ユーザーエンティティ（ドメインモデル）
///
/// ビジネスルールとライフサイクルメソッドを含みます。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    username: Username,
    email: Email,
    password_hash: Option<String>,
    role: UserRole,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    last_login: Option<chrono::DateTime<chrono::Utc>>, // Phase 9: 最終ログイン日時
}

impl User {
    /// 新しいユーザーを作成（コンストラクタ）
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use crate::user::{User, Username, Email};
    ///
    /// let username = Username::new("john_doe".to_string()).unwrap();
    /// let email = Email::new("john@example.com".to_string()).unwrap();
    /// let user = User::new(username, email);
    /// ```
    #[must_use]
    pub fn new(username: Username, email: Email) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: UserId::new(),
            username,
            email,
            password_hash: None,
            role: UserRole::default_role(),
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login: None, // Phase 9: 初回ログインは未実施
        }
    }

    /// 既存のデータからユーザーを復元（リポジトリ用）
    #[must_use]
    pub fn restore(
        id: UserId,
        username: Username,
        email: Email,
        password_hash: Option<String>,
        role: UserRole,
        is_active: bool,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        last_login: Option<chrono::DateTime<chrono::Utc>>, // Phase 9: 最終ログイン日時
    ) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            role,
            is_active,
            created_at,
            updated_at,
            last_login,
        }
    }

    // ========================================================================
    // ビジネスルール
    // ========================================================================

    /// ユーザーを有効化
    ///
    /// ビジネスルール: 無効化されたユーザーを再度有効化できる
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// ユーザーを無効化
    ///
    /// ビジネスルール: アクティブなユーザーを無効化できる
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// メールアドレスを変更
    ///
    /// ビジネスルール: 検証済みのメールアドレスのみ設定可能
    pub fn change_email(&mut self, new_email: Email) {
        self.email = new_email;
    }

    /// ユーザー名を変更
    ///
    /// ビジネスルール: 検証済みのユーザー名のみ設定可能
    pub fn change_username(&mut self, new_username: Username) {
        self.username = new_username;
    }

    // ========================================================================
    // ゲッター
    // ========================================================================

    #[must_use]
    pub const fn id(&self) -> UserId {
        self.id
    }

    #[must_use]
    pub const fn username(&self) -> &Username {
        &self.username
    }

    #[must_use]
    pub const fn email(&self) -> &Email {
        &self.email
    }

    #[must_use]
    pub const fn role(&self) -> UserRole {
        self.role
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.is_active
    }

    #[must_use]
    pub const fn password_hash(&self) -> Option<&String> {
        self.password_hash.as_ref()
    }

    #[must_use]
    pub const fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.created_at
    }

    #[must_use]
    pub const fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.updated_at
    }

    /// Phase 9: 最終ログイン日時を取得
    #[must_use]
    pub const fn last_login(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.last_login
    }

    // ========================================================================
    // Phase 9: 認証関連メソッド
    // ========================================================================

    /// パスワードハッシュを設定
    ///
    /// ビジネスルール: パスワードは既にハッシュ化されている必要がある
    pub fn set_password_hash(&mut self, hash: String) {
        self.password_hash = Some(hash);
        self.updated_at = chrono::Utc::now();
    }

    /// 最終ログイン日時を更新
    ///
    /// ビジネスルール: ログイン成功時に呼び出される
    pub fn update_last_login(&mut self) {
        self.last_login = Some(chrono::Utc::now());
        self.updated_at = chrono::Utc::now();
    }

    // ========================================================================
    // ロール関連メソッド
    // ========================================================================

    /// ロールを変更
    ///
    /// ビジネスルール: 権限レベルの変更は管理者のみ実行可能
    pub fn change_role(&mut self, new_role: UserRole) {
        self.role = new_role;
    }

    /// 管理者かどうかを判定
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        self.role.is_admin()
    }

    /// 編集権限を持つかどうかを判定
    #[must_use]
    pub const fn can_edit(&self) -> bool {
        self.role.can_edit()
    }

    /// 執筆権限を持つかどうかを判定
    #[must_use]
    pub const fn can_author(&self) -> bool {
        self.role.can_author()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Value Objects Tests
    mod value_objects {
        use super::*;

        mod user_id {
            use super::*;

            #[test]
            fn test_new_generates_unique_ids() {
                let id1 = UserId::new();
                let id2 = UserId::new();
                assert_ne!(id1, id2);
            }

            #[test]
            fn test_from_uuid() {
                let uuid = Uuid::new_v4();
                let user_id = UserId::from_uuid(uuid);
                assert_eq!(user_id.as_uuid(), &uuid);
            }

            #[test]
            fn test_display() {
                let id = UserId::new();
                let display = format!("{id}");
                assert!(!display.is_empty());
            }
        }

        mod email {
            use super::*;

            #[test]
            fn test_valid_email() {
                let email = Email::new("test@example.com".to_string()).unwrap();
                assert_eq!(email.as_str(), "test@example.com");
            }

            #[test]
            fn test_empty_email_fails() {
                let result = Email::new(String::new());
                assert!(matches!(result, Err(EmailError::Empty)));
            }

            #[test]
            fn test_missing_at_sign_fails() {
                let result = Email::new("invalid-email".to_string());
                assert!(matches!(result, Err(EmailError::MissingAtSign)));
            }

            #[test]
            fn test_too_long_email_fails() {
                let long_email = "a".repeat(250) + "@example.com";
                let result = Email::new(long_email);
                assert!(matches!(result, Err(EmailError::TooLong)));
            }

            #[test]
            fn test_display() {
                let email = Email::new("test@example.com".to_string()).unwrap();
                assert_eq!(format!("{email}"), "test@example.com");
            }
        }

        mod username {
            use super::*;

            #[test]
            fn test_valid_username() {
                let username = Username::new("john_doe".to_string()).unwrap();
                assert_eq!(username.as_str(), "john_doe");
            }

            #[test]
            fn test_too_short_username_fails() {
                let result = Username::new("ab".to_string());
                assert!(matches!(result, Err(UsernameError::TooShort)));
            }

            #[test]
            fn test_too_long_username_fails() {
                let long_name = "a".repeat(31);
                let result = Username::new(long_name);
                assert!(matches!(result, Err(UsernameError::TooLong)));
            }

            #[test]
            fn test_invalid_characters_fail() {
                let result = Username::new("john@doe".to_string());
                assert!(matches!(result, Err(UsernameError::InvalidCharacters)));
            }

            #[test]
            fn test_alphanumeric_and_underscore_allowed() {
                let username = Username::new("user_123".to_string()).unwrap();
                assert_eq!(username.as_str(), "user_123");
            }
        }
    }

    // Entity Tests
    mod entity {
        use super::*;

        #[test]
        fn test_new_user() {
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let user = User::new(username, email);

            assert!(user.is_active());
            assert_eq!(user.username().as_str(), "testuser");
            assert_eq!(user.email().as_str(), "test@example.com");
        }

        #[test]
        fn test_activate_deactivate() {
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let mut user = User::new(username, email);

            assert!(user.is_active());

            user.deactivate();
            assert!(!user.is_active());

            user.activate();
            assert!(user.is_active());
        }

        #[test]
        fn test_change_email() {
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("old@example.com".to_string()).unwrap();
            let mut user = User::new(username, email);

            let new_email = Email::new("new@example.com".to_string()).unwrap();
            user.change_email(new_email);

            assert_eq!(user.email().as_str(), "new@example.com");
        }

        #[test]
        fn test_change_username() {
            let username = Username::new("oldname".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let mut user = User::new(username, email);

            let new_username = Username::new("newname".to_string()).unwrap();
            user.change_username(new_username);

            assert_eq!(user.username().as_str(), "newname");
        }

        #[test]
        fn test_restore() {
            let id = UserId::new();
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let role = UserRole::Author;
            let now = chrono::Utc::now();
            let user = User::restore(
                id,
                username.clone(),
                email.clone(),
                None,
                role,
                false,
                now,
                now,
                None, // Phase 9: last_login
            );

            assert_eq!(user.id(), id);
            assert_eq!(user.username(), &username);
            assert_eq!(user.role(), role);
            assert!(!user.is_active());
            assert!(user.last_login().is_none());
        }

        /// ⚠️ 追加エッジケーステスト
        mod edge_cases {
            use super::*;

            #[test]
            fn test_boundary_email_length() {
                // 最大長（254文字）の有効なメール
                let long_local = "a".repeat(242); // 242 + @ + example.com (11) = 254
                let email_str = format!("{}@example.com", long_local);
                assert_eq!(email_str.len(), 254);
                let email = Email::new(email_str).unwrap();
                assert_eq!(email.as_str().len(), 254);
            }

            #[test]
            fn test_boundary_username_length() {
                // 最小（3）と最大（30）
                let min_username = Username::new("abc".to_string()).unwrap();
                assert_eq!(min_username.as_str().len(), 3);

                let max_username = Username::new("a".repeat(30).to_string()).unwrap();
                assert_eq!(max_username.as_str().len(), 30);
            }

            #[test]
            fn test_multiple_email_changes() {
                let username = Username::new("testuser".to_string()).unwrap();
                let email1 = Email::new("email1@example.com".to_string()).unwrap();
                let mut user = User::new(username, email1);

                let email2 = Email::new("email2@example.com".to_string()).unwrap();
                user.change_email(email2);
                assert_eq!(user.email().as_str(), "email2@example.com");

                let email3 = Email::new("email3@example.com".to_string()).unwrap();
                user.change_email(email3);
                assert_eq!(user.email().as_str(), "email3@example.com");
            }

            #[test]
            fn test_multiple_username_changes() {
                let username1 = Username::new("user1".to_string()).unwrap();
                let email = Email::new("test@example.com".to_string()).unwrap();
                let mut user = User::new(username1, email);

                let username2 = Username::new("user2".to_string()).unwrap();
                user.change_username(username2);
                assert_eq!(user.username().as_str(), "user2");

                let username3 = Username::new("user3".to_string()).unwrap();
                user.change_username(username3);
                assert_eq!(user.username().as_str(), "user3");
            }

            #[test]
            fn test_activation_state_persistence() {
                let username = Username::new("testuser".to_string()).unwrap();
                let email = Email::new("test@example.com".to_string()).unwrap();
                let mut user = User::new(username, email);

                // 複数の状態遷移
                assert!(user.is_active());
                user.deactivate();
                assert!(!user.is_active());
                user.deactivate(); // 二重無効化
                assert!(!user.is_active());
                user.activate();
                assert!(user.is_active());
                user.activate(); // 二重有効化
                assert!(user.is_active());
            }

            #[test]
            fn test_user_id_uniqueness_across_multiple_creations() {
                let ids: Vec<_> = (0..100).map(|_| UserId::new()).collect();
                let unique_ids: std::collections::HashSet<_> = ids.iter().copied().collect();
                assert_eq!(unique_ids.len(), 100, "All 100 user IDs should be unique");
            }
        }

        /// Value Objects の相互運用性テスト
        mod interoperability {
            use super::*;

            #[test]
            fn test_email_serialization_roundtrip() {
                let original = Email::new("test@example.com".to_string()).unwrap();
                let json = serde_json::to_string(&original).unwrap();
                let restored: Email = serde_json::from_str(&json).unwrap();
                assert_eq!(original, restored);
            }

            #[test]
            fn test_username_serialization_roundtrip() {
                let original = Username::new("testuser".to_string()).unwrap();
                let json = serde_json::to_string(&original).unwrap();
                let restored: Username = serde_json::from_str(&json).unwrap();
                assert_eq!(original, restored);
            }

            #[test]
            fn test_user_id_serialization_roundtrip() {
                let original = UserId::new();
                let json = serde_json::to_string(&original).unwrap();
                let restored: UserId = serde_json::from_str(&json).unwrap();
                assert_eq!(original, restored);
            }
        }

        // ====================================================================
        // Phase 9: 認証関連テスト
        // ====================================================================

        #[test]
        fn test_set_password_hash() {
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let mut user = User::new(username, email);

            assert!(user.password_hash().is_none());

            let hash = "hashed_password_123".to_string();
            user.set_password_hash(hash.clone());

            assert_eq!(user.password_hash(), Some(&hash));
        }

        #[test]
        fn test_update_last_login() {
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let mut user = User::new(username, email);

            // 初期状態ではNone
            assert!(user.last_login().is_none());

            // 最初のログイン
            user.update_last_login();
            let first_login = user.last_login();
            assert!(first_login.is_some());

            // 少し待ってから再ログイン
            std::thread::sleep(std::time::Duration::from_millis(10));
            user.update_last_login();
            let second_login = user.last_login();
            assert!(second_login.is_some());

            // 最終ログイン日時が更新されている
            assert!(second_login > first_login);
        }

        #[test]
        fn test_restore_with_last_login() {
            let id = UserId::new();
            let username = Username::new("testuser".to_string()).unwrap();
            let email = Email::new("test@example.com".to_string()).unwrap();
            let role = UserRole::Author;
            let now = chrono::Utc::now();
            let last_login_time = Some(now - chrono::Duration::days(1));

            let user = User::restore(
                id,
                username.clone(),
                email.clone(),
                Some("hashed_password".to_string()),
                role,
                true,
                now - chrono::Duration::days(7),
                now,
                last_login_time,
            );

            assert_eq!(user.last_login(), last_login_time);
            assert_eq!(user.password_hash(), Some(&"hashed_password".to_string()));
        }
    }
}
