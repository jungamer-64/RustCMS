//! Validation helpers backed by `shared-core`.
//!
//! Keeps the long-lived `crate::utils::validation` namespace while delegating
//! to `shared_core::validation` for the heavy lifting.

use shared_core::validation as core;
use std::collections::HashMap;
use validator::ValidationErrors;

pub use core::validate_email;

/// Local adapter on top of the shared implementation so we can keep using the
/// workspace-wide `validator` version without type-mismatch headaches.
#[must_use]
pub fn format_validation_errors(errors: &ValidationErrors) -> HashMap<String, Vec<String>> {
    let mut formatted = HashMap::new();

    for (field, field_errors) in errors.field_errors() {
        let messages = field_errors
            .iter()
            .map(|error| {
                error
                    .message
                    .as_ref()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_else(|| format!("Invalid value for field '{field}'"))
            })
            .collect::<Vec<_>>();
        formatted.insert(field.to_string(), messages);
    }

    formatted
}

/// Validate user input using the shared-core implementation but returning
/// `crate::Result<()>`.
pub fn validate_user_input(email: &str, password: Option<&str>) -> crate::Result<()> {
    core::validate_user_input(email, password).map_err(crate::AppError::from)
}
