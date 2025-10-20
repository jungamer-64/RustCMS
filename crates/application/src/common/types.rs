//! Application-layer specific error types and aliases.

use thiserror::Error;

use crate::ports;

pub use domain::common::{DomainError, DomainResult};

/// Error type representing failures within the application layer.
#[derive(Debug, Error, Clone)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    DomainError(#[from] DomainError),

    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("Use case validation failed: {0}")]
    ValidationError(String),

    #[error("Invalid UUID: {0}")]
    InvalidUuid(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Authorization failed: {0}")]
    Unauthorized(String),

    #[error("Event publish failed: {0}")]
    EventPublishError(String),

    #[error("Unknown application error: {0}")]
    Unknown(String),
}

impl From<ports::repositories::RepositoryError> for ApplicationError {
    fn from(err: ports::repositories::RepositoryError) -> Self {
        use ports::repositories::RepositoryError as RE;
        match err {
            RE::NotFound(msg) => ApplicationError::NotFound(msg),
            RE::Duplicate(msg) => ApplicationError::Conflict(msg),
            RE::ValidationError(msg) => ApplicationError::ValidationError(msg),
            RE::ConversionError(msg) => {
                ApplicationError::ValidationError(format!("Conversion error: {msg}"))
            }
            RE::ConnectionError(msg) => {
                ApplicationError::RepositoryError(format!("Connection error: {msg}"))
            }
            RE::DatabaseError(msg) | RE::Unknown(msg) => ApplicationError::RepositoryError(msg),
        }
    }
}

impl From<ports::events::EventError> for ApplicationError {
    fn from(err: ports::events::EventError) -> Self {
        ApplicationError::EventPublishError(err.to_string())
    }
}

/// Convenient alias for results returned from application services.
pub type ApplicationResult<T> = std::result::Result<T, ApplicationError>;

/// Unified result alias maintained for backwards compatibility.
pub type Result<T> = ApplicationResult<T>;
