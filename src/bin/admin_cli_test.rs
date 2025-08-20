use cms_backend::{AppState, config::Config};
use cms_backend::handlers::admin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _cfg = Config::from_env()?;
    let state = AppState::from_env().await?;

    // Call list_posts handler directly
    let headers = axum::http::HeaderMap::new();
    // Should return Unauthorized without ADMIN_TOKEN
    match admin::list_posts(axum::extract::State(state.clone()), headers).await {
        Ok(json) => println!("OK: {} posts", json.0.len()),
        Err(e) => println!("Expected error (no token): {}", e),
    }

    Ok(())
}
