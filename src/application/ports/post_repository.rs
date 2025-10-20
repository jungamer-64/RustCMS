// src/application/ports/post_repository.rs
use crate::domain::value_objects::{PostId, UserId};
use async_trait::async_trait;

use super::user_repository::RepositoryError;

#[async_trait]
pub trait PostRepository: Send + Sync {
    type Post;

    async fn find_by_id(&self, id: PostId) -> Result<Option<Self::Post>, RepositoryError>;
    async fn save(&self, post: &Self::Post) -> Result<(), RepositoryError>;
    /// Create a new post from a request object
    #[cfg(not(feature = "restructure_domain"))]
    async fn create(
        &self,
        request: crate::models::CreatePostRequest,
    ) -> Result<Self::Post, RepositoryError>;

    /// Update an existing post
    #[cfg(not(feature = "restructure_domain"))]
    async fn update(
        &self,
        id: crate::domain::value_objects::PostId,
        request: crate::models::UpdatePostRequest,
    ) -> Result<Self::Post, RepositoryError>;
    async fn delete(&self, id: PostId) -> Result<(), RepositoryError>;
    async fn list_by_author(
        &self,
        author: UserId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Self::Post>, RepositoryError>;

    /// Find posts with filtering and pagination support.
    async fn find_paginated(
        &self,
        page: u32,
        per_page: u32,
        status: Option<String>,
        author: Option<UserId>,
        tag: Option<String>,
        sort: Option<String>,
    ) -> Result<Vec<Self::Post>, RepositoryError>;

    /// Count posts matching optional filters. Used to compute pagination totals.
    async fn count_filtered(
        &self,
        status: Option<String>,
        author: Option<UserId>,
        tag: Option<String>,
    ) -> Result<usize, RepositoryError>;
}
