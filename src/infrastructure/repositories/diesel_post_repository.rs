use async_trait::async_trait;

use crate::application::ports::post_repository::PostRepository;
use crate::application::ports::user_repository::RepositoryError;
use crate::domain::value_objects::PostId;

/// Diesel-backed PostRepository implementation that delegates to
/// the existing `crate::database::Database` helpers.
#[derive(Clone)]
pub struct DieselPostRepository {
    db: crate::database::Database,
}

impl DieselPostRepository {
    #[must_use]
    pub fn new(db: crate::database::Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PostRepository for DieselPostRepository {
    type Post = crate::models::post::Post;

    async fn find_by_id(&self, id: PostId) -> Result<Option<Self::Post>, RepositoryError> {
        match self.db.get_post_by_id(*id.as_uuid()) {
            Ok(p) => Ok(Some(p)),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Ok(None),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn save(&self, post: &Self::Post) -> Result<(), RepositoryError> {
        // Try to update; if update returns NotFound, surface NotFound to caller.
        let update_req = crate::models::UpdatePostRequest {
            title: Some(post.title.clone()),
            content: Some(post.content.clone()),
            excerpt: post.excerpt.clone(),
            slug: Some(post.slug.clone()),
            published: None,
            tags: Some(post.tags.clone()),
            category: post.categories.get(0).cloned(),
            featured_image: None,
            meta_title: post.meta_title.clone(),
            meta_description: post.meta_description.clone(),
            published_at: post.published_at,
            status: None,
        };

        match self.db.update_post(post.id, &update_req) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Err(RepositoryError::NotFound),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn create(
        &self,
        request: crate::models::CreatePostRequest,
    ) -> Result<Self::Post, RepositoryError> {
        match self.db.create_post(request) {
            Ok(p) => Ok(p),
            Err(e) => Err(RepositoryError::Unexpected(e.to_string())),
        }
    }

    async fn update(
        &self,
        id: PostId,
        request: crate::models::UpdatePostRequest,
    ) -> Result<Self::Post, RepositoryError> {
        match self.db.update_post(*id.as_uuid(), &request) {
            Ok(p) => Ok(p),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Err(RepositoryError::NotFound),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn delete(&self, id: PostId) -> Result<(), RepositoryError> {
        match self.db.delete_post(*id.as_uuid()) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                crate::AppError::NotFound(_) => Err(RepositoryError::NotFound),
                _ => Err(RepositoryError::Unexpected(e.to_string())),
            },
        }
    }

    async fn list_by_author(
        &self,
        author: crate::domain::value_objects::UserId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Self::Post>, RepositoryError> {
        // Convert offset/limit to page/per_page for the existing Database API
        let per_page = limit;
        let page = if per_page == 0 {
            1
        } else {
            (offset / per_page) + 1
        };
        match self
            .db
            .get_posts(page, per_page, None, Some(*author.as_uuid()), None, None)
        {
            Ok(posts) => Ok(posts),
            Err(e) => Err(RepositoryError::Unexpected(e.to_string())),
        }
    }

    async fn find_paginated(
        &self,
        page: u32,
        per_page: u32,
        status: Option<String>,
        author: Option<crate::domain::value_objects::UserId>,
        tag: Option<String>,
        sort: Option<String>,
    ) -> Result<Vec<Self::Post>, RepositoryError> {
        // Delegate to the Database helper which already supports these filters
        let author_uuid = author.map(|a| *a.as_uuid());
        match self
            .db
            .get_posts(page, per_page, status, author_uuid, tag, sort)
        {
            Ok(posts) => Ok(posts),
            Err(e) => Err(RepositoryError::Unexpected(e.to_string())),
        }
    }

    async fn count_filtered(
        &self,
        status: Option<String>,
        author: Option<crate::domain::value_objects::UserId>,
        tag: Option<String>,
    ) -> Result<usize, RepositoryError> {
        let author_uuid = author.map(|a| *a.as_uuid());
        match self.db.count_posts_filtered(status, author_uuid, tag) {
            Ok(n) => Ok(n),
            Err(e) => Err(RepositoryError::Unexpected(e.to_string())),
        }
    }
}
