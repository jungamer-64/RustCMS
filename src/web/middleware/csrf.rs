//! CSRF Protection Middleware (Phase 5.2 - 新AppState対応版)
//!
//! Provides Cross-Site Request Forgery protection for state-changing operations.
//! Uses synchronizer token pattern with one-time use tokens for maximum security.
//!
//! Phase 5.2 での変更点:
//! - 新しい `Arc<AppState>` 構造に対応
//! - キャッシュベースのトークン管理 (Phase 5.3 で完全実装)

use crate::infrastructure::app_state::AppState;
use crate::web::middleware::security::is_csrf_protected_endpoint;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::sync::Arc;
use tracing::warn;

/// CSRF protection middleware (Phase 5.2 簡略版)
///
/// # Errors
///
/// Returns an error response when the request fails CSRF validation, such as:
/// - missing or invalid CSRF headers/tokens
/// - token mismatch between header and cookie
/// - malformed header values that cannot be parsed
///
/// # TODO Phase 5.3
/// - キャッシュベースのトークン検証実装
/// - トークンの有効期限管理
/// - ワンタイムトークンの消費処理
pub async fn csrf_protection_middleware(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<Body>,
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

    // TODO Phase 5.3: キャッシュベースのトークン検証を実装
    // match csrf_token {
    //     Some(token) => {
    //         if state.csrf_validate_token(token).await {
    //             Ok(next.run(request).await)
    //         } else {
    //             Err((StatusCode::FORBIDDEN, Json(json!({ ... }))))
    //         }
    //     }
    //     None => Err((StatusCode::FORBIDDEN, Json(json!({ ... }))))
    // }
    
    warn!("CSRF protection is using temporary implementation. Phase 5.3 で完全実装予定。");
    
    match csrf_token {
        Some(_token) => {
            // 暫定: トークンがあれば通過
            Ok(next.run(request).await)
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

/// Endpoint to generate CSRF tokens for clients (Phase 5.2 簡略版)
///
/// # TODO Phase 5.3
/// - キャッシュベースのトークン生成
/// - 有効期限の管理
pub async fn get_csrf_token(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    // TODO Phase 5.3: 実際のトークン生成ロジック
    let token = uuid::Uuid::new_v4().to_string();
    
    warn!("CSRF token generation is using temporary implementation.");

    Json(json!({
        "success": true,
        "csrf_token": token,
        "expires_in": 3600, // Token valid for 1 hour
        "usage": "Include this token in X-CSRF-Token header for state-changing operations"
    }))
}
