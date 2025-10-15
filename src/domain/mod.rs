// Re-export surface for domain-level types and models
// This file was added as part of an incremental restructure step. Do NOT move files yet.

#[cfg(feature = "database")]
pub mod models {
    pub use crate::models::*;
}

#[cfg(not(feature = "database"))]
pub mod models {
    // Database feature is disabled: provide an empty placeholder so callers
    // can still refer to `crate::domain::models` without causing build errors.
}

pub mod value_objects;

// Re-export common domain types

/// Domain prelude: common types that application code may import during
/// the incremental migration to the domain layer.
pub mod prelude {

    pub use super::value_objects::*;
}
