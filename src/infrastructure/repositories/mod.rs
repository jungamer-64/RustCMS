//! Repository implementations
//!
//! Concrete implementations of repository traits for different backends.
//! This module consolidates all repository implementations (database, cache, etc.).

// ============================================================================
// Error Handling Helpers (Phase 3 Refactoring)
// ============================================================================

#[cfg(feature = "restructure_domain")]
pub mod error_helpers;

// ============================================================================
// Database Repositories (Diesel ORM)
// ============================================================================

// Phase 3 Repositories (restructure_domain feature required)
#[cfg(all(feature = "database", feature = "restructure_domain"))]
pub mod diesel_category_repository;
#[cfg(feature = "database")]
pub mod diesel_comment_repository;
#[cfg(feature = "database")]
pub mod diesel_post_repository;
#[cfg(all(feature = "database", feature = "restructure_domain"))]
pub mod diesel_tag_repository;
#[cfg(feature = "database")]
pub mod diesel_user_repository;

// Re-exports for convenience
#[cfg(all(feature = "database", feature = "restructure_domain"))]
pub use diesel_category_repository::DieselCategoryRepository;
#[cfg(feature = "database")]
pub use diesel_comment_repository::DieselCommentRepository;
#[cfg(feature = "database")]
pub use diesel_post_repository::DieselPostRepository;
#[cfg(all(feature = "database", feature = "restructure_domain"))]
pub use diesel_tag_repository::DieselTagRepository;
#[cfg(feature = "database")]
pub use diesel_user_repository::DieselUserRepository;
