//! Aggregated re-exports for domain value objects.
//!
//! The individual entity modules own their value object definitions. This
//! module provides a compatibility layer so existing code using
//! `crate::domain::value_objects::*` continues to compile while the codebase
//! migrates to direct module imports.

pub use crate::domain::category::{CategoryDescription, CategoryId, CategoryName, CategorySlug};
pub use crate::domain::comment::{CommentId, CommentStatus, CommentText};
pub use crate::domain::post::{Content, PostId, PostStatus, PublishedAt, Slug, Title};
pub use crate::domain::tag::{Tag, TagDescription, TagId, TagName};
pub use crate::domain::user::{Email, EmailError, User, UserId, Username, UsernameError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_user_types() {
        let email = Email::new("test@example.com".to_string()).expect("valid email");
        assert_eq!(email.as_str(), "test@example.com");

        let id = UserId::new();
        assert_ne!(id, UserId::new());

        let username = Username::new("valid_name".to_string()).expect("valid username");
        assert_eq!(username.as_str(), "valid_name");
    }

    #[test]
    fn aggregates_post_types() {
        let slug = Slug::new("valid-slug".to_string()).expect("valid slug");
        assert_eq!(slug.as_str(), "valid-slug");
    }
}
