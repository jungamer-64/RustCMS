//! Domain layer module aggregator aligned with the flattened Phase 2 structure.
//!
//! This module exposes the domain entities and value objects as top-level
//! modules, matching the architecture plan documented in `RESTRUCTURE_PLAN.md`.

pub mod category;
pub mod comment;
pub mod post;
pub mod tag;
pub mod user;

// Optional new structure for domain services/events behind feature flag
#[cfg(feature = "restructure_domain")]
pub mod events;
#[cfg(feature = "restructure_domain")]
pub mod services;

// Database models re-export or placeholder depending on feature
#[cfg(feature = "database")]
pub mod models {
    pub use crate::models::*;
}

#[cfg(not(feature = "database"))]
pub mod models {}

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
