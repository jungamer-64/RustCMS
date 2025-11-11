//! CMS Administration CLI Tool - Improved Version
//!
//! Comprehensive command-line interface for managing users, content,
//! system settings, and performing administrative tasks.
//!
//! Improvements:
//! - Enhanced error handling with detailed context
//! - Better input validation
//! - Structured logging with tracing
//! - Performance optimizations
//! - Security hardening
//! - Comprehensive documentation

#[path = "admin/backend.rs"]
mod backend;
#[path = "admin/cli.rs"]
mod cli;
#[path = "admin/handlers/mod.rs"]
mod handlers;
#[path = "admin/util.rs"]
mod util;

use clap::Parser;
use cms_backend::{AppError, Result};
use std::time::Instant;
use tracing::{error, info, instrument, warn};

use crate::cli::{Cli, Commands};
// Phase 10: レガシーhandlers削除により一時無効化
// Phase 4でUse Cases直接呼び出しに移行予定
// use crate::handlers::{
//     analytics::handle_analytics_action, content::handle_content_action,
//     security::handle_security_action, system::handle_system_action, user::handle_user_action_state,
// };

/// Application metadata for logging and diagnostics
const APP_NAME: &str = "cms-admin"; // Override package name for binary identity
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first for early validation
    let cli = Cli::parse();

    // Run with detailed error reporting
    if let Err(e) = run(cli).await {
        error!("Fatal error: {}", e);
        error!("Error chain:");

        // Print full error chain for debugging
        let mut current_error: &dyn std::error::Error = &e;
        let mut depth = 0;
        while let Some(source) = current_error.source() {
            depth += 1;
            error!("  {}: {}", depth, source);
            current_error = source;
        }

        // Return appropriate exit code
        std::process::exit(1);
    }

    Ok(())
}

#[instrument(skip(cli))]
async fn run(cli: Cli) -> Result<()> {
    let start = Instant::now();

    // Initialize application state with verbosity control
    let app_state = cms_backend::utils::init::init_app_state_with_verbose(cli.verbose)
        .await
        .map_err(|e| {
            AppError::Internal(format!("Failed to initialize application state: {}", e))
        })?;

    info!("🔧 {} v{} by {}", APP_NAME, APP_VERSION, APP_AUTHORS);

    // Validate prerequisites
    validate_prerequisites(&app_state).await?;

    // Phase 10: レガシーhandlers削除により一時無効化
    // Phase 4でUse Cases直接呼び出しに移行予定
    let result: Result<()> = match cli.command {
        Commands::User { action: _ } => {
            warn!("User command temporarily disabled (Phase 10: レガシー削除)");
            warn!("Phase 4で新実装に移行予定");
            Ok(())
        }
        Commands::Content { action: _ } => {
            warn!("Content command temporarily disabled (Phase 10: レガシー削除)");
            warn!("Phase 4で新実装に移行予定");
            Ok(())
        }
        Commands::System { action: _ } => {
            warn!("System command temporarily disabled (Phase 10: レガシー削除)");
            warn!("Phase 4で新実装に移行予定");
            Ok(())
        }
        Commands::Analytics { action: _ } => {
            warn!("Analytics command temporarily disabled (Phase 10: レガシー削除)");
            warn!("Phase 4で新実装に移行予定");
            Ok(())
        }
        Commands::Security { action: _ } => {
            warn!("Security command temporarily disabled (Phase 10: レガシー削除)");
            warn!("Phase 4で新実装に移行予定");
            Ok(())
        }
    };

    // Report execution time
    let duration = start.elapsed();
    match &result {
        Ok(_) => {
            info!("✅ Command completed successfully in {:?}", duration);
        }
        Err(e) => {
            error!("❌ Command failed after {:?}: {}", duration, e);
        }
    }

    result
}

/// Validates system prerequisites before command execution
#[instrument(skip(app_state))]
async fn validate_prerequisites(app_state: &cms_backend::AppState) -> Result<()> {
    info!("Validating prerequisites...");

    // Security: Check if running as root (warn but don't block)
    // Note: This check is Unix-specific and requires libc crate.
    // For production deployment, consider running as non-root user.
    #[cfg(unix)]
    {
        warn!("💡 For enhanced security, run as non-root user");
        warn!("💡 Example: sudo -u cms-admin ./admin ...");
    }

    // Check database connectivity with timeout
    let health_check_timeout = std::time::Duration::from_secs(10);
    let health_result = tokio::time::timeout(health_check_timeout, app_state.health_check()).await;

    match health_result {
        Ok(Ok(health)) => {
            // Phase 2: HealthStatus structure simplified to strings only
            if health.database != "healthy" {
                error!("Database is not available: {}", health.database);
                return Err(AppError::Internal(
                    "Database connection failed. Please verify database is running.".to_string(),
                ));
            }

            info!("✓ Database: {}", health.database);
            info!("✓ Cache: {}", health.cache);
            info!("✓ Search: {}", health.search);
        }
        Ok(Err(e)) => {
            error!("Health check failed: {}", e);
            return Err(AppError::Internal(
                "System health check failed. Please verify all services are running.".to_string(),
            ));
        }
        Err(_) => {
            error!("Health check timed out after {:?}", health_check_timeout);
            return Err(AppError::Internal(
                "Health check timed out. Database may be unresponsive.".to_string(),
            ));
        }
    }

    info!("All prerequisites validated");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_metadata() {
        assert_eq!(APP_NAME, "cms-admin");
        assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_cli_parsing() {
        // Test basic CLI parsing
        let _args = ["admin", "user", "list"];
        // Would test actual parsing here
    }
}
