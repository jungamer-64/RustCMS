//! アプリケーション状態とサービス管理
#![allow(clippy::empty_line_after_outer_attr)]
//!
//! 本モジュールは CMS の中核状態 `AppState` とその周辺(メトリクス/ヘルスチェック/初期化)を提供します。
//! - データベース接続（接続プール）
//! - 認証サービス（biscuit-auth + `WebAuthn`）
//! - キャッシュサービス（Redis + メモリ）
//! - 検索サービス（Tantivy 全文検索）
//! - ヘルス監視とメトリクス収集
//!
//! Feature フラグによりサービスの有効/無効が決まります。無効な場合はヘルスチェックが
//! `not_configured` を返すなど、挙動が変化します。詳細は `docs/FEATURES_JA.md` を参照してください。
//! Centralized application state containing all services for the CMS:
//! - Database connections with pooling
//! - Authentication service with biscuit-auth + `WebAuthn`
//! - Cache service with Redis + in-memory layers
//! - Search service with Tantivy full-text search
//! - Health monitoring and metrics collection

use crate::limiter::FixedWindowLimiter;
use crate::{Result, config::Config};

#[cfg(feature = "auth")]
use crate::auth::AuthService;
#[cfg(feature = "cache")]
use crate::cache::CacheService;
#[cfg(feature = "database")]
use crate::database::Database;
#[cfg(feature = "search")]
use crate::search::SearchService;
#[cfg(feature = "search")]
use crate::utils::search_index::SearchEntity;
use serde::{Deserialize, Serialize};
#[cfg(feature = "cache")]
use std::time::Duration;
use std::{sync::Arc, time::Instant};
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};
use utoipa::ToSchema;

// --- Generic instrumentation macro ---
// 単一マクロで開始→await→経過時間→成功時処理 を汎用化。
// 呼び出し側で metric 種別を &str で指定し、対応する更新ロジックに分岐。
macro_rules! timed_op {
    ($self:ident, $kind:expr, $future:expr) => {{
        let start = std::time::Instant::now();
        let res = $future.await;
        // use floating-point seconds to avoid precision-loss cast from u128
        let elapsed = start.elapsed().as_secs_f64() * 1000.0; // milliseconds as f64
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
    #[allow(clippy::empty_line_after_outer_attr)]
    #[cfg(feature = "cache")]
    pub cache: CacheService,

    /// Full-text search service
    #[cfg(feature = "search")]
    pub search: SearchService,

    /// CSRF protection service (security hardening)
    pub csrf: crate::middleware::security::CsrfService,

    /// Application configuration
    pub config: Arc<Config>,

    /// Application metrics
    pub metrics: Arc<RwLock<AppMetrics>>,

    /// Unified IP rate limiter (fixed window strategy)
    pub rate_limiter: Arc<FixedWindowLimiter>,

    /// Application start time for uptime calculations
    pub start_time: Instant,
    /// Broadcast sender used to notify background tasks to exit during shutdown
    pub shutdown_tx: broadcast::Sender<()>,

    /// Event bus for event-driven architecture
    /// Enables decoupling of cross-cutting concerns (search indexing, cache invalidation)
    pub event_bus: crate::events::EventBus,

    /// Application container with constructed adapters and use-cases.
    /// Present when the `database` feature is enabled and initialized in
    /// `AppState::from_config`.
    #[cfg(feature = "database")]
    pub container: Option<Arc<crate::application::AppContainer>>,
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
enum FailureMode {
    Down,
    Degraded,
}

#[inline]
async fn to_service_health<F, T>(fut: F, failure: FailureMode) -> ServiceHealth
where
    F: std::future::Future<Output = crate::Result<T>>,
{
    let start = std::time::Instant::now();
    match fut.await {
        Ok(_) => ServiceHealth {
            status: "up".to_string(),
            response_time_ms: start.elapsed().as_secs_f64() * 1000.0,
            error: None,
            details: serde_json::json!({}),
        },
        Err(e) => {
            let status = match failure {
                FailureMode::Down => "down",
                FailureMode::Degraded => "degraded",
            };
            ServiceHealth {
                status: status.to_string(),
                response_time_ms: start.elapsed().as_secs_f64() * 1000.0,
                // use inlined debug formatting
                error: Some(format!("{e:?}")),
                details: serde_json::json!({}),
            }
        }
    }
}

#[inline]
#[allow(dead_code)]
fn service_not_configured(msg: &str) -> ServiceHealth {
    ServiceHealth {
        status: "not_configured".to_string(),
        response_time_ms: 0.0,
        error: None,
        details: serde_json::json!({"message": msg}),
    }
}

// (旧) update_running_avg は各 record_* 内へインライン化済み

/// Builder pattern for `AppState` to handle conditional compilation
pub struct AppStateBuilder {
    pub config: Arc<Config>,
    pub metrics: Arc<RwLock<AppMetrics>>,
    pub start_time: Instant,

    #[cfg(feature = "database")]
    pub database: Option<Database>,
    #[cfg(feature = "database")]
    pub container: Option<Arc<crate::application::AppContainer>>,
    /// Optional pre-constructed event bus to allow early wiring of components
    /// (e.g. constructing AppContainer before full AppState is built).
    pub event_bus: Option<crate::events::EventBus>,
    #[cfg(feature = "auth")]
    pub auth: Option<AuthService>,
    #[cfg(feature = "cache")]
    pub cache: Option<CacheService>,
    #[cfg(feature = "search")]
    pub search: Option<SearchService>,
}

impl AppStateBuilder {
    /// Build `AppState` from the collected parts.
    ///
    /// # Panics
    /// 有効な feature に対応するサービスが初期化されていない場合は panic します。これはコンパイル時の feature と実行時の初期化の整合性を保証するためです。
    #[must_use]
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
            // Initialize CSRF protection service (security hardening)
            csrf: crate::middleware::security::CsrfService::new(),
            config: self.config,
            metrics: self.metrics,
            rate_limiter: Arc::new(FixedWindowLimiter::new(100, 60)), // default; real values set in from_config
            start_time: self.start_time,
            shutdown_tx: broadcast::channel(1).0,
            event_bus: self
                .event_bus
                .unwrap_or_else(|| crate::events::create_event_bus(1024).0),
            #[cfg(feature = "database")]
            container: self.container,
        }
    }
}

// When auth feature is disabled provide minimal stubs on AppState so code that
// calls auth wrappers (middleware/handlers) compiles. These should not be used
// at runtime unless the auth feature is enabled.
#[cfg(not(feature = "auth"))]
impl AppState {
    pub async fn auth_verify_biscuit(
        &self,
        _token: &str,
    ) -> crate::Result<crate::auth::AuthContext> {
        Err(crate::AppError::NotImplemented(
            "auth feature not enabled".into(),
        ))
    }

    pub async fn auth_create_user(
        &self,
        _req: crate::models::CreateUserRequest,
    ) -> crate::Result<crate::models::User> {
        Err(crate::AppError::NotImplemented(
            "auth feature not enabled".into(),
        ))
    }

    pub async fn auth_authenticate(
        &self,
        _req: crate::auth::LoginRequest,
    ) -> crate::Result<crate::models::User> {
        Err(crate::AppError::NotImplemented(
            "auth feature not enabled".into(),
        ))
    }

    pub async fn auth_build_success_response(
        &self,
        _user: crate::models::User,
        _remember: bool,
    ) -> crate::Result<crate::utils::auth_response::AuthSuccessResponse> {
        Err(crate::AppError::NotImplemented(
            "auth feature not enabled".into(),
        ))
    }

    pub async fn auth_build_auth_response(
        &self,
        _user: crate::models::User,
        _remember: bool,
    ) -> crate::Result<crate::auth::AuthResponse> {
        Err(crate::AppError::NotImplemented(
            "auth feature not enabled".into(),
        ))
    }

    pub async fn auth_refresh_access_token(
        &self,
        _refresh_token: &str,
    ) -> crate::Result<(crate::utils::auth_response::AuthTokens, crate::models::User)> {
        Err(crate::AppError::NotImplemented(
            "auth feature not enabled".into(),
        ))
    }
}

// Helper functions for from_config

/// Helper: Get environment variable interval or default
fn get_interval_from_env(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Helper: Initialize database service if feature enabled
#[cfg(feature = "database")]
async fn initialize_database(config: &Config) -> Result<Database> {
    info!("🗄️ Connecting to PostgreSQL database...");
    let db = Database::new(&config.database).await?;
    info!("✅ Database connection established");
    Ok(db)
}

/// Helper: Initialize auth service if feature enabled
#[cfg(feature = "auth")]
async fn initialize_auth(
    config: &crate::config::AuthConfig,
    #[cfg(feature = "database")] database: Option<&Database>,
    #[cfg(feature = "database")] container: Option<&Arc<crate::application::AppContainer>>,
) -> Result<AuthService> {
    info!("🔐 Setting up authentication service...");

    #[cfg(feature = "database")]
    {
        if let Some(db) = database {
            // Phase 5-4: AppContainer は Use Case を提供するが、
            // 直接的な repository アクセスは AppState 経由で行う
            // （container.state() -> database -> repository）
            if let Some(_c) = container {
                // 現在の設計では AuthService は database 経由で初期化
                // （AppContainer の Use Case は別の責務）
                let auth = AuthService::new(config, db)?;
                info!("✅ Authentication service initialized (AppState-backed)");
                return Ok(auth);
            }

            // Fallback to legacy constructor which will construct a short-lived
            // Diesel adapter internally.
            let auth = AuthService::new(config, db)?;
            info!("✅ Authentication service initialized");
            Ok(auth)
        } else {
            Err(crate::AppError::Internal(
                "Auth requires database but database not initialized".to_string(),
            ))
        }
    }

    #[cfg(not(feature = "database"))]
    {
        // Auth feature requires database - this should not compile
        compile_error!("auth feature requires database feature to be enabled");
    }
}

/// Helper: Initialize cache service if feature enabled
#[cfg(feature = "cache")]
async fn initialize_cache(config: &crate::config::RedisConfig) -> Result<CacheService> {
    info!("🚀 Setting up cache service...");
    let cache = CacheService::new(config).await?;
    info!("✅ Cache service initialized");
    Ok(cache)
}

/// Helper: Initialize search service if feature enabled
#[cfg(feature = "search")]
async fn initialize_search(config: crate::config::SearchConfig) -> Result<SearchService> {
    info!("🔍 Setting up search service...");
    let search = SearchService::new(config).await?;
    info!("✅ Search service initialized");
    Ok(search)
}

/// Helper: Spawn auth session cleanup background task
#[cfg(feature = "auth")]
fn spawn_auth_cleanup_task(state: &AppState) {
    let state_clone = state.clone();
    let mut shutdown_rx = state_clone.shutdown_tx.subscribe();
    let interval_secs = get_interval_from_env("AUTH_SESSION_CLEAN_INTERVAL_SECS", 300);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    // cleanup_expired_sessions doesn't return Result, just call it
                    state_clone.auth.cleanup_expired_sessions().await;

                    let active = state_clone.auth.get_active_session_count().await as u64;
                    if let Ok(mut m) = state_clone.metrics.try_write() {
                        m.active_sessions = active;
                    }
                }
                Ok(()) = shutdown_rx.recv() => {
                    info!("Auth cleanup task received shutdown");
                    break;
                }
                else => { break; }
            }
        }
    });
}

