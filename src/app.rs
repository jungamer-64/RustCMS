//! Application State and Service Management
//!
//! Centralized application state containing all services for the CMS:
//! - Database connections with pooling
//! - Authentication service with biscuit-auth + WebAuthn
//! - Cache service with Redis + in-memory layers
//! - Search service with Tantivy full-text search
//! - Health monitoring and metrics collection

use crate::{config::Config, Result};
use crate::limiter::FixedWindowLimiter;

#[cfg(feature = "auth")]
use crate::auth::AuthService;
#[cfg(feature = "cache")]
use crate::cache::CacheService;
#[cfg(feature = "database")]
use crate::database::Database;
#[cfg(feature = "search")]
use crate::search::SearchService;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::{sync::Arc, time::Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
#[cfg(feature = "cache")]
use std::time::Duration;

// --- Generic instrumentation macro ---
// 単一マクロで開始→await→経過時間→成功時処理 を汎用化。
// 呼び出し側で metric 種別を &str で指定し、対応する更新ロジックに分岐。
macro_rules! timed_op {
    ($self:ident, $kind:expr, $future:expr) => {{
        let start = std::time::Instant::now();
        let res = $future.await;
        let elapsed = start.elapsed().as_millis() as f64;
        if res.is_ok() {
            match $kind {
                #[cfg(feature = "database")] "db" => { $self.record_db_query(elapsed).await; },
                #[cfg(feature = "search")] "search" => { $self.record_search_query(elapsed).await; },
                #[cfg(feature = "auth")] "auth" => {
                    // auth は同時に auth 試行 & DB クエリ計測（ユーザー/セッション更新含む）
                    $self.record_auth_attempt(true).await;
                    #[cfg(feature = "database")] { $self.record_db_query(elapsed).await; }
                },
                _ => {},
            }
        } else {
            #[cfg(feature = "auth")]
            if $kind == "auth" { $self.record_auth_attempt(false).await; }
        }
        res
    }};
}

// Generic health check macro (module scope)
macro_rules! define_health_check {
    ($fn_name:ident, $feature:literal, $field:ident, $failure_mode:expr, $missing:expr) => {
        #[cfg(feature = $feature)]
        async fn $fn_name(&self) -> ServiceHealth {
            let h = to_service_health(self.$field.health_check(), $failure_mode).await;
            match h.status.as_str() {
                "down" => error!(service = $feature, error = ?h.error, "health check failed"),
                "degraded" => warn!(service = $feature, error = ?h.error, "health check degraded"),
                _ => {}
            }
            h
        }
        #[cfg(not(feature = $feature))]
        async fn $fn_name(&self) -> ServiceHealth { service_not_configured($missing) }
    };
}

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

    /// Unified IP rate limiter (fixed window strategy)
    pub rate_limiter: Arc<FixedWindowLimiter>,

    /// Application start time for uptime calculations
    pub start_time: Instant,
}


/// Application metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

    /// Cache invalidation counters
    pub cache_invalidations: u64,
    pub cache_invalidation_errors: u64,

    /// Error counts by type
    pub errors_total: u64,
    pub errors_auth: u64,
    pub errors_db: u64,
    pub errors_cache: u64,
    pub errors_search: u64,

    /// Active auth sessions (computed periodically)
    pub active_sessions: u64,
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
            cache_invalidations: 0,
            cache_invalidation_errors: 0,
            errors_total: 0,
            errors_auth: 0,
            errors_db: 0,
            errors_cache: 0,
            errors_search: 0,
            active_sessions: 0,
        }
    }
}

// ----------------- Shared Instrumentation Helpers -----------------
#[derive(Copy, Clone)]
enum FailureMode { Down, Degraded }

