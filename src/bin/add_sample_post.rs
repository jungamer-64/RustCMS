use chrono::Utc;
use cms_backend::{
    database::Database,
    models::post::{CreatePostRequest, PostStatus},
    Config,
};

#[tokio::main]
async fn main() {
    // Load config and database
    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    let db = match Database::new(&cfg.database).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to init database: {}", e);
            std::process::exit(1);
        }
    };

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

    match db.create_post(req).await {
        Ok(post) => {
            println!("Inserted sample post: {} (id={})", post.title, post.id);
        }
        Err(e) => {
            eprintln!("Failed to insert sample post: {}", e);
            std::process::exit(1);
        }
    }
}
