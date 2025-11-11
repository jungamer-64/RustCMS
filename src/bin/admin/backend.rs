// src/bin/admin/backend.rs
use async_trait::async_trait;
use cms_backend::{
    AppState, Result,
    models::{CreateUserRequest, UpdateUserRequest, User},
};

/// Abstraction over `AppState` database operations so CLI logic can be tested in isolation.
#[async_trait]
pub trait AdminBackend: Sync + Send {
    async fn list_users(&self, role: Option<&str>, active_only: Option<bool>) -> Result<Vec<User>>;

    async fn create_user(&self, req: CreateUserRequest) -> Result<User>;

    async fn reset_user_password(&self, user_id: uuid::Uuid, new_password: &str) -> Result<()>;

    async fn get_user_by_id(&self, id: uuid::Uuid) -> Result<User>;

    async fn get_user_by_username(&self, username: &str) -> Result<User>;

    async fn update_user(&self, id: uuid::Uuid, req: UpdateUserRequest) -> Result<User>;

    async fn delete_user(&self, id: uuid::Uuid) -> Result<()>;
}

#[async_trait]
impl AdminBackend for AppState {
    async fn list_users(&self, role: Option<&str>, active_only: Option<bool>) -> Result<Vec<User>> {
        // Phase 2: database field is Option, use database() method
        self.database().list_users(role, active_only).await
    }

    async fn create_user(&self, req: CreateUserRequest) -> Result<User> {
        // Phase 2: db_create_user renamed to create_user
        self.create_user(req).await
    }

    async fn reset_user_password(&self, user_id: uuid::Uuid, new_password: &str) -> Result<()> {
        // Phase 2: db_reset_user_password renamed to reset_user_password
        self.reset_user_password(user_id, new_password).await
    }

    async fn get_user_by_id(&self, id: uuid::Uuid) -> Result<User> {
        self.get_user_by_id(id).await
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User> {
        self.get_user_by_username(username).await
    }

    async fn update_user(&self, id: uuid::Uuid, req: UpdateUserRequest) -> Result<User> {
        self.update_user(id, req).await
    }

    async fn delete_user(&self, id: uuid::Uuid) -> Result<()> {
        self.delete_user(id).await
    }
}
