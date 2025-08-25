//! Health Check Handlers
//!
//! Provides system health monitoring and status endpoints

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Serialize;
use serde_json::json;

use crate::utils::api_types::ApiResponse;
use crate::{AppState, Result};

/// Overall system health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub services: ServiceHealthDetails,
    pub system: SystemInfo,
}

/// Service health details
#[derive(Debug, Serialize)]
pub struct ServiceHealthDetails {
    pub database: ServiceStatus,
    pub cache: ServiceStatus,
    pub search: ServiceStatus,
    pub auth: ServiceStatus,
}

/// Individual service status
#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub status: String,
    pub response_time_ms: Option<u128>,
    pub details: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// System information
#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub uptime_seconds: u64,
    pub rust_version: String,
    pub build_profile: String,
}

/// Comprehensive health check
pub async fn health_check(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let _start_time = std::time::Instant::now();

    // Check all services conditionally
    let mut services = ServiceHealthDetails {
        database: ServiceStatus {
            status: "not_configured".to_string(),
            response_time_ms: None,
            details: None,
            error: None,
        },
        cache: ServiceStatus {
            status: "not_configured".to_string(),
            response_time_ms: None,
            details: None,
            error: None,
        },
        search: ServiceStatus {
            status: "not_configured".to_string(),
            response_time_ms: None,
            details: None,
            error: None,
        },
        auth: ServiceStatus {
            status: "not_configured".to_string(),
            response_time_ms: None,
            details: None,
            error: None,
        },
    };

    let mut healthy_services = 0;
    let mut total_services = 0;

    #[cfg(feature = "database")]
    {
        services.database = check_database_health(&state).await;
        total_services += 1;
        if services.database.status == "healthy" {
            healthy_services += 1;
        }
    }

    #[cfg(feature = "cache")]
    {
        services.cache = check_cache_health(&state).await;
        total_services += 1;
        if services.cache.status == "healthy" {
            healthy_services += 1;
        }
    }
    #[cfg(not(feature = "cache"))]
    {
        // Treat missing cache as healthy for reduced-feature builds
        services.cache.status = "not_configured".to_string();
    }

    #[cfg(feature = "search")]
    {
        services.search = check_search_health(&state).await;
        total_services += 1;
        if services.search.status == "healthy" {
            healthy_services += 1;
        }
    }
    #[cfg(not(feature = "search"))]
    {
        // Treat missing search as not configured for reduced-feature builds
        services.search.status = "not_configured".to_string();
    }

    #[cfg(feature = "auth")]
    {
        // Use AppState wrapper to perform auth health check and capture timing/metrics
        match state.auth_health_check().await {
            Ok(h) => {
                services.auth = ServiceStatus {
                    status: h.status,
                    response_time_ms: Some(h.response_time_ms as u128),
                    details: Some(h.details),
                    error: h.error,
                };
                total_services += 1;
                if services.auth.status == "healthy" {
                    healthy_services += 1;
                }
            }
            Err(e) => {
                services.auth = ServiceStatus {
                    status: "unhealthy".to_string(),
                    response_time_ms: None,
                    details: None,
                    error: Some(e.to_string()),
                };
                total_services += 1;
            }
        }
    }

    // Determine overall status
    let overall_status = if total_services == 0 {
        "minimal" // No external services configured
    } else if healthy_services == total_services {
        "healthy"
    } else if healthy_services > 0 {
        "degraded"
    } else {
        "unhealthy"
    };

    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        services,
        system: SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: state.start_time.elapsed().as_secs(),
            rust_version: "stable".to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
        },
    };

    let status_code = if overall_status == "healthy" || overall_status == "minimal" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((status_code, Json(ApiResponse::success(response))))
}

/// Database health check
#[cfg(feature = "database")]
async fn check_database_health(state: &AppState) -> ServiceStatus {
    let start = std::time::Instant::now();

    match state.database.health_check().await {
        Ok(details) => ServiceStatus {
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis()),
            details: Some(details),
            error: None,
        },
        Err(e) => ServiceStatus {
            status: "unhealthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis()),
            details: None,
            error: Some(e.to_string()),
        },
    }
}

/// Cache health check
#[cfg(feature = "cache")]
async fn check_cache_health(state: &AppState) -> ServiceStatus {
    let start = std::time::Instant::now();

    match state.cache.health_check().await {
        Ok(details) => ServiceStatus {
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis()),
            details: Some(json!(details)),
            error: None,
        },
        Err(e) => ServiceStatus {
            status: "unhealthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis()),
            details: None,
            error: Some(e.to_string()),
        },
    }
}

/// Search health check
#[cfg(feature = "search")]
async fn check_search_health(state: &AppState) -> ServiceStatus {
    let start = std::time::Instant::now();

    match state.search.health_check().await {
        Ok(_) => ServiceStatus {
            status: "healthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis()),
            details: None,
            error: None,
        },
        Err(e) => ServiceStatus {
            status: "unhealthy".to_string(),
            response_time_ms: Some(start.elapsed().as_millis()),
            details: None,
            error: Some(e.to_string()),
        },
    }
}

// Auth health is checked via AppState wrapper `auth_health_check` now.

/// Liveness probe (simple check)
pub async fn liveness() -> impl IntoResponse {
    Json(ApiResponse::success(json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Readiness probe (check if ready to serve traffic)
pub async fn readiness(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // Quick checks to see if essential services are ready
    let mut ready = true;
    let mut status_json = json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    #[cfg(feature = "database")]
    {
        let database_ready = state.database.health_check().await.is_ok();
        status_json["database"] = json!(database_ready);
        if !database_ready {
            ready = false;
        }
    }

    #[cfg(feature = "cache")]
    {
        let cache_ready = state.cache.health_check().await.is_ok();
        status_json["cache"] = json!(cache_ready);
        if !cache_ready {
            ready = false;
        }
    }
    #[cfg(not(feature = "cache"))]
    {
        status_json["cache"] = json!("not_configured");
    }

    if !ready {
        status_json["status"] = json!("not_ready");
    }

    Ok(Json(ApiResponse::success(status_json)))
}
