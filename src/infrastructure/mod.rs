// Re-export surface for infrastructure implementations (database, cache, search, repositories)
// This file exposes concrete implementations while keeping original paths intact.

// Database adapter re-exports are feature gated because `crate::database` is
// only compiled when the `database` feature is enabled.
#[cfg(feature = "database")]
pub mod database {
    pub use crate::database::*;
}

// Cache adapter re-exports are feature gated because `crate::cache` is
// only compiled when the `cache` feature is enabled.
#[cfg(feature = "cache")]
pub mod cache {
    pub use crate::cache::*;
}

// Search adapter
#[cfg(feature = "search")]
pub mod search {
    pub use crate::search::*;
}

// Auth-related infrastructure
#[cfg(feature = "auth")]
pub mod auth {
    pub use crate::auth::*;
}

// Repositories are defined unconditionally but may themselves be feature-gated
// internally. Re-export them so callers can refer to `crate::infrastructure::repositories`.
pub mod repositories;

// Re-export the gated modules at the top level where appropriate.
#[cfg(feature = "database")]
pub use database::*;

#[cfg(feature = "cache")]
pub use cache::*;

#[cfg(feature = "search")]
pub use search::*;

#[cfg(feature = "auth")]
pub use auth::*;

pub use repositories::*;
