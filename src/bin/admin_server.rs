use std::sync::Arc;
use std::net::SocketAddr;
use cms_backend::{routes::create_router, config::Config, AppState};
use axum::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cfg = Arc::new(Config::from_env()?);
    let state = AppState::from_env().await?;
    let app = create_router().with_state(state);
    let addr: SocketAddr = "127.0.0.1:3001".parse()?;
    println!("Starting admin server on http://{} (use ADMIN_TOKEN header)", addr);
    Server::bind(&addr).serve(app.into_make_service()).await?;
    Ok(())
}
