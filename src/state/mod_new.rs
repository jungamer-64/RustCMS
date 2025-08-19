// Compatibility shim: forward to canonical `state` module
// Keep this file so code that imports `state::mod_new` continues to compile.

pub use crate::state::*;
