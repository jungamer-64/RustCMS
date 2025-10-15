pub mod ports;

pub use ports::*;
#[cfg(feature = "database")]
pub mod container;
#[cfg(feature = "database")]
pub use container::AppContainer;
// Re-export surface for application-layer services, handlers and use-cases
// This file intentionally re-exports existing handlers and services so callers
// can start referring to `crate::application::...` during the restructure.

pub mod handlers {
    pub use crate::handlers::*;
}

pub mod use_cases;
pub use use_cases::*;

pub mod services {
    // Re-exports for service-like modules (eg: limiter, auth glue) can go here.
}

pub use handlers::*;

/// Application prelude: commonly used handler/service types for migrating
/// call sites to `crate::application`.
pub mod prelude {
    pub use super::handlers::*;
}
