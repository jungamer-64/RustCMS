//! Domain value objects
//!
//! Type-safe wrappers for core domain concepts like UserId, PostId, Email.

use std::fmt;
use uuid::Uuid;

/// Type-safe user identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(Uuid);

impl UserId {
    /// Create a new random UserId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a UserId from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Type-safe post identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostId(Uuid);

impl PostId {
    /// Create a new random PostId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a PostId from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for PostId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for PostId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Type-safe email value object
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    /// Create a new Email (basic validation)
    pub fn new(email: String) -> Result<Self, String> {
        if email.contains('@') && !email.is_empty() {
            Ok(Email(email))
        } else {
            Err("Invalid email format".to_string())
        }
    }

    /// Get email as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Email {
    fn from(s: String) -> Self {
        Email(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let user_id = UserId::from_uuid(uuid);
        assert_eq!(*user_id.as_uuid(), uuid);
    }

    #[test]
    fn test_post_id_new() {
        let id1 = PostId::new();
        let id2 = PostId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_email_validation() {
        assert!(Email::new("test@example.com".to_string()).is_ok());
        assert!(Email::new("invalid".to_string()).is_err());
    }
}
