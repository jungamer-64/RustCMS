use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Password hashing error: {0}")]
    PasswordHash(String),
    #[error("Biscuit error: {0}")]
    Biscuit(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("WebAuthn error: {0}")]
    WebAuthn(String),
}

impl From<AuthError> for crate::AppError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InsufficientPermissions => Self::Authorization(err.to_string()),
            _ => Self::Authentication(err.to_string()),
        }
    }
}
