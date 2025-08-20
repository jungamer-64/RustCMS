use std::{net::SocketAddr, sync::Arc};
use tracing::{info, error};
use axum::Router;
use tokio::net::TcpListener;

use cms_backend::{AppState, routes::create_router, config::Config};

async fn shutdown_signal() {
    // Wait for CTRL+C
    if let Err(e) = tokio::signal::ctrl_c().await {
        error!("Failed to listen for shutdown signal: {}", e);
    } else {
        info!("Shutdown signal received");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry/logging (best-effort)
    let _ = cms_backend::telemetry::init_telemetry();

    info!("Starting admin server");

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
    let app: Router = create_router().with_state(state);

    // Bind to configured address
    let addr = format!("{}:{}", config.server.host, config.server.port)
        .parse::<SocketAddr>()?;

    info!("Binding admin server to {}", addr);

    // Bind a TcpListener and run axum's serve helper. This uses the
    // tokio listener and axum::serve(listener, app) which delegates to
    // hyper under the hood and avoids importing hyper server types
    // directly. It supports graceful shutdown via the returned future.
    let listener = TcpListener::bind(addr).await?;

    info!("Admin server running on {}", addr);

    let serve_future = axum::serve(listener, app);

    // Run the server with graceful shutdown triggered by shutdown_signal().
    tokio::select! {
        res = serve_future => {
            if let Err(e) = res {
                error!("Server error: {}", e);
            } else {
                info!("Server exited cleanly");
            }
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received, stopping server");
        }
    }
    Ok(())
}
