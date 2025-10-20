//! Domain-specific error definitions and aliases.

use thiserror::Error;

/// Errors that can occur within the domain layer.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
    #[error("Invalid user ID: {0}")]
    InvalidUserId(String),
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
    #[error("Invalid post ID: {0}")]
    InvalidPostId(String),
    #[error("Invalid slug: {0}")]
    InvalidSlug(String),
    #[error("Invalid title: {0}")]
    InvalidTitle(String),
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    #[error("Invalid published at: {0}")]
    InvalidPublishedAt(String),
    #[error("Invalid post status: {0}")]
    InvalidPostStatus(String),
    #[error("Invalid comment text: {0}")]
    InvalidCommentText(String),
    #[error("Invalid comment author: {0}")]
    InvalidCommentAuthor(String),
    #[error("Invalid comment post: {0}")]
    InvalidCommentPost(String),
    #[error("Invalid comment status: {0}")]
    InvalidCommentStatus(String),
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Invalid tag name: {0}")]
    InvalidTagName(String),
    #[error("Invalid tag description: {0}")]
    InvalidTagDescription(String),
    #[error("Invalid tag status: {0}")]
    InvalidTagStatus(String),
    #[error("Invalid category name: {0}")]
    InvalidCategoryName(String),
    #[error("Invalid category slug: {0}")]
    InvalidCategorySlug(String),
    #[error("Invalid category description: {0}")]
    InvalidCategoryDescription(String),
    #[error("Invalid category status: {0}")]
    InvalidCategoryStatus(String),
    #[error("Unknown domain error: {0}")]
    Unknown(String),
}

/// Convenience result alias for domain operations.
pub type DomainResult<T> = std::result::Result<T, DomainError>;

impl From<crate::user::EmailError> for DomainError {
    fn from(err: crate::user::EmailError) -> Self {
        DomainError::InvalidEmail(err.to_string())
    }
}

impl From<crate::user::UsernameError> for DomainError {
    fn from(err: crate::user::UsernameError) -> Self {
        DomainError::InvalidUsername(err.to_string())
    }
}
