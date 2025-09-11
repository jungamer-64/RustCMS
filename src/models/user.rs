use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum UserRole {
    SuperAdmin,
    Admin,
    Editor,
    Author,
    Contributor,
    Subscriber,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::SuperAdmin => "super_admin",
            UserRole::Admin => "admin",
            UserRole::Editor => "editor",
            UserRole::Author => "author",
            UserRole::Contributor => "contributor",
            UserRole::Subscriber => "subscriber",
        }
    }

    pub fn parse_str(s: &str) -> Result<Self, AppError> {
        match s {
            "super_admin" => Ok(UserRole::SuperAdmin),
            "admin" => Ok(UserRole::Admin),
            "editor" => Ok(UserRole::Editor),
            "author" => Ok(UserRole::Author),
            "contributor" => Ok(UserRole::Contributor),
            "subscriber" => Ok(UserRole::Subscriber),
            _ => Err(AppError::BadRequest(format!("Invalid user role: {s}", s = s))),
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
    pub fn empty() -> Self {
        Self {
            username: None,
            email: None,
            first_name: None,
            last_name: None,
            role: None,
            is_active: None,
        }
    }
    pub fn deactivate() -> Self {
        Self {
            is_active: Some(false),
            ..Self::empty()
        }
    }
    pub fn with_role(role: UserRole) -> Self {
        Self {
            role: Some(role),
            ..Self::empty()
        }
    }
}

impl User {
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

    pub fn new_passkey_user(
        username: String,
        email: String,
        first_name: Option<String>,
        last_name: Option<String>,
        role: UserRole,
    ) -> Self {
        Self::new(username, email, None, first_name, last_name, role)
    }

    pub fn new_with_password(
        username: String,
        email: String,
        password: &str,
        first_name: Option<String>,
        last_name: Option<String>,
        role: UserRole,
    ) -> Result<Self, crate::AppError> {
        let password_hash = password::hash_password(password)?;

        Ok(Self::new(
            username,
            email,
            Some(password_hash),
            first_name,
            last_name,
            role,
        ))
    }

    pub fn find_by_id(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
    ) -> Result<User, AppError> {
        use crate::database::schema::users::dsl::*;
        users.find(user_id).first(conn).map_err(AppError::from)
    }

    pub fn find_by_username(
        conn: &mut crate::database::PooledConnection,
        user_username: &str,
    ) -> Result<User, AppError> {
        use crate::database::schema::users::dsl::*;
        users
            .filter(username.eq(user_username))
            .first(conn)
            .map_err(AppError::from)
    }

    pub fn find_by_email(
        conn: &mut crate::database::PooledConnection,
        user_email: &str,
    ) -> Result<User, AppError> {
        use crate::database::schema::users::dsl::*;
        users
            .filter(email.eq(user_email))
            .first(conn)
            .map_err(AppError::from)
    }

    pub fn create(
        conn: &mut crate::database::PooledConnection,
        user: &User,
    ) -> Result<User, AppError> {
        use crate::database::schema::users;
        diesel::insert_into(users::table)
            .values(user)
            .get_result(conn)
            .map_err(AppError::from)
    }

    pub fn update(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
        updates: &UpdateUserRequest,
    ) -> Result<User, AppError> {
        use crate::database::schema::users::dsl::*;

        // Build a tuple of all possible updates
        let mut update_sets = Vec::new();

        if let Some(ref new_username) = updates.username {
            update_sets.push(format!("username = '{}'", new_username));
        }
        if let Some(ref new_email) = updates.email {
            update_sets.push(format!("email = '{}'", new_email));
        }
        if let Some(ref new_first_name) = updates.first_name {
            update_sets.push(format!("first_name = '{}'", new_first_name));
        }
        if let Some(ref new_last_name) = updates.last_name {
            update_sets.push(format!("last_name = '{}'", new_last_name));
        }
        if let Some(ref new_role) = updates.role {
            update_sets.push(format!("role = '{}'", new_role.as_str()));
        }
        if let Some(new_is_active) = updates.is_active {
            update_sets.push(format!("is_active = {}", new_is_active));
        }

        // Always update the timestamp
        update_sets.push("updated_at = NOW()".to_string());

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
        User::find_by_id(conn, user_id)
    }

    pub fn delete(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
    ) -> Result<usize, AppError> {
        use crate::database::schema::users::dsl::*;
        diesel::delete(users.find(user_id))
            .execute(conn)
            .map_err(AppError::from)
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, AppError> {
        match &self.password_hash {
            Some(hash) => password::verify_password(password, hash),
            None => Ok(false), // Passkey-only user
        }
    }

    pub fn update_last_login(
        conn: &mut crate::database::PooledConnection,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        use crate::database::schema::users::dsl::*;
        diesel::update(users.find(user_id))
            .set(last_login.eq(Some(Utc::now())))
            .execute(conn)?;
        Ok(())
    }
}
