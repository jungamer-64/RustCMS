//! Domain layer module aggregator aligned with the flattened Phase 2 structure.
//!
//! This module exposes the domain entities and value objects as top-level
//! modules, matching the architecture plan documented in `RESTRUCTURE_PLAN.md`.

pub mod category;
pub mod comment;
pub mod post;
pub mod tag;
pub mod user;
pub mod common;

pub mod events;
pub mod services;

// Phase 6-A: Removed legacy models re-export
// Database models are now in infrastructure/database/models.rs
// Domain entities are defined in this module (user.rs, post.rs, etc.)

// Value objects facade (may be expanded during migration)
pub mod value_objects;

// Domain prelude: ease imports during migration
pub mod prelude {
    pub use super::category::*;
    pub use super::comment::*;
    pub use super::post::*;
    pub use super::tag::*;
    pub use super::user::*;
}
