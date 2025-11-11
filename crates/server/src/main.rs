//! Unified CMS server binary.
//!
//! This binary hosts the production-grade Axum server and replaces the legacy
//! `src/main.rs` entrypoint in the root crate so the server can evolve
//! independently inside `crates/server`.

use axum::Router as AxumRouter;
use hyper::Error as HyperError;
use std::net::SocketAddr;
use thiserror::Error;
use tracing::info;

use cms_backend::error::AppError as BackendAppError;
use cms_backend::routes::create_router;

/// Unified CMS server entrypoint.
///
/// Integrates functionality from:
/// - cms-lightweight: Initialization and config loading
/// - cms-simple: In-memory development mode and web interface
/// - cms-unified: Consolidated API endpoints
#[derive(Debug, Error)]
enum ServerError {
    #[error("Failed to initialize application state: {0}")]
    Init(#[from] BackendAppError),

    #[error("Invalid socket address: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("Failed to bind TCP listener: {0}")]
    Bind(#[from] std::io::Error),

    #[error("Server error: {0}")]
    Serve(#[from] HyperError),
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    // Initialize application state (database/cache/auth/search according to features)
    let state = init_app_state().await?;

    info!("🚀 Starting Unified CMS Server");
    info!("   Integrating cms-lightweight + cms-simple + cms-unified functionality");

    // Compute address from config before moving state
    let addr: SocketAddr =
        format!("{}:{}", state.config.server.host, state.config.server.port).parse()?;

    // Build router and attach state (we keep a clone to call shutdown later)
    let state_clone_for_router = state.clone();
    let router: AxumRouter = create_router().with_state(state_clone_for_router);

    // Start HTTP server
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🌐 CMS Server listening on http://{}", addr);
    info!("📚 API Documentation: http://{}/api/docs", addr);
    info!("🔍 Health Check: http://{}/api/v1/health", addr);
    info!("📈 Metrics: http://{}/api/v1/metrics", addr);

    #[cfg(feature = "auth")]
    info!("🔐 Authentication endpoints available at /api/v1/auth/*");

    #[cfg(feature = "database")]
    info!("💾 Database-backed endpoints available");

    #[cfg(not(feature = "database"))]
    {
        use tracing::warn;
        warn!("⚠️  Running in lightweight mode - no database features available");
    }

    #[cfg(feature = "search")]
    info!("🔍 Search endpoints available at /api/v1/search/*");

    // Start the server with graceful shutdown handling
    let server = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal());
    server.await?;

    // After server returns (graceful shutdown triggered), run AppState shutdown
    state.shutdown().await;

    info!("✅ Server shutdown complete");
    Ok(())
}

/// Listens for shutdown signals (Ctrl+C and SIGTERM on Unix) and returns
/// once one is received so the server can start graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("🔌 Signal received, starting graceful shutdown");
}

fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}

#[cfg(feature = "restructure_domain")]
async fn init_app_state() -> Result<std::sync::Arc<cms_backend::AppState>, BackendAppError> {
    use cms_backend::infrastructure::app_state::AppState;
    use std::sync::Arc;

    init_env();
    let config = cms_backend::Config::from_env()?;
    let mut builder = AppState::builder(config);

    #[cfg(feature = "database")]
    {
        builder = builder.with_database()?;
    }

    Ok(Arc::new(builder.build()?))
}
