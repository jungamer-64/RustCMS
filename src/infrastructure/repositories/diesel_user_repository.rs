// File is feature-gated by parent module; avoid duplicated attributes.

use async_trait::async_trait;
use diesel::prelude::*;

use crate::application::ports::UserRepository;
use crate::application::ports::user_repository::RepositoryError;
use crate::domain::value_objects::{Email, UserId};
use crate::repositories::user_repository::BoxFuture;
use uuid::Uuid;

/// Diesel-backed implementation of the `application::ports::UserRepository` port.
///
/// This adapter is intentionally small and pragmatic: it delegates to the
/// existing `crate::database::Database` helpers (which centralize connection
/// handling and error mapping) for reads and updates, and uses a direct Diesel
/// insert when an upsert is required and the `Database` API does not expose a
/// convenient helper for creation from a full `crate::models::User` instance.
#[derive(Clone)]
pub struct DieselUserRepository {
    db: crate::database::Database,
}

impl DieselUserRepository {
    /// Create a new adapter instance.
    #[must_use]
    pub fn new(db: crate::database::Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for DieselUserRepository {
    type User = crate::models::User;

    async fn find_by_id(&self, id: UserId) -> Result<Option<Self::User>, RepositoryError> {
        match self.db.get_user_by_id(*id.as_uuid()).await {
            Ok(u) => Ok(Some(u)),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Ok(None),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<Self::User>, RepositoryError> {
        match self.db.get_user_by_email(email.as_str()).await {
            Ok(u) => Ok(Some(u)),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Ok(None),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn create(
        &self,
        request: crate::models::CreateUserRequest,
    ) -> Result<Self::User, RepositoryError> {
        match self.db.create_user(request).await {
            Ok(u) => Ok(u),
            Err(e) => match e {
                crate::AppError::Conflict(s) => Err(RepositoryError::Conflict(s)),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn update(
        &self,
        id: UserId,
        request: crate::models::UpdateUserRequest,
    ) -> Result<Self::User, RepositoryError> {
        match self.db.update_user(*id.as_uuid(), &request) {
            Ok(u) => Ok(u),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Err(RepositoryError::NotFound),
                crate::AppError::Conflict(s) => Err(RepositoryError::Conflict(s)),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn save(&self, user: &Self::User) -> Result<(), RepositoryError> {
        // Try update first: if the user already exists, update via the
        // existing Database API which centralizes update logic.
        let id = user.id;

        // Build an UpdateUserRequest from the model for partial updates.
        let role_opt = match crate::models::UserRole::parse_str(&user.role) {
            Ok(r) => Some(r),
            Err(_) => None,
        };

        let update_req = crate::models::UpdateUserRequest {
            username: Some(user.username.clone()),
            email: Some(user.email.clone()),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: role_opt,
            is_active: Some(user.is_active),
        };

        match self.db.update_user(id, &update_req) {
            Ok(_) => return Ok(()),
            Err(e) => {
                // If update failed because the row did not exist, fall through
                // to perform an insert. Other errors are treated as unexpected.
                if !matches!(e, crate::AppError::NotFound(_)) {
                    return Err(RepositoryError::Unexpected(e.to_string()));
                }
            }
        }

        // Insert new user with a direct Diesel insert using a pooled
        // connection. We use the model type directly because it implements
        // `Insertable` for the `users` table.
        let mut conn = match self.db.get_connection() {
            Ok(c) => c,
            Err(e) => return Err(RepositoryError::Unexpected(e.to_string())),
        };

        use crate::database::schema::users::dsl as users_dsl;

        diesel::insert_into(users_dsl::users)
            .values(user)
            .execute(&mut conn)
            .map(|_| ())
            .map_err(|e| RepositoryError::Unexpected(e.to_string()))
    }

    async fn delete(&self, id: UserId) -> Result<(), RepositoryError> {
        match self.db.delete_user(*id.as_uuid()) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Err(RepositoryError::NotFound),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn find_paginated(
        &self,
        page: u32,
        per_page: u32,
        role: Option<String>,
        active: Option<bool>,
        _sort: Option<String>,
    ) -> Result<Vec<Self::User>, RepositoryError> {
        match self.db.get_users(page, per_page, role, active, _sort) {
            Ok(users) => Ok(users),
            Err(e) => Err(RepositoryError::Unexpected(e.to_string())),
        }
    }

    async fn count_filtered(
        &self,
        role: Option<String>,
        active: Option<bool>,
    ) -> Result<usize, RepositoryError> {
        match self.db.count_users_filtered(role, active) {
            Ok(count) => Ok(count),
            Err(e) => Err(RepositoryError::Unexpected(e.to_string())),
        }
    }
}

// Backwards-compatibility: implement the original, BoxFuture-based
// `crate::repositories::UserRepository` so existing callers can receive
// a `DieselUserRepository` where the original trait object is required.
impl crate::repositories::user_repository::UserRepository for DieselUserRepository {
    fn get_user_by_email(&self, email: &str) -> BoxFuture<'_, crate::Result<crate::models::User>> {
        let this = self.clone();
        let email_owned = email.to_string();
        Box::pin(async move { this.db.get_user_by_email(&email_owned).await })
    }

    fn get_user_by_id(&self, id: Uuid) -> BoxFuture<'_, crate::Result<crate::models::User>> {
        let this = self.clone();
        Box::pin(async move { this.db.get_user_by_id(id).await })
    }

    fn update_last_login(&self, id: Uuid) -> BoxFuture<'_, crate::Result<()>> {
        let this = self.clone();
        Box::pin(async move { this.db.update_last_login(id) })
    }
}
