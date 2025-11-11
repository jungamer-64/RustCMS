//! 認証エラー型（改善版）
//!
//! より詳細なエラー情報を保持し、デバッグとセキュリティログを改善

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    // === 認証エラー ===
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("User account is inactive")]
    UserInactive,

    #[error("Invalid password")]
    InvalidPassword,

    // === トークンエラー（詳細化） ===
    #[error("Token expired")]
    TokenExpired,

    /// 汎用トークンエラー（後方互換性のため残す）
    /// 新しいコードでは InvalidTokenFormat または InvalidTokenSignature を使用すること
    #[error("Invalid token")]
    #[deprecated(note = "Use InvalidTokenFormat or InvalidTokenSignature instead")]
    InvalidToken,

    #[error("Invalid token format")]
    InvalidTokenFormat,

    #[error("Invalid token signature")]
    InvalidTokenSignature,

    #[error("Token type mismatch: expected {expected}, got {actual}")]
    TokenTypeMismatch { expected: String, actual: String },

    #[error("Token parse error: {0}")]
    TokenParseError(String),

    // === セッションエラー ===
    #[error("Session not found")]
    SessionNotFound,

    #[error("Session expired")]
    SessionExpired,

    #[error("Session version mismatch (token reuse detected)")]
    SessionVersionMismatch,

    // === 認可エラー ===
    #[error("Insufficient permissions: requires {required}")]
    InsufficientPermissions { required: String },

    #[error("Resource access denied: {resource}")]
    ResourceAccessDenied { resource: String },

    // === システムエラー ===
    #[error("Password hashing error: {0}")]
    PasswordHashError(String),

    /// Biscuitエラー（後方互換性のため残す）
    /// 新しいコードでは BiscuitError を使用すること
    #[error("Biscuit error: {0}")]
    #[deprecated(note = "Use BiscuitError instead")]
    Biscuit(String),

    #[error("Biscuit error: {0}")]
    BiscuitError(String),

    #[error("JWT error: {0}")]
    JwtError(String),

    #[error("Key management error: {0}")]
    KeyManagementError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("WebAuthn error: {0}")]
    WebAuthnError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl AuthError {
    /// エラーがユーザーに表示しても安全かどうか
    ///
    /// セキュリティ上、詳細なエラー情報を隠すべき場合はfalse
    pub fn is_safe_to_expose(&self) -> bool {
        matches!(
            self,
            Self::InvalidCredentials
                | Self::UserNotFound
                | Self::TokenExpired
                | Self::SessionExpired
                | Self::InsufficientPermissions { .. }
                | Self::ResourceAccessDenied { .. }
        )
    }

    /// ユーザー向けの安全なエラーメッセージ
    pub fn user_message(&self) -> String {
        if self.is_safe_to_expose() {
            self.to_string()
        } else {
            "Authentication failed".to_string()
        }
    }

    /// HTTPステータスコードへの変換
    #[allow(deprecated)]
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::InvalidCredentials
            | Self::InvalidPassword
            | Self::TokenExpired
            | Self::SessionExpired
            | Self::InvalidToken
            | Self::InvalidTokenFormat
            | Self::InvalidTokenSignature
            | Self::TokenTypeMismatch { .. }
            | Self::TokenParseError(_) => 401, // Unauthorized

            Self::UserNotFound | Self::SessionNotFound => 404, // Not Found (ただしセキュリティ上401を返すことも検討)

            Self::InsufficientPermissions { .. }
            | Self::ResourceAccessDenied { .. }
            | Self::UserInactive => 403, // Forbidden

            Self::SessionVersionMismatch => 409, // Conflict

            #[allow(deprecated)]
            Self::Biscuit(_)
            | Self::PasswordHashError(_)
            | Self::BiscuitError(_)
            | Self::JwtError(_)
            | Self::KeyManagementError(_)
            | Self::DatabaseError(_)
            | Self::WebAuthnError(_)
            | Self::ConfigError(_) => 500, // Internal Server Error
        }
    }

    /// ログレベルの判定
    #[allow(deprecated)]
    pub fn log_level(&self) -> tracing::Level {
        match self {
            Self::InvalidCredentials
            | Self::UserNotFound
            | Self::TokenExpired
            | Self::SessionExpired => tracing::Level::WARN,

            Self::SessionVersionMismatch => tracing::Level::ERROR, // セキュリティイベント

            #[allow(deprecated)]
            Self::Biscuit(_)
            | Self::PasswordHashError(_)
            | Self::BiscuitError(_)
            | Self::JwtError(_)
            | Self::KeyManagementError(_)
            | Self::DatabaseError(_)
            | Self::ConfigError(_) => tracing::Level::ERROR,

            _ => tracing::Level::INFO,
        }
    }
}

// 下位エラーからの変換実装
impl From<argon2::password_hash::Error> for AuthError {
    fn from(err: argon2::password_hash::Error) -> Self {
        Self::PasswordHashError(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => Self::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => Self::InvalidTokenSignature,
            _ => Self::JwtError(err.to_string()),
        }
    }
}

impl From<biscuit_auth::error::Token> for AuthError {
    fn from(err: biscuit_auth::error::Token) -> Self {
        Self::BiscuitError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_to_expose() {
        assert!(AuthError::InvalidCredentials.is_safe_to_expose());
        assert!(AuthError::TokenExpired.is_safe_to_expose());
        assert!(!AuthError::BiscuitError("internal".to_string()).is_safe_to_expose());
        assert!(!AuthError::DatabaseError("sql error".to_string()).is_safe_to_expose());
    }

    #[test]
    fn test_user_message() {
        let err = AuthError::InvalidCredentials;
        assert_eq!(err.user_message(), "Invalid credentials");

        let err = AuthError::DatabaseError("connection failed".to_string());
        assert_eq!(err.user_message(), "Authentication failed");
    }

    #[test]
    fn test_http_status_code() {
        assert_eq!(AuthError::InvalidCredentials.http_status_code(), 401);
        assert_eq!(
            AuthError::InsufficientPermissions {
                required: "admin".to_string()
            }
            .http_status_code(),
            403
        );
        assert_eq!(AuthError::SessionVersionMismatch.http_status_code(), 409);
        assert_eq!(
            AuthError::DatabaseError("error".to_string()).http_status_code(),
            500
        );
    }

    #[test]
    fn test_log_level() {
        use tracing::Level;

        assert_eq!(AuthError::InvalidCredentials.log_level(), Level::WARN);
        assert_eq!(AuthError::SessionVersionMismatch.log_level(), Level::ERROR);
        assert_eq!(
            AuthError::DatabaseError("error".to_string()).log_level(),
            Level::ERROR
        );
    }
}
