//! Health Check Handlers
//!
//! Provides system health monitoring and status endpoints

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use utoipa::ToSchema;
use serde_json::json;

// removed unused ok helper after migrating to IntoApiOk
use crate::utils::response_ext::ApiOk;
use crate::{AppState, Result};

/// Overall system health response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub services: ServiceHealthDetails,
    pub system: SystemInfo,
}

/// Service health details
#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceHealthDetails {
    pub database: ServiceStatus,
    pub cache: ServiceStatus,
    pub search: ServiceStatus,
    pub auth: ServiceStatus,
}

/// Individual service status
#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceStatus {
    pub status: String,
    pub response_time_ms: Option<u128>,
    pub details: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// System information
#[derive(Debug, Serialize, ToSchema)]
pub struct SystemInfo {
    pub version: String,
    pub uptime_seconds: u64,
    pub rust_version: String,
    pub build_profile: String,
}

/// Comprehensive health check
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "Health",
    responses(
        (status = 200, description = "System healthy or minimal", body = HealthResponse, examples((
            "Healthy" = (
                summary = "正常例",
                value = json!({
                    "status": "healthy",
                    "timestamp": "2025-09-05T12:00:00Z",
                    "services": {
                        "database": {"status": "healthy"},
                        "cache": {"status": "healthy"},
                        "search": {"status": "healthy"},
                        "auth": {"status": "healthy"}
                    },
                    "system": {"version": "2.0.0", "uptime_seconds": 1234, "rust_version": "stable", "build_profile": "debug"}
                })
            )
        ))),
        (status = 503, description = "System degraded or unhealthy", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // Delegate to centralized AppState health_check for unified logic and metrics
    let h = state.health_check().await?;

    // Map AppState ServiceHealth ("up"/"down"/"degraded"/"not_configured") into handler's shape
    fn map_service(hs: &crate::app::ServiceHealth) -> ServiceStatus {
        let status = match hs.status.as_str() {
            "up" => "healthy",
            "down" => "unhealthy",
            other => other, // "degraded" or "not_configured"
        };
        ServiceStatus {
            status: status.to_string(),
            response_time_ms: Some(hs.response_time_ms as u128),
            details: Some(hs.details.clone()),
            error: hs.error.clone(),
        }
    }

    let services = ServiceHealthDetails {
        database: map_service(&h.database),
        cache: map_service(&h.cache),
        search: map_service(&h.search),
        auth: map_service(&h.auth),
    };

    let response = HealthResponse {
        status: h.status.clone(),
        timestamp: h.timestamp.to_rfc3339(),
        services,
        system: SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: state.start_time.elapsed().as_secs(),
            rust_version: "stable".to_string(),
            build_profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),
        },
    };

    let status_code = match h.status.as_str() {
        "healthy" => StatusCode::OK,
        "degraded" => StatusCode::SERVICE_UNAVAILABLE,
        "unhealthy" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::OK, // fallback
    };

    Ok((status_code, ApiOk(response)))
}

// Per-service checks removed; centralized in AppState::health_check

/// Liveness probe (simple check)
#[utoipa::path(get, path = "/api/v1/health/liveness", tag = "Health", responses((status = 200, description = "Liveness OK", examples((
    "Alive" = (
        summary = "Liveness例",
        value = json!({"status": "alive", "timestamp": "2025-09-05T12:00:00Z"})
    )
)))))]
pub async fn liveness() -> impl IntoResponse {
    ApiOk(json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Readiness probe (check if ready to serve traffic)
#[utoipa::path(get, path = "/api/v1/health/readiness", tag = "Health", responses((status = 200, description = "Readiness status JSON", examples((
    "Ready" = (
        summary = "Readiness例",
        value = json!({"status": "ready", "timestamp": "2025-09-05T12:00:00Z"})
    )
)))))]
pub async fn readiness(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // Delegate to centralized health_check and derive readiness from essential services
    let h = state.health_check().await?;

    // Essential readiness criteria:
    // - Database must be up when feature enabled
    // - Cache is optional: if enabled, consider ready when up; if not enabled, treat as not_configured
    let db_ready = matches!(h.database.status.as_str(), "up" | "not_configured");
    let cache_ready = matches!(h.cache.status.as_str(), "up" | "not_configured");

    let ready = db_ready && cache_ready;

    let status_json = json!({
        "status": if ready { "ready" } else { "not_ready" },
        "timestamp": h.timestamp.to_rfc3339(),
        "database": h.database.status,
        "cache": h.cache.status,
    });

    Ok(ApiOk(status_json))
}
