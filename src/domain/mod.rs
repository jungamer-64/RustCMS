// Top-level domain module facade for restructure
// Re-export existing domain modules or provide placeholders for phased migration
pub mod user;
pub mod post;
pub mod comment;
pub mod tag;
pub mod category;

// Optional new structure for domain services/events behind feature flag
#[cfg(feature = "restructure_domain")]
pub mod services;
#[cfg(feature = "restructure_domain")]
pub mod events;

// Database models re-export or placeholder depending on feature
#[cfg(feature = "database")]
pub mod models {
    pub use crate::models::*;
}

#[cfg(not(feature = "database"))]
pub mod models {
    // placeholder when database feature disabled
}

// Value objects facade (may be expanded during migration)
pub mod value_objects;

// Domain prelude: ease imports during migration
pub mod prelude {
    pub use super::value_objects::*;
}
