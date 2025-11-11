//! Validation helpers shared across layers.
//!
//! These utilities live in `shared-core` so that every crate (domain,
//! application, infrastructure, and the main binary) can reuse the same input
//! validation logic without duplicating conversions or depending on the root
//! crate.

use crate::error::AppError;
use std::collections::HashMap;
use validator::ValidationErrors;

/// Normalize `validator` errors into a simple field -> messages map so API
/// handlers can serialize consistent responses.
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

/// Simple email validation.
#[must_use]
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

/// Validate basic user credentials (email + optional password).
///
/// # Errors
/// Returns [`AppError::BadRequest`] when the email format or password length is
/// invalid.
pub fn validate_user_input(email: &str, password: Option<&str>) -> Result<(), AppError> {
    if !validate_email(email) {
        return Err(AppError::BadRequest("Invalid email format".to_string()));
    }

    if let Some(pwd) = password {
        if pwd.len() < 6 {
            return Err(AppError::BadRequest(
                "Password must be at least 6 characters".to_string(),
            ));
        }
    }

    Ok(())
}
