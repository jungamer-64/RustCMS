//! Unified CMS server entrypoint - integrates functionality from cms-lightweight, cms-simple, and cms-unified
//!
//! This server supports both production mode (with database) and development mode (in-memory).
//! It serves as the main unified entry point for the RustCMS backend.

use axum::Router as AxumRouter;
use std::net::SocketAddr;
use tracing::info;

use cms_backend::routes::create_router;

/// Unified CMS server entrypoint
///
/// Integrates functionality from:
/// - cms-lightweight: Initialization and config loading
/// - cms-simple: In-memory development mode and web interface  
/// - cms-unified: Consolidated API endpoints
///
/// This replaces the need for separate CMS binaries by providing a single,
/// unified entry point that can operate in different modes.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize full AppState using shared helper
    let state = cms_backend::utils::init::init_app_state().await?;

    info!("🚀 Starting Unified CMS Server");
    info!("   Integrating cms-lightweight + cms-simple + cms-unified functionality");

    // Compute address from config before moving state
    let addr: SocketAddr =
    format!("{}:{}", state.config.server.host, state.config.server.port).parse()?;

    // Build router and attach state (state is moved into router)
    let router: AxumRouter = create_router().with_state(state);

    // Actually start the HTTP server (this was missing in the original implementation)
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🌐 CMS Server listening on http://{}", addr);
    info!("📚 API Documentation: http://{}/api/docs", addr);
    info!("🔍 Health Check: http://{}/api/v1/health", addr);
    info!("📈 Metrics: http://{}/api/v1/metrics", addr);

    // Log available endpoints based on enabled features
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

    // Start the server
    axum::serve(listener, router).await?;

    Ok(())
}
