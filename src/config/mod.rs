//! 設定モジュール
//!
//! 本モジュールはアプリ全体の設定値を表す型定義と、設定の読み込みロジックを提供します。
//! 読み込みの優先順位は次のとおりです（後勝ちで上書き）：
//! 1) `config/default.toml`
//! 2) `config/{profile}.toml`（例: production, staging。development 以外のときに適用）
//! 3) `config/local.toml`（ローカル開発者向けの上書き）
//! 4) 環境変数 `CMS__*`(例: `CMS__SERVER__PORT=3000`) で統合し、`AppState` 構築時に使用します。

use secrecy::ExposeSecret;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(serialize_with = "serialize_secret_masked")]
    pub url: SecretString,
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
    #[serde(serialize_with = "serialize_secret_masked")]
    pub biscuit_root_key: SecretString,
    pub webauthn: WebAuthnConfig,
    pub bcrypt_cost: u32,
    pub session_timeout: u64,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub remember_me_access_ttl_secs: u64,
    #[serde(default = "default_role_permissions")]
    pub role_permissions: HashMap<String, Vec<String>>,
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
    #[serde(serialize_with = "serialize_secret_masked")]
    pub smtp_password: SecretString,
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

fn default_enable_login_rate_limiting() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub cors_origins: Vec<String>,
    pub rate_limit_requests: u64,
    pub rate_limit_window: u64,
    #[serde(default = "default_enable_login_rate_limiting")]
    pub enable_login_rate_limiting: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_origins: vec![String::from("http://localhost:3000")],
            rate_limit_requests: 100,
            rate_limit_window: 60,
            enable_login_rate_limiting: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub format: LogFormat,
}

fn default_log_level() -> String {
    // String cannot be returned from a const fn yet in stable, but helper stays small and pure
    String::from("info")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: LogFormat::default(),
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

const fn default_metrics_port() -> u16 {
    9090
}
const fn default_health_interval() -> u64 {
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

fn default_role_permissions() -> HashMap<String, Vec<String>> {
    let mut perms = HashMap::new();
    perms.insert(
        "super_admin".to_string(),
        vec![
            "admin".to_string(),
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
        ],
    );
    perms.insert(
        "admin".to_string(),
        vec![
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
        ],
    );
    perms.insert(
        "editor".to_string(),
        vec!["read".to_string(), "write".to_string()],
    );
    perms.insert(
        "author".to_string(),
        vec!["read".to_string(), "write_own".to_string()],
    );
    perms.insert("contributor".to_string(), vec!["read".to_string()]);
    perms.insert("subscriber".to_string(), vec!["read".to_string()]);
    perms
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: String::from(Self::DEFAULT_HOST),
            port: Self::DEFAULT_PORT,
            max_request_size: Self::DEFAULT_MAX_REQUEST_SIZE,
            request_timeout: Self::DEFAULT_REQUEST_TIMEOUT,
            worker_threads: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: SecretString::new("postgresql://localhost/cms".into()),
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
            url: String::from("redis://localhost:6379"),
            pool_size: 10,
            default_ttl: 3600,
            key_prefix: String::from("cms:"),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("./data/search_index"),
            writer_memory: Self::DEFAULT_WRITER_MEMORY,
            max_results: Self::DEFAULT_MAX_RESULTS,
            enable_fuzzy: true,
            fuzzy_distance: 2,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            biscuit_root_key: SecretString::new("your-biscuit-root-key".into()),
            webauthn: WebAuthnConfig::default(),
            bcrypt_cost: 12,
            session_timeout: 86_400, // 24 hours
            access_token_ttl_secs: 3_600,
            refresh_token_ttl_secs: 86_400,
            remember_me_access_ttl_secs: 86_400, // 24 hours
            role_permissions: default_role_permissions(),
        }
    }
}

