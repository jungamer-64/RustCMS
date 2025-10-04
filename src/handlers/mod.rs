//! API Handlers - Request processing and business logic
//!
//! Simplified handlers for compilation testing

use crate::openapi::ApiDoc;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::HashSet;
use utoipa::OpenApi;

pub mod admin;
pub mod api_keys;
#[cfg(feature = "auth")]
pub mod auth;
pub mod health;
pub mod metrics;
pub mod posts;
pub mod search;
pub mod users;

// (previously had redundant re-exports here; modules are public via `pub mod` already)

/// Home page handler - integrates functionality from cms-simple
/// Provides a web interface with quick navigation links to all available endpoints
pub async fn home() -> impl IntoResponse {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Rust CMS - Unified Backend</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }
        h2 { color: #495057; margin-top: 30px; }
        .api-link { background: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; display: inline-block; margin: 5px; transition: background 0.3s; }
        .api-link:hover { background: #0056b3; }
        .feature-list { background: #f8f9fa; padding: 15px; border-radius: 5px; margin: 15px 0; }
        .status { background: #d4edda; color: #155724; padding: 10px; border-radius: 4px; margin: 15px 0; }
        .endpoint { background: #e9ecef; padding: 10px; margin: 5px 0; border-left: 4px solid #007bff; border-radius: 4px; }
        .integration-note { background: #fff3cd; color: #856404; padding: 10px; border-radius: 4px; margin: 15px 0; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Rust CMS - Unified Backend</h1>
        
        <div class="status">
            <strong>Status:</strong> ✅ Unified server integrating cms-lightweight + cms-simple + cms-unified functionality
        </div>
        
        <div class="integration-note">
            <strong>🔗 Integration Complete:</strong> This unified server replaces the separate cms-lightweight, cms-simple, and cms-unified binaries with a single, comprehensive solution.
        </div>
        
        <p>High-performance, unified Content Management System built with Rust and Axum.</p>
        
        <h2>🔗 Quick Links</h2>
        <a href="/api/v1/health" class="api-link">Health Check</a>
        <a href="/api/docs" class="api-link">API Documentation</a>
        <a href="/api/v1" class="api-link">API Info</a>
        <a href="/api/v1/metrics" class="api-link">Metrics</a>
        
        <h2>📋 Available Endpoints</h2>
        <div class="endpoint"><strong>GET /</strong> - This home page</div>
        <div class="endpoint"><strong>GET /api/v1</strong> - API information</div>
        <div class="endpoint"><strong>GET /api/docs</strong> - Interactive API documentation</div>
        <div class="endpoint"><strong>GET /api/v1/health/*</strong> - Health check endpoints</div>
        <div class="endpoint"><strong>GET /api/v1/metrics</strong> - Prometheus metrics</div>
        
        <h2>🎯 Integrated Features</h2>
        <div class="feature-list">
            <h3>From cms-lightweight:</h3>
            <ul>
                <li>✅ Minimal startup and configuration</li>
                <li>✅ Lightweight initialization</li>
                <li>✅ Shared AppState management</li>
            </ul>
            
            <h3>From cms-simple:</h3>
            <ul>
                <li>✅ Web interface and home page</li>
                <li>✅ In-memory data store for development</li>
                <li>✅ CORS support</li>
                <li>✅ Comprehensive API documentation</li>
            </ul>
            
            <h3>From cms-unified:</h3>
            <ul>
                <li>✅ Consolidated endpoint structure</li>
                <li>✅ Unified API response format</li>
                <li>✅ Pagination support</li>
            </ul>
            
            <h3>Production Features:</h3>
            <ul>
                <li>🔐 Authentication (when enabled)</li>
                <li>💾 Database support (when enabled)</li>
                <li>🔍 Full-text search (when enabled)</li>
                <li>📊 Metrics and monitoring</li>
                <li>🛡️ Rate limiting</li>
            </ul>
        </div>
        
        <h2>💡 Usage</h2>
        <p>This unified server automatically adapts based on enabled features:</p>
        <ul>
            <li><strong>Development mode:</strong> Run without database features for quick prototyping</li>
            <li><strong>Production mode:</strong> Enable all features for full functionality</li>
        </ul>
    </div>
</body>
</html>
    "#.to_string())
}

/// Returns the core API information response.
fn get_api_info_response() -> impl IntoResponse {
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
        "status": "operational",
        "integration": "unified-cms (cms-lightweight + cms-simple + cms-unified)"
    }))
}

/// API information endpoint (v1 root).
#[utoipa::path(
    get,
    path = "/api/v1",
    responses(
        (status = 200, description = "Get API Information", body = inline(serde_json::Value))
    )
)]
pub async fn api_info_v1() -> impl IntoResponse {
    get_api_info_response()
}

/// API information endpoint (info).
#[utoipa::path(
    get,
    path = "/api/v1/info",
    responses(
        (status = 200, description = "Get API Information", body = inline(serde_json::Value))
    )
)]
pub async fn api_info_info() -> impl IntoResponse {
    get_api_info_response()
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

/// Return generated `OpenAPI` JSON from the compile-time `ApiDoc`
///
/// # Panics
///
/// `ApiDoc` のシリアライズに失敗した場合、内部で `expect` によりパニックします。
pub async fn openapi_json() -> impl IntoResponse {
    // bring trait into scope to call `openapi()`
    let doc = ApiDoc::openapi();
    // serialize to serde_json::Value so axum::Json can return it
    let mut v: JsonValue =
        serde_json::to_value(&doc).expect("failed to convert openapi to json value");

    // 手動で securitySchemes を追加 (macro が security_schemes を未サポートなため暫定措置)
    if let Some(components) = v.get_mut("components").and_then(|c| c.as_object_mut()) {
        use serde_json::json;
        let security_schemes = components
            .entry("securitySchemes")
            .or_insert_with(|| json!({}));
        if let Some(map) = security_schemes.as_object_mut() {
            // Biscuit 認証のみをサポート (Bearer スキームでも内部的には Biscuit トークン)
            map.entry("BiscuitAuth").or_insert(json!({
                "type": "http",
                "scheme": "bearer",
                "bearerFormat": "Biscuit",
                "description": "Biscuit token authentication. Send as: Authorization: Bearer <biscuit-token> or Authorization: Biscuit <biscuit-token>. All authentication mechanisms are unified to use Biscuit tokens internally."
            }));
            // API Key ヘッダ (X-API-Key) 仕様も追加
            // 注: API Key認証も内部的にはBiscuit AuthContextに変換されます
            map.entry("ApiKeyHeader").or_insert(json!({
                "type": "apiKey",
                "name": "X-API-Key",
                "in": "header",
                "description": "API key authentication header. API keys are internally converted to Biscuit-based authentication contexts for unified security processing."
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
    ]
    .into_iter()
    .collect();

    // 認証必須エンドポイントに Biscuit 認証を適用: public 以外の /api/v1/* で、logout/profile/reindex/users/posts など。
    if let Some(paths) = v.get_mut("paths").and_then(|p| p.as_object_mut()) {
        for (path, item) in paths.iter_mut() {
            let Some(item_obj) = item.as_object_mut() else {
                continue;
            }; // path item object
            for method in ["get", "post", "put", "delete", "patch"] {
                if let Some(op) = item_obj.get_mut(method).and_then(|o| o.as_object_mut()) {
                    let key = (path.as_str(), method);
                    if public.contains(&key) {
                        // 明示的に security を削除 (公開)
                        op.remove("security");
                        continue;
                    }
                    // /api/v1/ で始まるもののみ対象 (他はスキップ)
                    if !path.starts_with("/api/v1/") {
                        continue;
                    }
                    // 全ての保護エンドポイントで Biscuit 認証を使用
                    op.insert(
                        "security".to_string(),
                        serde_json::json!([
                            {"BiscuitAuth": []}
                        ]),
                    );
                }
            }
        }
    }
    Json(v)
}
