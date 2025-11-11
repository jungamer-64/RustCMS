use chrono::Utc;
use cms_backend::{
    database::Database, // Config is now loaded via init helper
    models::post::{CreatePostRequest, PostStatus},
};

#[tokio::main]
async fn main() -> cms_backend::Result<()> {
    // Load config and initialize database; propagate errors to the caller
    let cfg = init_logging_and_config()?;
    let db = Database::new(&cfg.database).await?;

    // Build a minimal CreatePostRequest
    let req = CreatePostRequest {
        title: "Sample post from add_sample_post".to_string(),
        content: "This is a sample post added by the add_sample_post tool.".to_string(),
        published: Some(true),
        published_at: Some(Utc::now()),
        status: Some(PostStatus::Published),
        slug: Some("sample-post-from-tool".to_string()),
        tags: Some(vec!["sample".to_string(), "tool".to_string()]),
        ..Default::default()
    };

    let post = db.create_post(req)?;
    println!("Inserted sample post: {} (id={})", post.title, post.id);

    Ok(())
}

fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}

fn init_logging_and_config() -> cms_backend::Result<cms_backend::Config> {
    init_env();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    cms_backend::Config::from_env()
}
