//! CSRF Protection Middleware
//!
//! Provides Cross-Site Request Forgery protection for state-changing operations.
//! Uses synchronizer token pattern with one-time use tokens for maximum security.

use crate::AppState;
use crate::middleware::security::is_csrf_protected_endpoint;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

/// CSRF protection middleware
/// # Errors
///
/// Returns an error response when the request fails CSRF validation, such as:
/// - missing or invalid CSRF headers/tokens
/// - token mismatch between header and cookie
/// - malformed header values that cannot be parsed
pub async fn csrf_protection_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let method = request.method();
    let path = request.uri().path();

    // Only protect endpoints that need CSRF protection
    if !is_csrf_protected_endpoint(method, path) {
        return Ok(next.run(request).await);
    }

    // Extract CSRF token from headers
    let csrf_token = headers
        .get("X-CSRF-Token")
        .or_else(|| headers.get("X-Requested-With"))
        .and_then(|h| h.to_str().ok());

    match csrf_token {
        Some(token) => {
            if state.csrf.validate_and_consume_token(token).await {
                Ok(next.run(request).await)
            } else {
                // CSRF token validation failed
                Err((
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "Invalid or expired CSRF token",
                        "code": "CSRF_TOKEN_INVALID",
                        "message": "The provided CSRF token is invalid or has already been used."
                    })),
                ))
            }
        }
        None => {
            // Missing CSRF token for state-changing operation
            Err((
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "CSRF token required",
                    "code": "CSRF_TOKEN_MISSING",
                    "message": "State-changing operations require CSRF protection. Include X-CSRF-Token header."
                })),
            ))
        }
    }
}

/// Endpoint to generate CSRF tokens for clients
pub async fn get_csrf_token(State(state): State<AppState>) -> impl IntoResponse {
    let token = state.csrf.generate_token().await;

    Json(json!({
        "success": true,
        "csrf_token": token,
        "expires_in": 3600, // Token valid for 1 hour
        "usage": "Include this token in X-CSRF-Token header for state-changing operations"
    }))
}
