//! Application State and Service Management
//! 
//! Centralized application state containing all services for the CMS:
//! - Database connections with pooling
//! - Authentication service with biscuit-auth + WebAuthn
//! - Cache service with Redis + in-memory layers
//! - Search service with Tantivy full-text search
//! - Health monitoring and metrics collection

use crate::{
    config::Config,
    Result,
};

#[cfg(feature = "auth")]
use crate::auth::AuthService;
#[cfg(feature = "cache")]
use crate::cache::CacheService;
#[cfg(feature = "search")]
use crate::search::SearchService;
#[cfg(feature = "database")]
use crate::database::Database;
use std::{sync::Arc, time::Instant};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// Central application state containing all services
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool and operations
    #[cfg(feature = "database")]
    pub database: Database,
    
    /// Authentication and authorization service
    #[cfg(feature = "auth")]
    pub auth: AuthService,
    
    /// Multi-tier caching service
    #[cfg(feature = "cache")]
    pub cache: CacheService,
    
    /// Full-text search service
    #[cfg(feature = "search")]
    pub search: SearchService,
    
    /// Application configuration
    pub config: Arc<Config>,
    
    /// Application metrics
    pub metrics: Arc<RwLock<AppMetrics>>,
    
    /// Application start time for uptime calculations
    pub start_time: Instant,
}

/// Application metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    
    /// Number of active connections
    pub active_connections: u64,
    
    /// Cache hit/miss statistics
    pub cache_hits: u64,
    pub cache_misses: u64,
    
    /// Search query statistics
    pub search_queries: u64,
    pub search_avg_response_time_ms: f64,
    
    /// Authentication statistics
    pub auth_attempts: u64,
    pub auth_successes: u64,
    pub auth_failures: u64,
    
    /// Database operation statistics
    pub db_queries: u64,
    pub db_avg_response_time_ms: f64,
    
    /// Error counts by type
    pub errors_total: u64,
    pub errors_auth: u64,
    pub errors_db: u64,
    pub errors_cache: u64,
    pub errors_search: u64,
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            active_connections: 0,
            cache_hits: 0,
            cache_misses: 0,
            search_queries: 0,
            search_avg_response_time_ms: 0.0,
            auth_attempts: 0,
            auth_successes: 0,
            auth_failures: 0,
            db_queries: 0,
            db_avg_response_time_ms: 0.0,
            errors_total: 0,
            errors_auth: 0,
            errors_db: 0,
            errors_cache: 0,
            errors_search: 0,
        }
    }
}

/// Builder pattern for AppState to handle conditional compilation
pub struct AppStateBuilder {
    pub config: Arc<Config>,
    pub metrics: Arc<RwLock<AppMetrics>>,
    pub start_time: Instant,
    
    #[cfg(feature = "database")]
    pub database: Option<Database>,
    #[cfg(feature = "auth")]
    pub auth: Option<AuthService>,
    #[cfg(feature = "cache")]
    pub cache: Option<CacheService>,
    #[cfg(feature = "search")]
    pub search: Option<SearchService>,
}

impl AppStateBuilder {
    pub fn build(self) -> AppState {
        AppState {
            #[cfg(feature = "database")]
            database: self.database.unwrap_or_else(|| panic!("Database service not initialized but feature enabled")),
            #[cfg(feature = "auth")]
            auth: self.auth.unwrap_or_else(|| panic!("Auth service not initialized but feature enabled")),
            #[cfg(feature = "cache")]
            cache: self.cache.unwrap_or_else(|| panic!("Cache service not initialized but feature enabled")),
            #[cfg(feature = "search")]
            search: self.search.unwrap_or_else(|| panic!("Search service not initialized but feature enabled")),
            config: self.config,
            metrics: self.metrics,
            start_time: self.start_time,
        }
    }
}

/// Health status for individual services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall status: "healthy", "degraded", "unhealthy"
    pub status: String,
    
    /// Database health
    pub database: ServiceHealth,
    
    /// Cache service health
    pub cache: ServiceHealth,
    
    /// Search service health
    pub search: ServiceHealth,
    
    /// Authentication service health
    pub auth: ServiceHealth,
    
    /// Timestamp of health check
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Individual service health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service status: "up", "down", "degraded"
    pub status: String,
    
    /// Response time in milliseconds
    pub response_time_ms: f64,
    
    /// Optional error message
    pub error: Option<String>,
    
    /// Service-specific details
    pub details: serde_json::Value,
}

