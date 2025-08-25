//! Minimal server entrypoint adjusted to current Config and AppState implementations

use axum::Router as AxumRouter;
use std::net::SocketAddr;
use tracing::info;

use cms_backend::routes::create_router;

/// Minimal server entrypoint using shared init helpers
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize full AppState using shared helper
    let state = cms_backend::utils::init::init_app_state().await?;

    info!("Starting CMS server (minimal entry)");

    // Compute address from config before moving state
    let addr: SocketAddr =
        format!("{}:{}", state.config.server.host, state.config.server.port).parse()?;

    // Build router and attach state (state is moved into router)
    let _router: AxumRouter = create_router().with_state(state);
    info!("Initialized services; server binding skipped in minimal build (would listen on http://{}).", addr);

    Ok(())
}
