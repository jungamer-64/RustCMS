use cms_backend::handlers::admin;
use cms_backend::{config::Config, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _cfg = Config::from_env()?;
    let state = AppState::from_env().await?;

    // Call list_posts handler directly
    let headers = axum::http::HeaderMap::new();
    // Should return Unauthorized without ADMIN_TOKEN
    match admin::list_posts(axum::extract::State(state.clone()), headers).await {
        Ok(json) => {
            let count = json.0.data.as_ref().map(|v| v.len()).unwrap_or(0);
            println!("OK: {} posts", count)
        }
        Err(e) => println!("Expected error (no token): {}", e),
    }

    Ok(())
}
