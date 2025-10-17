//! Shim module re-exporting the canonical Diesel implementation.
//!
//! This keeps the Phase 3 `restructure_application` surface area intact while
//! forwarding every call to the shipping adapter in
//! `crate::infrastructure::repositories`.
pub use crate::infrastructure::repositories::DieselTagRepository;
