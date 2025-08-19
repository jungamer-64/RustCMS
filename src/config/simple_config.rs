//! 実用的なCMS設定管理
//! 環境変数ベースの設定読み込み

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub cors: CorsConfig,
    pub upload: UploadConfig,
    pub security: SecurityConfig,
    pub github: Option<GitHubConfig>,
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub max_connections: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub name: String,
    pub pool_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expires_in: String,
    pub issuer: String,
    pub audience: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allow_origins: Vec<String>,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadConfig {
    pub max_file_size: u64,
    pub allowed_types: Vec<String>,
    pub upload_dir: String,
    pub serve_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub rate_limit_per_minute: u32,
    pub bcrypt_cost: u32,
    pub session_timeout: u64,
    pub max_login_attempts: u32,
    pub require_email_verification: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub version: String,
    pub docs_enabled: bool,
    pub rate_limit: u32,
    pub key_required: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3001,
                workers: None,
                max_connections: None,
            },
            database: DatabaseConfig {
                url: "postgres://user:password@localhost:5432/rust_cms".to_string(),
                name: "rust_cms".to_string(),
                pool_size: None,
            },
            jwt: JwtConfig {
                secret: "default_jwt_secret_change_in_production".to_string(),
                expires_in: "24h".to_string(),
                issuer: "rust-cms".to_string(),
                audience: "rust-cms-users".to_string(),
            },
            cors: CorsConfig {
                allow_origins: vec!["*".to_string()],
                allow_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "OPTIONS".to_string()],
                allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
                allow_credentials: true,
                max_age: Some(3600),
            },
            upload: UploadConfig {
                max_file_size: 10 * 1024 * 1024, // 10MB
                allowed_types: vec!["image/jpeg".to_string(), "image/png".to_string(), "image/gif".to_string(), "image/webp".to_string()],
                upload_dir: "./uploads".to_string(),
                serve_path: "/uploads".to_string(),
            },
            security: SecurityConfig {
                rate_limit_per_minute: 60,
                bcrypt_cost: 12,
                session_timeout: 3600,
                max_login_attempts: 5,
                require_email_verification: false,
            },
            github: None,
            api: ApiConfig {
                version: "v1".to_string(),
                docs_enabled: true,
                rate_limit: 1000,
                key_required: false,
            },
        }
    }
}

impl Config {
    /// 環境変数から設定を読み込み
    pub fn from_env() -> Result<Self> {
        let config = Self {
            server: ServerConfig {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: std::env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3001".to_string())
                    .parse()
                    .unwrap_or(3001),
                workers: std::env::var("SERVER_WORKERS")
                    .ok()
                    .and_then(|v| v.parse().ok()),
                max_connections: std::env::var("SERVER_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|v| v.parse().ok()),
            },
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgres://user:pass@localhost:5432/rust_cms".to_string()),
                name: std::env::var("DATABASE_NAME")
                    .unwrap_or_else(|_| "rust_cms".to_string()),
                pool_size: std::env::var("DATABASE_POOL_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok()),
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "default_jwt_secret_change_in_production".to_string()),
                expires_in: std::env::var("JWT_EXPIRES_IN")
                    .unwrap_or_else(|_| "24h".to_string()),
                issuer: std::env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "rust-cms".to_string()),
                audience: std::env::var("JWT_AUDIENCE")
                    .unwrap_or_else(|_| "rust-cms-users".to_string()),
            },
            cors: CorsConfig {
                allow_origins: std::env::var("CORS_ALLOW_ORIGINS")
                    .unwrap_or_else(|_| "*".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                allow_methods: std::env::var("CORS_ALLOW_METHODS")
                    .unwrap_or_else(|_| "GET,POST,PUT,DELETE,OPTIONS".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                allow_headers: std::env::var("CORS_ALLOW_HEADERS")
                    .unwrap_or_else(|_| "Content-Type,Authorization".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                allow_credentials: std::env::var("CORS_ALLOW_CREDENTIALS")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                max_age: std::env::var("CORS_MAX_AGE")
                    .ok()
                    .and_then(|v| v.parse().ok()),
            },
            upload: UploadConfig {
                max_file_size: std::env::var("UPLOAD_MAX_FILE_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10 * 1024 * 1024), // 10MB default
                allowed_types: std::env::var("UPLOAD_ALLOWED_TYPES")
                    .unwrap_or_else(|_| "image/jpeg,image/png,image/gif,image/webp".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                upload_dir: std::env::var("UPLOAD_DIR")
                    .unwrap_or_else(|_| "./uploads".to_string()),
                serve_path: std::env::var("UPLOAD_SERVE_PATH")
                    .unwrap_or_else(|_| "/uploads".to_string()),
            },
            security: SecurityConfig {
                rate_limit_per_minute: std::env::var("SECURITY_RATE_LIMIT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60),
                bcrypt_cost: std::env::var("SECURITY_BCRYPT_COST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(12),
                session_timeout: std::env::var("SECURITY_SESSION_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3600),
                max_login_attempts: std::env::var("SECURITY_MAX_LOGIN_ATTEMPTS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5),
                require_email_verification: std::env::var("SECURITY_REQUIRE_EMAIL_VERIFICATION")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
            github: if std::env::var("GITHUB_CLIENT_ID").is_ok() {
                Some(GitHubConfig {
                    client_id: std::env::var("GITHUB_CLIENT_ID").unwrap(),
                    client_secret: std::env::var("GITHUB_CLIENT_SECRET").unwrap(),
                    redirect_uri: std::env::var("GITHUB_REDIRECT_URI")
                        .unwrap_or_else(|_| "http://localhost:3001/auth/github/callback".to_string()),
                })
            } else {
                None
            },
            api: ApiConfig {
                version: "v1".to_string(),
                docs_enabled: std::env::var("API_DOCS_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                rate_limit: std::env::var("API_RATE_LIMIT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1000),
                key_required: std::env::var("API_KEY_REQUIRED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
        };
        
        config.validate()?;
        Ok(config)
    }
    
    /// 設定の検証
    pub fn validate(&self) -> Result<()> {
        // サーバー設定検証
        if self.server.port == 0 {
            anyhow::bail!("Server port cannot be 0");
        }
        
        // データベース設定検証
        if self.database.url.is_empty() {
            anyhow::bail!("Database URL cannot be empty");
        }
        
        if self.database.name.is_empty() {
            anyhow::bail!("Database name cannot be empty");
        }
        
        // JWT設定検証
        if self.jwt.secret.len() < 32 {
            tracing::warn!("JWT secret is too short, consider using a longer secret");
        }
        
        Ok(())
    }
}
