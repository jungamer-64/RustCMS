//! Minimal server entrypoint adjusted to current Config and AppState implementations

use std::{net::SocketAddr, sync::Arc};
use tracing::{info, error};
use axum::Router as AxumRouter;

use cms_backend::{AppState, routes::create_router, config::Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry/logging (best-effort)
    let _ = cms_backend::telemetry::init_telemetry();

    info!("Starting CMS server (minimal entry)");

    // Load configuration
    let config = Arc::new(Config::from_env().map_err(|e| {
        error!("Configuration error: {}", e);
        format!("Failed to load configuration: {}", e)
    })?);

    // Initialize application state
    let state = AppState::from_env().await.map_err(|e| {
        error!("Failed to initialize application: {}", e);
        format!("Application initialization failed: {}", e)
    })?;

    // Build router and attach state
    let _router: AxumRouter = create_router().with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    info!("Initialized services; server binding skipped in minimal build (would listen on http://{}).", addr);

    // Note: server start is intentionally skipped in this minimal entry so the
    // binary can compile cleanly during migration and CI checks. Use full
    // server entrypoint when running the real service.
    Ok(())
}