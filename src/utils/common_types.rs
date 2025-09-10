//! Common response types for API
//!
//! Unified response structures used across all handlers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::dto_from_model;
use crate::models::{User, UserRole}; // macro

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

/// Post summary for admin interface
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PostSummary {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub status: String,
    pub created_at: String,
}
