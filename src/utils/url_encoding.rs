//! URL helpers backed by the shared-core implementation.
//!
//! This module keeps the historical `crate::utils::url_encoding::*` surface but
//! delegates to `shared_core::helpers::url_encoding` so the logic only lives in
//! a single place. Functions that can fail map the shared-core `AppError` into
//! the local `crate::AppError` so existing call sites continue to return
//! `crate::Result`.

use shared_core::helpers::url_encoding as core;

pub use core::{encode_slug, encode_url_param, generate_safe_slug, url_encode};

/// Decode a percent-encoded URL parameter.
pub fn decode_url_param(input: &str) -> crate::Result<String> {
    core::decode_url_param(input).map_err(crate::AppError::from)
}

/// Decode a percent-encoded slug.
pub fn decode_slug(input: &str) -> crate::Result<String> {
    core::decode_slug(input).map_err(crate::AppError::from)
}

/// Decode a URL produced by [`url_encode`].
pub fn url_decode(input: &str) -> crate::Result<String> {
    core::url_decode(input).map_err(crate::AppError::from)
}

/// Validate the length of a URL parameter.
pub fn validate_param_length(
    param: &str,
    max_length: usize,
    param_name: &str,
) -> crate::Result<()> {
    core::validate_param_length(param, max_length, param_name).map_err(crate::AppError::from)
}

/// Validate common URL parameter safety rules.
pub fn validate_url_param(input: &str) -> crate::Result<()> {
    core::validate_url_param(input).map_err(crate::AppError::from)
}

/// Validate slug safety rules.
pub fn validate_slug(input: &str) -> crate::Result<()> {
    core::validate_slug(input).map_err(crate::AppError::from)
}
