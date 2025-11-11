//! Postgres-based setup CLI
//!
//! Creates an initial admin user and a sample post if the database is empty.

use cms_backend::{
    Result,
    models::{CreatePostRequest, CreateUserRequest, PostStatus, UserRole},
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simple logging for CLI
    info!(
        "🔧 Running Postgres setup CLI v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Initialize AppState (includes database)
    let state = init_app_state().await?;

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
        ..Default::default()
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
        published: Some(true),
        status: Some(PostStatus::Published),
        tags: Some(vec!["welcome".to_string(), "cms".to_string()]),
        ..Default::default()
    };

    let _post = state.db_create_post(sample_post).await?;

    info!("✅ Sample post created");

    warn!("⚠️  Default admin password is 'admin123' — change it immediately in production");

    Ok(())
}

fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}

#[cfg(feature = "restructure_domain")]
async fn init_app_state() -> cms_backend::Result<std::sync::Arc<cms_backend::AppState>> {
    use cms_backend::infrastructure::app_state::AppState;
    use std::sync::Arc;

    init_env();
    let config = cms_backend::Config::from_env()?;
    let mut builder = AppState::builder(config);

    #[cfg(feature = "database")]
    {
        builder = builder.with_database()?;
    }

    Ok(Arc::new(builder.build()?))
}
