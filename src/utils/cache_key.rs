//! Compatibility shim for cache key helpers.
//!
//! Re-exports the shared implementations so existing modules can continue to
//! reference `crate::utils::cache_key::*`.

pub use shared_core::helpers::cache_helpers::*;
