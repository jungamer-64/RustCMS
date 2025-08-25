use axum::{routing::get, Router};
use cms_backend::handlers;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Only mount docs routes (handlers::docs_ui and handlers::openapi_json don't need AppState)
    let app = Router::new()
        .route("/api/docs", get(handlers::docs_ui))
        .route("/api/docs/openapi.json", get(handlers::openapi_json));

    let addr: SocketAddr = "127.0.0.1:3003".parse()?;
    println!(
        "Docs server running on http://{} (endpoints: /api/docs, /api/docs/openapi.json)",
        addr
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