/// Helper: Spawn CSRF token cleanup background task
fn spawn_csrf_cleanup_task(state: &AppState) {
    let state_clone = state.clone();
    let mut shutdown_rx = state_clone.shutdown_tx.subscribe();
    let cleanup_interval_secs = get_interval_from_env("CSRF_CLEANUP_INTERVAL_SECS", 1800);

    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_secs(cleanup_interval_secs));
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    // cleanup_expired_tokens doesn't return Result, just call it
                    state_clone.csrf.cleanup_expired_tokens().await;
                }
                Ok(()) = shutdown_rx.recv() => {
                    info!("CSRF cleanup task received shutdown");
                    break;
                }
                else => { break; }
            }
        }
    });
}

/// Helper: Spawn all background maintenance tasks
fn spawn_background_tasks(state: &AppState) {
    #[cfg(feature = "auth")]
    spawn_auth_cleanup_task(state);

    spawn_csrf_cleanup_task(state);

    // Spawn event listeners for search indexing and cache invalidation
    crate::listeners::spawn_event_listeners(state.clone(), state.event_bus.clone());
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
    /// 環境設定からアプリケーション状態を生成
    ///
    /// 概要: 環境変数や設定ファイルから `Config` を組み立て、`from_config` に委譲します。
    ///
    /// # Errors
    ///
    /// 設定の読み込みに失敗した場合、または `from_config` がエラーを返した場合に `Err` を返します。
    ///
    /// Create application state from environment configuration
    ///
    /// # Errors
    /// 設定の読み込みや各サービスの初期化に失敗した場合はエラーを返します。
    pub async fn from_env() -> Result<Self> {
        // Load configuration and delegate to from_config
        let config = Config::from_env()?;
        Self::from_config(config).await
    }

    /// 与えられた `Config` からアプリケーション状態を生成（集中初期化用）
    ///
    /// # Panics
    ///
    /// 有効な feature に対応するサービスが初期化されていない場合、`AppStateBuilder::build` が panic します。
    ///
    /// # Errors
    ///
    /// DB/キャッシュ/検索/認証の初期化に失敗した場合に `Err` を返します。
    /// Create application state from a provided `Config` (useful for central init)
    ///
    /// # Panics
    /// 有効化された feature に対するサービスが初期化されていない場合は panic します（`AppStateBuilder::build` 内）。
    ///
    /// # Errors
    /// 各サービス（DB/キャッシュ/検索/認証）の初期化に失敗した場合、エラーを返します。
    #[allow(clippy::too_many_lines)] // Still long due to feature gates, but improved
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
            #[cfg(feature = "database")]
            container: None,
            event_bus: None,
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
            // Create event bus early so we can construct an AppContainer
            // before the full AppState is built. This allows services like
            // `AuthService` to be initialized with container-provided
            // repositories rather than constructing adapters ad-hoc.
            let (early_event_bus, _early_rx) = crate::events::create_event_bus(1024);
            app_state_builder.event_bus = Some(early_event_bus.clone());

            app_state_builder.database = Some(initialize_database(&config).await?);

            // Note: AppContainer は後で AppState 全体を受け取るため、
            // ここでは早期構築をスキップする（Phase 5-4 設計変更）
            // app_state_builder.container は build() 後に設定
        }

        #[cfg(feature = "auth")]
        {
            app_state_builder.auth = Some(
                initialize_auth(
                    &config.auth,
                    #[cfg(feature = "database")]
                    app_state_builder.database.as_ref(),
                    #[cfg(feature = "database")]
                    app_state_builder.container.as_ref(),
                )
                .await?,
            );
        }

        #[cfg(feature = "cache")]
        {
            app_state_builder.cache = Some(initialize_cache(&config.redis).await?);
        }

        #[cfg(feature = "search")]
        {
            app_state_builder.search = Some(initialize_search(config.search.clone()).await?);
        }

        let mut app_state = app_state_builder.build();

        // Construct application-level container (DI) if database feature is enabled.
        // AppContainer は AppState 全体への Arc を受け取る（Phase 5-4 設計）
        #[cfg(feature = "database")]
        {
            let app_state_arc = Arc::new(app_state.clone());
            let container = crate::application::AppContainer::new(app_state_arc);
            app_state.container = Some(Arc::new(container));
        }

        // Configure rate limiter
        let max_reqs = u32::try_from(config.security.rate_limit_requests).unwrap_or_else(|_| {
            warn!("Invalid rate_limit_requests value, using u32::MAX");
            u32::MAX
        });
        app_state.rate_limiter = Arc::new(FixedWindowLimiter::new(
            max_reqs,
            config.security.rate_limit_window,
        ));

        // Spawn background maintenance tasks
        spawn_background_tasks(&app_state);

        info!("🎉 Application state initialized successfully");
        Ok(app_state)
    }

    /// Perform comprehensive health check of all services
    ///
    /// # Errors
    /// 現在この関数自体はエラーを返しません（各サービスの失敗は `ServiceHealth` の `status`/`error` に反映されます）。
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

    define_health_check!(
        check_database_health,
        "database",
        database,
        FailureMode::Down,
        "Database feature not enabled"
    );
    define_health_check!(
        check_cache_health,
        "cache",
        cache,
        FailureMode::Degraded,
        "Cache feature not enabled"
    );
    define_health_check!(
        check_search_health,
        "search",
        search,
        FailureMode::Degraded,
        "Search feature not enabled"
    );
    define_health_check!(
        check_auth_health,
        "auth",
        auth,
        FailureMode::Down,
        "Auth feature not enabled"
    );

    /// Get current application metrics
    pub async fn get_metrics(&self) -> AppMetrics {
        let metrics = self.metrics.read().await;
        let mut current_metrics = metrics.clone();
        // Release the read lock before performing any await to avoid holding the lock across .await
        drop(metrics);

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
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
        }
    }

    /// Update authentication metrics
    pub async fn record_auth_attempt(&self, success: bool) {
        {
            let mut metrics = self.metrics.write().await;
            metrics.auth_attempts += 1;
            if success {
                metrics.auth_successes += 1;
            } else {
                metrics.auth_failures += 1;
            }
        }
    }

    /// Update search metrics
    pub async fn record_search_query(&self, response_time_ms: f64) {
        let mut m = self.metrics.write().await;
        m.search_queries += 1;
        // Average update using numerically stable, fewer-FLOPs formula:
        // avg += (x - avg) / n
        #[allow(clippy::cast_precision_loss)]
        let n = m.search_queries as f64;
        m.search_avg_response_time_ms += (response_time_ms - m.search_avg_response_time_ms) / n;
    }

    /// Update database metrics
    pub async fn record_db_query(&self, response_time_ms: f64) {
        let mut m = self.metrics.write().await;
        m.db_queries += 1;
        // avg += (x - avg) / n
        #[allow(clippy::cast_precision_loss)]
        let n = m.db_queries as f64;
        m.db_avg_response_time_ms += (response_time_ms - m.db_avg_response_time_ms) / n;
    }

    /// Record error by type
    pub async fn record_error(&self, error_type: &str) {
        {
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
    }

    /// Get application uptime in seconds
    #[must_use]
    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Signal background tasks and services to shutdown. This will send a broadcast
    /// notification to any subscribed background tasks and perform best-effort
    /// cleanup of services (flush writers, close pools, etc.). This function
    /// should be called during application graceful shutdown.
    // Allow cognitive complexity: feature-gated cleanup requires branching for multiple services.
    #[allow(clippy::cognitive_complexity)]
    pub async fn shutdown(&self) {
        info!("AppState shutdown initiated");
        // Send broadcast (ignore error if there are no receivers)
        let _ = self.shutdown_tx.send(());

        // Attempt service-specific shutdown/cleanup
        #[cfg(feature = "search")]
        {
            if let Err(e) = self.search.shutdown().await {
                warn!(error=?e, "search shutdown failed during AppState shutdown");
            }
        }

        #[cfg(feature = "database")]
        {
            // Best-effort DB pool close if supported
            if let Err(e) = self.database.close().await {
                warn!(error=?e, "database close failed during shutdown");
            }
        }

        // Telemetry shutdown (best-effort no-op by default)
        #[cfg(feature = "monitoring")]
        crate::telemetry::shutdown_telemetry();

        info!("AppState shutdown complete (signalled background tasks)");
    }

    /// Rate limit helper for IP addresses. Returns true if request allowed.
    #[must_use]
    pub fn allow_ip(&self, ip: &std::net::IpAddr) -> bool {
        self.rate_limiter.allow(&ip.to_string())
    }

    /// Convenience helper to get a pooled DB connection from `AppState`
    ///
    /// # Errors
    /// - DB プールが枯渇・停止しているなど、接続の取得に失敗した場合。
    #[cfg(feature = "database")]
    pub fn get_conn(&self) -> crate::Result<crate::database::PooledConnection> {
        self.database.get_connection()
    }

    /// Access the application-level container (if present).
    #[cfg(feature = "database")]
    #[must_use]
    pub fn get_container(&self) -> Option<Arc<crate::application::AppContainer>> {
        self.container.clone()
    }

    /// Helper: obtain a CreateUser use-case instance.
    /// Prefers the centrally-constructed AppContainer; if absent constructs
    /// a short-lived adapter/use-case for backward compatibility.
    #[cfg(feature = "database")]
    #[must_use]
    pub fn get_create_user_uc(
        &self,
    ) -> std::sync::Arc<
        crate::application::use_cases::CreateUserUseCase<
            crate::infrastructure::repositories::DieselUserRepository,
        >,
    > {
        if let Some(container) = self.container.as_ref() {
            return container.create_user();
        }
        let repo =
            crate::infrastructure::repositories::DieselUserRepository::new(self.database.clone());
        std::sync::Arc::new(crate::application::use_cases::CreateUserUseCase::new(
            std::sync::Arc::new(repo),
        ))
    }

    #[cfg(feature = "database")]
    #[must_use]
    pub fn get_user_by_id_uc(
        &self,
    ) -> std::sync::Arc<
        crate::application::use_cases::GetUserByIdUseCase<
            crate::infrastructure::repositories::DieselUserRepository,
        >,
    > {
        if let Some(container) = self.container.as_ref() {
            return container.get_user_by_id();
        }
        let repo =
            crate::infrastructure::repositories::DieselUserRepository::new(self.database.clone());
        std::sync::Arc::new(crate::application::use_cases::GetUserByIdUseCase::new(
            std::sync::Arc::new(repo),
        ))
    }

    #[cfg(feature = "database")]
    #[must_use]
    pub fn get_update_user_uc(
        &self,
    ) -> std::sync::Arc<
        crate::application::use_cases::UpdateUserUseCase<
            crate::infrastructure::repositories::DieselUserRepository,
        >,
    > {
        if let Some(container) = self.container.as_ref() {
            return container.update_user();
        }
        let repo =
            crate::infrastructure::repositories::DieselUserRepository::new(self.database.clone());
        std::sync::Arc::new(crate::application::use_cases::UpdateUserUseCase::new(
            std::sync::Arc::new(repo),
        ))
    }

    // Note: Post-related use-cases (CreatePostUseCase, UpdatePostUseCase, etc.) are not currently
    // implemented in the application layer. They remain available through direct database calls
    // via handlers and event listeners until the refactoring is complete.
    //
    // TODO: Implement post-related use-cases as part of the next refactoring iteration.

    // ---------------- Cache helper (get or compute & store) ----------------
    #[cfg(feature = "cache")]
    /// キャッシュから値を取得し、存在しない場合は計算して保存します。
    ///
    /// # Errors
    /// - `builder` が返す非同期処理がエラーを返した場合。
    /// - 値のシリアライズ/デシリアライズに失敗した場合。
    /// - Redis への保存でエラーが発生した場合（致命ではないため、内部ではログに留めます）。
    pub async fn cache_get_or_set<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        builder: F,
    ) -> crate::Result<T>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::Result<T>>,
    {
        if let Ok(Some(v)) = self.cache.get::<T>(key).await {
            return Ok(v);
        }
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
        // 先に I/O（await）を終えてからメトリクスのロックを取得し、ロック保持時間を最小化
        let res = self.cache.delete(prefix).await;
        match res {
            Ok(()) => {
                self.metrics.write().await.cache_invalidations += 1;
            }
            Err(e) => {
                self.metrics.write().await.cache_invalidation_errors += 1;
                warn!("cache invalidate failed prefix={} err={}", prefix, e);
            }
        }
    }

    // ---------------- Entity specific cache helpers (to reduce handler duplication) ----------------
    #[cfg(feature = "cache")]
    pub async fn invalidate_post_caches(&self, id: uuid::Uuid) {
        use crate::utils::cache_key::{CACHE_PREFIX_POST_ID, CACHE_PREFIX_POSTS};
        let key = format!("{CACHE_PREFIX_POST_ID}{id}");
        let res = self.cache.delete(&key).await;
        match res {
            Ok(()) => {
                self.metrics.write().await.cache_invalidations += 1;
            }
            Err(e) => {
                self.metrics.write().await.cache_invalidation_errors += 1;
                warn!(post_id=%id, error=%e, "post cache delete failed");
            }
        }
        let posts_prefix = format!("{CACHE_PREFIX_POSTS}*");
        self.cache_invalidate_prefix(&posts_prefix).await; // prefix helper already logs
    }

    #[cfg(feature = "cache")]
    pub async fn invalidate_user_caches(&self, id: uuid::Uuid) {
        use crate::utils::cache_key::{
            CACHE_PREFIX_USER_ID, CACHE_PREFIX_USER_POSTS, CACHE_PREFIX_USERS,
        };
        let key = format!("{CACHE_PREFIX_USER_ID}{id}");
        let res = self.cache.delete(&key).await;
        match res {
            Ok(()) => {
                self.metrics.write().await.cache_invalidations += 1;
            }
            Err(e) => {
                self.metrics.write().await.cache_invalidation_errors += 1;
                warn!(user_id=%id, error=%e, "user cache delete failed");
            }
        }
        let users_prefix = format!("{CACHE_PREFIX_USERS}*");
        self.cache_invalidate_prefix(&users_prefix).await;
        let user_posts_prefix = format!("{CACHE_PREFIX_USER_POSTS}{id}:*");
        self.cache_invalidate_prefix(&user_posts_prefix).await;
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

    // ---------------- Generic search index dispatcher (reduces closure duplication) ----------------
    #[cfg(feature = "search")]
    pub async fn search_index_entity_safe(&self, entity: SearchEntity<'_>) {
        match entity {
            SearchEntity::Post(p) => {
                if let Err(e) = self.search.index_post(p).await {
                    warn!(post_id = %p.id, error=?e, "search index post failed");
                }
            }
            SearchEntity::User(u) => {
                if let Err(e) = self.search.index_user(u).await {
                    warn!(user_id = %u.id, error=?e, "search index user failed");
                }
            }
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
    /// 検索を実行します。
    ///
    /// # Errors
    ///
    /// 検索バックエンドへの問い合わせに失敗した場合にエラーを返します。
    pub async fn search_execute(
        &self,
        req: crate::search::SearchRequest,
    ) -> crate::Result<crate::search::SearchResults<serde_json::Value>> {
        timed_op!(self, "search", self.search.search(req))
    }

    #[cfg(feature = "search")]
    /// サジェストを取得します。
    ///
    /// # Errors
    ///
    /// 検索バックエンドからの応答取得に失敗した場合にエラーを返します。
    pub async fn search_suggest(&self, prefix: &str, limit: usize) -> crate::Result<Vec<String>> {
        timed_op!(self, "search", self.search.suggest(prefix, limit))
    }

    #[cfg(feature = "search")]
    /// 検索統計情報を取得します。
    ///
    /// # Errors
    ///
    /// 統計取得時にバックエンド呼び出しが失敗した場合にエラーを返します。
    pub async fn search_get_stats(&self) -> crate::Result<crate::search::SearchStats> {
        timed_op!(self, "search", self.search.get_stats())
    }

    // --- Auth service wrappers to record auth metrics centrally ---
    #[cfg(feature = "auth")]
    /// ユーザーを作成します。
    ///
    /// # Errors
    ///
    /// 入力検証や保存処理、外部サービス連携に失敗した場合にエラーを返します。
    pub async fn auth_create_user(
        &self,
        request: crate::models::CreateUserRequest,
    ) -> crate::Result<crate::models::User> {
        // Prefer application-layer use-case that depends on the repository port
        // when database feature is present. This allows incremental migration of
        // business logic while preserving the existing timing/metrics wrapper.
        #[cfg(feature = "database")]
        {
            use crate::application::ports::user_repository::RepositoryError as RepoErr;

            // Prefer container-provided use-case when available
            if let Some(container) = self.container.as_ref() {
                let uc = container.create_user();
                return timed_op!(self, "db", async move {
                    match uc.execute(request).await {
                        Ok(u) => Ok(u),
                        Err(RepoErr::Conflict(s)) => Err(crate::AppError::Conflict(s)),
                        Err(RepoErr::NotFound) => {
                            Err(crate::AppError::NotFound("User not found".to_string()))
                        }
                        Err(RepoErr::Unexpected(s)) => Err(crate::AppError::Internal(s)),
                    }
                });
            }

            use crate::application::use_cases::CreateUserUseCase;
            use crate::infrastructure::repositories::DieselUserRepository;
            use std::sync::Arc;

            let repo = DieselUserRepository::new(self.database.clone());
            let uc = CreateUserUseCase::new(Arc::new(repo));
            timed_op!(self, "db", async move {
                match uc.execute(request).await {
                    Ok(u) => Ok(u),
                    Err(RepoErr::Conflict(s)) => Err(crate::AppError::Conflict(s)),
                    Err(RepoErr::NotFound) => {
                        Err(crate::AppError::NotFound("User not found".to_string()))
                    }
                    Err(RepoErr::Unexpected(s)) => Err(crate::AppError::Internal(s)),
                }
            })
        }

        // Fallback (or when database feature not present): keep existing direct DB helper
        #[cfg(not(feature = "database"))]
        {
            timed_op!(self, "db", self.database.create_user(request))
        }
    }

    #[cfg(feature = "auth")]
    /// ユーザー認証を実行します。
    ///
    /// # Errors
    ///
    /// 資格情報が不正、または内部処理に失敗した場合にエラーを返します。
    pub async fn auth_authenticate(
        &self,
        request: crate::auth::LoginRequest,
    ) -> crate::Result<crate::models::User> {
        timed_op!(self, "auth", self.auth.authenticate_user(request))
    }

    #[cfg(feature = "auth")]
    /// セッションを作成します。
    ///
    /// # Errors
    ///
    /// セッション発行時の保存や暗号化処理に失敗した場合にエラーを返します。
    pub async fn auth_create_session(&self, user_id: uuid::Uuid) -> crate::Result<String> {
        timed_op!(self, "auth", self.auth.create_session(user_id))
    }

    #[cfg(feature = "auth")]
    /// `AuthResponse` を生成します。
    ///
    /// # Errors
    ///
    /// トークン生成やユーザー情報組み立てに失敗した場合にエラーを返します。
    pub async fn auth_build_auth_response(
        &self,
        user: crate::models::User,
        remember_me: bool,
    ) -> crate::Result<crate::auth::AuthResponse> {
        timed_op!(
            self,
            "auth",
            self.auth.create_auth_response(user, remember_me)
        )
    }

    /// Convenience wrapper: directly build unified `AuthSuccessResponse` (tokens + user (+ deprecated flat fields)).
    /// Use this in handlers instead of manually converting `AuthResponse`.
    /// NOTE: Keeps backward compatibility because the underlying creation path is unchanged.
    #[cfg(feature = "auth")]
    ///
    /// # Errors
    ///
    /// 内部の `auth_build_auth_response` が失敗した場合にエラーを返します。
    pub async fn auth_build_success_response(
        &self,
        user: crate::models::User,
        remember_me: bool,
    ) -> crate::Result<crate::utils::auth_response::AuthSuccessResponse> {
        let auth = self.auth_build_auth_response(user, remember_me).await?; // metrics already recorded in inner call
        Ok(crate::utils::auth_response::AuthSuccessResponse::from(auth))
    }

    #[cfg(feature = "auth")]
    /// アクセストークンをリフレッシュします。
    ///
    /// # Errors
    ///
    /// リフレッシュトークンが不正、または内部の検証/保存処理に失敗した場合にエラーを返します。
    pub async fn auth_refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> crate::Result<(
        crate::utils::auth_response::AuthTokens,
        crate::utils::common_types::UserInfo,
    )> {
        timed_op!(self, "auth", self.auth.refresh_access_token(refresh_token))
    }

    /// Convenience wrapper: refresh using a refresh token and return unified `AuthSuccessResponse` directly.
    #[cfg(feature = "auth")]
    ///
    /// # Errors
    ///
    /// 内部の `auth_refresh_access_token` が失敗した場合にエラーを返します。
    pub async fn auth_refresh_success_response(
        &self,
        refresh_token: &str,
    ) -> crate::Result<crate::utils::auth_response::AuthSuccessResponse> {
        let (tokens, user) = self.auth_refresh_access_token(refresh_token).await?; // metrics recorded in inner call
        Ok(crate::utils::auth_response::AuthSuccessResponse::from_parts(&tokens, user))
    }

    #[cfg(feature = "auth")]
    /// セッションIDを指定して現在のセッションを無効化します。
    ///
    /// # Errors
    ///
    /// セッションの削除処理に失敗した場合にエラーを返します。
    pub async fn auth_logout(&self, session_id: &str) -> crate::Result<()> {
        timed_op!(self, "auth", self.auth.logout(session_id))
    }

    /// Validate a token using the `AuthService`; returns the authenticated user on success and records an auth attempt
    #[cfg(feature = "auth")]
    ///
    /// # Errors
    ///
    /// トークンが不正、または検証過程で失敗した場合にエラーを返します。
    pub async fn auth_validate_token(&self, token: &str) -> crate::Result<crate::models::User> {
        timed_op!(self, "auth", self.auth.validate_token(token))
    }

    #[cfg(feature = "auth")]
    /// ビスケットトークンを検証します。
    ///
    /// # Errors
    ///
    /// ビスケットトークンの検証に失敗した場合にエラーを返します。
    pub async fn auth_verify_biscuit(
        &self,
        token: &str,
    ) -> crate::Result<crate::auth::AuthContext> {
        timed_op!(self, "auth", self.auth.verify_biscuit(token))
    }

    #[cfg(feature = "auth")]
    /// `API Key` に基づいて `Biscuit` ベースの `AuthContext` を作成します。
    ///
    /// `API Key` 認証を経由した場合も、システム内では統一的に `Biscuit` ベースの認証コンテキストを
    /// 使用することで、認証メカニズムを統一化します。
    ///
    /// # Errors
    ///
    /// ユーザーが見つからない場合やコンテキストの作成に失敗した場合にエラーを返します。
    pub async fn auth_create_biscuit_from_api_key(
        &self,
        user_id: uuid::Uuid,
        permissions: Vec<String>,
    ) -> crate::Result<crate::auth::AuthContext> {
        timed_op!(
            self,
            "auth",
            self.auth.create_biscuit_from_api_key(user_id, permissions)
        )
    }

    /// Health check wrapper for `AuthService` that records timing
    #[cfg(feature = "auth")]
    ///
    /// # Errors
    ///
    /// 健康チェックの内部処理で失敗した場合にエラーを返します。
    pub async fn auth_health_check(&self) -> crate::Result<crate::app::ServiceHealth> {
        // auth の内部 DB 呼び出しを個別にカウントする必要があれば AuthService 側で timed 化する想定
        Ok(self.check_auth_health().await)
    }

    // --- Database wrapper helpers that record metrics centrally on AppState ---
    #[cfg(feature = "database")]
    /// ユーザーを作成します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や保存処理に失敗した場合にエラーを返します。
    pub async fn db_create_user(
        &self,
        req: crate::models::CreateUserRequest,
    ) -> crate::Result<crate::models::User> {
        timed_op!(self, "db", self.database.create_user(req))
    }

    #[cfg(feature = "database")]
    /// ユーザーIDでユーザーを取得します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_user_by_id(&self, id: uuid::Uuid) -> crate::Result<crate::models::User> {
        timed_op!(self, "db", self.database.get_user_by_id(id))
    }

    #[cfg(feature = "database")]
    /// メールアドレスでユーザーを取得します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_user_by_email(&self, email: &str) -> crate::Result<crate::models::User> {
        timed_op!(self, "db", self.database.get_user_by_email(email))
    }

    #[cfg(feature = "database")]
    /// ユーザー一覧を取得します（フィルタ/ソート対応）。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_users(
        &self,
        page: u32,
        limit: u32,
        role: Option<String>,
        active: Option<bool>,
        sort: Option<String>,
    ) -> crate::Result<Vec<crate::models::User>> {
        timed_op!(self, "db", async {
            self.database.get_users(page, limit, role, active, sort)
        })
    }

    #[cfg(feature = "database")]
    /// 最終ログイン時刻を更新します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や更新クエリに失敗した場合にエラーを返します。
    pub async fn db_update_last_login(&self, id: uuid::Uuid) -> crate::Result<()> {
        timed_op!(self, "db", async { self.database.update_last_login(id) })
    }

    // Additional user helpers used by handlers/CLI
    #[cfg(feature = "database")]
    /// ユーザー名でユーザーを取得します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_user_by_username(
        &self,
        username: &str,
    ) -> crate::Result<crate::models::User> {
        timed_op!(self, "db", self.database.get_user_by_username(username))
    }

    #[cfg(feature = "database")]
    /// ユーザー情報を更新します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や更新クエリに失敗した場合にエラーを返します。
    pub async fn db_update_user(
        &self,
        id: uuid::Uuid,
        request: crate::models::UpdateUserRequest,
    ) -> crate::Result<crate::models::User> {
        let user = timed_op!(self, "db", async {
            self.database.update_user(id, &request)
        })?;
        #[cfg(feature = "cache")]
        self.invalidate_user_caches(id).await;
        Ok(user)
    }

    #[cfg(feature = "database")]
    /// ユーザーを削除します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や削除クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_delete_user(&self, id: uuid::Uuid) -> crate::Result<()> {
        timed_op!(self, "db", async { self.database.delete_user(id) })?;
        #[cfg(feature = "cache")]
        self.invalidate_user_caches(id).await;
        Ok(())
    }

    #[cfg(feature = "database")]
    /// ユーザーパスワードをリセットします。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や更新処理に失敗した場合にエラーを返します。
    pub async fn db_reset_user_password(
        &self,
        id: uuid::Uuid,
        new_password: &str,
    ) -> crate::Result<()> {
        timed_op!(self, "db", async {
            self.database.reset_user_password(id, new_password)
        })
    }

    #[cfg(feature = "database")]
    /// ユーザー数を返します。
    ///
    /// # Errors
    ///
    /// 集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_count_users(&self) -> crate::Result<usize> {
        timed_op!(self, "db", async { self.database.count_users() })
    }

    #[cfg(feature = "database")]
    /// 条件付きのユーザー数を返します。
    ///
    /// # Errors
    ///
    /// 集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_count_users_filtered(
        &self,
        role: Option<String>,
        active: Option<bool>,
    ) -> crate::Result<usize> {
        timed_op!(self, "db", async {
            self.database.count_users_filtered(role, active)
        })
    }

    #[cfg(feature = "database")]
    /// 投稿を作成します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や保存処理に失敗した場合にエラーを返します。
    pub async fn db_create_post(
        &self,
        req: crate::models::CreatePostRequest,
    ) -> crate::Result<crate::models::Post> {
        timed_op!(self, "db", async { self.database.create_post(req) })
    }

    #[cfg(feature = "database")]
    /// 投稿IDで投稿を取得します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_post_by_id(&self, id: uuid::Uuid) -> crate::Result<crate::models::Post> {
        timed_op!(self, "db", async { self.database.get_post_by_id(id) })
    }

    #[cfg(feature = "database")]
    /// 投稿一覧を取得します（フィルタ/ソート対応）。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_posts(
        &self,
        page: u32,
        limit: u32,
        status: Option<String>,
        author: Option<uuid::Uuid>,
        tag: Option<String>,
        sort: Option<String>,
    ) -> crate::Result<Vec<crate::models::Post>> {
        timed_op!(self, "db", async {
            self.database
                .get_posts(page, limit, status, author, tag, sort)
        })
    }

    #[cfg(feature = "database")]
    /// 投稿を更新します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や更新クエリに失敗した場合にエラーを返します。
    pub async fn db_update_post(
        &self,
        id: uuid::Uuid,
        req: crate::models::UpdatePostRequest,
    ) -> crate::Result<crate::models::Post> {
        timed_op!(self, "db", async { self.database.update_post(id, &req) })
    }

    #[cfg(feature = "database")]
    /// 投稿を削除します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や削除クエリの実行に失敗した場合、または対象の投稿が見つからない場合にエラーを返します。
    pub async fn db_delete_post(&self, id: uuid::Uuid) -> crate::Result<()> {
        timed_op!(self, "db", async { self.database.delete_post(id) })
    }

    #[cfg(feature = "database")]
    /// 投稿数を返します（任意のタグでフィルター）。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_count_posts(&self, tag: Option<&str>) -> crate::Result<usize> {
        timed_op!(self, "db", async { self.database.count_posts(tag) })
    }

    #[cfg(feature = "database")]
    /// 指定ユーザーの投稿数を返します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_count_posts_by_author(&self, author_id: uuid::Uuid) -> crate::Result<usize> {
        timed_op!(self, "db", async {
            self.database.count_posts_by_author(author_id)
        })
    }

    #[cfg(feature = "database")]
    /// ステータス/著者/タグでフィルターした投稿数を返します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_count_posts_filtered(
        &self,
        status: Option<String>,
        author: Option<uuid::Uuid>,
        tag: Option<String>,
    ) -> crate::Result<usize> {
        timed_op!(self, "db", async {
            self.database.count_posts_filtered(status, author, tag)
        })
    }

    // --- API Key wrappers ---
    // NOTE: 他の DB ラッパは timed_db! macro を直接使えるが、ここは一部で in-place クロージャを
    // 使っており都度 start/elapsed を書いていたため共通化。
    #[cfg(all(feature = "database", feature = "auth"))]
    /// 新しい API キーを作成します。
    ///
    /// # Errors
    ///
    /// - 入力値の検証に失敗した場合（名前や権限が不正など）。
    /// - データベース接続の取得に失敗した場合。
    /// - 生成した API キーの保存に失敗した場合。
    pub async fn db_create_api_key(
        &self,
        name: String,
        user_id: uuid::Uuid,
        permissions: Vec<String>,
    ) -> crate::Result<(crate::models::ApiKeyResponse, String)> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let (model, raw) = ApiKey::new_validated(name, user_id, permissions)?;
            let mut conn = self.database.get_connection()?;
            let saved = ApiKey::create(&mut conn, &model)?;
            Ok((saved.to_response(), raw))
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// API キー ID から API キーを取得します。
    ///
    /// # Errors
    ///
    /// - データベース接続の取得に失敗した場合。
    /// - 該当する API キーが存在しない、または取得時にエラーが発生した場合。
    pub async fn db_get_api_key(
        &self,
        id: uuid::Uuid,
    ) -> crate::Result<crate::models::ApiKeyResponse> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let model = ApiKey::find_by_id(&mut conn, id)?;
            Ok(model.to_response())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// API キーを削除します。
    ///
    /// # Errors
    ///
    /// - データベース接続の取得や削除クエリの実行に失敗した場合。
    /// - 指定した ID の API キーが存在しない場合は `NotFound` エラーを返します。
    pub async fn db_delete_api_key(&self, key_id: uuid::Uuid) -> crate::Result<()> {
        use crate::database::schema::api_keys::dsl::{api_keys, id as api_key_id};
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let affected =
                diesel::delete(api_keys.filter(api_key_id.eq(key_id))).execute(&mut conn)?;
            if affected == 0 {
                return Err(crate::AppError::NotFound("api key not found".to_string()));
            }
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// 既存の API キーを新しい API キーにローテーションし、旧キーを失効させます。
    ///
    /// # Errors
    ///
    /// - 元の API キー取得に失敗した場合（存在しない等）。
    /// - 新しい API キー生成・保存に失敗した場合。
    /// - 旧キーの失効更新（`expires_at` の更新）でエラーが発生した場合。
    pub async fn db_rotate_api_key(
        &self,
        id: uuid::Uuid,
        new_name: Option<String>,
        new_permissions: Option<Vec<String>>,
    ) -> crate::Result<(crate::models::ApiKeyResponse, String)> {
        // Fetch existing
        let existing = self.db_get_api_key_model(id).await?;
        let name = new_name.unwrap_or_else(|| existing.name.clone());
        let perms = new_permissions.unwrap_or_else(|| existing.get_permissions());
        // Create replacement (same user)
        let (new_model_resp, raw) = self
            .db_create_api_key(name, existing.user_id, perms)
            .await?;
        // Expire old key (soft: set expires_at = now)
        #[cfg(all(feature = "database", feature = "auth"))]
        {
            use crate::database::schema::api_keys::dsl::{api_keys, expires_at};
            use diesel::prelude::*;
            let mut conn = self.database.get_connection()?;
            let now = chrono::Utc::now();
            diesel::update(api_keys.find(existing.id))
                .set(expires_at.eq(Some(now)))
                .execute(&mut conn)?;
        }
        Ok((new_model_resp, raw))
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// API キーを失効（削除）させます。
    ///
    /// # Errors
    ///
    /// - データベース接続の取得や削除処理に失敗した場合。
    /// - 指定 ID の API キーが存在しない場合には `NotFound` 等のエラーが返る可能性があります。
    pub async fn db_revoke_api_key(&self, id: uuid::Uuid) -> crate::Result<()> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            ApiKey::delete(&mut conn, id)?;
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// 所有者チェック込みで API キーを削除します。
    ///
    /// # Errors
    ///
    /// - DB 接続/削除クエリ実行に失敗した場合。
    /// - 指定 ID が存在するが所有者が一致しない場合は Authorization エラー。
    /// - 指定 ID が存在しない場合は `NotFound` エラー。
    pub async fn db_revoke_api_key_owned(
        &self,
        key_id: uuid::Uuid,
        user: uuid::Uuid,
    ) -> crate::Result<()> {
        use crate::database::schema::api_keys::dsl::{api_keys, id as api_key_id, user_id};
        use crate::models::ApiKey;
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let affected =
                diesel::delete(api_keys.filter(api_key_id.eq(key_id).and(user_id.eq(user))))
                    .execute(&mut conn)?;
            if affected == 0 {
                let exists = ApiKey::find_by_id(&mut conn, key_id).ok();
                if exists.is_some() {
                    return Err(crate::AppError::Authorization("not owner".to_string()));
                }
                return Err(crate::AppError::NotFound("api key not found".to_string()));
            }
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// API キーの最終使用時刻を更新します。
    ///
    /// # Errors
    ///
    /// データベース接続や更新処理に失敗した場合にエラーを返します。
    pub async fn db_touch_api_key(&self, id: uuid::Uuid) -> crate::Result<()> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            ApiKey::update_last_used(&mut conn, id)?;
            Ok(())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// ユーザーの API キー一覧を返します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や取得クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_list_api_keys(
        &self,
        user_id: uuid::Uuid,
        include_expired: bool,
    ) -> crate::Result<Vec<crate::models::ApiKeyResponse>> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let models = ApiKey::list_for_user(&mut conn, user_id, include_expired)?;
            Ok(models.into_iter().map(|m| m.to_response()).collect())
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// lookup ハッシュで API キーを取得します。
    ///
    /// # Errors
    ///
    /// データベース接続や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_api_key_by_lookup_hash(
        &self,
        lookup: &str,
    ) -> crate::Result<crate::models::ApiKey> {
        use crate::models::ApiKey;
        let lookup = lookup.to_string();
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let model = ApiKey::find_by_lookup_hash(&mut conn, &lookup)?;
            Ok(model)
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// API キー ID から API キーのモデルを取得します。
    ///
    /// # Errors
    ///
    /// データベース接続や取得クエリに失敗した場合にエラーを返します。
    pub async fn db_get_api_key_model(
        &self,
        id: uuid::Uuid,
    ) -> crate::Result<crate::models::ApiKey> {
        use crate::models::ApiKey;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let model = ApiKey::find_by_id(&mut conn, id)?;
            Ok(model)
        })
    }

    /// Backfill `api_key_lookup_hash` for legacy rows (where it's an empty string), using a raw API key.
    /// Returns `Some(ApiKey)` if a matching legacy key was found and updated; `None` otherwise.
    ///
    /// # Errors
    ///
    /// - データベース接続や取得/更新クエリの実行に失敗した場合。
    /// - ハッシュ計算や検証処理で問題が発生した場合。
    #[cfg(all(feature = "database", feature = "auth"))]
    pub async fn db_backfill_api_key_lookup_for_raw(
        &self,
        raw: &str,
    ) -> crate::Result<Option<crate::models::ApiKey>> {
        use crate::database::schema::api_keys::dsl::{api_key_lookup_hash, api_keys};
        use crate::models::ApiKey as ApiKeyModel;
        use diesel::prelude::*;

        let raw = raw.to_string();
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            // fetch candidates with empty lookup
            let mut candidates: Vec<ApiKeyModel> = api_keys
                .filter(api_key_lookup_hash.eq(""))
                .load(&mut conn)?;

            // try to find a verifying match
            for cand in &mut candidates {
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
    /// 管理者用: 最近の投稿一覧を取得します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得やクエリ実行に失敗した場合、エラーを返します。
    pub async fn db_admin_list_recent_posts(
        &self,
        limit: i64,
    ) -> crate::Result<Vec<crate::utils::common_types::PostSummary>> {
        use diesel::prelude::*;
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
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;

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
    /// lookup ハッシュが未設定の API キー一覧を返します。
    ///
    /// # Errors
    ///
    /// データベース接続や取得クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_list_api_keys_missing_lookup(
        &self,
    ) -> crate::Result<Vec<crate::models::ApiKey>> {
        use crate::database::schema::api_keys::dsl::{api_key_lookup_hash, api_keys};
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let rows: Vec<crate::models::ApiKey> = api_keys
                .filter(api_key_lookup_hash.eq(""))
                .load(&mut conn)?;
            Ok(rows)
        })
    }

    #[cfg(all(feature = "database", feature = "auth"))]
    /// lookup ハッシュ未設定の API キーを一括で失効させます。
    ///
    /// # Errors
    ///
    /// データベース接続や更新クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_expire_api_keys_missing_lookup(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> crate::Result<usize> {
        use crate::database::schema::api_keys::dsl::{api_key_lookup_hash, api_keys, expires_at};
        use diesel::prelude::*;
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
    /// 管理者用: 指定投稿を削除します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や削除クエリの実行に失敗、または該当投稿が存在しない場合にエラーを返します。
    pub async fn db_admin_delete_post(&self, post_id: uuid::Uuid) -> crate::Result<()> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let affected = diesel::delete(posts_dsl::posts.filter(posts_dsl::id.eq(post_id)))
                .execute(&mut conn)?;
            if affected == 0 {
                return Err(crate::AppError::NotFound("post not found".to_string()));
            }
            Ok(())
        })
    }

    #[cfg(feature = "database")]
    /// 管理者用: ユーザー数を返します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_admin_users_count(&self) -> crate::Result<i64> {
        use crate::database::schema::users::dsl as users_dsl;
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let count: i64 = users_dsl::users.count().get_result(&mut conn)?;
            Ok(count)
        })
    }

    #[cfg(feature = "database")]
    /// 管理者用: 投稿数を返します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や集計クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_admin_posts_count(&self) -> crate::Result<i64> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let count: i64 = posts_dsl::posts.count().get_result(&mut conn)?;
            Ok(count)
        })
    }

    #[cfg(feature = "database")]
    /// 管理者用: admin ユーザーを検索します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得や検索クエリの実行に失敗した場合にエラーを返します。
    pub async fn db_admin_find_admin_user(&self) -> crate::Result<Option<crate::models::User>> {
        use crate::database::schema::users::dsl as users_dsl;
        use diesel::prelude::*;
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
    ///
    /// # Errors
    ///
    /// データベース接続の取得や SQL 実行に失敗した場合にエラーを返します。
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

    /// Fetch applied migration versions from `schema_migrations` or `__diesel_schema_migrations`
    ///
    /// # Errors
    ///
    /// データベース接続やクエリ実行/フォールバック時の処理に失敗した場合にエラーを返します。
    #[cfg(feature = "database")]
    pub async fn db_fetch_applied_migrations(&self) -> crate::Result<Vec<String>> {
        use diesel::prelude::*;
        #[derive(diesel::QueryableByName)]
        struct MigrationVersion {
            #[diesel(sql_type = diesel::sql_types::Text)]
            version: String,
        }
        timed_op!(self, "db", async {
            let mut conn = self.database.get_connection()?;
            let try_primary: std::result::Result<Vec<MigrationVersion>, diesel::result::Error> =
                diesel::sql_query("SELECT version FROM schema_migrations ORDER BY version ASC")
                    .load(&mut conn);
            let rows = match try_primary {
                Ok(r) => r,
                Err(_) => diesel::sql_query(
                    "SELECT version FROM __diesel_schema_migrations ORDER BY version ASC",
                )
                .load(&mut conn)
                .map_err(|e| crate::AppError::Internal(e.to_string()))?,
            };
            Ok(rows.into_iter().map(|r| r.version).collect())
        })
    }

    /// Ensure `schema_migrations` exists and copy rows from legacy table
    ///
    /// # Errors
    ///
    /// SQL 実行に失敗した場合にエラーを返します。
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
    /// 保留中のマイグレーションを適用します。
    ///
    /// # Errors
    ///
    /// DB 接続の取得やマイグレーション適用処理に失敗した場合にエラーを返します。
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
    /// 直前のマイグレーションをロールバックします。
    ///
    /// # Errors
    ///
    /// DB 接続の取得やロールバック処理に失敗した場合にエラーを返します。
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
    /// 保留中のマイグレーション名の一覧を返します。
    ///
    /// # Errors
    ///
    /// データベース接続の取得やマイグレーション情報の取得に失敗した場合、エラーを返します。
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

    // ============================================================================
    // Event Emission Helper Methods
    // ============================================================================
    //
    // These methods encapsulate event creation and emission, providing a clean
    // API for handlers to notify the system of domain events. The fire-and-forget
    // pattern ensures that event emission never fails the primary operation.

    /// Emit a UserCreated event
    ///
    /// This method extracts the essential user data and broadcasts a UserCreated
    /// event to all listeners. It uses the fire-and-forget pattern, so failures
    /// are silently ignored (the event bus returns Err when there are no subscribers).
    #[cfg(feature = "database")]
    pub fn emit_user_created(&self, user: &crate::models::User) {
        let event_data = crate::events::UserEventData::from_user(user);
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::UserCreated(event_data));
    }

    /// Emit a UserUpdated event
    #[cfg(feature = "database")]
    pub fn emit_user_updated(&self, user: &crate::models::User) {
        let event_data = crate::events::UserEventData::from_user(user);
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::UserUpdated(event_data));
    }

    /// Emit a UserDeleted event
    pub fn emit_user_deleted(&self, user_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::UserDeleted(user_id));
    }

    /// Emit a PostCreated event
    #[cfg(feature = "database")]
    pub fn emit_post_created(&self, post: &crate::models::Post) {
        let event_data = crate::events::PostEventData::from_post(post);
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::PostCreated(event_data));
    }

    /// Emit a PostUpdated event
    #[cfg(feature = "database")]
    pub fn emit_post_updated(&self, post: &crate::models::Post) {
        let event_data = crate::events::PostEventData::from_post(post);
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::PostUpdated(event_data));
    }

    /// Emit a PostDeleted event
    pub fn emit_post_deleted(&self, post_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::PostDeleted(post_id));
    }

    /// Emit a PostPublished event
    #[cfg(feature = "database")]
    pub fn emit_post_published(&self, post: &crate::models::Post) {
        let event_data = crate::events::PostEventData::from_post(post);
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::PostPublished(event_data));
    }

    // Compatibility no-ops when DB feature is disabled
    #[cfg(not(feature = "database"))]
    pub fn emit_user_created(&self, _user: &crate::models::User) {
        // no-op
    }

    #[cfg(not(feature = "database"))]
    pub fn emit_user_updated(&self, _user: &crate::models::User) {
        // no-op
    }

    /// Emit a CommentCreated event (placeholder)
    /// DEPRECATED: Phase 5 で削除予定。新しい Use Case は構造体形式を使用
    #[deprecated(note = "Use structured AppEvent::CommentCreated { .. } instead")]
    pub fn emit_comment_created(&self, _comment_id: uuid::Uuid) {
        // このメソッドは Phase 5 で削除予定
        // 新しい Use Case は AppEvent::CommentCreated { comment_id, post_id, author_id, text } を使用
    }

    /// Emit a CommentUpdated event (placeholder)
    pub fn emit_comment_updated(&self, comment_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::CommentUpdated(comment_id));
    }

    /// Emit a CommentDeleted event (placeholder)
    pub fn emit_comment_deleted(&self, comment_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::CommentDeleted(comment_id));
    }

    /// Emit a CategoryCreated event (placeholder)
    pub fn emit_category_created(&self, category_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::CategoryCreated(category_id));
    }

    /// Emit a CategoryUpdated event (placeholder)
    pub fn emit_category_updated(&self, category_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::CategoryUpdated(category_id));
    }

    /// Emit a CategoryDeleted event (placeholder)
    pub fn emit_category_deleted(&self, category_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::CategoryDeleted(category_id));
    }

    /// Emit a TagCreated event (placeholder)
    pub fn emit_tag_created(&self, tag_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::TagCreated(tag_id));
    }

    /// Emit a TagUpdated event (placeholder)
    pub fn emit_tag_updated(&self, tag_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::TagUpdated(tag_id));
    }

    /// Emit a TagDeleted event (placeholder)
    pub fn emit_tag_deleted(&self, tag_id: uuid::Uuid) {
        let _ = self
            .event_bus
            .send(crate::events::AppEvent::TagDeleted(tag_id));
    }
}

