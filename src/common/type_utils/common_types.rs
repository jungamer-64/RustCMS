//! Common response types for API
//!
//! Phase 6-B: Legacy models dependency, protected with feature flag
//! Use application/dto/user.rs for new structure

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(not(feature = "restructure_domain"))]
use crate::dto_from_model;
#[cfg(not(feature = "restructure_domain"))]
use crate::models::{User, UserRole}; // macro

#[cfg(feature = "restructure_domain")]
use crate::application::dto::user::UserDto;

/// Unified user information for API responses
#[cfg(not(feature = "restructure_domain"))]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Phase 6-C: Simplified UserInfo for new structure (role as String)
#[cfg(feature = "restructure_domain")]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String, // Converted from domain::user::UserRole
    pub is_active: bool,
    pub email_verified: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Phase 9: Implement conversion from new domain::user::User entity
#[cfg(feature = "restructure_domain")]
impl From<&crate::domain::user::User> for UserInfo {
    fn from(user: &crate::domain::user::User) -> Self {
        use uuid::Uuid;
        Self {
            id: Uuid::from(user.id()).to_string(),
            username: user.username().as_str().to_string(),
            email: user.email().as_str().to_string(),
            first_name: None, // TODO: Add to domain::user::User if needed
            last_name: None,  // TODO: Add to domain::user::User if needed
            role: user.role().as_str().to_string(),
            is_active: user.is_active(),
            email_verified: false, // TODO: Add to domain::user::User if needed
            last_login: None,      // TODO: Add to domain::user::User if needed
            created_at: chrono::Utc::now(), // TODO: Add timestamps to domain::user::User
            updated_at: chrono::Utc::now(), // TODO: Add timestamps to domain::user::User
        }
    }
}

// Phase 9: Implement conversion from owned User (for move semantics)
#[cfg(feature = "restructure_domain")]
impl From<crate::domain::user::User> for UserInfo {
    fn from(user: crate::domain::user::User) -> Self {
        Self::from(&user)
    }
}

// Strongly-typed session identifier to avoid mixing with other strings
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
pub struct SessionId(pub String);

impl SessionId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(not(feature = "restructure_domain"))]
dto_from_model!(UserInfo, User, |u| UserInfo {
    id: u.id.to_string(),
    username: u.username.clone(),
    email: u.email.clone(),
    first_name: u.first_name.clone(),
    last_name: u.last_name.clone(),
    role: UserRole::parse_str(&u.role).unwrap_or(UserRole::Subscriber),
    is_active: u.is_active,
    email_verified: u.email_verified,
    last_login: u.last_login,
    created_at: u.created_at,
    updated_at: u.updated_at,
});

/// Authentication tokens
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

/// Authentication response with tokens and user info
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    pub tokens: AuthTokens,
    pub user: UserInfo,
}

/// Post summary for admin interface
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PostSummary {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub status: String,
    pub created_at: String,
}
