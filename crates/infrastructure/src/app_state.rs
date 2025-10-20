//! Application-wide shared state.
//!
//! Provides access to configuration, database pools, cache, and event bus.

use crate::common::{InfrastructureError, InfrastructureResult};
use crate::config::Config;
use crate::events::AppEvent;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

#[cfg(feature = "database")]
use crate::database::connection::DatabasePool;
#[cfg(feature = "database")]
use secrecy::ExposeSecret;

#[cfg(feature = "cache")]
use parking_lot::RwLock;
#[cfg(feature = "cache")]
use std::collections::HashMap;

#[cfg(feature = "auth")]
use crate::auth::{InMemorySessionStore, SessionStore};

/// Broadcast channel used to publish domain events.
pub type EventBus = broadcast::Sender<AppEvent>;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    event_bus: EventBus,
    #[cfg(feature = "database")]
    database: Option<DatabasePool>,
    #[cfg(feature = "cache")]
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    #[cfg(feature = "auth")]
    jwt_service: Option<Arc<crate::auth::JwtService>>,
    #[cfg(feature = "auth")]
    session_store: Option<Arc<dyn SessionStore>>, // dyn for extensibility
}

impl AppState {
    /// Start building a new [`AppState`].
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

    /// Access configuration reference.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Access the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Access the JWT service if configured.
    #[cfg(feature = "auth")]
    pub fn jwt_service(&self) -> Option<&Arc<crate::auth::JwtService>> {
        self.jwt_service.as_ref()
    }

    /// Access the session store if configured.
    #[cfg(feature = "auth")]
    pub fn session_store(&self) -> Option<&Arc<dyn SessionStore>> {
        self.session_store.as_ref()
    }

    /// Access database pool if available.
    #[cfg(feature = "database")]
    pub fn database(&self) -> Option<&DatabasePool> {
        self.database.as_ref()
    }

    /// Require a database pool or return an error.
    #[cfg(feature = "database")]
    pub fn database_required(&self) -> InfrastructureResult<&DatabasePool> {
        self.database
            .as_ref()
            .ok_or_else(|| InfrastructureError::DatabaseError("Database not initialized".into()))
    }

    /// Access cache handle if enabled.
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Arc<RwLock<HashMap<String, Vec<u8>>>> {
        Arc::clone(&self.cache)
    }

    /// Publish an event via the event bus (fire-and-forget).
    pub fn emit_event(&self, event: AppEvent) {
        let _ = self.event_bus.send(event);
    }

    /// Obtain a Diesel-backed user repository.
    #[cfg(feature = "database")]
    pub fn user_repository(
        &self,
    ) -> InfrastructureResult<crate::database::repositories::DieselUserRepository> {
        use crate::database::repositories::DieselUserRepository;
        let pool = self.database_required()?.get_pool();
        Ok(DieselUserRepository::new(pool))
    }

    /// Obtain a Diesel-backed post repository.
    #[cfg(feature = "database")]
    pub fn post_repository(
        &self,
    ) -> InfrastructureResult<crate::database::repositories::DieselPostRepository> {
        use crate::database::repositories::DieselPostRepository;
        let pool = self.database_required()?.get_pool();
        Ok(DieselPostRepository::new(pool))
    }

    /// Obtain a Diesel-backed comment repository.
    #[cfg(feature = "database")]
    pub fn comment_repository(
        &self,
    ) -> InfrastructureResult<crate::database::repositories::DieselCommentRepository> {
        use crate::database::repositories::DieselCommentRepository;
        let pool = self.database_required()?.get_pool();
        Ok(DieselCommentRepository::new(pool))
    }

    /// Gather simple health indicators for infrastructure components.
    pub async fn health_check(&self) -> InfrastructureResult<HealthStatus> {
        let mut status = HealthStatus::default();

        #[cfg(feature = "database")]
        {
            status.database = match self.database.as_ref() {
                Some(db) => match db.health_check() {
                    Ok(_) => "healthy".into(),
                    Err(_) => "unhealthy".into(),
                },
                None => "unavailable".into(),
            };
        }

        #[cfg(feature = "cache")]
        {
            status.cache = "healthy".into();
        }

        Ok(status)
    }

    /// Cleanup resources.
    pub async fn shutdown(&self) {
        info!("Starting AppState shutdown...");

        #[cfg(feature = "database")]
        if self.database.is_some() {
            info!("Closing database connections...");
        }

        info!("AppState shutdown complete");
    }

    /// Spawn background cleanup for session store.
    #[cfg(feature = "auth")]
    pub fn start_session_cleanup(&self) {
        if let Some(session_store) = self.session_store.clone() {
            tokio::spawn(async move {
                use chrono::Utc;
                use tokio::time::{interval, Duration};

                let mut cleanup_interval = interval(Duration::from_secs(3600));
                loop {
                    cleanup_interval.tick().await;
                    session_store.cleanup_expired(Utc::now()).await;
                    info!("Session cleanup completed");
                }
            });
            info!("Session cleanup task started");
        }
    }
}

/// Overall infrastructure health snapshot.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub database: String,
    pub cache: String,
    pub search: String,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            database: "unavailable".into(),
            cache: "unavailable".into(),
            search: "unavailable".into(),
        }
    }
}

/// Builder for [`AppState`].
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
    /// Configure database pool.
    #[cfg(feature = "database")]
    pub fn with_database(mut self) -> InfrastructureResult<Self> {
        info!("Initializing database connection pool...");

        let database_url = self.config.database.url.expose_secret().to_owned();
        match DatabasePool::new(&database_url) {
            Ok(pool) => {
                info!("Database pool initialized successfully");
                self.database = Some(pool);
                Ok(self)
            }
            Err(e) => {
                warn!("Failed to initialize database: {e}");
                Ok(self)
            }
        }
    }

    /// Configure JWT service.
    #[cfg(feature = "auth")]
    pub fn with_jwt_service(mut self, jwt_service: crate::auth::JwtService) -> Self {
        self.jwt_service = Some(Arc::new(jwt_service));
        self
    }

    /// Configure a custom session store.
    #[cfg(feature = "auth")]
    pub fn with_session_store_arc(mut self, session_store: Arc<dyn SessionStore>) -> Self {
        self.session_store = Some(session_store);
        self
    }

    /// Configure a custom session store from a concrete type.
    #[cfg(feature = "auth")]
    pub fn with_session_store<S>(self, session_store: S) -> Self
    where
        S: SessionStore + 'static,
    {
        self.with_session_store_arc(Arc::new(session_store))
    }

    /// Use an in-memory session store.
    #[cfg(feature = "auth")]
    pub fn with_in_memory_session_store(self) -> Self {
        self.with_session_store(InMemorySessionStore::new())
    }

    /// Finalise and build [`AppState`].
    pub fn build(self) -> InfrastructureResult<AppState> {
        info!("Building AppState...");
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
    fn builder_initialises() {
        let config = Config::default();
        let builder = AppState::builder(config);
        assert!(!builder.config.environment.is_empty());
    }

    #[test]
    fn build_creates_state() {
        let config = Config::default();
        let builder = AppState::builder(config);
        let state = builder.build().unwrap();
        assert!(!state.config().environment.is_empty());
    }
}
