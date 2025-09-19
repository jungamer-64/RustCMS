use crate::AppError;
use std::collections::HashMap;
use validator::ValidationErrors;

#[must_use]
pub fn format_validation_errors(errors: &ValidationErrors) -> HashMap<String, Vec<String>> {
    let mut formatted = HashMap::new();

    for (field, field_errors) in errors.field_errors() {
        let mut messages = Vec::new();
        for error in field_errors {
            if let Some(message) = &error.message {
                messages.push(message.to_string());
            } else {
                messages.push(format!("Invalid value for field '{field}'"));
            }
        }
        formatted.insert(field.to_string(), messages);
    }

    formatted
}

/// Simple email validation
#[must_use]
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

/// ユーザー入力バリデーション
/// # Errors
/// Returns [`AppError::BadRequest`] when email format is invalid or password is too short.
pub fn validate_user_input(email: &str, password: Option<&str>) -> Result<(), AppError> {
    if !validate_email(email) {
    return Err(AppError::BadRequest("Invalid email format".to_string().into()));
    }

    if let Some(pwd) = password
        && pwd.len() < 6
    {
        return Err(AppError::BadRequest(
            "Password must be at least 6 characters".to_string().into(),
        ));
    }

    Ok(())
}
