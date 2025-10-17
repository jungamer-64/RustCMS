//! Shim module re-exporting the canonical Diesel implementation.
//!
//! This keeps the Phase 3 `restructure_application` wiring intact while
//! delegating every operation to the production-ready adapter in
//! `crate::infrastructure::repositories`.
pub use crate::infrastructure::repositories::DieselPostRepository;
