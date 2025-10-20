//! Application State - DDD準拠の新実装（Phase 5）
//!
//! 本モジュールは CMS の中核状態 `AppState` を提供します。
//! レガシー実装（src/app.rs）を完全削除し、DDD原則に沿ったクリーンな実装に置き換えます。
//!
//! ## 機能
//! - Database/Auth/Cache/Search サービスの統合
//! - EventBus 統合（domain events 発行）
//! - Metrics 収集
//! - Builder パターンによる段階的初期化
//! - Feature flags による条件付きコンパイル
//!
//! ## 設計方針
//! - domain層の型のみ使用（旧models依存を完全排除）
//! - Repository/Use Case 経由のアクセス
//! - 最小限のAPI提供（サービス取得、イベント発行、ヘルスチェック）
//!
//! ## Phase 5.1 Update
//! database/cache/searchサービスを段階的に統合中です。

use crate::{AppError, Config, Result};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

// Database support
#[cfg(feature = "database")]
use crate::infrastructure::database::connection::DatabasePool;

// Cache support
#[cfg(feature = "cache")]
use parking_lot::RwLock;
#[cfg(feature = "cache")]
use std::collections::HashMap;

// Auth support (Phase 5.5)
#[cfg(feature = "auth")]
use crate::auth::{InMemorySessionStore, SessionStore};

// Events module location depends on feature flag
#[cfg(not(feature = "restructure_domain"))]
use crate::events::AppEvent;
#[cfg(feature = "restructure_domain")]
use crate::infrastructure::events::AppEvent;

/// EventBus type alias
#[cfg(not(feature = "restructure_domain"))]
pub type EventBus = broadcast::Sender<crate::events::AppEvent>;
#[cfg(feature = "restructure_domain")]
pub type EventBus = broadcast::Sender<crate::infrastructure::events::AppEvent>;

/// Central application state for the CMS
///
/// AppStateはアプリケーションの中核状態を管理します。
/// Phase 5.1: database/cache サービスを統合
/// Phase 5.4: JWT 認証サービスを統合
/// Phase 5.5: セッションストアを統合
#[derive(Clone)]
pub struct AppState {
    /// Application configuration (public for backward compatibility)
    pub config: Arc<Config>,

    /// Event bus for domain events
    event_bus: EventBus,

    /// Database connection pool (optional)
    #[cfg(feature = "database")]
    database: Option<DatabasePool>,

    /// In-memory cache (optional)
    #[cfg(feature = "cache")]
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,

    /// JWT authentication service (Phase 5.4)
    #[cfg(feature = "auth")]
    jwt_service: Option<Arc<crate::auth::JwtService>>,

    /// Session store (Phase 5.5)
    #[cfg(feature = "auth")]
    session_store: Option<Arc<dyn SessionStore>>,
}

impl AppState {
    /// Returns a builder for constructing AppState
    pub fn builder(config: Config) -> AppStateBuilder {
        AppStateBuilder {
            config: Arc::new(config),
            #[cfg(feature = "database")]
            database: None,
            #[cfg(feature = "auth")]
            jwt_service: None,
            #[cfg(feature = "auth")]
            session_store: None,
        }
    }

    /// Get reference to configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get reference to event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get reference to JWT service (Phase 5.4)
    #[cfg(feature = "auth")]
    pub fn jwt_service(&self) -> Option<&Arc<crate::auth::JwtService>> {
        self.jwt_service.as_ref()
    }

    /// Get reference to session store (Phase 5.5)
    #[cfg(feature = "auth")]
    pub fn session_store(&self) -> Option<&Arc<dyn SessionStore>> {
        self.session_store.as_ref()
    }

    /// Get database pool (if available)
    #[cfg(feature = "database")]
    pub fn database(&self) -> Option<&DatabasePool> {
        self.database.as_ref()
    }

    /// Get database pool or return error
    #[cfg(feature = "database")]
    pub fn database_required(&self) -> Result<&DatabasePool> {
        self.database
            .as_ref()
            .ok_or_else(|| AppError::Internal("Database not initialized".to_string()))
    }

