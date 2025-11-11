//! Infrastructure layer providing low-level components for database and authentication.

#[cfg(feature = "database")]
pub mod database;
pub mod auth;

// Re-export shared-core types for convenience
pub use shared_core::error::{InfrastructureError, Result as InfrastructureResult};

