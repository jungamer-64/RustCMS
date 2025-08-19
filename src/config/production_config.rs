use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // サーバー設定
    pub port: u16,
    pub host: String,
    
    // データベース設定
    pub database_url: String,
    pub database_max_connections: u32,
    
    // Elasticsearch設定
    pub elasticsearch_url: String,
    pub elasticsearch_index: String,
    
    // WebAuthn設定
    pub webauthn_rp_id: String,
    pub webauthn_rp_name: String,
    pub webauthn_origin: String,
    
    // 認証設定
    pub jwt_secret: String,
    pub session_secret: String,
    pub token_expiry_hours: i64,
    
    // ファイルアップロード設定
    pub upload_dir: String,
    pub max_file_size: usize,
    
    // CORS設定
    pub cors_origins: Vec<String>,
    
    // レート制限設定
    pub rate_limit_requests: u64,
    pub rate_limit_window_seconds: u64,
    
    // ログ設定
    pub log_level: String,
    pub log_format: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // 環境変数からの読み込み、デフォルト値を設定
        let config = Config {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Invalid PORT value")?,
            
            host: env::var("HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS value")?,
            
            elasticsearch_url: env::var("ELASTICSEARCH_URL")
                .unwrap_or_else(|_| "http://localhost:9200".to_string()),
            
            elasticsearch_index: env::var("ELASTICSEARCH_INDEX")
                .unwrap_or_else(|_| "cms_posts".to_string()),
            
            webauthn_rp_id: env::var("WEBAUTHN_RP_ID")
                .unwrap_or_else(|_| "localhost".to_string()),
            
            webauthn_rp_name: env::var("WEBAUTHN_RP_NAME")
                .unwrap_or_else(|_| "Production CMS".to_string()),
            
            webauthn_origin: env::var("WEBAUTHN_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            
            jwt_secret: env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,
            
            session_secret: env::var("SESSION_SECRET")
                .context("SESSION_SECRET must be set")?,
            
            token_expiry_hours: env::var("TOKEN_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .context("Invalid TOKEN_EXPIRY_HOURS value")?,
            
            upload_dir: env::var("UPLOAD_DIR")
                .unwrap_or_else(|_| "./uploads".to_string()),
            
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                .parse()
                .context("Invalid MAX_FILE_SIZE value")?,
            
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://localhost:3001".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            
            rate_limit_requests: env::var("RATE_LIMIT_REQUESTS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid RATE_LIMIT_REQUESTS value")?,
            
            rate_limit_window_seconds: env::var("RATE_LIMIT_WINDOW_SECONDS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("Invalid RATE_LIMIT_WINDOW_SECONDS value")?,
            
            log_level: env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            
            log_format: env::var("LOG_FORMAT")
                .unwrap_or_else(|_| "json".to_string()),
        };
        
        // 設定の検証
        config.validate()?;
        
        Ok(config)
    }
    
    fn validate(&self) -> Result<()> {
        // ポート番号の検証
        if self.port == 0 {
            anyhow::bail!("Port must be greater than 0");
        }
        
        // データベースURLの検証
        if !self.database_url.starts_with("postgres://") && !self.database_url.starts_with("postgresql://") {
            anyhow::bail!("DATABASE_URL must start with postgres:// or postgresql://");
        }
        
        // Elasticsearch URLの検証
        if !self.elasticsearch_url.starts_with("http://") && !self.elasticsearch_url.starts_with("https://") {
            anyhow::bail!("ELASTICSEARCH_URL must start with http:// or https://");
        }
        
        // WebAuthn Originの検証
        if !self.webauthn_origin.starts_with("http://") && !self.webauthn_origin.starts_with("https://") {
            anyhow::bail!("WEBAUTHN_ORIGIN must start with http:// or https://");
        }
        
        // JWTシークレットの長さ検証
        if self.jwt_secret.len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 characters long");
        }
        
        // セッションシークレットの長さ検証
        if self.session_secret.len() < 32 {
            anyhow::bail!("SESSION_SECRET must be at least 32 characters long");
        }
        
        Ok(())
    }
    
    pub fn is_production(&self) -> bool {
        env::var("ENVIRONMENT").unwrap_or_default() == "production"
    }
    
    pub fn database_max_connections(&self) -> u32 {
        self.database_max_connections
    }
    
    pub fn upload_path(&self) -> &str {
        &self.upload_dir
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 3000,
            host: "0.0.0.0".to_string(),
            database_url: "postgresql://localhost:5432/cms".to_string(),
            database_max_connections: 20,
            elasticsearch_url: "http://localhost:9200".to_string(),
            elasticsearch_index: "cms_posts".to_string(),
            webauthn_rp_id: "localhost".to_string(),
            webauthn_rp_name: "Production CMS".to_string(),
            webauthn_origin: "http://localhost:3000".to_string(),
            jwt_secret: "your-super-secret-jwt-key-must-be-at-least-32-characters".to_string(),
            session_secret: "your-super-secret-session-key-must-be-at-least-32-characters".to_string(),
            token_expiry_hours: 24,
            upload_dir: "./uploads".to_string(),
            max_file_size: 10 * 1024 * 1024, // 10MB
            cors_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:3001".to_string(),
            ],
            rate_limit_requests: 100,
            rate_limit_window_seconds: 60,
            log_level: "info".to_string(),
            log_format: "json".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "0.0.0.0");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        
        // 無効なポートでテスト
        config.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_is_production() {
        env::set_var("ENVIRONMENT", "production");
        let config = Config::default();
        assert!(config.is_production());
        
        env::remove_var("ENVIRONMENT");
        assert!(!config.is_production());
    }
}
