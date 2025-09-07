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
pub mod metrics;
pub mod posts;
pub mod search;
pub mod users;
pub mod api_keys;

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
    let mut v: JsonValue = serde_json::to_value(&doc).expect("failed to convert openapi to json value");

    // 手動で securitySchemes を追加 (macro が security_schemes を未サポートなため暫定措置)
    if let Some(components) = v.get_mut("components").and_then(|c| c.as_object_mut()) {
        use serde_json::json;
        let security_schemes = components.entry("securitySchemes").or_insert_with(|| json!({}));
        if let Some(map) = security_schemes.as_object_mut() {
            map.entry("BearerAuth").or_insert(json!({
                "type": "http",
                "scheme": "bearer",
                "bearerFormat": "Biscuit"
            }));
            map.entry("BiscuitAuth").or_insert(json!({
                "type": "apiKey",
                "name": "Authorization",
                "in": "header",
                "description": "Send as: Authorization: Biscuit <base64-token>"
            }));
            // API Key ヘッダ (X-API-Key) 仕様も追加 (将来公開予定)
            map.entry("ApiKeyHeader").or_insert(json!({
                "type": "apiKey",
                "name": "X-API-Key",
                "in": "header",
                "description": "API key authentication header"
            }));
        }

        // API Key permission リストを拡張メタで提供 (フロントや CLI が参照できる)
        let perms = crate::models::api_key::ApiKey::ALLOWED_PERMISSIONS;
        components.insert("x-apiKey-permissions".to_string(), json!(perms));
    }
    // ルートのグローバル security は使わず削除 (個別操作に OR 条件を付与)
    if let Some(obj) = v.as_object_mut() {
        obj.remove("security");
    }

    // 公開（認証不要）エンドポイント一覧 (path, method)
    use std::collections::HashSet;
    let public: HashSet<(&'static str, &'static str)> = [
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
    ].into_iter().collect();

    // 認証必須 (Bearer または Biscuit 任意) とする操作: public 以外の /api/v1/* で、logout/profile/reindex/users/posts など。
    if let Some(paths) = v.get_mut("paths").and_then(|p| p.as_object_mut()) {
        // Biscuit のみ許可したいエンドポイント (method, path)
        let biscuit_only: std::collections::HashSet<(&'static str, &'static str)> = [
            ("post", "/api/v1/search/reindex"),
        ].into_iter().collect();
        for (path, item) in paths.iter_mut() {
            let Some(item_obj) = item.as_object_mut() else { continue }; // path item object
            for method in ["get","post","put","delete","patch"] {
                if let Some(op) = item_obj.get_mut(method).and_then(|o| o.as_object_mut()) {
                    let key = (path.as_str(), method);
                    if public.contains(&key) {
                        // 明示的に security を削除 (公開)
                        op.remove("security");
                        continue;
                    }
                    // /api/v1/ で始まるもののみ対象 (他はスキップ)
                    if !path.starts_with("/api/v1/") { continue; }
                    if biscuit_only.contains(&(method, path.as_str())) {
                        // Biscuit のみ
                        op.insert("security".to_string(), serde_json::json!([
                            {"BiscuitAuth": []}
                        ]));
                    } else {
                        // OR 条件 (Bearer または Biscuit)
                        op.insert("security".to_string(), serde_json::json!([
                            {"BearerAuth": []},
                            {"BiscuitAuth": []}
                        ]));
                    }
                }
            }
        }
    }
    Json(v)
}
