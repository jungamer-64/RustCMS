use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use uuid::Uuid;

use crate::database::postgres::{Database, DbPool};
use crate::services::{
    biscuit_auth::BiscuitAuthService,
    webauthn::WebAuthnService,
    elasticsearch::ElasticsearchService,
};
use crate::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub elasticsearch_url: String,
    pub elasticsearch_index: String,
    pub domain: String,
    pub origin: String,
    pub upload_dir: String,
    pub max_file_size: u64,
    pub bcrypt_cost: u32,
    pub redis_url: Option<String>,
    pub environment: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://cms:cms@localhost/cms".to_string()),
            elasticsearch_url: std::env::var("ELASTICSEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:9200".to_string()),
            elasticsearch_index: std::env::var("ELASTICSEARCH_INDEX")
                .unwrap_or_else(|_| "cms_posts".to_string()),
            domain: std::env::var("WEBAUTHN_DOMAIN")
                .unwrap_or_else(|_| "localhost".to_string()),
            origin: std::env::var("WEBAUTHN_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            upload_dir: std::env::var("UPLOAD_DIR")
                .unwrap_or_else(|_| "uploads".to_string()),
            max_file_size: std::env::var("MAX_FILE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10 * 1024 * 1024), // 10MB
            bcrypt_cost: std::env::var("BCRYPT_COST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(12),
            redis_url: std::env::var("REDIS_URL").ok(),
            environment: std::env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub pool: DbPool,
    pub config: Arc<AppConfig>,
    pub biscuit_auth: Arc<BiscuitAuthService>,
    pub webauthn: Arc<WebAuthnService>,
    pub elasticsearch: Arc<ElasticsearchService>,
    pub session_store: Arc<DashMap<String, SessionData>>,
    pub rate_limiter: Arc<DashMap<String, RateLimitData>>,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub user_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct RateLimitData {
    pub count: u32,
    pub window_start: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, AppError> {
        // Initialize database
        let db = Database::new()?;
        let pool = db.pool.clone();

        // Run migrations
        db.run_migrations()?;

        // Initialize services
        let biscuit_auth = Arc::new(BiscuitAuthService::new()?);
        
        let webauthn = Arc::new(WebAuthnService::new(
            &config.domain,
            &config.origin,
        )?);

        let elasticsearch = Arc::new(ElasticsearchService::new(
            &config.elasticsearch_url,
            config.elasticsearch_index.clone(),
        ).await?);

        Ok(Self {
            db,
            pool,
            config: Arc::new(config),
            biscuit_auth,
            webauthn,
            elasticsearch,
            session_store: Arc::new(DashMap::new()),
            rate_limiter: Arc::new(DashMap::new()),
        })
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        // Check database connection
        self.db.health_check().await?;
        
        // TODO: Add Elasticsearch health check
        // TODO: Add Redis health check if enabled
        
        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) {
        let now = chrono::Utc::now();
        self.session_store.retain(|_, session| session.expires_at > now);
    }

    pub async fn cleanup_rate_limits(&self) {
        let now = chrono::Utc::now();
        let window_duration = chrono::Duration::minutes(1);
        
        self.rate_limiter.retain(|_, rate_limit| {
            now.signed_duration_since(rate_limit.window_start) < window_duration
        });
    }

    pub fn get_db_connection(&self) -> Result<crate::database::postgres::DbConnection, AppError> {
        self.db.get_conn()
    }
}
