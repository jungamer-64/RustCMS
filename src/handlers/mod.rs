//! API Handlers - Request processing and business logic
//! 
//! Simplified handlers for compilation testing

use axum::{
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde_json::json;

#[cfg(feature = "auth")]
pub mod auth;
pub mod posts;
pub mod users;
pub mod search;
pub mod health;
pub mod admin;

// (previously had redundant re-exports here; modules are public via `pub mod` already)

/// API information endpoint
pub async fn api_info() -> impl IntoResponse {
    Json(json!({
        "api_version": "v1",
        "endpoints": {
            "auth": "/api/v1/auth",
            "posts": "/api/v1/posts", 
            "users": "/api/v1/users",
            "search": "/api/v1/search",
            "health": "/api/v1/health"
        },
        "documentation": "/api/v1/docs",
        "status": "operational"
    }))
}

/// 404 handler
pub async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "success": false,
            "error": "Endpoint not found",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    )
}