impl AppState {
    /// Create application state from environment configuration
    pub async fn from_env() -> Result<Self> {
        info!("🔧 Initializing application state from environment");
        
        // Load configuration
        let config = Arc::new(Config::from_env()?);
        debug!("✅ Configuration loaded");
        
        let mut app_state_builder = AppStateBuilder {
            config: config.clone(),
            metrics: Arc::new(RwLock::new(AppMetrics::default())),
            start_time: Instant::now(),
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "auth")]
            auth: None,
            #[cfg(feature = "cache")]
            cache: None,
            #[cfg(feature = "search")]
            search: None,
        };
        
        // Initialize services conditionally based on features
        #[cfg(feature = "database")]
        {
            info!("🗄️ Connecting to PostgreSQL database...");
            app_state_builder.database = Some(Database::new(&config.database).await?);
            info!("✅ Database connection established");
        }
        
        #[cfg(feature = "auth")]
        {
            info!("🔐 Setting up authentication service...");
            #[cfg(feature = "database")]
            {
                app_state_builder.auth = Some(AuthService::new(&config.auth, app_state_builder.database.as_ref().unwrap()).await?);
            }
            #[cfg(not(feature = "database"))]
            {
                app_state_builder.auth = Some(AuthService::new_standalone(&config.auth).await?);
            }
            info!("✅ Authentication service initialized");
        }
        
        #[cfg(feature = "cache")]
        {
            info!("🚀 Setting up cache service...");
            app_state_builder.cache = Some(CacheService::new(&config.redis).await?);
            info!("✅ Cache service initialized");
        }
        
        #[cfg(feature = "search")]
        {
            info!("🔍 Setting up search service...");
            app_state_builder.search = Some(SearchService::new(config.search.clone()).await?);
            info!("✅ Search service initialized");
        }
        
        let app_state = app_state_builder.build();
        
        
        info!("🎉 Application state initialized successfully");
        Ok(app_state)
    }
    
    /// Perform comprehensive health check of all services
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let start_time = Instant::now();
        
        // Check database health
        let db_health = self.check_database_health().await;
        
        // Check cache health
        let cache_health = self.check_cache_health().await;
        
        // Check search health
        let search_health = self.check_search_health().await;
        
        // Check auth health
        let auth_health = self.check_auth_health().await;
        
        // Determine overall status
        let overall_status = if [&db_health, &cache_health, &search_health, &auth_health]
            .iter()
            .all(|h| h.status == "up")
        {
            "healthy"
        } else if [&db_health, &cache_health, &search_health, &auth_health]
            .iter()
            .any(|h| h.status == "down")
        {
            "unhealthy"
        } else {
            "degraded"
        };
        
        let health_status = HealthStatus {
            status: overall_status.to_string(),
            database: db_health,
            cache: cache_health,
            search: search_health,
            auth: auth_health,
            timestamp: chrono::Utc::now(),
        };
        
        let check_duration = start_time.elapsed();
        debug!("Health check completed in {:.2}ms", check_duration.as_millis());
        
        Ok(health_status)
    }
    
    /// Check database service health
    #[cfg(feature = "database")]
    async fn check_database_health(&self) -> ServiceHealth {
        let start_time = Instant::now();
        
        match self.database.health_check().await {
            Ok(_) => ServiceHealth {
                status: "up".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as f64,
                error: None,
                details: serde_json::json!({}),
            },
            Err(e) => {
                error!("Database health check failed: {}", e);
                ServiceHealth {
                    status: "down".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as f64,
                    error: Some(e.to_string()),
                    details: serde_json::json!({}),
                }
            }
        }
    }
    
    #[cfg(not(feature = "database"))]
    async fn check_database_health(&self) -> ServiceHealth {
        ServiceHealth {
            status: "not_configured".to_string(),
            response_time_ms: 0.0,
            error: None,
            details: serde_json::json!({"message": "Database feature not enabled"}),
        }
    }
    
    /// Check cache service health
    #[cfg(feature = "cache")]
    async fn check_cache_health(&self) -> ServiceHealth {
        let start_time = Instant::now();
        
        match self.cache.health_check().await {
            Ok(_) => ServiceHealth {
                status: "up".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as f64,
                error: None,
                details: serde_json::json!({}),
            },
            Err(e) => {
                warn!("Cache health check failed: {}", e);
                ServiceHealth {
                    status: "degraded".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as f64,
                    error: Some(e.to_string()),
                    details: serde_json::json!({}),
                }
            }
        }
    }
    
    #[cfg(not(feature = "cache"))]
    async fn check_cache_health(&self) -> ServiceHealth {
        ServiceHealth {
            status: "not_configured".to_string(),
            response_time_ms: 0.0,
            error: None,
            details: serde_json::json!({"message": "Cache feature not enabled"}),
        }
    }
    
    /// Check search service health
    #[cfg(feature = "search")]
    async fn check_search_health(&self) -> ServiceHealth {
        let start_time = Instant::now();
        
        match self.search.health_check().await {
            Ok(_) => ServiceHealth {
                status: "up".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as f64,
                error: None,
                details: serde_json::json!({}),
            },
            Err(e) => {
                warn!("Search health check failed: {}", e);
                ServiceHealth {
                    status: "degraded".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as f64,
                    error: Some(e.to_string()),
                    details: serde_json::json!({}),
                }
            }
        }
    }
    
    #[cfg(not(feature = "search"))]
    async fn check_search_health(&self) -> ServiceHealth {
        ServiceHealth {
            status: "not_configured".to_string(),
            response_time_ms: 0.0,
            error: None,
            details: serde_json::json!({"message": "Search feature not enabled"}),
        }
    }
    
    /// Check authentication service health
    #[cfg(feature = "auth")]
    async fn check_auth_health(&self) -> ServiceHealth {
        let start_time = Instant::now();
        
        match self.auth.health_check().await {
            Ok(_) => ServiceHealth {
                status: "up".to_string(),
                response_time_ms: start_time.elapsed().as_millis() as f64,
                error: None,
                details: serde_json::json!({}),
            },
            Err(e) => {
                error!("Auth health check failed: {}", e);
                ServiceHealth {
                    status: "down".to_string(),
                    response_time_ms: start_time.elapsed().as_millis() as f64,
                    error: Some(e.to_string()),
                    details: serde_json::json!({}),
                }
            }
        }
    }
    
    #[cfg(not(feature = "auth"))]
    async fn check_auth_health(&self) -> ServiceHealth {
        ServiceHealth {
            status: "not_configured".to_string(),
            response_time_ms: 0.0,
            error: None,
            details: serde_json::json!({"message": "Auth feature not enabled"}),
        }
    }
    
    /// Get current application metrics
    pub async fn get_metrics(&self) -> AppMetrics {
        let metrics = self.metrics.read().await;
        let mut current_metrics = metrics.clone();
        
        // Add real-time computed metrics if cache is available
        #[cfg(feature = "cache")]
        {
            let cache_stats = self.cache.get_stats().await;
            current_metrics.cache_hits = cache_stats.redis_hits + cache_stats.memory_hits;
            current_metrics.cache_misses = cache_stats.redis_misses + cache_stats.memory_misses;
        }
        
        current_metrics
    }
    
    /// Update request metrics
    pub async fn record_request(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
    }
    
    /// Update authentication metrics
    pub async fn record_auth_attempt(&self, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.auth_attempts += 1;
        if success {
            metrics.auth_successes += 1;
        } else {
            metrics.auth_failures += 1;
        }
    }
    
    /// Update search metrics
    pub async fn record_search_query(&self, response_time_ms: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.search_queries += 1;
        
        // Update rolling average
        let total_queries = metrics.search_queries as f64;
        metrics.search_avg_response_time_ms = 
            (metrics.search_avg_response_time_ms * (total_queries - 1.0) + response_time_ms) / total_queries;
    }
    
    /// Update database metrics
    pub async fn record_db_query(&self, response_time_ms: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.db_queries += 1;
        
        // Update rolling average
        let total_queries = metrics.db_queries as f64;
        metrics.db_avg_response_time_ms = 
            (metrics.db_avg_response_time_ms * (total_queries - 1.0) + response_time_ms) / total_queries;
    }
    
    /// Record error by type
    pub async fn record_error(&self, error_type: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.errors_total += 1;
        
        match error_type {
            "auth" => metrics.errors_auth += 1,
            "db" => metrics.errors_db += 1,
            "cache" => metrics.errors_cache += 1,
            "search" => metrics.errors_search += 1,
            _ => {}
        }
    }
    
    /// Get application uptime in seconds
    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Application environment setup
pub struct AppEnvironment;

impl AppEnvironment {
    /// Initialize logging and environment
    pub fn init() {
        // Initialize environment from .env file if available
        if let Err(_) = dotenvy::dotenv() {
            // .env file not found, continue with system environment
        }
        
        // Initialize structured logging
        let env_filter = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info,cms_backend=debug,sqlx=warn".to_string());
            
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .init();
            
        info!("🚀 Application environment initialized");
        info!("📊 Logging level: {}", std::env::var("RUST_LOG").unwrap_or_else(|_| "default".to_string()));
    }
}
