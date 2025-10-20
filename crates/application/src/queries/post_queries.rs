// src/application/queries/post_queries.rs
/// Post Queries - Read-only queries for post data retrieval
///
/// Implements CQRS Query pattern for post-related data access.
/// Includes list queries with filters and full-text search integration.
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::dto::post::PostDto;
use crate::ports::repositories::{PostRepository, RepositoryError};
use crate::queries::pagination::{PaginationParams, PaginationResult};
use domain::post::{PostId, PostStatus};
use domain::user::UserId;

// ============================================================================
// Filter & Sort Types
// ============================================================================

/// Post list filter criteria
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PostFilter {
    /// Filter by post status (None = all statuses)
    pub status: Option<PostStatus>,
    /// Filter by author user ID
    pub author_id: Option<UserId>,
    /// Filter by creation date range (inclusive)
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    /// Filter by publish date range (inclusive)
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
    /// Filter by slug substring (case-insensitive)
    pub slug_contains: Option<String>,
}

impl PostFilter {
    /// Create empty filter (returns all posts)
    pub fn all() -> Self {
        Self::default()
    }

    /// Create filter for published posts only
    pub fn published_only() -> Self {
        Self {
            status: Some(PostStatus::Published),
            ..Default::default()
        }
    }

    /// Create filter for draft posts only
    pub fn drafts_only() -> Self {
        Self {
            status: Some(PostStatus::Draft),
            ..Default::default()
        }
    }

    /// Create filter for archived posts only
    pub fn archived_only() -> Self {
        Self {
            status: Some(PostStatus::Archived),
            ..Default::default()
        }
    }

    /// Filter by specific author
    pub fn by_author(author_id: UserId) -> Self {
        Self {
            author_id: Some(author_id),
            ..Default::default()
        }
    }

    /// Add status filter
    pub fn with_status(mut self, status: PostStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Add author filter
    pub fn with_author(mut self, author_id: UserId) -> Self {
        self.author_id = Some(author_id);
        self
    }

    /// Add date range filter (creation date)
    pub fn created_between(mut self, after: DateTime<Utc>, before: DateTime<Utc>) -> Self {
        self.created_after = Some(after);
        self.created_before = Some(before);
        self
    }

    /// Add date range filter (publish date)
    pub fn published_between(mut self, after: DateTime<Utc>, before: DateTime<Utc>) -> Self {
        self.published_after = Some(after);
        self.published_before = Some(before);
        self
    }

    /// Add slug filter
    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug_contains = Some(slug.into());
        self
    }
}

/// Post sort field options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostSortField {
    /// Sort by creation date
    CreatedAt,
    /// Sort by last update date
    UpdatedAt,
    /// Sort by publish date
    PublishedAt,
    /// Sort by title (alphabetical)
    Title,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    /// Ascending order (A-Z, 0-9, oldest first)
    Asc,
    /// Descending order (Z-A, 9-0, newest first)
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Desc
    }
}

/// Post sort parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PostSort {
    pub field: PostSortField,
    pub direction: SortDirection,
}

impl Default for PostSort {
    fn default() -> Self {
        Self {
            field: PostSortField::CreatedAt,
            direction: SortDirection::Desc, // Newest first by default
        }
    }
}

// ============================================================================
// ListPostsQuery
// ============================================================================

/// Query to list posts with filtering, sorting, and pagination
///
/// # Examples
///
/// ```rust,no_run
/// use cms_backend::application::queries::{ListPostsQuery, PostFilter, PaginationParams};
/// use cms_backend::domain::post::PostStatus;
///
/// # async fn example(repo: std::sync::Arc<dyn cms_backend::application::ports::repositories::PostRepository>) {
/// let query = ListPostsQuery::new(repo);
/// let filter = PostFilter::published_only();
/// let pagination = PaginationParams::page(1, 10);
///
/// let result = query.execute(filter, None, pagination).await.unwrap();
/// println!("Found {} published posts", result.items.len());
/// # }
/// ```
pub struct ListPostsQuery {
    post_repo: Arc<dyn PostRepository>,
}

