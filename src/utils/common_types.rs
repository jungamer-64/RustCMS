//! Common response types for API
//!
//! Unified response structures used across all handlers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::{User, UserRole};

/// Unified user information for API responses
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

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            username: user.username.clone(),
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: UserRole::parse_str(&user.role).unwrap_or(UserRole::Subscriber),
            is_active: user.is_active,
            email_verified: user.email_verified,
            last_login: user.last_login,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self::from(&user)
    }
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
