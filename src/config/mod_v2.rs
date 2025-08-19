// Compatibility shim: forward to canonical `config` module
// Keeps existing import paths like `config::mod_v2` working.

pub use crate::config::*;
