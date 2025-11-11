//! Shared Security Utilities (legacy shim).
//!
//! Re-export the canonical security helpers from `shared-core` so legacy paths
//! (`crate::common::security::*`) continue to work while the implementation
//! lives in a single crate.

pub use shared_core::security::{
    self,
    password,
    security_validation,
};

pub use shared_core::security::*;
