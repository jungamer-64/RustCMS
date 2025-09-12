use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub environment: String,
    #[cfg(feature = "database")]
    #[serde(default)]
    pub database: DatabaseConfig,
    #[cfg(feature = "cache")]
    #[serde(default)]
    pub redis: RedisConfig,
    #[cfg(feature = "search")]
    #[serde(default)]
    pub search: SearchConfig,
    #[cfg(feature = "auth")]
    #[serde(default)]
    pub auth: AuthConfig,
    pub media: MediaConfig,
    pub notifications: NotificationConfig,
    pub security: SecurityConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_request_size: usize,
    pub request_timeout: u64,
    pub worker_threads: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
    pub enable_migrations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub default_ttl: u64,
    pub key_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub index_path: PathBuf,
    pub writer_memory: usize,
    pub max_results: usize,
    pub enable_fuzzy: bool,
    pub fuzzy_distance: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub biscuit_root_key: String,
    pub webauthn: WebAuthnConfig,
    pub bcrypt_cost: u32,
    pub session_timeout: u64,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnConfig {
    pub rp_id: String,
    pub rp_name: String,
    pub rp_origin: String,
    pub timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
    pub upload_dir: PathBuf,
    pub max_file_size: usize,
    pub allowed_types: Vec<String>,
    pub thumbnail_sizes: Vec<(u32, u32)>,
    pub cdn_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    pub email: EmailConfig,
    pub webhooks: WebhookConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub from_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub timeout: u64,
    pub retry_attempts: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            environment: "development".to_string(),
            #[cfg(feature = "database")]
            database: DatabaseConfig::default(),
            #[cfg(feature = "cache")]
            redis: RedisConfig::default(),
            #[cfg(feature = "search")]
            search: SearchConfig::default(),
            #[cfg(feature = "auth")]
            auth: AuthConfig::default(),
            media: MediaConfig::default(),
            notifications: NotificationConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub cors_origins: Vec<String>,
    pub rate_limit_requests: u64,
    pub rate_limit_window: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_origins: vec!["http://localhost:3000".to_string()],
            rate_limit_requests: 100,
            rate_limit_window: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String, // "text" | "json"
}

fn default_log_level() -> String {
    "info".into()
}
fn default_log_format() -> String {
    "text".into()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default)]
    pub enable_metrics: bool,
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
    #[serde(default = "default_health_interval")]
    pub health_check_interval: u64,
}

fn default_metrics_port() -> u16 {
    9090
}
fn default_health_interval() -> u64 {
    30
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: false,
            metrics_port: default_metrics_port(),
            health_check_interval: default_health_interval(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout: 30,
            worker_threads: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/cms".to_string(),
            max_connections: 20,
            min_connections: 5,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
            enable_migrations: true,
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            default_ttl: 3600,
            key_prefix: "cms:".to_string(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("./data/search_index"),
            writer_memory: 50_000_000, // 50MB
            max_results: 100,
            enable_fuzzy: true,
            fuzzy_distance: 2,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            biscuit_root_key: "your-biscuit-root-key".to_string(),
            webauthn: WebAuthnConfig::default(),
            bcrypt_cost: 12,
            session_timeout: 86_400, // 24 hours
            access_token_ttl_secs: 3_600,
            refresh_token_ttl_secs: 86_400,
        }
    }
}

impl Default for WebAuthnConfig {
    fn default() -> Self {
        Self {
            rp_id: "localhost".to_string(),
            rp_name: "CMS".to_string(),
            rp_origin: "http://localhost:3000".to_string(),
            timeout: 60_000,
        }
    }
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            upload_dir: PathBuf::from("uploads"),
            max_file_size: 50 * 1024 * 1024, // 50MB
            allowed_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "application/pdf".to_string(),
            ],
            thumbnail_sizes: vec![(150, 150), (300, 300), (600, 600)],
            cdn_url: None,
        }
    }
}

// NotificationConfig derives Default

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            from_address: "noreply@example.com".to_string(),
            from_name: "CMS".to_string(),
        }
    }
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout: 30,
            retry_attempts: 3,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, crate::AppError> {
        dotenvy::dotenv().ok();
        // プロファイル (development / production / staging など) を環境変数から取得
        let profile = env::var("CMS_PROFILE")
            .or_else(|_| env::var("RUN_MODE"))
            .unwrap_or_else(|_| "development".to_string());
        // 基本: default → {profile}.toml → local.toml → env(CMS_*) の順で上書き
        let mut builder = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false));
        if profile != "development" {
            let profile_path = format!("config/{}", profile);
            builder = builder.add_source(
                config::File::with_name(&profile_path).required(false),
            );
        }
        builder = builder
            .add_source(config::File::with_name("config/local").required(false))
            .add_source(config::Environment::with_prefix("CMS").separator("_"));

        let raw = builder
            .build()
            .map_err(|e| crate::AppError::Config(e.to_string()))?;
        let mut cfg: Self = raw
            .try_deserialize()
            .map_err(|e| crate::AppError::Config(e.to_string()))?;
        cfg.environment = profile;

        // If the config contains the literal "${DATABASE_URL}", expand it from the environment
        // This keeps secrets out of the repo while allowing config files to reference the env var.
        #[cfg(feature = "database")]
        {
            // If DATABASE_URL is present in the environment (for example from a .env file),
            // prefer it over values from config files. This allows developers to set
            // DATABASE_URL in `.env` during local development.
            if let Ok(real) = env::var("DATABASE_URL") {
                if !real.is_empty() {
                    cfg.database.url = real;
                }
            } else if cfg.database.url.contains("${DATABASE_URL}") {
                // If the config explicitly references the placeholder, fail if env var is missing.
                return Err(crate::AppError::Config(
                    "DATABASE_URL must be set when referenced in config".to_string(),
                ));
            }
        }

        // 追加: LOG_LEVEL / LOG_FORMAT 環境変数があれば上書き
        if let Ok(lvl) = env::var("LOG_LEVEL") && !lvl.is_empty() {
            cfg.logging.level = lvl;
        }
        if let Ok(fmt) = env::var("LOG_FORMAT") && !fmt.is_empty() {
            cfg.logging.format = fmt;
        }

        // 旧 production_config.rs / simple_config.rs 由来の変数を一部マッピング (後方互換)
        if let Ok(v) = env::var("ACCESS_TOKEN_TTL_SECS")
            && let Ok(n) = v.parse()
        {
            cfg.auth.access_token_ttl_secs = n;
        }
        if let Ok(v) = env::var("REFRESH_TOKEN_TTL_SECS")
            && let Ok(n) = v.parse()
        {
            cfg.auth.refresh_token_ttl_secs = n;
        }

        // sanitize cors origins (toml で文字列カンマ列だった場合をケア) ※ production.toml 古い形式互換
        if cfg.security.cors_origins.len() == 1 && cfg.security.cors_origins[0].contains(',') {
            let joined = cfg.security.cors_origins[0].clone();
            cfg.security.cors_origins = joined
                .split(',')
                .map(str::trim)
                .map(str::to_string)
                .filter(|s| !s.is_empty())
                .collect();
        }

        // warn if deprecated envs used
        for k in [
            "PORT",
            "HOST",
            "RATE_LIMIT_REQUESTS",
            "RATE_LIMIT_WINDOW_SECONDS",
        ] {
            if env::var(k).is_ok() {
                warn!(
                    "deprecated env var '{k}' detected; use CMS_SERVER__* or CMS_SECURITY__* via prefixed config"
                );
            }
        }

        Ok(cfg)
    }
}