    /// Get cache reference (if available)
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Arc<RwLock<HashMap<String, Vec<u8>>>> {
        Arc::clone(&self.cache)
    }

    /// Emit a domain event to the event bus
    ///
    /// Fire-and-Forget設計: エラーは無視します
    #[cfg(not(feature = "restructure_domain"))]
    pub fn emit_event(&self, event: crate::events::AppEvent) {
        let _ = self.event_bus.send(event);
    }

    /// Emit a domain event to the event bus (restructure_domain version)
    #[cfg(feature = "restructure_domain")]
    pub fn emit_event(&self, event: crate::infrastructure::events::AppEvent) {
        let _ = self.event_bus.send(event);
    }

    /// Get user repository
    #[cfg(all(feature = "database", feature = "restructure_domain"))]
    pub fn user_repository(
        &self,
    ) -> Result<crate::infrastructure::database::repositories::DieselUserRepository> {
        use crate::infrastructure::database::repositories::DieselUserRepository;
        let pool = self.database_required()?.get_pool();
        Ok(DieselUserRepository::new(pool))
    }

    /// Get post repository
    #[cfg(all(feature = "database", feature = "restructure_domain"))]
    pub fn post_repository(
        &self,
    ) -> Result<crate::infrastructure::database::repositories::DieselPostRepository> {
        use crate::infrastructure::database::repositories::DieselPostRepository;
        let pool = self.database_required()?.get_pool();
        Ok(DieselPostRepository::new(pool))
    }

    /// Get comment repository
    #[cfg(all(feature = "database", feature = "restructure_domain"))]
    pub fn comment_repository(
        &self,
    ) -> Result<crate::infrastructure::database::repositories::DieselCommentRepository> {
        use crate::infrastructure::database::repositories::DieselCommentRepository;
        let pool = self.database_required()?.get_pool();
        Ok(DieselCommentRepository::new(pool))
    }

    /// Health check - verify all services are operational
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let mut status = HealthStatus {
            database: "unavailable".to_string(),
            cache: "unavailable".to_string(),
            search: "unavailable".to_string(),
        };

        #[cfg(feature = "database")]
        if let Some(db) = &self.database {
            status.database = match db.health_check() {
                Ok(_) => "healthy".to_string(),
                Err(_) => "unhealthy".to_string(),
            };
        }

        #[cfg(feature = "cache")]
        {
            // Cache is always available if feature is enabled
            status.cache = "healthy".to_string();
        }

        // TODO: Add search health check when implemented

        Ok(status)
    }

    /// Graceful shutdown - cleanup resources
    pub async fn shutdown(&self) {
        info!("Starting AppState shutdown...");

        // Future: Add cleanup for background tasks, connections, etc.
        #[cfg(feature = "database")]
        if self.database.is_some() {
            info!("Closing database connections...");
            // Connection pool will be dropped automatically when AppState is dropped
        }

        info!("AppState shutdown complete");
    }

    /// Start background session cleanup task (Phase 5.5.3)
    ///
    /// Spawns a background task that periodically removes expired sessions.
    /// The task runs every hour.
    #[cfg(feature = "auth")]
    pub fn start_session_cleanup(&self) {
        if let Some(session_store) = self.session_store.clone() {
            tokio::spawn(async move {
                use chrono::Utc;
                use tokio::time::{Duration, interval};

                let mut cleanup_interval = interval(Duration::from_secs(3600)); // 1 hour

                loop {
                    cleanup_interval.tick().await;
                    let now = Utc::now();
                    session_store.cleanup_expired(now).await;
                    info!("Session cleanup completed");
                }
            });
            info!("Session cleanup task started");
        }
    }
}

/// Health status for all services
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub database: String,
    pub cache: String,
    pub search: String,
}

