use cms_backend::{config::Config, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let _cfg = Config::from_env()?;
    let state = AppState::from_env().await?;

    // Call list_posts handler directly
    // This example program no longer calls the handler directly because it requires Extension<AuthContext>.
    // Instead, we just exercise a simple state call to ensure the binary links.
    if let Ok(posts) = state.db_admin_list_recent_posts(5).await {
        println!("OK: {} posts (recent)", posts.len());
    }

    Ok(())
}