impl ListPostsQuery {
    /// Create new list posts query
    pub fn new(post_repo: Arc<dyn PostRepository>) -> Self {
        Self { post_repo }
    }

    /// Execute query with filters and pagination
    ///
    /// # Arguments
    /// * `filter` - Post filter criteria
    /// * `sort` - Sort parameters (None = default sort)
    /// * `pagination` - Pagination parameters
    ///
    /// # Returns
    /// Paginated list of PostDto
    ///
    /// # Errors
    /// Returns `RepositoryError` if database query fails
    pub async fn execute(
        &self,
        filter: PostFilter,
        sort: Option<PostSort>,
        pagination: PaginationParams,
    ) -> Result<PaginationResult<PostDto>, RepositoryError> {
        // NOTE: Current implementation uses list_all() from Repository
        // In production, this should use a dedicated query method with SQL filters
        // e.g., PostRepository::list_with_filters(filter, sort, pagination)

        let _sort = sort.unwrap_or_default();

        // If author filter is set, use find_by_author for efficiency
        if let Some(author_id) = filter.author_id {
            let posts = self
                .post_repo
                .find_by_author(author_id, pagination.limit(), pagination.offset())
                .await?;

            // Apply remaining filters in memory
            let filtered_posts = self.apply_filters(posts, &filter);
            let total = filtered_posts.len() as i64;
            let dtos: Vec<PostDto> = filtered_posts.into_iter().map(PostDto::from).collect();

            return Ok(PaginationResult::new(dtos, total, pagination));
        }

        // Get all posts (Phase 3: simplified implementation)
        let all_posts = self
            .post_repo
            .list_all(pagination.limit(), pagination.offset())
            .await?;

        // Apply filters in memory (Phase 3: will be moved to SQL in Phase 4)
        let filtered_posts = self.apply_filters(all_posts, &filter);
        let total = filtered_posts.len() as i64;

        // Convert to DTOs
        let dtos: Vec<PostDto> = filtered_posts.into_iter().map(PostDto::from).collect();

        Ok(PaginationResult::new(dtos, total, pagination))
    }

    /// Execute query for a single post by ID
    ///
    /// # Arguments
    /// * `post_id` - Post identifier
    ///
    /// # Returns
    /// PostDto if found, None otherwise
    ///
    /// # Errors
    /// Returns `RepositoryError` if database query fails
    pub async fn get_by_id(&self, post_id: PostId) -> Result<Option<PostDto>, RepositoryError> {
        let post = self.post_repo.find_by_id(post_id).await?;
        Ok(post.map(PostDto::from))
    }

