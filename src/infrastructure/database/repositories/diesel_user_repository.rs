//! Shim module re-exporting the canonical Diesel implementation.
//!
//! This keeps Phase 3 `restructure_application` builds wiring-compatible while
//! delegating all behavior to the already vetted adapter under
//! `crate::infrastructure::repositories`.
pub use crate::infrastructure::repositories::DieselUserRepository;
