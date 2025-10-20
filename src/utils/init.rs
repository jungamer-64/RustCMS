//! Application initialization utilities for bin/ files
//! Phase 5: Updated for new AppState implementation

use crate::{Config, Result};
use std::sync::Arc;

/// Initialize environment variables from .env file
///
/// Loads environment variables for local development.
/// In production, environment variables should be set by the deployment system.
pub fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        // .env file not found is OK in production
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}

/// Initialize logging and return Config
///
/// Sets up tracing subscriber and loads configuration.
pub fn init_logging_and_config() -> Result<Config> {
    init_env();

    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    Config::from_env()
}

/// Initialize AppState with all services
///
/// Phase 5.1: database/cache サービスを統合
#[cfg(feature = "restructure_domain")]
pub async fn init_app_state() -> Result<Arc<crate::AppState>> {
    use crate::infrastructure::app_state::AppState;

    let config = Config::from_env()?;
    let mut builder = AppState::builder(config);

    // Initialize database if feature is enabled
    #[cfg(feature = "database")]
    {
        builder = builder.with_database()?;
    }

    // TODO: Initialize search, auth services

    Ok(Arc::new(builder.build()?))
}

/// Initialize AppState with verbose logging
#[cfg(feature = "restructure_domain")]
pub async fn init_app_state_with_verbose(_verbose: bool) -> Result<Arc<crate::AppState>> {
    init_app_state().await
}

// ============================================================================
// Legacy removed - app.rs deleted in Phase 5
// ============================================================================
