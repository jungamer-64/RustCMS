use crate::domain::value_objects::{Email, UserId};
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Not found")]
    NotFound,
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Unexpected: {0}")]
    Unexpected(String),
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    type User;

    async fn find_by_id(&self, id: UserId) -> Result<Option<Self::User>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<Self::User>, RepositoryError>;
    async fn create(
        &self,
        request: crate::models::CreateUserRequest,
    ) -> Result<Self::User, RepositoryError>;
    async fn update(
        &self,
        id: UserId,
        request: crate::models::UpdateUserRequest,
    ) -> Result<Self::User, RepositoryError>;
    async fn save(&self, user: &Self::User) -> Result<(), RepositoryError>;
    async fn delete(&self, id: UserId) -> Result<(), RepositoryError>;

    /// Find users with filtering and pagination support.
    async fn find_paginated(
        &self,
        page: u32,
        per_page: u32,
        role: Option<String>,
        active: Option<bool>,
        sort: Option<String>,
    ) -> Result<Vec<Self::User>, RepositoryError>;

    /// Count users matching optional filters.
    async fn count_filtered(
        &self,
        role: Option<String>,
        active: Option<bool>,
    ) -> Result<usize, RepositoryError>;
}
