/// User Queries - Read-only queries for user data retrieval
///
/// Implements CQRS Query pattern for user-related data access.
/// These queries are optimized for reading and can bypass domain layer.
use std::sync::Arc;

use crate::application::dto::user::UserDto;
use crate::application::ports::repositories::{RepositoryError, UserRepository};
use crate::application::queries::pagination::{PaginationParams, PaginationResult};
use crate::domain::user::UserId;
use serde::{Deserialize, Serialize};

// ============================================================================
// Filter & Sort Types
// ============================================================================

/// User list filter criteria
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserFilter {
    /// Filter by active status (None = all users)
    pub is_active: Option<bool>,
    /// Filter by username substring (case-insensitive)
    pub username_contains: Option<String>,
    /// Filter by email substring (case-insensitive)
    pub email_contains: Option<String>,
}

impl UserFilter {
    /// Create empty filter (returns all users)
    pub fn all() -> Self {
        Self::default()
    }

    /// Create filter for active users only
    pub fn active_only() -> Self {
        Self {
            is_active: Some(true),
            ..Default::default()
        }
    }

    /// Create filter for inactive users only
    pub fn inactive_only() -> Self {
        Self {
            is_active: Some(false),
            ..Default::default()
        }
    }

    /// Add username filter
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username_contains = Some(username.into());
        self
    }

    /// Add email filter
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email_contains = Some(email.into());
        self
    }
}

/// User sort field options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserSortField {
    /// Sort by creation date
    CreatedAt,
    /// Sort by last update date
    UpdatedAt,
    /// Sort by username (alphabetical)
    Username,
    /// Sort by email (alphabetical)
    Email,
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

/// User sort parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UserSort {
    pub field: UserSortField,
    pub direction: SortDirection,
}

impl Default for UserSort {
    fn default() -> Self {
        Self {
            field: UserSortField::CreatedAt,
            direction: SortDirection::Desc, // Newest first by default
        }
    }
}

// ============================================================================
// ListUsersQuery
// ============================================================================

/// Query to list users with filtering, sorting, and pagination
///
/// # Examples
///
/// ```rust,no_run
/// use cms_backend::application::queries::{ListUsersQuery, UserFilter, PaginationParams};
///
/// # async fn example(repo: std::sync::Arc<dyn cms_backend::application::ports::repositories::UserRepository>) {
/// let query = ListUsersQuery::new(repo);
/// let filter = UserFilter::active_only().with_username("john");
/// let pagination = PaginationParams::page(1, 20);
///
/// let result = query.execute(filter, None, pagination).await.unwrap();
/// println!("Found {} active users matching 'john'", result.items.len());
/// # }
/// ```
pub struct ListUsersQuery {
    user_repo: Arc<dyn UserRepository>,
}

impl ListUsersQuery {
    /// Create new list users query
    pub fn new(user_repo: Arc<dyn UserRepository>) -> Self {
        Self { user_repo }
    }

    /// Execute query with filters and pagination
    ///
    /// # Arguments
    /// * `filter` - User filter criteria
    /// * `sort` - Sort parameters (None = default sort)
    /// * `pagination` - Pagination parameters
    ///
    /// # Returns
    /// Paginated list of UserDto
    ///
    /// # Errors
    /// Returns `RepositoryError` if database query fails
    pub async fn execute(
        &self,
        filter: UserFilter,
        sort: Option<UserSort>,
        pagination: PaginationParams,
    ) -> Result<PaginationResult<UserDto>, RepositoryError> {
        // NOTE: Current implementation uses list_all() from Repository
        // In production, this should use a dedicated query method with SQL filters
        // e.g., UserRepository::list_with_filters(filter, sort, pagination)

        let _sort = sort.unwrap_or_default();

        // Get all users (Phase 3: simplified implementation)
        let all_users = self
            .user_repo
            .list_all(pagination.limit(), pagination.offset())
            .await?;

        // Apply filters in memory (Phase 3: will be moved to SQL in Phase 4)
        let filtered_users: Vec<_> = all_users
            .into_iter()
            .filter(|user| {
                // Filter by active status
                if let Some(is_active) = filter.is_active {
                    if user.is_active() != is_active {
                        return false;
                    }
                }

                // Filter by username substring (case-insensitive)
                if let Some(ref username_pattern) = filter.username_contains {
                    let username_lower = user.username().as_str().to_lowercase();
                    let pattern_lower = username_pattern.to_lowercase();
                    if !username_lower.contains(&pattern_lower) {
                        return false;
                    }
                }

                // Filter by email substring (case-insensitive)
                if let Some(ref email_pattern) = filter.email_contains {
                    let email_lower = user.email().as_str().to_lowercase();
                    let pattern_lower = email_pattern.to_lowercase();
                    if !email_lower.contains(&pattern_lower) {
                        return false;
                    }
                }

                true
            })
            .collect();

        let total = filtered_users.len() as i64;

        // Convert to DTOs
        let dtos: Vec<UserDto> = filtered_users.into_iter().map(UserDto::from).collect();

        Ok(PaginationResult::new(dtos, total, pagination))
    }

    /// Execute query for a single user by ID
    ///
    /// # Arguments
    /// * `user_id` - User identifier
    ///
    /// # Returns
    /// UserDto if found, None otherwise
    ///
    /// # Errors
    /// Returns `RepositoryError` if database query fails
    pub async fn get_by_id(&self, user_id: UserId) -> Result<Option<UserDto>, RepositoryError> {
        let user = self.user_repo.find_by_id(user_id).await?;
        Ok(user.map(UserDto::from))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_filter_all() {
        let filter = UserFilter::all();
        assert!(filter.is_active.is_none());
        assert!(filter.username_contains.is_none());
        assert!(filter.email_contains.is_none());
    }

    #[test]
    fn test_user_filter_active_only() {
        let filter = UserFilter::active_only();
        assert_eq!(filter.is_active, Some(true));
    }

    #[test]
    fn test_user_filter_builder() {
        let filter = UserFilter::active_only()
            .with_username("john")
            .with_email("example.com");

        assert_eq!(filter.is_active, Some(true));
        assert_eq!(filter.username_contains, Some("john".to_string()));
        assert_eq!(filter.email_contains, Some("example.com".to_string()));
    }

    #[test]
    fn test_user_sort_default() {
        let sort = UserSort::default();
        assert_eq!(sort.field, UserSortField::CreatedAt);
        assert_eq!(sort.direction, SortDirection::Desc);
    }
}
