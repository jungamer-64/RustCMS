use axum::{Router, response::Json, routing::get};
use serde_json::json;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    cms_backend::utils::init::init_env();

    // Simple health check endpoint
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(root));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    println!("🚀 Simple server starting on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "message": "Rust backend is running!",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Rust CMS Backend API",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": ["/health"]
    }))
}
