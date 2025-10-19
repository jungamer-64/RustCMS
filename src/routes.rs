//! Routes - HTTP ルーター定義
//!
//! アプリケーション全体のルーティング構成を提供します。
//! API v1 エンドポイントとヘルスチェック、ドキュメントなどを統合。

use axum::{
    Router,
    routing::get,
};

#[cfg(feature = "restructure_domain")]
use crate::infrastructure::app_state::AppState;

/// メインルーターを作成
///
/// 統合されたCMSサーバーのすべてのエンドポイントを設定します。
/// - ルートページ (/)
/// - API v1 エンドポイント (/api/v1/*)
/// - ヘルスチェック (/api/v1/health)
/// - ドキュメント (/api/docs)
#[cfg(feature = "restructure_domain")]
pub fn create_router() -> Router<std::sync::Arc<AppState>> {
    use crate::web::handlers::{api_info_v1, api_info_info, home, not_found};
    use crate::web::handlers::health_v2::{health_check, detailed_health_check};
    use crate::web::handlers::auth_v2::refresh_token;
    use axum::routing::post;
    use std::sync::Arc;
    
    // ベースルーター (with_state will convert to Router<Arc<AppState>>)
    let router: Router<Arc<AppState>> = Router::new()
        .route("/", get(home))
        .route("/health", get(health_check));

    // API v2 Auth ルート (Phase 5.4.5)
    let api_v2_auth: Router<Arc<AppState>> = Router::new()
        .route("/refresh", post(refresh_token));

    // API v1 ルート
    let api_v1: Router<Arc<AppState>> = Router::new()
        .route("/", get(api_info_v1))
        .route("/info", get(api_info_info))
        .route("/health", get(detailed_health_check))
        .route("/health/liveness", get(health_check))
        .route("/health/readiness", get(detailed_health_check));

    // ルーターを統合
    router
        .nest("/api/v1", api_v1)
        .nest("/api/v2/auth", api_v2_auth)
        .fallback(not_found)
}

/// メインルーターを作成 (restructure_domain なし)
///
/// 最小限のヘルスチェックエンドポイントのみを提供します。
#[cfg(not(feature = "restructure_domain"))]
pub fn create_router() -> Router {
    use axum::Json;
    use serde_json::json;
    
    let health_handler = || async {
        Json(json!({
            "status": "healthy",
            "message": "CMS Backend is running",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    };
    
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/health", get(health_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        // ルーターが作成できることを確認
        let _router = create_router();
    }
}
