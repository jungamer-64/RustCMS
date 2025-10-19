// src/web/handlers/mod.rs
//! API Handlers - V2 Handlers (New DDD Structure)
//!
//! Phase 7: Legacy handlers removed, only v2 handlers remain

use crate::AppError;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde_json::{Value as JsonValue, json};

// V2 Handlers (New DDD Structure)
pub mod categories_v2;
pub mod comments_v2;
pub mod health_v2;
pub mod posts_v2;
pub mod users_v2;

// ============================================================================
// Phase 4 New Structure - CQRS統合ハンドラ（監査推奨）
// ============================================================================
#[cfg(feature = "restructure_domain")]
pub mod users_v2;

#[cfg(feature = "restructure_domain")]
pub mod posts_v2;

#[cfg(feature = "restructure_domain")]
pub mod comments_v2;

#[cfg(feature = "restructure_domain")]
pub mod categories_v2;

#[cfg(feature = "restructure_domain")]
pub mod health_v2;

// Public endpoints and HTTP methods used to determine security configuration.
// NOTE: When adding new unauthenticated endpoints, update this list to keep the
// generated OpenAPI specification accurate.
const PUBLIC_ENDPOINTS: &[(&str, &str)] = &[
    ("/api/v1/auth/register", "post"),
    ("/api/v1/auth/login", "post"),
    ("/api/v1/auth/refresh", "post"),
    ("/api/v1/health", "get"),
    ("/api/v1/health/liveness", "get"),
    ("/api/v1/health/readiness", "get"),
    ("/api/v1/metrics", "get"),
    ("/api/v1/search", "get"),
    ("/api/v1/search/suggest", "get"),
    ("/api/v1/search/stats", "get"),
    ("/api/v1/search/health", "get"),
];

const HTTP_METHODS: &[&str] = &["get", "post", "put", "delete", "patch"];

// (previously had redundant re-exports here; modules are public via `pub mod` already)

/// Home page handler - integrates functionality from cms-simple
/// Provides a web interface with quick navigation links to all available endpoints
pub async fn home() -> impl IntoResponse {
    Html(include_str!("../../../templates/home.html"))
}

/// Returns the core API information payload used by the `/api/v1` endpoints.
fn build_api_info() -> JsonValue {
    json!({
        "api_version": "v1",
        "endpoints": {
            "auth": "/api/v1/auth",
            "posts": "/api/v1/posts",
            "users": "/api/v1/users",
            "search": "/api/v1/search",
            "health": "/api/v1/health"
        },
        "documentation": "/api/docs",
        "status": "operational",
        "integration": "unified-cms (cms-lightweight + cms-simple + cms-unified)"
    })
}

/// API information endpoint (v1 root). Documented for OpenAPI.
#[utoipa::path(
    get,
    path = "/api/v1",
    responses((status = 200, description = "Get API Information", body = inline(serde_json::Value)))
)]
pub async fn api_info_v1() -> impl IntoResponse {
    Json(build_api_info())
}

/// Alias endpoint: `/api/v1/info` (NOT documented in OpenAPI to avoid duplicate schema emission).
/// Returns the same payload as `/api/v1`.
pub async fn api_info_info() -> impl IntoResponse {
    Json(build_api_info())
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
    let html = include_str!("../../../templates/swagger-ui.html");
    Html(html.to_string())
}

// Legacy `/docs` support removed - use `/api/docs`.

/// Return generated `OpenAPI` JSON from the compile-time `ApiDoc`
///
/// Errors are surfaced as `AppError` instead of panicking to make the handler robust.
pub async fn openapi_json() -> Result<impl IntoResponse, AppError> {
    let doc = ApiDoc::openapi();
    let mut spec = serde_json::to_value(&doc).map_err(AppError::Serde)?;
    configure_openapi_security(&mut spec)?;
    Ok(Json(spec))
}

fn configure_openapi_security(spec: &mut JsonValue) -> Result<(), AppError> {
    add_security_schemes(spec)?;
    add_api_key_permissions(spec);
    remove_global_security(spec);
    apply_endpoint_security(spec)?;
    Ok(())
}

fn add_security_schemes(spec: &mut JsonValue) -> Result<(), AppError> {
    let components = spec
        .get_mut("components")
        .and_then(|c| c.as_object_mut())
        .ok_or_else(|| AppError::Internal("Missing components section in OpenAPI spec".into()))?;

    let schemes = components
        .entry("securitySchemes")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| AppError::Internal("securitySchemes must be an object".into()))?;

    schemes.entry("BiscuitAuth").or_insert(json!({
        "type": "http",
        "scheme": "bearer",
        "bearerFormat": "Biscuit",
        "description": "Biscuit token authentication"
    }));

    schemes.entry("ApiKeyHeader").or_insert(json!({
        "type": "apiKey",
        "name": "X-API-Key",
        "in": "header",
        "description": "API key authentication"
    }));

    Ok(())
}

fn add_api_key_permissions(spec: &mut JsonValue) {
    if let Some(components) = spec.get_mut("components").and_then(|c| c.as_object_mut()) {
        let permissions = crate::models::api_key::ApiKey::ALLOWED_PERMISSIONS;
        components.insert("x-apiKey-permissions".to_string(), json!(permissions));
    }
}

fn remove_global_security(spec: &mut JsonValue) {
    if let Some(obj) = spec.as_object_mut() {
        obj.remove("security");
    }
}

fn apply_endpoint_security(spec: &mut JsonValue) -> Result<(), AppError> {
    let paths = spec
        .get_mut("paths")
        .and_then(|p| p.as_object_mut())
        .ok_or_else(|| AppError::Internal("Missing paths section in OpenAPI spec".into()))?;

    for (path, item) in paths.iter_mut() {
        let Some(item_obj) = item.as_object_mut() else {
            continue;
        };

        for method in HTTP_METHODS {
            if let Some(operation) = item_obj.get_mut(*method).and_then(|o| o.as_object_mut()) {
                configure_operation_security(operation, path, method);
            }
        }
    }

    Ok(())
}

fn configure_operation_security(
    operation: &mut serde_json::Map<String, JsonValue>,
    path: &str,
    method: &str,
) {
    if is_public_endpoint(path, method) {
        operation.remove("security");
    } else if path.starts_with("/api/v1/") {
        operation.insert("security".to_string(), json!([{ "BiscuitAuth": [] }]));
    }
}

fn is_public_endpoint(path: &str, method: &str) -> bool {
    PUBLIC_ENDPOINTS
        .iter()
        .any(|(public_path, public_method)| path == *public_path && method == *public_method)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_public_endpoints_unique() {
        let mut seen = HashSet::new();
        for endpoint in PUBLIC_ENDPOINTS {
            assert!(
                seen.insert(endpoint),
                "duplicate endpoint entry: {endpoint:?}"
            );
        }
    }

    #[test]
    fn test_api_info_structure() {
        let info = build_api_info();
        assert_eq!(info["api_version"].as_str(), Some("v1"));
        assert!(info["endpoints"].is_object());
        assert_eq!(info["status"].as_str(), Some("operational"));
        assert_eq!(info["documentation"].as_str(), Some("/api/docs"));
    }

    #[test]
    fn test_http_methods_coverage() {
        assert!(HTTP_METHODS.contains(&"get"));
        assert!(HTTP_METHODS.contains(&"post"));
        assert!(HTTP_METHODS.contains(&"put"));
        assert!(HTTP_METHODS.contains(&"delete"));
        assert!(HTTP_METHODS.contains(&"patch"));
        assert!(!HTTP_METHODS.contains(&"trace"));
    }
}
