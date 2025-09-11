use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{error, info};

use cms_backend::routes::create_router;

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
    // Initialize full AppState and get config via AppState
    let app_state = cms_backend::utils::init::init_app_state().await?;
    let config = app_state.config.clone();

    info!("Starting admin server");

    // Use the initialized state
    let state = app_state;

    // Build router and attach state
    let app: Router = create_router().with_state(state);

    // Bind to configured address
    let host = config.server.host.clone();
    let port = config.server.port;
        let addr = format!("{}:{}", state.config.server.host, state.config.server.port).parse::<SocketAddr>()?;

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
