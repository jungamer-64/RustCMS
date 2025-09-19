use cms_backend::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    // Initialize AppState from environment
    let state = AppState::from_env().await?;

    // Fetch recent posts; propagate errors so this binary acts as a proper smoke test
    let posts = state.db_admin_list_recent_posts(5).await?;
    println!("OK: Successfully fetched {} recent posts.", posts.len());

    Ok(())
}
