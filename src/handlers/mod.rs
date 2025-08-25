//! API Handlers - Request processing and business logic
//!
//! Simplified handlers for compilation testing

use crate::openapi::ApiDoc;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde_json::json;
use serde_json::Value as JsonValue;
use utoipa::OpenApi;

pub mod admin;
#[cfg(feature = "auth")]
pub mod auth;
pub mod health;
pub mod posts;
pub mod search;
pub mod users;

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
    "documentation": "/api/docs",
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
        })),
    )
}

/// Serve the bundled Swagger UI HTML (templates/swagger-ui.html) at /api/docs
pub async fn docs_ui() -> impl IntoResponse {
    // include_str! resolves relative to the crate root in many setups; templates/ is at project root
    let html = include_str!("../../templates/swagger-ui.html");
    Html(html.to_string())
}

// Legacy `/docs` support removed - use `/api/docs`.

/// Return generated OpenAPI JSON from the compile-time `ApiDoc`
pub async fn openapi_json() -> impl IntoResponse {
    // bring trait into scope to call `openapi()`
    let doc = ApiDoc::openapi();
    // serialize to serde_json::Value so axum::Json can return it
    let v: JsonValue = serde_json::to_value(&doc).expect("failed to convert openapi to json value");
    Json(v)
}
