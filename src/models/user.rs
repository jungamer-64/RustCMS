use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use clap::ValueEnum;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::AppError;
use crate::database::schema::users;
use crate::utils::password;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Queryable, Insertable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, ValueEnum)]
#[clap(rename_all = "kebab_case")]
pub enum UserRole {
    SuperAdmin,
    Admin,
    Editor,
    Author,
    Contributor,
    Subscriber,
}

impl UserRole {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SuperAdmin => "super_admin",
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Author => "author",
            Self::Contributor => "contributor",
            Self::Subscriber => "subscriber",
        }
    }

    /// 文字列から `UserRole` をパースする
    ///
    /// # Errors
    /// 未知のロール文字列が与えられた場合、`AppError::BadRequest` を返します。
    pub fn parse_str(s: &str) -> Result<Self, AppError> {
        match s {
            "super_admin" => Ok(Self::SuperAdmin),
            "admin" => Ok(Self::Admin),
            "editor" => Ok(Self::Editor),
            "author" => Ok(Self::Author),
            "contributor" => Ok(Self::Contributor),
            "subscriber" => Ok(Self::Subscriber),
            _ => Err(AppError::BadRequest(format!("Invalid user role: {s}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<UserRole>,
    pub is_active: Option<bool>,
}

impl UpdateUserRequest {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            username: None,
            email: None,
            first_name: None,
            last_name: None,
            role: None,
            is_active: None,
        }
    }
    #[must_use]
    pub fn deactivate() -> Self {
        Self {
            is_active: Some(false),
            ..Self::empty()
        }
    }
    #[must_use]
    pub fn with_role(role: UserRole) -> Self {
        Self {
            role: Some(role),
            ..Self::empty()
        }
    }
}

impl User {
    #[must_use]
    pub fn new(
        username: String,
        email: String,
        password_hash: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        role: UserRole,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            first_name,
            last_name,
            role: role.as_str().to_string(),
            is_active: true,
            email_verified: false,
            last_login: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[must_use]
    pub fn new_passkey_user(
        username: String,
        email: String,
        first_name: Option<String>,
        last_name: Option<String>,
        role: UserRole,
    ) -> Self {
        Self::new(username, email, None, first_name, last_name, role)
    }

    /// パスワード付きの新規ユーザーを作成する
    ///
    /// # Errors
    /// パスワードのハッシュ化に失敗した場合、エラーを返します。
    pub fn new_with_password(
        username: String,
        email: String,
        password: &str,
        first_name: Option<String>,
        last_name: Option<String>,
        role: &UserRole,
    ) -> Result<Self, crate::AppError> {
        let password_hash = password::hash_password(password)?;

        Ok(Self::new(
            username,
            email,
            Some(password_hash),
            first_name,
            last_name,
            *role,
        ))
    }

    /// ID でユーザーを取得する
    ///
    /// # Errors
    /// データベース検索に失敗した場合、エラーを返します。
    pub fn find_by_id(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
    ) -> Result<Self, AppError> {
        use crate::database::schema::users::dsl::users;
        users.find(user_id).first(conn).map_err(AppError::from)
    }

    /// ユーザー名でユーザーを取得する
    ///
    /// # Errors
    /// データベース検索に失敗した場合、エラーを返します。
    pub fn find_by_username(
        conn: &mut crate::database::PooledConnection,
        user_username: &str,
    ) -> Result<Self, AppError> {
        use crate::database::schema::users::dsl::{username, users};
        users
            .filter(username.eq(user_username))
            .first(conn)
            .map_err(AppError::from)
    }

    /// メールアドレスでユーザーを取得する
    ///
    /// # Errors
    /// データベース検索に失敗した場合、エラーを返します。
    pub fn find_by_email(
        conn: &mut crate::database::PooledConnection,
        user_email: &str,
    ) -> Result<Self, AppError> {
        use crate::database::schema::users::dsl::{email, users};
        users
            .filter(email.eq(user_email))
            .first(conn)
            .map_err(AppError::from)
    }

    /// ユーザーを作成する
    ///
    /// # Errors
    /// データベースへの挿入に失敗した場合、エラーを返します。
    pub fn create(
        conn: &mut crate::database::PooledConnection,
        user: &Self,
    ) -> Result<Self, AppError> {
        use crate::database::schema::users;
        diesel::insert_into(users::table)
            .values(user)
            .get_result(conn)
            .map_err(AppError::from)
    }

    /// 指定したフィールドを更新する
    ///
    /// # Errors
    /// データベース更新に失敗した場合、エラーを返します。
    pub fn update(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
        updates: &UpdateUserRequest,
    ) -> Result<Self, AppError> {
        use crate::database::schema::users::dsl::{
            email, first_name, is_active, last_name, role, updated_at, username, users,
        };

        // Use conditional updates based on what fields are provided
        if let Some(ref new_username) = updates.username {
            diesel::update(users.find(user_id))
                .set(username.eq(new_username))
                .execute(conn)?;
        }
        if let Some(ref new_email) = updates.email {
            diesel::update(users.find(user_id))
                .set(email.eq(new_email))
                .execute(conn)?;
        }
        if let Some(ref new_first_name) = updates.first_name {
            diesel::update(users.find(user_id))
                .set(first_name.eq(new_first_name))
                .execute(conn)?;
        }
        if let Some(ref new_last_name) = updates.last_name {
            diesel::update(users.find(user_id))
                .set(last_name.eq(new_last_name))
                .execute(conn)?;
        }
        if let Some(ref new_role) = updates.role {
            diesel::update(users.find(user_id))
                .set(role.eq(new_role.as_str()))
                .execute(conn)?;
        }
        if let Some(new_is_active) = updates.is_active {
            diesel::update(users.find(user_id))
                .set(is_active.eq(new_is_active))
                .execute(conn)?;
        }

        // Always update timestamp
        diesel::update(users.find(user_id))
            .set(updated_at.eq(Utc::now()))
            .execute(conn)?;

        // Return the updated user
        Self::find_by_id(conn, user_id)
    }

    /// 指定したユーザーを削除する
    ///
    /// # Errors
    /// データベース操作に失敗した場合、エラーを返します。
    pub fn delete(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
    ) -> Result<usize, AppError> {
        use crate::database::schema::users::dsl::users;
        diesel::delete(users.find(user_id))
            .execute(conn)
            .map_err(AppError::from)
    }

    /// ユーザーのパスワードを検証する
    ///
    /// # Errors
    /// パスワードハッシュの検証処理に失敗した場合、エラーを返します。
    pub fn verify_password(&self, password: &str) -> Result<bool, AppError> {
        self.password_hash.as_ref().map_or_else(
            || Ok(false),
            |hash| password::verify_password(password, hash),
        )
    }

    /// ログイン時刻を現在時刻で更新する
    ///
    /// # Errors
    /// データベース更新に失敗した場合、エラーを返します。
    pub fn update_last_login(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        use crate::database::schema::users::dsl::{last_login, users};
        diesel::update(users.find(user_id))
            .set(last_login.eq(Some(Utc::now())))
            .execute(conn)?;
        Ok(())
    }
}
