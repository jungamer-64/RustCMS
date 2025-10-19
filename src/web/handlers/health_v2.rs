//! Health Check Handler - システム稼働状態確認
//!
//! Kubernetes/負荷分散器用のヘルスチェックエンドポイント

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[cfg(feature = "restructure_domain")]
use crate::AppState;

// ============================================================================
// Response Types
// ============================================================================

/// ヘルスチェックレスポンス
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// 簡易ヘルスチェック（Liveness Probe用）
///
/// GET /health
pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

/// 詳細ヘルスチェック（Readiness Probe用）
///
/// GET /api/v2/health
#[cfg(feature = "restructure_domain")]
pub async fn detailed_health_check(State(state): State<std::sync::Arc<AppState>>) -> impl IntoResponse {
    // Get health status from AppState
    let health = match state.health_check().await {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    status: "unhealthy".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    database: "error".to_string(),
                    cache: None,
                }),
            );
        }
    };

    let cache_status = if cfg!(feature = "cache") {
        Some(health.cache)
    } else {
        None
    };

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: health.database,
        cache: cache_status,
    };

    (StatusCode::OK, Json(response))
}

/// Readiness Probe（Kubernetes用）
///
/// GET /ready
pub async fn readiness_check(State(_state): State<std::sync::Arc<AppState>>) -> StatusCode {
    // TODO: Phase 5+ でサービス統合後に実装
    // 必須コンポーネントの確認
    StatusCode::OK
}

/// Liveness Probe（Kubernetes用）
///
/// GET /live
pub async fn liveness_check() -> StatusCode {
    // プロセスが生存していることを確認
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "3.0.0".to_string(),
            database: "connected".to_string(),
            cache: Some("connected".to_string()),
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"version\":\"3.0.0\""));
    }

    #[test]
    fn test_health_response_without_cache() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "3.0.0".to_string(),
            database: "connected".to_string(),
            cache: None,
        };

        let json = serde_json::to_string(&response).expect("Failed to serialize");
        assert!(!json.contains("\"cache\""));
    }
}