#[inline]
async fn to_service_health<F, T>(fut: F, failure: FailureMode) -> ServiceHealth
where
    F: std::future::Future<Output = crate::Result<T>>,
{
    let start = std::time::Instant::now();
    match fut.await {
        Ok(_) => ServiceHealth {
            status: "up".to_string(),
            response_time_ms: start.elapsed().as_millis() as f64,
            error: None,
            details: serde_json::json!({}),
        },
    Err(e) => {
            let status = match failure { FailureMode::Down => "down", FailureMode::Degraded => "degraded" };
            ServiceHealth {
                status: status.to_string(),
                response_time_ms: start.elapsed().as_millis() as f64,
        error: Some(format!("{:?}", e)),
                details: serde_json::json!({}),
            }
        }
    }
}

#[inline]
#[allow(dead_code)]
fn service_not_configured(msg: &str) -> ServiceHealth {
    ServiceHealth { status: "not_configured".into(), response_time_ms: 0.0, error: None, details: serde_json::json!({"message": msg}) }
}

// (旧) update_running_avg は各 record_* 内へインライン化済み

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
            database: self
                .database
                .unwrap_or_else(|| panic!("Database service not initialized but feature enabled")),
            #[cfg(feature = "auth")]
            auth: self
                .auth
                .unwrap_or_else(|| panic!("Auth service not initialized but feature enabled")),
            #[cfg(feature = "cache")]
            cache: self
                .cache
                .unwrap_or_else(|| panic!("Cache service not initialized but feature enabled")),
            #[cfg(feature = "search")]
            search: self
                .search
                .unwrap_or_else(|| panic!("Search service not initialized but feature enabled")),
            config: self.config,
            metrics: self.metrics,
            rate_limiter: Arc::new(FixedWindowLimiter::new(100, 60)), // default; real values set in from_config
            start_time: self.start_time,
        }
    }
}