// When the `database` feature is disabled provide no-op stubs for the
// AppState database wrapper methods so the rest of the crate can compile and
// present a clear runtime error if invoked.
#[cfg(not(feature = "database"))]
impl AppState {
    async fn db_not_enabled<T>(&self) -> crate::Result<T> {
        Err(crate::AppError::NotImplemented(
            "database feature not enabled".into(),
        ))
    }

    // The following methods mirror the signatures present when `database`
    // feature is enabled and forward to a uniform NotImplemented error.
    pub async fn db_create_user(
        &self,
        _req: crate::models::CreateUserRequest,
    ) -> crate::Result<crate::models::User> {
        self.db_not_enabled().await
    }
    pub async fn db_get_user_by_id(&self, _id: uuid::Uuid) -> crate::Result<crate::models::User> {
        self.db_not_enabled().await
    }
    pub async fn db_get_user_by_email(&self, _email: &str) -> crate::Result<crate::models::User> {
        self.db_not_enabled().await
    }
    pub async fn db_get_users(
        &self,
        _page: u32,
        _limit: u32,
        _role: Option<String>,
        _active: Option<bool>,
        _sort: Option<String>,
    ) -> crate::Result<Vec<crate::models::User>> {
        self.db_not_enabled().await
    }
    pub async fn db_update_last_login(&self, _id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_get_user_by_username(
        &self,
        _username: &str,
    ) -> crate::Result<crate::models::User> {
        self.db_not_enabled().await
    }
    pub async fn db_update_user(
        &self,
        _id: uuid::Uuid,
        _request: crate::models::UpdateUserRequest,
    ) -> crate::Result<crate::models::User> {
        self.db_not_enabled().await
    }
    pub async fn db_delete_user(&self, _id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_reset_user_password(
        &self,
        _id: uuid::Uuid,
        _new_password: &str,
    ) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_count_users(&self) -> crate::Result<usize> {
        self.db_not_enabled().await
    }
    pub async fn db_count_users_filtered(
        &self,
        _role: Option<String>,
        _active: Option<bool>,
    ) -> crate::Result<usize> {
        self.db_not_enabled().await
    }

    pub async fn db_create_post(
        &self,
        _req: crate::models::CreatePostRequest,
    ) -> crate::Result<crate::models::Post> {
        self.db_not_enabled().await
    }
    pub async fn db_get_post_by_id(&self, _id: uuid::Uuid) -> crate::Result<crate::models::Post> {
        self.db_not_enabled().await
    }
    pub async fn db_get_posts(
        &self,
        _page: u32,
        _limit: u32,
        _status: Option<String>,
        _author: Option<uuid::Uuid>,
        _tag: Option<String>,
        _sort: Option<String>,
    ) -> crate::Result<Vec<crate::models::Post>> {
        self.db_not_enabled().await
    }
    pub async fn db_update_post(
        &self,
        _id: uuid::Uuid,
        _req: crate::models::UpdatePostRequest,
    ) -> crate::Result<crate::models::Post> {
        self.db_not_enabled().await
    }
    pub async fn db_delete_post(&self, _id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_count_posts(&self, _tag: Option<&str>) -> crate::Result<usize> {
        self.db_not_enabled().await
    }
    pub async fn db_count_posts_by_author(&self, _author_id: uuid::Uuid) -> crate::Result<usize> {
        self.db_not_enabled().await
    }
    pub async fn db_count_posts_filtered(
        &self,
        _status: Option<String>,
        _author: Option<uuid::Uuid>,
        _tag: Option<String>,
    ) -> crate::Result<usize> {
        self.db_not_enabled().await
    }

    pub async fn db_create_api_key(
        &self,
        _name: String,
        _user_id: uuid::Uuid,
        _permissions: Vec<String>,
    ) -> crate::Result<(crate::models::ApiKeyResponse, String)> {
        self.db_not_enabled().await
    }
    pub async fn db_get_api_key(
        &self,
        _id: uuid::Uuid,
    ) -> crate::Result<crate::models::ApiKeyResponse> {
        self.db_not_enabled().await
    }
    pub async fn db_delete_api_key(&self, _key_id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_rotate_api_key(
        &self,
        _id: uuid::Uuid,
        _new_name: Option<String>,
        _new_permissions: Option<Vec<String>>,
    ) -> crate::Result<(crate::models::ApiKeyResponse, String)> {
        self.db_not_enabled().await
    }
    pub async fn db_revoke_api_key(&self, _id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_revoke_api_key_owned(
        &self,
        _key_id: uuid::Uuid,
        _user: uuid::Uuid,
    ) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_touch_api_key(&self, _id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_list_api_keys(
        &self,
        _user_id: uuid::Uuid,
        _include_expired: bool,
    ) -> crate::Result<Vec<crate::models::ApiKeyResponse>> {
        self.db_not_enabled().await
    }
    pub async fn db_get_api_key_by_lookup_hash(
        &self,
        _lookup: &str,
    ) -> crate::Result<crate::models::ApiKey> {
        self.db_not_enabled().await
    }
    pub async fn db_get_api_key_model(
        &self,
        _id: uuid::Uuid,
    ) -> crate::Result<crate::models::ApiKey> {
        self.db_not_enabled().await
    }
    pub async fn db_backfill_api_key_lookup_for_raw(
        &self,
        _raw: &str,
    ) -> crate::Result<Option<crate::models::ApiKey>> {
        self.db_not_enabled().await
    }

    pub async fn db_admin_list_recent_posts(
        &self,
        _limit: i64,
    ) -> crate::Result<Vec<crate::utils::common_types::PostSummary>> {
        self.db_not_enabled().await
    }
    pub async fn db_list_api_keys_missing_lookup(
        &self,
    ) -> crate::Result<Vec<crate::models::ApiKey>> {
        self.db_not_enabled().await
    }
    pub async fn db_expire_api_keys_missing_lookup(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> crate::Result<usize> {
        self.db_not_enabled().await
    }
    pub async fn db_admin_delete_post(&self, _post_id: uuid::Uuid) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_admin_users_count(&self) -> crate::Result<i64> {
        self.db_not_enabled().await
    }
    pub async fn db_admin_posts_count(&self) -> crate::Result<i64> {
        self.db_not_enabled().await
    }
    pub async fn db_admin_find_admin_user(&self) -> crate::Result<Option<crate::models::User>> {
        self.db_not_enabled().await
    }
    pub async fn db_execute_sql(&self, _sql: &str) -> crate::Result<usize> {
        self.db_not_enabled().await
    }
    pub async fn db_fetch_applied_migrations(&self) -> crate::Result<Vec<String>> {
        self.db_not_enabled().await
    }
    pub async fn db_ensure_schema_migrations_compat(&self) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_run_pending_migrations<T>(&self, _migrations: T) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_revert_last_migration<T>(&self, _migrations: T) -> crate::Result<()> {
        self.db_not_enabled().await
    }
    pub async fn db_list_pending_migrations<T>(
        &self,
        _migrations: T,
    ) -> crate::Result<Vec<String>> {
        self.db_not_enabled().await
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // =========================================================================
    // AppStateBuilder Tests (初期化ロジック)
    // =========================================================================

    #[test]
    fn test_app_state_builder_creates_default_metrics() {
        let config = Arc::new(Config::default());
        let metrics = Arc::new(RwLock::new(AppMetrics::default()));
        let _builder = AppStateBuilder {
            config,
            metrics: metrics.clone(),
            start_time: Instant::now(),
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "database")]
            container: None,
            event_bus: None,
            #[cfg(feature = "auth")]
            auth: None,
            #[cfg(feature = "cache")]
            cache: None,
            #[cfg(feature = "search")]
            search: None,
        };

        // Verify builder properties
        let metrics_guard = metrics.blocking_read();
        assert_eq!(metrics_guard.total_requests, 0);
        assert_eq!(metrics_guard.active_connections, 0);
    }

    #[test]
    fn test_app_metrics_default_initialization() {
        let metrics = AppMetrics::default();

        // All request counters should be 0
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.active_connections, 0);

        // Cache metrics should be 0
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);

        // Search metrics should be 0
        assert_eq!(metrics.search_queries, 0);
        assert_eq!(metrics.search_avg_response_time_ms, 0.0);

        // Auth metrics should be 0
        assert_eq!(metrics.auth_attempts, 0);
        assert_eq!(metrics.auth_successes, 0);
        assert_eq!(metrics.auth_failures, 0);

        // DB metrics should be 0
        assert_eq!(metrics.db_queries, 0);
        assert_eq!(metrics.db_avg_response_time_ms, 0.0);

        // Error counts should be 0
        assert_eq!(metrics.errors_total, 0);
        assert_eq!(metrics.errors_auth, 0);
        assert_eq!(metrics.errors_db, 0);
    }

    #[test]
    fn test_app_metrics_clone() {
        let metrics = AppMetrics {
            total_requests: 100,
            active_connections: 5,
            cache_hits: 50,
            cache_misses: 10,
            ..Default::default()
        };

        let cloned = metrics.clone();
        assert_eq!(cloned.total_requests, 100);
        assert_eq!(cloned.active_connections, 5);
        assert_eq!(cloned.cache_hits, 50);
        assert_eq!(cloned.cache_misses, 10);
    }

    // =========================================================================
    // ServiceHealth Tests (health check 構造)
    // =========================================================================

    #[test]
    fn test_service_health_up_status() {
        let health = ServiceHealth {
            status: "up".to_string(),
            response_time_ms: 10.5,
            error: None,
            details: serde_json::json!({}),
        };

        assert_eq!(health.status, "up");
        assert_eq!(health.response_time_ms, 10.5);
        assert!(health.error.is_none());
    }

    #[test]
    fn test_service_health_down_status_with_error() {
        let health = ServiceHealth {
            status: "down".to_string(),
            response_time_ms: 5000.0,
            error: Some("Connection timeout".to_string()),
            details: serde_json::json!({
                "reason": "Database unreachable"
            }),
        };

        assert_eq!(health.status, "down");
        assert!(health.error.is_some());
        assert_eq!(health.error.unwrap(), "Connection timeout".to_string());
    }

    #[test]
    fn test_service_health_degraded_status() {
        let health = ServiceHealth {
            status: "degraded".to_string(),
            response_time_ms: 2000.0,
            error: None,
            details: serde_json::json!({
                "message": "Performance degradation"
            }),
        };

        assert_eq!(health.status, "degraded");
        assert!(health.error.is_none());
        assert!(!health.response_time_ms.is_infinite());
    }

    // =========================================================================
    // HealthStatus Tests (レスポンス構造)
    // =========================================================================

    #[test]
    fn test_health_status_basic() {
        let now = chrono::Utc::now();
        let response = HealthStatus {
            status: "healthy".to_string(),
            database: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 5.0,
                error: None,
                details: serde_json::json!({}),
            },
            cache: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 2.0,
                error: None,
                details: serde_json::json!({}),
            },
            search: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 10.0,
                error: None,
                details: serde_json::json!({}),
            },
            auth: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 1.0,
                error: None,
                details: serde_json::json!({}),
            },
            timestamp: now,
        };

        assert_eq!(response.status, "healthy");
    }

