//! Deprecated legacy error module.
//! This file now re-exports the canonical crate-level `AppError` defined in `crate::error`.
//! TODO: Remove this shim after downstream imports migrate from `utils::error` to `error`.

#[allow(deprecated)]
pub use crate::error::AppError;
pub type AppResult<T> = crate::Result<T>;

// Backwards compatibility for code that previously matched additional variants.
// If specific legacy variants are required, extend `crate::error::AppError` instead
// of re‑introducing divergent definitions here.
