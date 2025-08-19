use std::sync::Arc;
use std::time::Instant;
use axum::http::StatusCode;
use crate::{
    config::Config,
    database::Database,
    services::{
        auth::AuthService,
        cache::CacheService,
        elasticsearch::ElasticsearchService,
        media::MediaService,
        notification::NotificationService,
    },
};

// Import health types to return structured health details expected by main.rs
use crate::handlers::health::{ServiceHealthDetails, ServiceStatus};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub database: Database,
    pub auth_service: AuthService,
    pub cache_service: CacheService,
    pub elasticsearch_service: ElasticsearchService,
    pub media_service: MediaService,
    pub notification_service: NotificationService,
    // track when the service started (used by health endpoints)
    pub start_time: Instant,
}

impl AppState {
    /// Backwards-compatible constructor used by main.rs
    pub async fn from_env() -> Result<Self, crate::AppError> {
        Self::new().await
    }

    /// Initialize all services and return AppState
    pub async fn new() -> Result<Self, crate::AppError> {
        let config = Config::from_env()?;

        // Initialize database
        let database = Database::new(&config.database)?;

        // Initialize services
        let auth_service = AuthService::new(&config.auth, &database)?;
        let cache_service = CacheService::new(&config.redis).await?;
        let elasticsearch_service = ElasticsearchService::new(&config.search)?;
        let media_service = MediaService::new(&config.media, &database)?;
        let notification_service = NotificationService::new(&config.notifications, &database)?;

        Ok(Self {
            config,
            database,
            auth_service,
            cache_service,
            elasticsearch_service,
            media_service,
            notification_service,
            start_time: Instant::now(),
        })
    }

    /// Perform a lightweight overall health status aggregation used by the HTTP handlers.
    /// This returns a structure compatible with the health handlers and main.rs checks.
    pub async fn get_health_status(&self) -> ServiceHealthDetails {
        // Default "not_configured" statuses
        let mut database_status = ServiceStatus { status: "not_configured".to_string(), response_time_ms: None, details: None, error: None };
        let mut cache_status = ServiceStatus { status: "not_configured".to_string(), response_time_ms: None, details: None, error: None };
        let mut search_status = ServiceStatus { status: "not_configured".to_string(), response_time_ms: None, details: None, error: None };
        let mut auth_status = ServiceStatus { status: "not_configured".to_string(), response_time_ms: None, details: None, error: None };

        #[cfg(feature = "database")]
        {
            let start = Instant::now();
            match self.database.health_check().await {
                Ok(details) => database_status = ServiceStatus { status: "healthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: Some(details), error: None },
                Err(e) => database_status = ServiceStatus { status: "unhealthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: None, error: Some(e.to_string()) },
            }
        }

        #[cfg(feature = "cache")]
        {
            let start = Instant::now();
            match self.cache_service.health_check().await {
                Ok(details) => cache_status = ServiceStatus { status: "healthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: Some(serde_json::to_value(details).ok()), error: None },
                Err(e) => cache_status = ServiceStatus { status: "unhealthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: None, error: Some(e.to_string()) },
            }
        }

        #[cfg(feature = "search")]
        {
            let start = Instant::now();
            match self.elasticsearch_service.health_check().await {
                Ok(_) => search_status = ServiceStatus { status: "healthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: None, error: None },
                Err(e) => search_status = ServiceStatus { status: "unhealthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: None, error: Some(e.to_string()) },
            }
        }

        #[cfg(feature = "auth")]
        {
            let start = Instant::now();
            match self.auth_service.health_check().await {
                Ok(_) => auth_status = ServiceStatus { status: "healthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: None, error: None },
                Err(e) => auth_status = ServiceStatus { status: "unhealthy".to_string(), response_time_ms: Some(start.elapsed().as_millis()), details: None, error: Some(e.to_string()) },
            }
        }

        ServiceHealthDetails {
            database: database_status,
            cache: cache_status,
            search: search_status,
            auth: auth_status,
        }
    }

    /// Simple placeholder metrics used by the HTTP metrics endpoint.
    pub async fn get_requests_per_second(&self) -> f64 {
        0.0
    }

    /// Simple placeholder average response time in milliseconds.
    pub async fn get_average_response_time(&self) -> f64 {
        0.0
    }

    /// Export metrics for Prometheus or return a placeholder when monitoring is not enabled.
    pub async fn export_metrics(&self) -> (StatusCode, String) {
        (StatusCode::OK, "metrics not available".to_string())
    }

    pub async fn health_check(&self) -> Result<(), crate::AppError> {
        // Check database
        self.database.health_check().await?;

        // Check cache
        self.cache_service.health_check().await?;

        // Check Elasticsearch
        self.elasticsearch_service.health_check().await?;

        Ok(())
    }
}