    #[test]
    fn test_health_status_multiple_services() {
        let now = chrono::Utc::now();
        let response = HealthStatus {
            status: "degraded".to_string(),
            database: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 5.0,
                error: None,
                details: serde_json::json!({}),
            },
            cache: ServiceHealth {
                status: "degraded".to_string(),
                response_time_ms: 500.0,
                error: Some("Slow response".to_string()),
                details: serde_json::json!({}),
            },
            search: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 10.0,
                error: None,
                details: serde_json::json!({}),
            },
            auth: ServiceHealth {
                status: "up".to_string(),
                response_time_ms: 1.0,
                error: None,
                details: serde_json::json!({}),
            },
            timestamp: now,
        };

        assert_eq!(response.status, "degraded");
    }

    // =========================================================================
    // Failure Mode Tests (エラーハンドリング)
    // =========================================================================

    #[test]
    fn test_failure_mode_down() {
        let mode = FailureMode::Down;
        match mode {
            FailureMode::Down => {}
            FailureMode::Degraded => panic!("Expected Down"),
        }
    }

    #[test]
    fn test_failure_mode_degraded() {
        let mode = FailureMode::Degraded;
        match mode {
            FailureMode::Down => panic!("Expected Degraded"),
            FailureMode::Degraded => {}
        }
    }

    // =========================================================================
    // Service Health Helpers Tests
    // =========================================================================

    #[test]
    fn test_service_not_configured() {
        let health = service_not_configured("Feature not enabled");

        assert_eq!(health.status, "not_configured");
        assert_eq!(health.response_time_ms, 0.0);
        assert!(health.error.is_none());
        assert_eq!(health.details["message"], "Feature not enabled");
    }

    #[tokio::test]
    async fn test_to_service_health_down_on_error() {
        let fut = async {
            // Simulate an error
            Err::<(), _>(crate::error::AppError::Internal("Test error".to_string()))
        };

        let health = to_service_health(fut, FailureMode::Down).await;
        assert_eq!(health.status, "down");
        assert!(health.error.is_some());
    }

    #[tokio::test]
    async fn test_to_service_health_ok_on_success() {
        let fut = async {
            // Simulate a success
            Ok::<(), crate::error::AppError>(())
        };

        let health = to_service_health(fut, FailureMode::Down).await;
        assert_eq!(health.status, "up");
        assert!(health.error.is_none());
        assert!(health.response_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_to_service_health_degraded_on_error() {
        let fut = async {
            Err::<(), _>(crate::error::AppError::Internal(
                "Partial failure".to_string(),
            ))
        };

        let health = to_service_health(fut, FailureMode::Degraded).await;
        assert_eq!(health.status, "degraded");
        assert!(health.error.is_some());
    }

    // =========================================================================
    // Metrics State Mutation Tests (ユースケース)
    // =========================================================================

    #[test]
    fn test_metrics_increment_total_requests() {
        let mut metrics = AppMetrics::default();
        assert_eq!(metrics.total_requests, 0);

        metrics.total_requests = 100;
        assert_eq!(metrics.total_requests, 100);
    }

    #[test]
    fn test_metrics_increment_cache_operations() {
        let metrics = AppMetrics {
            cache_hits: 10,
            cache_misses: 2,
            ..Default::default()
        };

        assert_eq!(metrics.cache_hits, 10);
        assert_eq!(metrics.cache_misses, 2);
    }

    #[test]
    fn test_metrics_error_tracking() {
        let metrics = AppMetrics {
            errors_total: 5,
            errors_auth: 2,
            errors_db: 3,
            ..Default::default()
        };

        assert_eq!(metrics.errors_total, 5);
        assert_eq!(metrics.errors_auth, 2);
        assert_eq!(metrics.errors_db, 3);
    }

    #[test]
    fn test_metrics_averages_computation() {
        let metrics = AppMetrics {
            db_avg_response_time_ms: 15.5,
            search_avg_response_time_ms: 50.0,
            ..Default::default()
        };

        assert!(metrics.db_avg_response_time_ms < metrics.search_avg_response_time_ms);
    }

    // =========================================================================
    // Edge Cases & Boundary Tests
    // =========================================================================

    #[test]
    fn test_app_metrics_large_values() {
        let metrics = AppMetrics {
            total_requests: u64::MAX / 2,
            cache_hits: u64::MAX / 2,
            search_avg_response_time_ms: f64::MAX / 2.0,
            ..Default::default()
        };

        assert!(metrics.total_requests > 0);
        assert!(metrics.cache_hits > 0);
        assert!(metrics.search_avg_response_time_ms > 0.0);
    }

    #[test]
    fn test_service_health_response_times_consistency() {
        let times = vec![1.5, 5.0, 10.0, 50.0, 1000.0];

        for time in times {
            let health = ServiceHealth {
                status: "up".to_string(),
                response_time_ms: time,
                error: None,
                details: serde_json::json!({}),
            };

            assert_eq!(health.response_time_ms, time);
            assert!(health.response_time_ms > 0.0);
        }
    }

    #[test]
    fn test_app_state_builder_event_bus_optional() {
        let config = Arc::new(Config::default());
        let builder = AppStateBuilder {
            config,
            metrics: Arc::new(RwLock::new(AppMetrics::default())),
            start_time: Instant::now(),
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "database")]
            container: None,
            event_bus: None,
            #[cfg(feature = "auth")]
            auth: None,
            #[cfg(feature = "cache")]
            cache: None,
            #[cfg(feature = "search")]
            search: None,
        };

        assert!(builder.event_bus.is_none());
    }
}