impl Default for WebAuthnConfig {
    fn default() -> Self {
        Self {
            rp_id: String::from("localhost"),
            rp_name: String::from("CMS"),
            rp_origin: String::from("http://localhost:3000"),
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
            smtp_username: String::new(),
            smtp_password: SecretString::new(String::new().into()),
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
    /// 環境変数と設定ファイルから `Config` を構築します。
    ///
    /// # Errors
    ///
    /// 設定ファイルの読み込みやデシリアライズに失敗した場合、
    /// もしくは `DATABASE_URL` プレースホルダーが設定内にあり環境変数が未設定の場合にエラーを返します。
    pub fn from_env() -> Result<Self, crate::AppError> {
        dotenvy::dotenv().ok();

        let profile = read_profile();
        let builder = build_builder(&profile);
        let raw = builder.build()?;
        let mut cfg: Self = raw.try_deserialize()?;
        cfg.environment = profile;

        #[cfg(feature = "database")]
        apply_database_url(&mut cfg)?;
        apply_log_env_overrides(&mut cfg);
        apply_legacy_env_overrides(&mut cfg);
        sanitize_cors(&mut cfg);
        warn_deprecated_envs();

        // Validation: fail fast if settings are semantically invalid
        cfg.validate()?;

        Ok(cfg)
    }

    /// 設定値が論理的に正しいか検証します。
    fn validate(&self) -> Result<(), crate::AppError> {
        #[cfg(feature = "database")]
        if self.database.min_connections > self.database.max_connections {
            return Err(crate::AppError::ConfigValidationError(
                "database.min_connections cannot be greater than max_connections".to_string(),
            ));
        }

        #[cfg(feature = "auth")]
        {
            if !(10..=16).contains(&self.auth.bcrypt_cost) {
                return Err(crate::AppError::ConfigValidationError(
                    "auth.bcrypt_cost must be between 10 and 16 for security and performance reasons".to_string(),
                ));
            }
        }

        if self.security.rate_limit_window == 0 {
            return Err(crate::AppError::ConfigValidationError(
                "security.rate_limit_window must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

// ==========================
// Constants for defaults (scoped into structs)
// ==========================
const MEGABYTE: usize = 1024 * 1024;

impl ServerConfig {
    pub const DEFAULT_HOST: &'static str = "0.0.0.0";
    pub const DEFAULT_PORT: u16 = 3000;
    pub const DEFAULT_REQUEST_TIMEOUT: u64 = 30;
    pub const DEFAULT_MAX_REQUEST_SIZE: usize = 10 * MEGABYTE; // 10MB
}

impl SearchConfig {
    pub const DEFAULT_WRITER_MEMORY: usize = 50 * MEGABYTE; // 50MB
    pub const DEFAULT_MAX_RESULTS: usize = 100;
}

// Placeholder constants
#[cfg(feature = "database")]
const DATABASE_URL_PLACEHOLDER: &str = "${DATABASE_URL}";

// ==========================
// Private helpers
// ==========================
fn read_profile() -> String {
    env::var("CMS__PROFILE")
        .or_else(|_| env::var("RUN_MODE"))
        .unwrap_or_else(|_| "development".to_string())
}

fn build_builder(profile: &str) -> config::ConfigBuilder<config::builder::DefaultState> {
    let mut builder = config::Config::builder()
        .add_source(config::File::with_name("config/default").required(false));
    if profile != "development" {
        builder = builder
            .add_source(config::File::with_name(&format!("config/{profile}")).required(false));
    }
    builder
        .add_source(config::File::with_name("config/local").required(false))
        // Add environment variable overrides with CMS_ prefix
        .add_source(
            config::Environment::with_prefix("CMS")
                .separator("__")
                .try_parsing(true)
        )
}

#[cfg(feature = "database")]
fn apply_database_url(cfg: &mut Config) -> Result<(), crate::AppError> {
    if let Ok(real) = env::var("DATABASE_URL") {
        if !real.is_empty() {
            cfg.database.url = SecretString::new(real.into());
            return Ok(());
        }
    }
    // SecretString doesn't implement PartialEq/PartialOrd; compare inner values explicitly
    if cfg.database.url.expose_secret() == DATABASE_URL_PLACEHOLDER {
        return Err(crate::AppError::ConfigValueMissing(
            "DATABASE_URL".to_string(),
        ));
    }
    Ok(())
}

fn apply_log_env_overrides(cfg: &mut Config) {
    if let Ok(lvl) = env::var("LOG_LEVEL") {
        if !lvl.is_empty() {
            cfg.logging.level = lvl;
        }
    }
    if let Ok(fmt) = env::var("LOG_FORMAT") {
        if !fmt.is_empty() {
            match fmt.to_lowercase().as_str() {
                "json" => cfg.logging.format = LogFormat::Json,
                "text" => cfg.logging.format = LogFormat::Text,
                other => warn!(
                    "invalid LOG_FORMAT '{}' detected; expected 'text' or 'json' — keeping default",
                    other
                ),
            }
        }
    }
}

fn apply_legacy_env_overrides(_cfg: &mut Config) {
    // Legacy environment variable overrides removed in v3.0.0
    // Use standard configuration file or AUTH_ACCESS_TOKEN_TTL_SECS/AUTH_REFRESH_TOKEN_TTL_SECS instead
}

fn sanitize_cors(cfg: &mut Config) {
    if cfg.security.cors_origins.len() == 1 && cfg.security.cors_origins[0].contains(',') {
        let joined = cfg.security.cors_origins[0].clone();
        cfg.security.cors_origins = joined
            .split(',')
            .map(str::trim)
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .collect();
    }
}

fn warn_deprecated_envs() {
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
}

// ==========================
// Serde helpers
// ==========================
fn serialize_secret_masked<S>(_: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Do not expose secrets in serialized output; mask with fixed placeholder
    serializer.serialize_str("******")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use temp_env;

    const FULL_DEFAULT_TOML: &str = r#"
environment = "test"

[server]
host = "127.0.0.1"
port = 8000
max_request_size = 10485760
request_timeout = 30

[security]
cors_origins = ["http://localhost:3000"]
rate_limit_requests = 100
rate_limit_window = 60
enable_login_rate_limiting = true

[logging]
level = "info"
format = "text"

[monitoring]
enable_metrics = false
metrics_port = 9090
health_check_interval = 30

[media]
upload_dir = "uploads"
max_file_size = 52428800
allowed_types = ["image/jpeg", "image/png"]
thumbnail_sizes = [[150, 150]]

[notifications.email]
smtp_host = "localhost"
smtp_port = 587
smtp_username = ""
smtp_password = ""
from_address = "noreply@example.com"
from_name = "CMS"

[notifications.webhooks]
enabled = false
timeout = 30
retry_attempts = 3

# The following sections are behind feature flags

[database]
url = "${DATABASE_URL}"
max_connections = 20
min_connections = 5
connection_timeout = 30
idle_timeout = 600
max_lifetime = 3600
enable_migrations = true

[redis]
url = "redis://127.0.0.1/"
pool_size = 10
default_ttl = 3600
key_prefix = "cms:"

[search]
index_path = "./search_index"
writer_memory = 52428800
max_results = 100
enable_fuzzy = true
fuzzy_distance = 2

[auth]
biscuit_root_key = "default-key"
bcrypt_cost = 12
session_timeout = 86400
access_token_ttl_secs = 3600
refresh_token_ttl_secs = 86400
remember_me_access_ttl_secs = 86400

[auth.webauthn]
rp_id = "localhost"
rp_name = "CMS"
rp_origin = "http://localhost:3000"
timeout = 60000
"#;

    // Helper to create a temporary config directory with files
    struct TempConfigDir {
        _dir: tempfile::TempDir,
        original_cwd: PathBuf,
    }

    impl TempConfigDir {
        fn new(files: &[(&str, &str)]) -> Self {
            let original_cwd = env::current_dir().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let config_path = dir.path().join("config");
            fs::create_dir(&config_path).unwrap();
            for (name, content) in files {
                fs::write(config_path.join(name), content).unwrap();
            }
            env::set_current_dir(dir.path()).unwrap();
            Self { _dir: dir, original_cwd }
        }
    }

    impl Drop for TempConfigDir {
        fn drop(&mut self) {
            env::set_current_dir(&self.original_cwd).unwrap();
        }
    }

    #[test]
    fn test_security_config_defaults() {
        let security_config = SecurityConfig::default();
        assert_eq!(security_config.cors_origins, vec!["http://localhost:3000"]);
        assert_eq!(security_config.rate_limit_requests, 100);
        assert_eq!(security_config.rate_limit_window, 60);
        assert!(security_config.enable_login_rate_limiting);
    }

    #[test]
    fn test_config_from_env_with_env_var_override() {
        temp_env::with_vars(
            [
                ("CMS__SERVER__PORT", Some("8081")),
                ("CMS__SECURITY__ENABLE_LOGIN_RATE_LIMITING", Some("false")),
                ("DATABASE_URL", Some("postgres://user:pass@host/db")),
            ],
            || {
                let _tdir = TempConfigDir::new(&[("default.toml", FULL_DEFAULT_TOML)]);
                let config = Config::from_env().unwrap();
                assert_eq!(config.server.port, 8081);
                assert!(!config.security.enable_login_rate_limiting);
            },
        );
    }

    #[test]
    fn test_config_loading_priority() {
        temp_env::with_vars([("DATABASE_URL", Some("postgres://user:pass@host/db"))], || {
            let _tdir = TempConfigDir::new(&[
                ("default.toml", FULL_DEFAULT_TOML),
                ("production.toml", "[server]\nport = 2\n[security]\nrate_limit_window = 2"),
                ("local.toml", "[server]\nport = 3"),
            ]);

            // 1. Test default only
            temp_env::with_var("CMS__PROFILE", Some("development"), || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.server.port, 3); // local.toml overrides default
                assert_eq!(config.security.rate_limit_window, 60); // from default.toml
            });

            // 2. Test profile override
            temp_env::with_var("CMS__PROFILE", Some("production"), || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.server.port, 3); // local.toml overrides production
                assert_eq!(config.security.rate_limit_window, 2);
            });

            // 3. Test env var override
            temp_env::with_vars([("CMS__SERVER__PORT", Some("4")), ("CMS__PROFILE", Some("production"))], || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.server.port, 4); // env var overrides everything
            });
        });
    }

    #[test]
    fn test_log_overrides() {
        temp_env::with_vars(
            [
                ("LOG_LEVEL", Some("debug")),
                ("LOG_FORMAT", Some("json")),
                ("DATABASE_URL", Some("postgres://user:pass@host/db")),
            ],
            || {
                let _tdir = TempConfigDir::new(&[("default.toml", FULL_DEFAULT_TOML)]);
                let config = Config::from_env().unwrap();
                assert_eq!(config.logging.level, "debug");
                assert!(matches!(config.logging.format, LogFormat::Json));
            },
        );
    }

    #[test]
    fn test_sanitize_cors() {
        // Test CSV parsing in TOML file  - cors_origins with CSV should be split
        let mut toml_with_csv = FULL_DEFAULT_TOML.to_string();
        // Replace the cors_origins line with CSV format
        toml_with_csv = toml_with_csv.replace(
            "cors_origins = [\"http://localhost:3000\"]",
            "cors_origins = [\"http://a.com, http://b.com\"]"
        );
        
        temp_env::with_vars(
            [
                ("DATABASE_URL", Some("postgres://user:pass@host/db")),
            ],
            || {
                let _tdir = TempConfigDir::new(&[("default.toml", &toml_with_csv)]);
                let config = Config::from_env().unwrap();
                // sanitize_cors should split the CSV string
                assert_eq!(config.security.cors_origins, vec!["http://a.com", "http://b.com"]);
            },
        );
    }

    #[test]
    fn test_validation_fails_on_invalid_rate_limit_window() {
        temp_env::with_vars(
            [
                ("CMS__SECURITY__RATE_LIMIT_WINDOW", Some("0")),
                ("DATABASE_URL", Some("postgres://user:pass@host/db")),
            ],
            || {
                let _tdir = TempConfigDir::new(&[("default.toml", FULL_DEFAULT_TOML)]);
                let result = Config::from_env();
                assert!(result.is_err());
                let err_msg = result.err().unwrap().to_string();
                assert!(err_msg.contains("security.rate_limit_window must be greater than 0"));
            },
        );
    }
}