    /// Apply filters to posts (in-memory filtering)
    ///
    /// Phase 3: In-memory filtering for simplicity
    /// Phase 4: Will be replaced with SQL WHERE clauses
    fn apply_filters(
        &self,
        posts: Vec<domain::post::Post>,
        filter: &PostFilter,
    ) -> Vec<domain::post::Post> {
        posts
            .into_iter()
            .filter(|post| {
                // Filter by status
                if let Some(status) = filter.status {
                    if post.status() != status {
                        return false;
                    }
                }

                // Filter by creation date range
                if let Some(after) = filter.created_after {
                    if post.created_at() < after {
                        return false;
                    }
                }
                if let Some(before) = filter.created_before {
                    if post.created_at() > before {
                        return false;
                    }
                }

                // Filter by publish date range
                if let Some(after) = filter.published_after {
                    if let Some(published_at) = post.published_at() {
                        if published_at < after {
                            return false;
                        }
                    } else {
                        return false; // Not published
                    }
                }
                if let Some(before) = filter.published_before {
                    if let Some(published_at) = post.published_at() {
                        if published_at > before {
                            return false;
                        }
                    } else {
                        return false; // Not published
                    }
                }

                // Filter by slug substring
                if let Some(ref slug_pattern) = filter.slug_contains {
                    let slug_lower = post.slug().as_str().to_lowercase();
                    let pattern_lower = slug_pattern.to_lowercase();
                    if !slug_lower.contains(&pattern_lower) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

// ============================================================================
// SearchPostsQuery (Full-Text Search)
// ============================================================================

/// Query to search posts using full-text search
///
/// # Note
/// Phase 3: Placeholder implementation (searches in title/content via DB)
/// Phase 4: Will integrate with Tantivy search engine
///
/// # Examples
///
/// ```rust,no_run
/// use cms_backend::application::queries::{SearchPostsQuery, PaginationParams};
///
/// # async fn example(repo: std::sync::Arc<dyn cms_backend::application::ports::repositories::PostRepository>) {
/// let query = SearchPostsQuery::new(repo);
/// let pagination = PaginationParams::page(1, 10);
///
/// let result = query.search("rust programming", pagination).await.unwrap();
/// println!("Found {} posts matching 'rust programming'", result.items.len());
/// # }
/// ```
pub struct SearchPostsQuery {
    post_repo: Arc<dyn PostRepository>,
}

impl SearchPostsQuery {
    /// Create new search posts query
    pub fn new(post_repo: Arc<dyn PostRepository>) -> Self {
        Self { post_repo }
    }

    /// Search posts by text query
    ///
    /// # Arguments
    /// * `query_text` - Search query string
    /// * `pagination` - Pagination parameters
    ///
    /// # Returns
    /// Paginated list of PostDto matching the search query
    ///
    /// # Errors
    /// Returns `RepositoryError` if database query fails
    ///
    /// # Phase 3 Note
    /// Current implementation does simple substring matching on title/content.
    /// Phase 4 will integrate Tantivy for proper full-text search with ranking.
    pub async fn search(
        &self,
        query_text: &str,
        pagination: PaginationParams,
    ) -> Result<PaginationResult<PostDto>, RepositoryError> {
        // TODO: Phase 4 - Integrate with Tantivy search service
        // For now, use simple substring matching

        let all_posts = self
            .post_repo
            .list_all(pagination.limit(), pagination.offset())
            .await?;

        let query_lower = query_text.to_lowercase();

        // Search in title and content (case-insensitive)
        let matching_posts: Vec<_> = all_posts
            .into_iter()
            .filter(|post| {
                // Only search published posts
                if post.status() != PostStatus::Published {
                    return false;
                }

                let title_lower = post.title().as_str().to_lowercase();
                let content_lower = post.content().as_str().to_lowercase();

                title_lower.contains(&query_lower) || content_lower.contains(&query_lower)
            })
            .collect();

        let total = matching_posts.len() as i64;
        let dtos: Vec<PostDto> = matching_posts.into_iter().map(PostDto::from).collect();

        Ok(PaginationResult::new(dtos, total, pagination))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_filter_all() {
        let filter = PostFilter::all();
        assert!(filter.status.is_none());
        assert!(filter.author_id.is_none());
    }

    #[test]
    fn test_post_filter_published_only() {
        let filter = PostFilter::published_only();
        assert_eq!(filter.status, Some(PostStatus::Published));
    }

    #[test]
    fn test_post_filter_builder() {
        let author_id = UserId::new();
        let filter = PostFilter::published_only()
            .with_author(author_id)
            .with_slug("rust");

        assert_eq!(filter.status, Some(PostStatus::Published));
        assert_eq!(filter.author_id, Some(author_id));
        assert_eq!(filter.slug_contains, Some("rust".to_string()));
    }

    #[test]
    fn test_post_sort_default() {
        let sort = PostSort::default();
        assert_eq!(sort.field, PostSortField::CreatedAt);
        assert_eq!(sort.direction, SortDirection::Desc);
    }
}
