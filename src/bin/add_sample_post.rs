use chrono::Utc;
use cms_backend::{
    Config,
    database::Database,
    models::post::{CreatePostRequest, PostStatus},
};

#[tokio::main]
async fn main() -> cms_backend::Result<()> {
    // Load config and initialize database; propagate errors to the caller
    let cfg = Config::from_env()?;
    let db = Database::new(&cfg.database).await?;

    // Build a minimal CreatePostRequest
    let req = CreatePostRequest {
        title: "Sample post from add_sample_post".to_string(),
        content: "This is a sample post added by the add_sample_post tool.".to_string(),
        excerpt: Some("Sample excerpt".to_string()),
        slug: Some("sample-post-from-tool".to_string()),
        published: Some(true),
        tags: Some(vec!["sample".to_string(), "tool".to_string()]),
        category: Some("news".to_string()),
        featured_image: None,
        meta_title: Some("Sample Post".to_string()),
        meta_description: Some("A sample post inserted by a development tool.".to_string()),
        published_at: Some(Utc::now()),
        status: Some(PostStatus::Published),
    };

    let post = db.create_post(req)?;
    println!("Inserted sample post: {} (id={})", post.title, post.id);

    Ok(())
}
