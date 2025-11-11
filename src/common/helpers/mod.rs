//! Shared helper utilities (legacy shim).
//!
//! This module now re-exports the canonical helper implementations from the
//! `shared-core` crate so dependent code can keep using `crate::common::helpers`
//! paths while the actual logic lives in a single place.

pub use shared_core::helpers::{
    self,
    cache_helpers,
    date,
    hash,
    text,
    url_encoding,
    vec_helpers,
};

// Keep top-level helper re-exports (now pointing to shared-core)
pub use shared_core::helpers::*;
