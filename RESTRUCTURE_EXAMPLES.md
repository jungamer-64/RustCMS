# RustCMS 構造再編 - 実装例

本ドキュメントは `RESTRUCTURE_PLAN.md` に記載された計画の具体的な実装例を示します。

> **更新情報**: 2025年版 監査済み構造（Sonnet 4.5）に基づく実装例を追加しました。  
> **ファイル統合パターン**: Entity + Value Objects を単一ファイルに統合する方式を採用しています。

## 目次

- [🆕 監査済み構造の実装例（推奨）](#監査済み構造の実装例推奨)
  - [domain/user.rs（Entity + Value Objects 統合）](#domainuserrs-entity--value-objects-統合)
  - [application/user.rs（CQRS統合）](#applicationuserrs-cqrs統合)
  - [application/ports/repositories.rs（Port定義）](#applicationportsrepositoriesrs-port定義)
  - [infrastructure/database/repositories.rs（実装）](#infrastructuredatabaserepositoriesrs-実装)
- [Value Objects の実装例（分割版）](#value-objects-の実装例分割版)
- [Entity の実装例](#entity-の実装例)
- [Repository Pattern の実装例](#repository-pattern-の実装例)
- [Use Case の実装例](#use-case-の実装例)
- [Handler の実装例](#handler-の実装例)
- [エラーハンドリングの実装例](#エラーハンドリングの実装例)

---

## 監査済み構造の実装例（推奨）

### domain/user.rs（Entity + Value Objects 統合）

監査結果に基づき、**Entity と Value Objects を単一ファイルに統合**します。この方式は:

- ✅ 関連する型の局所化（高凝集）
- ✅ import 文の削減
- ✅ 500行以下なら単一ファイルで十分

```rust
// src/domain/user.rs
// Entity + Value Objects 統合パターン（監査推奨）

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use crate::error::DomainError;

// ============================================================================
// Value Objects
// ============================================================================

/// ユーザーID（NewType Pattern）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
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

/// Email（検証済み）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn new(email: String) -> Result<Self, DomainError> {
        if !email.contains('@') || email.len() > 255 {
            return Err(DomainError::InvalidEmail);
        }
        Ok(Self(email))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Username（検証済み）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Username(String);

impl Username {
    pub fn new(username: String) -> Result<Self, DomainError> {
        if username.len() < 3 || username.len() > 30 {
            return Err(DomainError::InvalidUsername);
        }
        Ok(Self(username))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Entity
// ============================================================================

/// ユーザーエンティティ（ドメインモデル）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    username: Username,
    email: Email,
    is_active: bool,
}

impl User {
    /// 新しいユーザーを作成（コンストラクタ）
    pub fn new(username: Username, email: Email) -> Self {
        Self {
            id: UserId::new(),
            username,
            email,
            is_active: true,
        }
    }

    /// ビジネスルール: ユーザーを有効化
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// ビジネスルール: ユーザーを無効化
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// ゲッター
    pub const fn id(&self) -> UserId {
        self.id
    }

    pub fn username(&self) -> &Username {
        &self.username
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub const fn is_active(&self) -> bool {
        self.is_active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let username = Username::new("testuser".to_string()).unwrap();
        let email = Email::new("test@example.com".to_string()).unwrap();
        let user = User::new(username, email);

        assert!(user.is_active());
    }

    #[test]
    fn test_invalid_email() {
        let result = Email::new("invalid-email".to_string());
        assert!(result.is_err());
    }
}
```

### application/user.rs（CQRS統合）

監査結果に基づき、**Commands + Queries + DTOs を単一ファイルに統合**します。

```rust
// src/application/user.rs
// CQRS統合パターン（監査推奨）

use crate::domain::user::{User, UserId, Username, Email};
use crate::application::ports::repositories::UserRepository;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// DTOs
// ============================================================================

/// ユーザー作成リクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

/// ユーザーレスポンス
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
            username: user.username().as_str().to_string(),
            email: user.email().as_str().to_string(),
            is_active: user.is_active(),
        }
    }
}

// ============================================================================
// Commands（書き込み操作）
// ============================================================================

/// ユーザー登録コマンド
pub struct RegisterUser {
    repo: Arc<dyn UserRepository>,
}

impl RegisterUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, req: CreateUserRequest) -> Result<UserDto, AppError> {
        // Value Objects作成（検証込み）
        let username = Username::new(req.username)?;
        let email = Email::new(req.email)?;

        // エンティティ作成
        let user = User::new(username, email);

        // 永続化
        self.repo.save(user.clone()).await?;

        Ok(user.into())
    }
}

// ============================================================================
// Queries（読み取り操作）
// ============================================================================

/// ユーザー取得クエリ
pub struct GetUserById {
    repo: Arc<dyn UserRepository>,
}

impl GetUserById {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: UserId) -> Result<UserDto, AppError> {
        let user = self.repo.find_by_id(id).await?;
        Ok(user.into())
    }
}
```

### application/ports/repositories.rs（Port定義）

監査結果に基づき、**複数のRepository traitを単一ファイルに統合**します。

```rust
// src/application/ports/repositories.rs
// Repository Port定義（監査推奨: 単一ファイル統合）

use crate::domain::user::{User, UserId};
use crate::domain::post::{Post, PostId};
use crate::error::AppError;
use async_trait::async_trait;

/// ユーザーリポジトリ（Port/Interface）
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: User) -> Result<(), AppError>;
    async fn find_by_id(&self, id: UserId) -> Result<User, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn delete(&self, id: UserId) -> Result<(), AppError>;
}

/// 投稿リポジトリ（Port/Interface）
#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn save(&self, post: Post) -> Result<(), AppError>;
    async fn find_by_id(&self, id: PostId) -> Result<Post, AppError>;
    async fn list_all(&self) -> Result<Vec<Post>, AppError>;
    async fn delete(&self, id: PostId) -> Result<(), AppError>;
}
```

### infrastructure/database/repositories.rs（実装）

監査結果に基づき、**複数のRepository実装を単一ファイルに統合**します。

```rust
// src/infrastructure/database/repositories.rs
// Repository実装（監査推奨: 単一ファイル統合）

use crate::application::ports::repositories::{UserRepository, PostRepository};
use crate::domain::user::{User, UserId};
use crate::domain::post::{Post, PostId};
use crate::error::AppError;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use async_trait::async_trait;

pub struct DieselUserRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DieselUserRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn save(&self, user: User) -> Result<(), AppError> {
        // Diesel実装
        todo!()
    }

    async fn find_by_id(&self, id: UserId) -> Result<User, AppError> {
        // Diesel実装
        todo!()
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        // Diesel実装
        todo!()
    }

    async fn delete(&self, id: UserId) -> Result<(), AppError> {
        // Diesel実装
        todo!()
    }
}

pub struct DieselPostRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl DieselPostRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PostRepository for DieselPostRepository {
    async fn save(&self, post: Post) -> Result<(), AppError> {
        // Diesel実装
        todo!()
    }

    async fn find_by_id(&self, id: PostId) -> Result<Post, AppError> {
        // Diesel実装
        todo!()
    }

    async fn list_all(&self) -> Result<Vec<Post>, AppError> {
        // Diesel実装
        todo!()
    }

    async fn delete(&self, id: PostId) -> Result<(), AppError> {
        // Diesel実装
        todo!()
    }
}
```

---

## Value Objects の実装例（分割版）

以下は従来の分割版の実装例です。ファイル数が増えても良い場合はこちらを参照してください。

### UserId (NewType Pattern)

## Value Objects の実装例

### UserId (NewType Pattern)

```rust
// src/domain/value_objects/user_id.rs

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// ユーザーの一意識別子
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_user_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let user_id = UserId::from_uuid(uuid);
        assert_eq!(user_id.as_uuid(), &uuid);
    }

    #[test]
    fn test_serialization() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
```

### Email (検証済み値オブジェクト)

```rust
// src/domain/value_objects/email.rs

use crate::domain::errors::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 検証済みメールアドレス
///
/// # 不変条件
/// - 空でない
/// - '@'を含む
/// - 長さが254文字以内（RFC 5321）
/// - 基本的なフォーマットに従う
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Email(String);

impl Email {
    /// メールアドレスを検証して作成
    ///
    /// # Errors
    ///
    /// メールアドレスが無効な場合、`DomainError::InvalidEmail` を返す
    pub fn new(value: String) -> Result<Self, DomainError> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// 検証ロジック
    fn validate(value: &str) -> Result<(), DomainError> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::InvalidEmail(
                "Email address is required".to_string(),
            ));
        }

        if trimmed.len() > 254 {
            return Err(DomainError::InvalidEmail(
                "Email address is too long (max 254 characters)".to_string(),
            ));
        }

        // 基本的なフォーマット検証
        let parts: Vec<&str> = trimmed.split('@').collect();
        if parts.len() != 2 {
            return Err(DomainError::InvalidEmail(
                "Email address must contain exactly one '@'".to_string(),
            ));
        }

        let local = parts[0];
        let domain = parts[1];

        if local.is_empty() {
            return Err(DomainError::InvalidEmail(
                "Local part of email cannot be empty".to_string(),
            ));
        }

        if domain.is_empty() || !domain.contains('.') {
            return Err(DomainError::InvalidEmail(
                "Domain part of email is invalid".to_string(),
            ));
        }

        Ok(())
    }

    /// メールアドレスの文字列表現を取得
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// メールアドレスを消費して文字列を取得
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for Email {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Email> for String {
    fn from(email: Email) -> Self {
        email.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email() {
        let email = Email::new("user@example.com".to_string());
        assert!(email.is_ok());
    }

    #[test]
    fn test_empty_email() {
        let email = Email::new("".to_string());
        assert!(matches!(email, Err(DomainError::InvalidEmail(_))));
    }

    #[test]
    fn test_email_without_at() {
        let email = Email::new("userexample.com".to_string());
        assert!(matches!(email, Err(DomainError::InvalidEmail(_))));
    }

    #[test]
    fn test_email_too_long() {
        let long_email = format!("{}@example.com", "a".repeat(250));
        let email = Email::new(long_email);
        assert!(matches!(email, Err(DomainError::InvalidEmail(_))));
    }

    #[test]
    fn test_email_multiple_at() {
        let email = Email::new("user@@example.com".to_string());
        assert!(matches!(email, Err(DomainError::InvalidEmail(_))));
    }
}
```

---

## Entity の実装例

### User Entity

```rust
// src/domain/entities/user.rs

use crate::domain::errors::DomainError;
use crate::domain::value_objects::{Email, UserId, Username};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// ユーザーエンティティ
///
/// ビジネスルールとライフサイクルメソッドを含む
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: UserId,
    username: Username,
    email: Email,
    password_hash: String,
    role: UserRole,
    status: UserStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    User,
    Editor,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl User {
    /// 新しいユーザーを作成（ファクトリメソッド）
    ///
    /// # Errors
    ///
    /// パスワードが弱い場合、`DomainError` を返す
    pub fn create(
        username: Username,
        email: Email,
        password_hash: String,
    ) -> Result<Self, DomainError> {
        // ビジネスルール: パスワードハッシュは空でない
        if password_hash.is_empty() {
            return Err(DomainError::InvalidPassword(
                "Password hash cannot be empty".to_string(),
            ));
        }

        let now = Utc::now();
        Ok(Self {
            id: UserId::new(),
            username,
            email,
            password_hash,
            role: UserRole::User,
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        })
    }

    /// 既存データからユーザーを再構築（リポジトリ用）
    #[must_use]
    pub fn reconstruct(
        id: UserId,
        username: Username,
        email: Email,
        password_hash: String,
        role: UserRole,
        status: UserStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        last_login_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            username,
            email,
            password_hash,
            role,
            status,
            created_at,
            updated_at,
            last_login_at,
        }
    }

    // === Getters ===

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
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    #[must_use]
    pub const fn role(&self) -> UserRole {
        self.role
    }

    #[must_use]
    pub const fn status(&self) -> UserStatus {
        self.status
    }

    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[must_use]
    pub const fn last_login_at(&self) -> Option<DateTime<Utc>> {
        self.last_login_at
    }

    // === ビジネスメソッド ===

    /// ユーザーがアクティブかチェック
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    /// ユーザーが管理者かチェック
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }

    /// ログイン時刻を記録
    pub fn record_login(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// ロールを変更
    ///
    /// # Errors
    ///
    /// ユーザーがアクティブでない場合、エラーを返す
    pub fn change_role(&mut self, new_role: UserRole) -> Result<(), DomainError> {
        if !self.is_active() {
            return Err(DomainError::UserNotActive(
                "Cannot change role of inactive user".to_string(),
            ));
        }
        self.role = new_role;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// ユーザーを停止
    ///
    /// # Errors
    ///
    /// すでに停止済みの場合、エラーを返す
    pub fn suspend(&mut self) -> Result<(), DomainError> {
        if matches!(self.status, UserStatus::Suspended) {
            return Err(DomainError::InvalidOperation(
                "User is already suspended".to_string(),
            ));
        }
        self.status = UserStatus::Suspended;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// ユーザーを有効化
    ///
    /// # Errors
    ///
    /// すでにアクティブの場合、エラーを返す
    pub fn activate(&mut self) -> Result<(), DomainError> {
        if self.is_active() {
            return Err(DomainError::InvalidOperation(
                "User is already active".to_string(),
            ));
        }
        self.status = UserStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Username;

    fn create_test_user() -> User {
        User::create(
            Username::new("testuser".to_string()).unwrap(),
            Email::new("test@example.com".to_string()).unwrap(),
            "hashed_password".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn test_create_user() {
        let user = create_test_user();
        assert!(user.is_active());
        assert_eq!(user.role(), UserRole::User);
    }

    #[test]
    fn test_record_login() {
        let mut user = create_test_user();
        assert!(user.last_login_at().is_none());

        user.record_login();
        assert!(user.last_login_at().is_some());
    }

    #[test]
    fn test_change_role() {
        let mut user = create_test_user();
        user.change_role(UserRole::Admin).unwrap();
        assert!(user.is_admin());
    }

    #[test]
    fn test_suspend_user() {
        let mut user = create_test_user();
        user.suspend().unwrap();
        assert!(!user.is_active());
        assert_eq!(user.status(), UserStatus::Suspended);
    }
}
```

---

## Repository Pattern の実装例

### UserRepository Trait (Port)

```rust
// src/application/ports/user_repository.rs

use crate::domain::entities::User;
use crate::domain::value_objects::{Email, UserId};
use crate::infrastructure::database::errors::RepositoryError;
use async_trait::async_trait;

/// ユーザーリポジトリのポート（インターフェース）
///
/// インフラストラクチャ層がこのtraitを実装する
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// IDでユーザーを検索
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;

    /// メールアドレスでユーザーを検索
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;

    /// ユーザーを保存（新規作成または更新）
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;

    /// ユーザーを削除
    async fn delete(&self, id: UserId) -> Result<(), RepositoryError>;

    /// すべてのユーザーを取得（ページネーション付き）
    async fn find_all(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<User>, RepositoryError>;
}
```

### DieselUserRepository (実装)

```rust
// src/infrastructure/database/repositories/user_repository_impl.rs

use crate::application::ports::UserRepository;
use crate::domain::entities::{User, UserRole, UserStatus};
use crate::domain::value_objects::{Email, UserId, Username};
use crate::infrastructure::database::errors::RepositoryError;
use crate::infrastructure::database::models::UserModel;
use crate::infrastructure::database::DbPool;
use async_trait::async_trait;
use diesel::prelude::*;
use std::sync::Arc;

/// Dieselを使用したUserRepositoryの実装
pub struct DieselUserRepository {
    pool: Arc<DbPool>,
}

impl DieselUserRepository {
    #[must_use]
    pub const fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }

    /// ドメインエンティティをDBモデルに変換
    fn to_db_model(user: &User) -> UserModel {
        UserModel {
            id: *user.id().as_uuid(),
            username: user.username().as_str().to_string(),
            email: user.email().as_str().to_string(),
            password_hash: user.password_hash().to_string(),
            role: format!("{:?}", user.role()).to_lowercase(),
            status: format!("{:?}", user.status()).to_lowercase(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
            last_login_at: user.last_login_at(),
        }
    }

    /// DBモデルをドメインエンティティに変換
    fn from_db_model(model: UserModel) -> Result<User, RepositoryError> {
        let username = Username::new(model.username)
            .map_err(|e| RepositoryError::MappingFailed(e.to_string()))?;

        let email = Email::new(model.email)
            .map_err(|e| RepositoryError::MappingFailed(e.to_string()))?;

        let role = match model.role.as_str() {
            "user" => UserRole::User,
            "editor" => UserRole::Editor,
            "admin" => UserRole::Admin,
            _ => return Err(RepositoryError::MappingFailed("Invalid role".to_string())),
        };

        let status = match model.status.as_str() {
            "active" => UserStatus::Active,
            "suspended" => UserStatus::Suspended,
            "deleted" => UserStatus::Deleted,
            _ => return Err(RepositoryError::MappingFailed("Invalid status".to_string())),
        };

        Ok(User::reconstruct(
            UserId::from_uuid(model.id),
            username,
            email,
            model.password_hash,
            role,
            status,
            model.created_at,
            model.updated_at,
            model.last_login_at,
        ))
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        use crate::infrastructure::database::schema::users::dsl;

        let mut conn = self.pool.get().await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        let result = dsl::users
            .find(*id.as_uuid())
            .first::<UserModel>(&mut conn)
            .optional()
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))?;

        result.map(Self::from_db_model).transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        use crate::infrastructure::database::schema::users::dsl;

        let mut conn = self.pool.get().await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        let result = dsl::users
            .filter(dsl::email.eq(email.as_str()))
            .first::<UserModel>(&mut conn)
            .optional()
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))?;

        result.map(Self::from_db_model).transpose()
    }

    async fn save(&self, user: &User) -> Result<(), RepositoryError> {
        use crate::infrastructure::database::schema::users::dsl;

        let mut conn = self.pool.get().await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        let model = Self::to_db_model(user);

        diesel::insert_into(dsl::users)
            .values(&model)
            .on_conflict(dsl::id)
            .do_update()
            .set(&model)
            .execute(&mut conn)
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: UserId) -> Result<(), RepositoryError> {
        use crate::infrastructure::database::schema::users::dsl;

        let mut conn = self.pool.get().await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        diesel::delete(dsl::users.find(*id.as_uuid()))
            .execute(&mut conn)
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    async fn find_all(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<User>, RepositoryError> {
        use crate::infrastructure::database::schema::users::dsl;

        let mut conn = self.pool.get().await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        let offset = (page.saturating_sub(1)) * per_page;

        let models = dsl::users
            .limit(i64::from(per_page))
            .offset(i64::from(offset))
            .load::<UserModel>(&mut conn)
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))?;

        models.into_iter().map(Self::from_db_model).collect()
    }
}
```

---

## Use Case の実装例

### Register User Use Case

```rust
// src/application/use_cases/user/register_user.rs

use crate::application::dto::UserDto;
use crate::application::errors::ApplicationError;
use crate::application::ports::{EventPublisher, UserRepository};
use crate::domain::entities::User;
use crate::domain::events::UserCreatedEvent;
use crate::domain::value_objects::{Email, Username};
use std::sync::Arc;

/// ユーザー登録コマンド
#[derive(Debug)]
pub struct RegisterUserCommand {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// ユーザー登録ユースケース
pub struct RegisterUserUseCase<R: UserRepository, E: EventPublisher> {
    user_repo: Arc<R>,
    event_publisher: Arc<E>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl<R: UserRepository, E: EventPublisher> RegisterUserUseCase<R, E> {
    pub fn new(
        user_repo: Arc<R>,
        event_publisher: Arc<E>,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            user_repo,
            event_publisher,
            password_hasher,
        }
    }

    /// ユーザー登録を実行
    ///
    /// # Errors
    ///
    /// - メールアドレスが既に登録済みの場合
    /// - バリデーションエラー
    /// - データベースエラー
    pub async fn execute(
        &self,
        command: RegisterUserCommand,
    ) -> Result<UserDto, ApplicationError> {
        // 1. 値オブジェクトの作成（検証込み）
        let username = Username::new(command.username)?;
        let email = Email::new(command.email)?;

        // 2. メールアドレスの重複チェック
        if let Some(_existing) = self.user_repo.find_by_email(&email).await? {
            return Err(ApplicationError::EmailAlreadyExists);
        }

        // 3. パスワードのハッシュ化
        let password_hash = self
            .password_hasher
            .hash(&command.password)
            .map_err(|e| ApplicationError::PasswordHashFailed(e.to_string()))?;

        // 4. ユーザーエンティティの作成
        let user = User::create(username, email, password_hash)?;

        // 5. 永続化
        self.user_repo.save(&user).await?;

        // 6. ドメインイベントの発行
        self.event_publisher
            .publish(UserCreatedEvent::new(user.id()))
            .await?;

        // 7. DTOに変換して返却
        Ok(UserDto::from(&user))
    }
}

/// パスワードハッシュ化のインターフェース
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, Box<dyn std::error::Error>>;
    fn verify(&self, password: &str, hash: &str) -> Result<bool, Box<dyn std::error::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::repositories::MockUserRepository;
    use crate::infrastructure::events::MockEventPublisher;

    struct MockPasswordHasher;

    impl PasswordHasher for MockPasswordHasher {
        fn hash(&self, password: &str) -> Result<String, Box<dyn std::error::Error>> {
            Ok(format!("hashed_{}", password))
        }

        fn verify(&self, _password: &str, _hash: &str) -> Result<bool, Box<dyn std::error::Error>> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_register_user_success() {
        let user_repo = Arc::new(MockUserRepository::new());
        let event_publisher = Arc::new(MockEventPublisher::new());
        let password_hasher = Arc::new(MockPasswordHasher);

        let use_case = RegisterUserUseCase::new(user_repo, event_publisher, password_hasher);

        let command = RegisterUserCommand {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePassword123".to_string(),
        };

        let result = use_case.execute(command).await;
        assert!(result.is_ok());
    }
}
```

---

## Handler の実装例

### User Handler (プレゼンテーション層)

```rust
// src/presentation/http/handlers/user_handlers.rs

use crate::application::use_cases::user::{RegisterUserCommand, RegisterUserUseCase};
use crate::infrastructure::database::repositories::DieselUserRepository;
use crate::infrastructure::events::EventBus;
use crate::presentation::http::responses::{ApiResponse, ErrorResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// ユーザー登録リクエスト
#[derive(Debug, Deserialize)]
pub struct RegisterUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// ユーザー登録レスポンス
#[derive(Debug, Serialize)]
pub struct RegisterUserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
}

/// ユーザー登録ハンドラ
///
/// ハンドラの責務:
/// 1. HTTPリクエストの受け取り
/// 2. DTOへの変換
/// 3. Use Caseの呼び出し
/// 4. HTTPレスポンスへの変換
pub async fn register_user(
    State(use_case): State<Arc<RegisterUserUseCase<DieselUserRepository, EventBus>>>,
    Json(request): Json<RegisterUserRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // コマンドの作成
    let command = RegisterUserCommand {
        username: request.username,
        email: request.email,
        password: request.password,
    };

    // Use Caseの実行
    let user_dto = use_case.execute(command).await?;

    // レスポンスの作成
    let response = RegisterUserResponse {
        id: user_dto.id.to_string(),
        username: user_dto.username,
        email: user_dto.email,
    };

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(response)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_register_user_handler() {
        // テスト実装...
    }
}
```

---

## エラーハンドリングの実装例

### エラー階層

```rust
// src/domain/errors.rs

use thiserror::Error;

/// ドメイン層のエラー
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    #[error("User is not active: {0}")]
    UserNotActive(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
```

```rust
// src/application/errors.rs

use crate::domain::errors::DomainError;
use crate::infrastructure::database::errors::RepositoryError;
use thiserror::Error;

/// アプリケーション層のエラー
#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("User not found")]
    UserNotFound,

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Password hash failed: {0}")]
    PasswordHashFailed(String),

    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}
```

```rust
// src/presentation/http/responses/error_response.rs

use crate::application::errors::ApplicationError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// HTTPエラーレスポンス
pub struct ErrorResponse {
    status: StatusCode,
    code: String,
    message: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let body = json!({
            "error": {
                "code": self.code,
                "message": self.message,
            }
        });

        (self.status, Json(body)).into_response()
    }
}

impl From<ApplicationError> for ErrorResponse {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::UserNotFound => Self {
                status: StatusCode::NOT_FOUND,
                code: "USER_NOT_FOUND".to_string(),
                message: error.to_string(),
            },
            ApplicationError::EmailAlreadyExists => Self {
                status: StatusCode::CONFLICT,
                code: "EMAIL_ALREADY_EXISTS".to_string(),
                message: error.to_string(),
            },
            ApplicationError::Unauthorized => Self {
                status: StatusCode::UNAUTHORIZED,
                code: "UNAUTHORIZED".to_string(),
                message: error.to_string(),
            },
            ApplicationError::Domain(e) => Self {
                status: StatusCode::BAD_REQUEST,
                code: "DOMAIN_ERROR".to_string(),
                message: e.to_string(),
            },
            ApplicationError::Repository(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "REPOSITORY_ERROR".to_string(),
                message: "Internal server error".to_string(),
            },
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "INTERNAL_ERROR".to_string(),
                message: "Internal server error".to_string(),
            },
        }
    }
}
```

---

## まとめ

これらの実装例は、`RESTRUCTURE_PLAN.md` に記載された設計パターンの具体的なコードを示しています。

### 重要なポイント

1. **型安全性**: NewTypeパターンと検証済み値オブジェクトでコンパイル時の安全性を確保
2. **責任の分離**: 各レイヤーが明確な役割を持ち、依存関係が一方向
3. **テスタビリティ**: Traitベースの設計により、モックとスタブが容易
4. **エラーハンドリング**: 階層的なエラー型で、適切な変換とハンドリング
5. **ドメイン中心**: ビジネスロジックがドメイン層に集約

---

作成日: 2025年10月16日
バージョン: 1.0
