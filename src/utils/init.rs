use crate::config::Config;
use crate::error::Result;
use crate::telemetry;

/// Initialize logging/telemetry and return loaded `Config`
///
/// # Errors
///
/// 環境変数の読み込みや設定値の検証に失敗した場合、エラーを返します。
pub fn init_logging_and_config() -> Result<Config> {
    // Initialize tracing subscriber (idempotent)
    let _ = telemetry::init_telemetry();

    // Load configuration
    let config = Config::from_env()?;

    Ok(config)
}

/// Initialize telemetry with verbose option and return loaded `Config`.
/// Prefer this from CLI binaries that accept a --verbose flag.
pub fn init_logging_and_config_with_verbose(verbose: bool) -> Result<Config> {
    // Initialize tracing subscriber (idempotent)
    let _ = telemetry::init_telemetry_with_verbose(verbose);

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

/// Initialize `AppState` from environment via `Config`
///
/// # Errors
///
/// 設定のロードや `AppState` の初期化に失敗した場合、エラーを返します。
pub async fn init_app_state() -> crate::Result<crate::AppState> {
    let config = init_logging_and_config()?;
    crate::AppState::from_config(config).await
}

/// Initialize `AppState` honoring a verbose flag used by CLI tools.
pub async fn init_app_state_with_verbose(verbose: bool) -> crate::Result<crate::AppState> {
    let config = init_logging_and_config_with_verbose(verbose)?;
    crate::AppState::from_config(config).await
}

#[cfg(feature = "database")]
use crate::database::Database;

#[cfg(feature = "database")]
/// Initialize database and return `Database` handle
///
/// # Errors
///
/// 接続確立や初期化処理に失敗した場合、エラーを返します。
pub async fn init_database(config: &crate::config::DatabaseConfig) -> Result<Database> {
    let db = Database::new(config).await?;
    Ok(db)
}
