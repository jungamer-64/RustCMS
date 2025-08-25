use crate::config::Config;
use crate::error::Result;
use crate::telemetry;

/// Initialize logging/telemetry and return loaded Config
pub async fn init_logging_and_config() -> Result<Config> {
    // Initialize tracing subscriber (idempotent)
    let _ = telemetry::init_telemetry();

    // Load configuration
    let config = Config::from_env()?;

    Ok(config)
}

/// Synchronous environment initialization for small bins
pub fn init_env() {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Initialize telemetry/logging (best-effort)
    let _ = telemetry::init_telemetry();
}

/// Initialize AppState from environment via Config
pub async fn init_app_state() -> crate::Result<crate::AppState> {
    let config = init_logging_and_config().await?;
    crate::AppState::from_config(config).await
}

#[cfg(feature = "database")]
use crate::database::Database;

#[cfg(feature = "database")]
/// Initialize database and return Database handle
pub async fn init_database(config: &crate::config::DatabaseConfig) -> Result<Database> {
    let db = Database::new(config).await?;
    Ok(db)
}
