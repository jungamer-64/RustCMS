//! Repository implementations
//!
//! Concrete implementations of repository traits for different backends.

#[cfg(feature = "database")]
pub mod diesel_user_repository;
#[cfg(feature = "database")]
pub mod diesel_post_repository;

#[cfg(feature = "database")]
pub use diesel_user_repository::DieselUserRepository;
#[cfg(feature = "database")]
pub use diesel_post_repository::DieselPostRepository;