/// Builder for AppState
///
/// AppStateBuilderは段階的にサービスを初期化します。
/// Phase 5.1: database/cache サービスを統合
/// Phase 5.4: JWT サービスを統合
/// Phase 5.5: セッションストアを統合
pub struct AppStateBuilder {
    config: Arc<Config>,

    #[cfg(feature = "database")]
    database: Option<DatabasePool>,

    #[cfg(feature = "auth")]
    jwt_service: Option<Arc<crate::auth::JwtService>>,

    #[cfg(feature = "auth")]
    session_store: Option<Arc<dyn SessionStore>>,
}

impl AppStateBuilder {
    /// Initialize database connection pool
    #[cfg(feature = "database")]
    pub fn with_database(mut self) -> Result<Self> {
        use secrecy::ExposeSecret;

        info!("Initializing database connection pool...");

        // Get database URL from config
        let database_url = self.config.database.url.expose_secret();

        match DatabasePool::new(database_url) {
            Ok(pool) => {
                info!("Database pool initialized successfully");
                self.database = Some(pool);
                Ok(self)
            }
            Err(e) => {
                warn!("Failed to initialize database: {}", e);
                // Return builder with no database (optional feature)
                Ok(self)
            }
        }
    }

    /// Initialize JWT service (Phase 5.4)
    #[cfg(feature = "auth")]
    pub fn with_jwt_service(mut self, jwt_service: crate::auth::JwtService) -> Self {
        info!("Initializing JWT service...");
        self.jwt_service = Some(Arc::new(jwt_service));
        self
    }

    /// Initialize session store (Phase 5.5)
    #[cfg(feature = "auth")]
    pub fn with_session_store_arc(mut self, session_store: Arc<dyn SessionStore>) -> Self {
        info!("Initializing session store...");
        self.session_store = Some(session_store);
        self
    }

    /// Initialize session store from concrete implementation
    #[cfg(feature = "auth")]
    pub fn with_session_store<S>(self, session_store: S) -> Self
    where
        S: SessionStore + 'static,
    {
        self.with_session_store_arc(Arc::new(session_store))
    }

    /// Initialize in-memory session store (convenience)
    #[cfg(feature = "auth")]
    pub fn with_in_memory_session_store(self) -> Self {
        self.with_session_store(InMemorySessionStore::new())
    }

    /// Build AppState
    ///
    /// EventBusを作成し、全てのサービスを統合します。
    pub fn build(self) -> Result<AppState> {
        info!("Building AppState...");

        // Create event bus (capacity: 1000)
        let (event_bus, _) = broadcast::channel(1000);

        #[cfg(feature = "cache")]
        let cache = Arc::new(RwLock::new(HashMap::new()));

        #[cfg(feature = "auth")]
        let session_store = self
            .session_store
            .unwrap_or_else(|| Arc::new(InMemorySessionStore::new()) as Arc<dyn SessionStore>);

        let state = AppState {
            config: self.config,
            event_bus,
            #[cfg(feature = "database")]
            database: self.database,
            #[cfg(feature = "cache")]
            cache,
            #[cfg(feature = "auth")]
            jwt_service: self.jwt_service,
            #[cfg(feature = "auth")]
            session_store: Some(session_store),
        };

        info!("AppState built successfully");
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let config = Config::default();
        let builder = AppState::builder(config);
        // Builder should be created successfully
        assert!(
            builder.config.server.host == "127.0.0.1" || builder.config.server.host == "0.0.0.0"
        );
    }

    #[test]
    fn test_builder_build() {
        let config = Config::default();
        let builder = AppState::builder(config);
        let state = builder.build().unwrap();

        // AppState should be built successfully
        // Config default host can be either 127.0.0.1 or 0.0.0.0 depending on environment
        assert!(
            state.config().server.host == "127.0.0.1" || state.config().server.host == "0.0.0.0"
        );
    }

    #[test]
    fn test_event_bus_creation() {
        let (_event_bus, _rx) = broadcast::channel::<AppEvent>(10);

        // Channel should be created successfully
        // Note: Actual event sending/receiving tested elsewhere
    }
}
