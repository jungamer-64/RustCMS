//! Port definitions for application dependencies
//!
//! Ports define the interfaces that use-cases depend on,
//! allowing for loose coupling and testability.

pub mod post_repository;
pub mod user_repository;

pub use post_repository::PostRepository;
pub use user_repository::UserRepository;
