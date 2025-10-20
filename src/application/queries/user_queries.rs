// src/application/queries/user_queries.rs
//! User Queries - CQRS Query pattern for user data retrieval

use serde::{Deserialize, Serialize};

use crate::application::dto::user::UserDto;
use crate::application::ports::repositories::{RepositoryError, UserRepository};
use crate::application::queries::pagination::{PaginationParams, PaginationResult};
use crate::domain::user::UserId;

// ============================================================================
// Filter & Sort Types
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserFilter {
    pub is_active: Option<bool>,
    pub username_contains: Option<String>,
    pub email_contains: Option<String>,
}

impl UserFilter {
    pub const fn all() -> Self {
        Self {
            is_active: None,
            username_contains: None,
            email_contains: None,
        }
    }

    pub const fn active_only() -> Self {
        Self {
            is_active: Some(true),
            username_contains: None,
            email_contains: None,
        }
    }

    pub const fn inactive_only() -> Self {
        Self {
            is_active: Some(false),
            username_contains: None,
            email_contains: None,
        }
    }

    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username_contains = Some(username.into());
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email_contains = Some(email.into());
        self
    }

    /// フィルタリングを適用
    fn apply(&self, user: &crate::domain::user::User) -> bool {
        // アクティブ状態フィルタ
        if let Some(is_active) = self.is_active {
            if user.is_active() != is_active {
                return false;
            }
        }

        // ユーザー名フィルタ
        if let Some(ref pattern) = self.username_contains {
            let username_lower = user.username().as_str().to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            if !username_lower.contains(&pattern_lower) {
                return false;
            }
        }

        // メールフィルタ
        if let Some(ref pattern) = self.email_contains {
            let email_lower = user.email().as_str().to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            if !email_lower.contains(&pattern_lower) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserSortField {
    CreatedAt,
    UpdatedAt,
    Username,
    Email,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Desc
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UserSort {
    pub field: UserSortField,
    pub direction: SortDirection,
}

impl Default for UserSort {
    fn default() -> Self {
        Self {
            field: UserSortField::CreatedAt,
            direction: SortDirection::Desc,
        }
    }
}

// ============================================================================
// ListUsersQuery
// ============================================================================

pub struct ListUsersQuery<'a> {
    user_repo: &'a dyn UserRepository,
}

impl<'a> ListUsersQuery<'a> {
    pub const fn new(user_repo: &'a dyn UserRepository) -> Self {
        Self { user_repo }
    }

    pub async fn execute(
        &self,
        filter: UserFilter,
        _sort: Option<UserSort>,
        pagination: PaginationParams,
    ) -> Result<PaginationResult<UserDto>, RepositoryError> {
        // TODO: Phase 4 で SQL レベルのフィルタリングを実装
        let all_users = self
            .user_repo
            .list_all(pagination.limit(), pagination.offset())
            .await?;

        // メモリ内フィルタリング（Phase 3）
        let filtered_users: Vec<_> = all_users
            .into_iter()
            .filter(|user| filter.apply(user))
            .collect();

        let total = filtered_users.len() as i64;
        let dtos: Vec<UserDto> = filtered_users.into_iter().map(UserDto::from).collect();

        Ok(PaginationResult::new(dtos, total, pagination))
    }

    pub async fn get_by_id(&self, user_id: UserId) -> Result<Option<UserDto>, RepositoryError> {
        Ok(self.user_repo.find_by_id(user_id).await?.map(UserDto::from))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_filter_builder() {
        let filter = UserFilter::active_only()
            .with_username("john")
            .with_email("example.com");

        assert_eq!(filter.is_active, Some(true));
        assert_eq!(filter.username_contains.as_deref(), Some("john"));
        assert_eq!(filter.email_contains.as_deref(), Some("example.com"));
    }

    #[test]
    fn test_user_sort_default() {
        let sort = UserSort::default();
        assert_eq!(sort.field, UserSortField::CreatedAt);
        assert_eq!(sort.direction, SortDirection::Desc);
    }

    #[test]
    fn test_user_filter_apply() {
        use crate::domain::user::{Email, User, Username};

        let user = User::new(
            Username::new("testuser".into()).unwrap(),
            Email::new("test@example.com".into()).unwrap(),
        );

        // Active filter
        assert!(UserFilter::active_only().apply(&user));
        assert!(!UserFilter::inactive_only().apply(&user));

        // Username filter
        assert!(UserFilter::all().with_username("test").apply(&user));
        assert!(!UserFilter::all().with_username("other").apply(&user));

        // Email filter
        assert!(UserFilter::all().with_email("example").apply(&user));
        assert!(!UserFilter::all().with_email("other").apply(&user));
    }
}
