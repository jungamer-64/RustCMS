//! Security validation helpers backed by `shared-core`.
//!
//! Provides compatibility shims so existing code can keep importing from
//! `crate::utils::security_validation` while the underlying implementation
//! lives in `shared_core::security::security_validation`.

use shared_core::security::security_validation as core;
pub use core::{
    validate_and_sanitize_content,
    validate_email_secure,
    validate_file_path,
    validate_request_rate,
    validate_search_query,
    validate_title,
    validate_username_secure,
};
