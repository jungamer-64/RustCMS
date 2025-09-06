//! Postgres-based setup CLI
//!
//! Creates an initial admin user and a sample post if the database is empty.

use cms_backend::{models::{CreatePostRequest, CreateUserRequest, PostStatus, UserRole}, Result};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simple logging for CLI
    tracing_subscriber::fmt::init();

    info!(
        "🔧 Running Postgres setup CLI v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Initialize AppState (includes database)
    let state = cms_backend::utils::init::init_app_state().await?;

    // Use the migrate/seed logic similar to the migration tool: if DB empty, seed it
    info!("🌱 Checking database for existing users...");
    let existing_users: i64 = state.db_admin_users_count().await?;

    if existing_users > 0 {
        info!(
            "✅ Database already contains users ({}), skipping seeding",
            existing_users
        );
        return Ok(());
    }

    info!("👤 Creating admin user...");

    let admin_user = CreateUserRequest {
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password: "admin123".to_string(),
        role: UserRole::Admin,
        first_name: Some("".to_string()),
        last_name: Some("".to_string()),
    };

    let created_admin = state.db_create_user(admin_user).await?;

    info!(
        "✅ Admin user created: {} ({})",
        created_admin.username, created_admin.id
    );

    // Create a sample post
    info!("📝 Creating sample post...");

    let sample_post = CreatePostRequest {
        title: "Welcome to Enterprise CMS".to_string(),
        content: "This is a high-performance, scalable CMS built with Rust and PostgreSQL."
            .to_string(),
        excerpt: Some("Welcome to the future of content management.".to_string()),
        slug: None,
        published: Some(true),
        tags: Some(vec!["welcome".to_string(), "cms".to_string()]),
        category: None,
        featured_image: None,
        meta_title: Some("Welcome to Enterprise CMS".to_string()),
        meta_description: Some("A production-ready CMS built for scale.".to_string()),
        published_at: None,
        status: Some(PostStatus::Published),
    };

    let _post = state.db_create_post(sample_post).await?;

    info!("✅ Sample post created");

    warn!("⚠️  Default admin password is 'admin123' — change it immediately in production");

    Ok(())
}