/// Health status for individual services
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
        // Load configuration and delegate to from_config
        let config = Config::from_env()?;
        Self::from_config(config).await
    }

    /// Create application state from a provided `Config` (useful for central init)
    pub async fn from_config(config: Config) -> Result<Self> {
        info!("🔧 Initializing application state from provided Config");

        let config = Arc::new(config);
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
                app_state_builder.auth = Some(
                    AuthService::new(&config.auth, app_state_builder.database.as_ref().unwrap())
                        .await?,
                );
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

        let mut app_state = app_state_builder.build();
        // Override default limiter with configured values
        app_state.rate_limiter = Arc::new(FixedWindowLimiter::new(
            config.security.rate_limit_requests as u32,
            config.security.rate_limit_window,
        ));

        // --- Background maintenance tasks ---
        #[cfg(feature = "auth")]
        {
            // Clone for task
            let state_clone = app_state.clone();
            // Cleanup interval (seconds) via env or default 300
            let interval_secs: u64 = std::env::var("AUTH_SESSION_CLEAN_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300);
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                loop {
                    ticker.tick().await;
                    // Clean expired
                    state_clone.auth.cleanup_expired_sessions().await;
                    // Update metric snapshot
                    let active = state_clone.auth.get_active_session_count().await as u64;
                    if let Ok(mut m) = state_clone.metrics.try_write() { // try_write to avoid blocking
                        m.active_sessions = active;
                    }
                }
            });
        }

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
        debug!(
            "Health check completed in {:.2}ms",
            check_duration.as_millis()
        );

        Ok(health_status)
    }

    define_health_check!(check_database_health, "database", database, FailureMode::Down, "Database feature not enabled");
    define_health_check!(check_cache_health, "cache", cache, FailureMode::Degraded, "Cache feature not enabled");
    define_health_check!(check_search_health, "search", search, FailureMode::Degraded, "Search feature not enabled");
    define_health_check!(check_auth_health, "auth", auth, FailureMode::Down, "Auth feature not enabled");

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
    let mut m = self.metrics.write().await;
    m.search_queries += 1;
    let n = m.search_queries as f64;
    m.search_avg_response_time_ms = (m.search_avg_response_time_ms * (n - 1.0) + response_time_ms) / n;
    }

    /// Update database metrics
    pub async fn record_db_query(&self, response_time_ms: f64) {
    let mut m = self.metrics.write().await;
    m.db_queries += 1;
    let n = m.db_queries as f64;
    m.db_avg_response_time_ms = (m.db_avg_response_time_ms * (n - 1.0) + response_time_ms) / n;
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

    /// Rate limit helper for IP addresses. Returns true if request allowed.
    pub fn allow_ip(&self, ip: &std::net::IpAddr) -> bool {
        self.rate_limiter.allow(&ip.to_string())
    }

    /// Convenience helper to get a pooled DB connection from AppState
    #[cfg(feature = "database")]
    pub fn get_conn(&self) -> crate::Result<crate::database::PooledConnection> {
        self.database.get_connection()
    }

    // ---------------- Cache helper (get or compute & store) ----------------
    #[cfg(feature = "cache")]
    pub async fn cache_get_or_set<T, F, Fut>(&self, key: &str, ttl: Duration, builder: F) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::Result<T>>,
    {
        if let Ok(Some(v)) = self.cache.get::<T>(key).await { return Ok(v); }
        let value = builder().await?;
        // キャッシュ失敗は致命ではないので黙殺（ログのみ）
        if let Err(e) = self.cache.set(key.to_string(), &value, Some(ttl)).await {
            warn!("cache set failed key={} err={}", key, e);
        }
        Ok(value)
    }

    /// Prefix invalidation helper (supports patterns like "posts:*").
    /// Wildcard削除はサービス側の delete がパターン処理をサポートしている前提。
    #[cfg(feature = "cache")]
    pub async fn cache_invalidate_prefix(&self, prefix: &str) {
        let mut metrics = self.metrics.write().await;
        match self.cache.delete(prefix).await {
            Ok(_) => { metrics.cache_invalidations += 1; },
            Err(e) => { metrics.cache_invalidation_errors += 1; warn!("cache invalidate failed prefix={} err={}", prefix, e); }
        }
    }

    // ---------------- Entity specific cache helpers (to reduce handler duplication) ----------------
    #[cfg(feature = "cache")]
    pub async fn invalidate_post_caches(&self, id: uuid::Uuid) {
    use crate::utils::cache_key::{CACHE_PREFIX_POST_ID, CACHE_PREFIX_POSTS};
    let key = format!("{}{}", CACHE_PREFIX_POST_ID, id);
        let mut metrics = self.metrics.write().await;
        match self.cache.delete(&key).await {
            Ok(_) => { metrics.cache_invalidations += 1; },
            Err(e) => { metrics.cache_invalidation_errors += 1; warn!(post_id=%id, error=%e, "post cache delete failed"); }
        }
        drop(metrics);
        self.cache_invalidate_prefix(&format!("{}*", CACHE_PREFIX_POSTS)).await; // prefix helper already logs
    }

    #[cfg(feature = "cache")]
    pub async fn invalidate_user_caches(&self, id: uuid::Uuid) {
    use crate::utils::cache_key::{CACHE_PREFIX_USER_ID, CACHE_PREFIX_USERS, CACHE_PREFIX_USER_POSTS};
    let key = format!("{}{}", CACHE_PREFIX_USER_ID, id);
        let mut metrics = self.metrics.write().await;
        match self.cache.delete(&key).await {
            Ok(_) => { metrics.cache_invalidations += 1; },
            Err(e) => { metrics.cache_invalidation_errors += 1; warn!(user_id=%id, error=%e, "user cache delete failed"); }
        }
        drop(metrics);
        self.cache_invalidate_prefix(&format!("{}*", CACHE_PREFIX_USERS)).await;
        self.cache_invalidate_prefix(&format!("{}{}:*", CACHE_PREFIX_USER_POSTS, id)).await;
    }

    // ---------------- Search index safe helpers (avoid duplicated error logging) ----------------
    #[cfg(feature = "search")]
    pub async fn search_index_post_safe(&self, post: &crate::models::Post) {
        if let Err(e) = self.search.index_post(post).await {
            warn!(post_id = %post.id, error = ?e, "search index post failed");
        }
    }

    #[cfg(feature = "search")]
    pub async fn search_remove_post_safe(&self, id: uuid::Uuid) {
        if let Err(e) = self.search.remove_document(&id.to_string()).await {
            warn!(post_id = %id, error = ?e, "search remove post failed");
        }
    }

    #[cfg(feature = "search")]
    pub async fn search_index_user_safe(&self, user: &crate::models::User) {
        if let Err(e) = self.search.index_user(user).await {
            warn!(user_id = %user.id, error = ?e, "search index user failed");
        }
    }

    #[cfg(feature = "search")]
    pub async fn search_remove_user_safe(&self, id: uuid::Uuid) {
        if let Err(e) = self.search.remove_document(&id.to_string()).await {
            warn!(user_id = %id, error = ?e, "search remove user failed");
        }
    }

    // --- Search service wrappers to record search metrics centrally ---
    #[cfg(feature = "search")]
    pub async fn search_execute(&self, req: crate::search::SearchRequest) -> crate::Result<crate::search::SearchResults<serde_json::Value>> {
    timed_op!(self, "search", self.search.search(req))
    }

    #[cfg(feature = "search")]
    pub async fn search_suggest(&self, prefix: &str, limit: usize) -> crate::Result<Vec<String>> {
    timed_op!(self, "search", self.search.suggest(prefix, limit))
    }

    #[cfg(feature = "search")]
    pub async fn search_get_stats(&self) -> crate::Result<crate::search::SearchStats> {
    timed_op!(self, "search", self.search.get_stats())
    }

    // --- Auth service wrappers to record auth metrics centrally ---
    #[cfg(feature = "auth")]
    pub async fn auth_create_user(&self, request: crate::models::CreateUserRequest) -> crate::Result<crate::models::User> {
    timed_op!(self, "auth", self.auth.create_user(self, request))
    }

    #[cfg(feature = "auth")]
    pub async fn auth_authenticate(&self, request: crate::auth::LoginRequest) -> crate::Result<crate::models::User> {
    timed_op!(self, "auth", self.auth.authenticate_user(self, request))
    }

    #[cfg(feature = "auth")]
    pub async fn auth_create_session(&self, user_id: uuid::Uuid) -> crate::Result<String> {
    timed_op!(self, "auth", self.auth.create_session(user_id, self))
    }

    #[cfg(feature = "auth")]
    pub async fn auth_build_auth_response(&self, user: crate::models::User, remember_me: bool) -> crate::Result<crate::auth::AuthResponse> {
    timed_op!(self, "auth", self.auth.create_auth_response(user, remember_me))
    }

    #[cfg(feature = "auth")]
    pub async fn auth_refresh_access_token(&self, refresh_token: &str) -> crate::Result<crate::auth::RefreshResponse> {
    timed_op!(self, "auth", self.auth.refresh_access_token(refresh_token))
    }

    /// Validate a token using the AuthService; returns the authenticated user on success and records an auth attempt
    #[cfg(feature = "auth")]
    pub async fn auth_validate_token(&self, token: &str) -> crate::Result<crate::models::User> {
    timed_op!(self, "auth", self.auth.validate_token(self, token))
    }

    #[cfg(feature = "auth")]
    pub async fn auth_verify_biscuit(&self, token: &str) -> crate::Result<crate::auth::AuthContext> {
    timed_op!(self, "auth", self.auth.verify_biscuit(self, token))
    }

    /// Health check wrapper for AuthService that records timing
    #[cfg(feature = "auth")]
    pub async fn auth_health_check(&self) -> crate::Result<crate::app::ServiceHealth> {
    // auth の内部 DB 呼び出しを個別にカウントする必要があれば AuthService 側で timed 化する想定
    Ok(self.check_auth_health().await)
    }

    // --- Database wrapper helpers that record metrics centrally on AppState ---
    #[cfg(feature = "database")]
    pub async fn db_create_user(&self, req: crate::models::CreateUserRequest) -> crate::Result<crate::models::User> {
    timed_op!(self, "db", self.database.create_user(req))
    }

    #[cfg(feature = "database")]
    pub async fn db_get_user_by_id(&self, id: uuid::Uuid) -> crate::Result<crate::models::User> {
    timed_op!(self, "db", self.database.get_user_by_id(id))
    }

    #[cfg(feature = "database")]
    pub async fn db_get_user_by_email(&self, email: &str) -> crate::Result<crate::models::User> {
    timed_op!(self, "db", self.database.get_user_by_email(email))
    }

    #[cfg(feature = "database")]
    pub async fn db_get_users(&self, page: u32, limit: u32, role: Option<String>, active: Option<bool>, sort: Option<String>) -> crate::Result<Vec<crate::models::User>> {
    timed_op!(self, "db", self.database.get_users(page, limit, role, active, sort))
    }

    #[cfg(feature = "database")]
    pub async fn db_update_last_login(&self, id: uuid::Uuid) -> crate::Result<()> {
    timed_op!(self, "db", self.database.update_last_login(id))
    }

    #[cfg(feature = "database")]
    pub async fn db_count_users(&self) -> crate::Result<usize> {
    timed_op!(self, "db", self.database.count_users())
    }

    #[cfg(feature = "database")]
    pub async fn db_create_post(&self, req: crate::models::CreatePostRequest) -> crate::Result<crate::models::Post> {
    timed_op!(self, "db", self.database.create_post(req))
    }

    #[cfg(feature = "database")]
    pub async fn db_get_post_by_id(&self, id: uuid::Uuid) -> crate::Result<crate::models::Post> {
    timed_op!(self, "db", self.database.get_post_by_id(id))
    }

    #[cfg(feature = "database")]
    pub async fn db_get_posts(&self, page: u32, limit: u32, status: Option<String>, author: Option<uuid::Uuid>, tag: Option<String>, sort: Option<String>) -> crate::Result<Vec<crate::models::Post>> {
    timed_op!(self, "db", self.database.get_posts(page, limit, status, author, tag, sort))
    }

    #[cfg(feature = "database")]
    pub async fn db_update_post(&self, id: uuid::Uuid, req: crate::models::UpdatePostRequest) -> crate::Result<crate::models::Post> {
    timed_op!(self, "db", self.database.update_post(id, req))
    }

    #[cfg(feature = "database")]
    pub async fn db_delete_post(&self, id: uuid::Uuid) -> crate::Result<()> {
    timed_op!(self, "db", self.database.delete_post(id))
    }

    #[cfg(feature = "database")]
    pub async fn db_count_posts(&self, tag: Option<&str>) -> crate::Result<usize> {
    timed_op!(self, "db", self.database.count_posts(tag))
    }

    // --- API Key wrappers ---
    // NOTE: 他の DB ラッパは timed_db! macro を直接使えるが、ここは一部で in-place クロージャを
    // 使っており都度 start/elapsed を書いていたため共通化。
    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_create_api_key(&self, name: String, user_id: uuid::Uuid, permissions: Vec<String>) -> crate::Result<(crate::models::ApiKeyResponse, String)> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let (model, raw) = ApiKey::new_validated(name, user_id, permissions)?;
            let mut conn = self.database.get_connection()?;
            let saved = ApiKey::create(&mut conn, &model)?;
            Ok((saved.to_response(), raw))
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_get_api_key(&self, id: uuid::Uuid) -> crate::Result<crate::models::ApiKeyResponse> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let model = ApiKey::find_by_id(&mut conn, id)?;
            Ok(model.to_response())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_delete_api_key(&self, _id: uuid::Uuid) -> crate::Result<()> {
        use diesel::prelude::*;
        use crate::database::schema::api_keys::dsl::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let affected = diesel::delete(api_keys.filter(crate::database::schema::api_keys::dsl::id.eq(_id))).execute(&mut conn)?;
            if affected == 0 { return Err(crate::AppError::NotFound("api key not found".into())); }
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_rotate_api_key(&self, id: uuid::Uuid, new_name: Option<String>, new_permissions: Option<Vec<String>>) -> crate::Result<(crate::models::ApiKeyResponse, String)> {
        // Fetch existing
        let existing = self.db_get_api_key_model(id).await?;
        let name = new_name.unwrap_or(existing.name.clone());
        let perms = new_permissions.unwrap_or(existing.get_permissions());
        // Create replacement (same user)
        let (new_model_resp, raw) = self.db_create_api_key(name, existing.user_id, perms).await?;
        // Expire old key (soft: set expires_at = now)
        #[cfg(all(feature = "database", feature = "auth"))]
        {
            use diesel::prelude::*;
            use crate::database::schema::api_keys::dsl::*;
            let mut conn = self.database.get_connection()?;
            let now = chrono::Utc::now();
            diesel::update(api_keys.find(existing.id)).set(expires_at.eq(Some(now))).execute(&mut conn)?;
        }
        Ok((new_model_resp, raw))
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_revoke_api_key(&self, id: uuid::Uuid) -> crate::Result<()> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            ApiKey::delete(&mut conn, id)?;
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_revoke_api_key_owned(&self, key_id: uuid::Uuid, user: uuid::Uuid) -> crate::Result<()> {
        use crate::models::ApiKey;
        use diesel::prelude::*;
        use crate::database::schema::api_keys::dsl::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let affected = diesel::delete(api_keys.filter(crate::database::schema::api_keys::dsl::id.eq(key_id).and(user_id.eq(user)))).execute(&mut conn)?;
            if affected == 0 {
                let exists = ApiKey::find_by_id(&mut conn, key_id).ok();
                if exists.is_some() { return Err(crate::AppError::Authorization("not owner".into())); } else { return Err(crate::AppError::NotFound("api key not found".into())); }
            }
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_touch_api_key(&self, id: uuid::Uuid) -> crate::Result<()> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            ApiKey::update_last_used(&mut conn, id)?;
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_list_api_keys(&self, user_id: uuid::Uuid, include_expired: bool) -> crate::Result<Vec<crate::models::ApiKeyResponse>> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let models = ApiKey::list_for_user(&mut conn, user_id, include_expired)?;
            Ok(models.into_iter().map(|m| m.to_response()).collect())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_get_api_key_by_lookup_hash(&self, lookup: &str) -> crate::Result<crate::models::ApiKey> {
        use crate::models::ApiKey;
        let lookup = lookup.to_string();
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let model = ApiKey::find_by_lookup_hash(&mut conn, &lookup)?;
            Ok(model)
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_get_api_key_model(&self, id: uuid::Uuid) -> crate::Result<crate::models::ApiKey> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let model = ApiKey::find_by_id(&mut conn, id)?;
            Ok(model)
        })
    }

    /// Backfill api_key_lookup_hash for legacy rows (where it's an empty string), using a raw API key.
    /// Returns Some(ApiKey) if a matching legacy key was found and updated; None otherwise.
    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_backfill_api_key_lookup_for_raw(&self, raw: &str) -> crate::Result<Option<crate::models::ApiKey>> {
        use diesel::prelude::*;
        use crate::database::schema::api_keys::dsl::*;
        use crate::models::ApiKey as ApiKeyModel;

        let raw = raw.to_string();
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            // fetch candidates with empty lookup
            let mut candidates: Vec<ApiKeyModel> = api_keys
                .filter(api_key_lookup_hash.eq(""))
                .load(&mut conn)?;

            // try to find a verifying match
            for cand in candidates.iter_mut() {
                if cand.verify_key(&raw).unwrap_or(false) {
                    // compute new lookup and persist
                    let new_lookup = ApiKeyModel::lookup_hash(&raw);
                    diesel::update(api_keys.find(cand.id))
                        .set(api_key_lookup_hash.eq(new_lookup.clone()))
                        .execute(&mut conn)?;
                    cand.api_key_lookup_hash = new_lookup;
                    return Ok(Some(cand.clone()));
                }
            }
            Ok(None)
        })
    }

    // --- Admin-specific DB helpers (to avoid direct Diesel in handlers) ---
    #[cfg(feature = "database")]
    pub async fn db_admin_list_recent_posts(&self, limit: i64) -> crate::Result<Vec<crate::utils::common_types::PostSummary>> {
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;

            #[derive(diesel::QueryableByName)]
            struct Row {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                title: String,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                author_id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                status: String,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                created_at: chrono::DateTime<chrono::Utc>,
            }

            let rows: Vec<Row> = diesel::sql_query(
                "SELECT id, title, author_id, status, created_at FROM posts ORDER BY created_at DESC LIMIT $1",
            )
            .bind::<diesel::sql_types::BigInt, _>(limit)
            .load(&mut conn)?;

            let out = rows
                .into_iter()
                .map(|r| crate::utils::common_types::PostSummary {
                    id: r.id.to_string(),
                    title: r.title,
                    author_id: r.author_id.to_string(),
                    status: r.status,
                    created_at: r.created_at.to_rfc3339(),
                })
                .collect();

            Ok(out)
        })
    }

    // --- API Key maintenance helpers (legacy lookup_hash backfill visibility/ops) ---
    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_list_api_keys_missing_lookup(&self) -> crate::Result<Vec<crate::models::ApiKey>> {
        use diesel::prelude::*;
        use crate::database::schema::api_keys::dsl::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let rows: Vec<crate::models::ApiKey> = api_keys
                .filter(api_key_lookup_hash.eq(""))
                .load(&mut conn)?;
            Ok(rows)
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_expire_api_keys_missing_lookup(&self, now: chrono::DateTime<chrono::Utc>) -> crate::Result<usize> {
        use diesel::prelude::*;
        use crate::database::schema::api_keys::dsl::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            // expires_at は NULL 許容の想定のため Some(now)
            let affected = diesel::update(api_keys.filter(api_key_lookup_hash.eq("")))
                .set(expires_at.eq(Some(now)))
                .execute(&mut conn)?;
            Ok(affected)
        })
    }

    #[cfg(feature = "database")]
    pub async fn db_admin_delete_post(&self, post_id: uuid::Uuid) -> crate::Result<()> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let affected = diesel::delete(posts_dsl::posts.filter(posts_dsl::id.eq(post_id)))
                .execute(&mut conn)?;
            if affected == 0 {
                return Err(crate::AppError::NotFound("post not found".into()));
            }
            Ok(())
        })
    }

    #[cfg(feature = "database")]
    pub async fn db_admin_users_count(&self) -> crate::Result<i64> {
        use diesel::prelude::*;
        use crate::database::schema::users::dsl as users_dsl;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let count: i64 = users_dsl::users.count().get_result(&mut conn)?;
            Ok(count)
        })
    }

    #[cfg(feature = "database")]
    pub async fn db_admin_posts_count(&self) -> crate::Result<i64> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let count: i64 = posts_dsl::posts.count().get_result(&mut conn)?;
            Ok(count)
        })
    }

    #[cfg(feature = "database")]
    pub async fn db_admin_find_admin_user(&self) -> crate::Result<Option<crate::models::User>> {
        use diesel::prelude::*;
        use crate::database::schema::users::dsl as users_dsl;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let user = users_dsl::users
                .filter(users_dsl::username.eq("admin"))
                .first::<crate::models::User>(&mut conn)
                .optional()?;
            Ok(user)
        })
    }

    /// Execute a raw SQL statement and return affected rows
    #[cfg(feature = "database")]
    pub async fn db_execute_sql(&self, sql: &str) -> crate::Result<usize> {
        use diesel::prelude::*;
        let sql = sql.to_string();
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let n = diesel::sql_query(&sql).execute(&mut conn)?;
            Ok(n)
        })
    }

    /// Fetch applied migration versions from schema_migrations or __diesel_schema_migrations
    #[cfg(feature = "database")]
    pub async fn db_fetch_applied_migrations(&self) -> crate::Result<Vec<String>> {
        use diesel::prelude::*;
        #[derive(diesel::QueryableByName)]
        struct MigrationVersion { #[diesel(sql_type = diesel::sql_types::Text)] version: String }
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
        let try_primary: std::result::Result<Vec<MigrationVersion>, diesel::result::Error> =
                diesel::sql_query("SELECT version FROM schema_migrations ORDER BY version ASC").load(&mut conn);
            let rows = match try_primary {
                Ok(r) => r,
                Err(_) => diesel::sql_query("SELECT version FROM __diesel_schema_migrations ORDER BY version ASC")
                    .load(&mut conn)
            .map_err(|e| crate::AppError::Internal(e.to_string()))?,
            };
            Ok(rows.into_iter().map(|r| r.version).collect())
        })
    }

    /// Ensure schema_migrations exists and copy rows from legacy table
    #[cfg(feature = "database")]
    pub async fn db_ensure_schema_migrations_compat(&self) -> crate::Result<()> {
        let create_sql = "CREATE TABLE IF NOT EXISTS schema_migrations (version VARCHAR(255) PRIMARY KEY, applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW());";
        let copy_sql = "INSERT INTO schema_migrations(version, applied_at) SELECT version, run_on FROM __diesel_schema_migrations WHERE version NOT IN (SELECT version FROM schema_migrations);";
        let _ = self.db_execute_sql(create_sql).await?;
        let _ = self.db_execute_sql(copy_sql).await?;
        Ok(())
    }

    // --- Diesel migrations helpers to avoid direct conn in bins ---
    #[cfg(feature = "database")]
    pub async fn db_run_pending_migrations(
        &self,
        migrations: diesel_migrations::EmbeddedMigrations,
    ) -> crate::Result<()> {
        use diesel_migrations::MigrationHarness;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            conn.run_pending_migrations(migrations)
                .map_err(|e| crate::AppError::Internal(e.to_string()))?;
            Ok(())
        })
    }

    #[cfg(feature = "database")]
    pub async fn db_revert_last_migration(
        &self,
        migrations: diesel_migrations::EmbeddedMigrations,
    ) -> crate::Result<()> {
        use diesel_migrations::MigrationHarness;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            conn.revert_last_migration(migrations)
                .map_err(|e| crate::AppError::Internal(e.to_string()))?;
            Ok(())
        })
    }

    #[cfg(feature = "database")]
    pub async fn db_list_pending_migrations(
        &self,
        migrations: diesel_migrations::EmbeddedMigrations,
    ) -> crate::Result<Vec<String>> {
        use diesel_migrations::MigrationHarness;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let list = conn
                .pending_migrations(migrations)
                .map_err(|e| crate::AppError::Internal(e.to_string()))?;
            Ok(list.into_iter().map(|m| m.name().to_string()).collect())
        })
    }
}

/// Application environment setup
pub struct AppEnvironment;

impl AppEnvironment {
    /// Initialize logging and environment
    pub fn init() {
        // Initialize environment from .env file if available
            if dotenvy::dotenv().is_err() {
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
        info!(
            "📊 Logging level: {}",
            std::env::var("RUST_LOG").unwrap_or_else(|_| "default".to_string())
        );
    }
}
