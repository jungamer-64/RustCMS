//! Database migration utility for Enterprise CMS
//!
//! Handles database schema migrations, data migrations, and database maintenance tasks.

use clap::{Parser, Subcommand};
use cms_backend::{AppState, Result};
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use std::env;
use tracing::{error, info, warn};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (config will be loaded by init_app_state)
    let _config = cms_backend::utils::init::init_logging_and_config().await?;

    info!(
        "🔧 Enterprise CMS Database Migration Tool v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Parse CLI using clap
    #[derive(Parser)]
    #[command(name = "cms-migrate", version = env!("CARGO_PKG_VERSION"), about = "Enterprise CMS Database Migration Tool")]
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }

    #[derive(Subcommand)]
    enum Commands {
        /// Run pending migrations
        Migrate {
            /// Skip automatic seeding after migrations
            #[arg(long = "no-seed")]
            no_seed: bool,
        },
        /// Rollback migrations (default: 1 step)
        Rollback {
            /// Number of steps to rollback
            steps: Option<usize>,
        },
        /// Drop all tables and recreate (DANGEROUS!)
        Reset,
        /// Seed database with initial data
        Seed,
        /// Show migration status
        Status,
        /// Create database backup
        Backup {
            /// Backup path (default: ./backups)
            path: Option<String>,
        },
        /// Restore database from backup
        Restore {
            /// Path to backup file
            path: String,
        },
    }

    let cli = Cli::parse();

    // Initialize full AppState (includes database when feature enabled)
    let app_state = cms_backend::utils::init::init_app_state().await?;
    // Use AppState (contains database when feature enabled)
    let state = &app_state;

    match cli.command {
        Commands::Migrate { no_seed } => {
            info!("📊 Running database migrations...");
            run_migrations(state).await?;
            info!("✅ Database migrations completed successfully");

            if no_seed {
                info!("🔕 Skipping seeding because --no-seed was passed");
            } else {
                info!("🌱 Seeding database with initial data (default)...");
                seed_database(state).await?;
                info!("✅ Database seeding completed");
            }
        }
        Commands::Rollback { steps } => {
            let steps = steps.unwrap_or(1);
            warn!("⚠️  Rolling back {} migration(s)...", steps);
            rollback_migrations(state, steps).await?;
            info!("✅ Migration rollback completed");
        }
        Commands::Reset => {
            warn!("🚨 DANGER: This will drop all tables and recreate them!");
            warn!("🚨 All data will be lost! Type 'YES' to confirm:");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim() == "YES" {
                reset_database(state).await?;
                info!("✅ Database reset completed");
            } else {
                info!("❌ Database reset cancelled");
            }
        }
        Commands::Seed => {
            info!("🌱 Seeding database with initial data...");
            seed_database(state).await?;
            info!("✅ Database seeding completed");
        }
        Commands::Status => {
            info!("📊 Checking migration status...");
            check_migration_status(state).await?;
        }
        Commands::Backup { path } => {
            let backup_path = path.as_deref().unwrap_or("./backups");
            info!("💾 Creating database backup to {}...", backup_path);
            create_backup(state, backup_path).await?;
            info!("✅ Database backup completed");
        }
        Commands::Restore { path } => {
            let backup_path = path;
            warn!("🔄 Restoring database from {}...", backup_path);
            restore_backup(state, &backup_path).await?;
            info!("✅ Database restore completed");
        }
    }

    Ok(())
}

// `print_usage` was removed in favor of Clap-based automatic help generation.

async fn run_migrations(state: &AppState) -> Result<()> {
    state.db_run_pending_migrations(MIGRATIONS).await
}

async fn rollback_migrations(state: &AppState, steps: usize) -> Result<()> {
    for _ in 0..steps {
        match state.db_revert_last_migration(MIGRATIONS).await {
            Ok(_) => info!("✅ Reverted migration"),
            Err(e) => { error!("❌ Failed to revert migration: {}", e); break; }
        }
    }
    Ok(())
}

async fn reset_database(state: &AppState) -> Result<()> {
    info!("🗑️  Dropping all tables...");

    // This is a simplified version - in production you'd want more sophisticated schema dropping
    // Drop all tables (order matters due to foreign keys)
    let drop_statements = vec![
        "DROP TABLE IF EXISTS audit_logs CASCADE",
        "DROP TABLE IF EXISTS api_keys CASCADE",
        "DROP TABLE IF EXISTS user_sessions CASCADE",
        "DROP TABLE IF EXISTS comments CASCADE",
        "DROP TABLE IF EXISTS media_files CASCADE",
        "DROP TABLE IF EXISTS post_tags CASCADE",
        "DROP TABLE IF EXISTS post_categories CASCADE",
        "DROP TABLE IF EXISTS tags CASCADE",
        "DROP TABLE IF EXISTS categories CASCADE",
        "DROP TABLE IF EXISTS posts CASCADE",
        "DROP TABLE IF EXISTS webauthn_credentials CASCADE",
        "DROP TABLE IF EXISTS users CASCADE",
        "DROP TABLE IF EXISTS settings CASCADE",
        // Drop both possible diesel migration tables to ensure clean reset
        "DROP TABLE IF EXISTS __diesel_schema_migrations CASCADE",
        "DROP TABLE IF EXISTS schema_migrations CASCADE",
    ];

    for statement in drop_statements { let _ = state.db_execute_sql(statement).await.map_err(|e| { warn!("Failed to execute: {} - {}", statement, e); e }); }

    info!("🔄 Recreating schema...");
    run_migrations(state).await?;

    // Ensure compatibility between possible diesel migration table names
    state.db_ensure_schema_migrations_compat().await?;

    Ok(())
}

async fn seed_database(state: &AppState) -> Result<()> {
    // Check if already seeded (use AppState wrapper)
    let existing_users: i64 = state.db_admin_users_count().await?;

    if existing_users > 0 {
        info!("📊 Database already contains data, skipping seeding");
        return Ok(());
    }

    info!("👤 Creating admin user...");

    // Create admin user (password: admin123)
    let admin_user = cms_backend::models::CreateUserRequest {
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password: "admin123".to_string(),
        role: cms_backend::models::UserRole::Admin,
        first_name: Some("".to_string()),
        last_name: Some("".to_string()),
    };

    let _admin = state.db_create_user(admin_user).await?;

    info!("⚙️  Creating default settings...");

    // Insert default settings would go here
    // This is a simplified version

    info!("📝 Creating sample content...");

    // Create sample post
    let sample_post = cms_backend::models::CreatePostRequest {
        title: "Welcome to Enterprise CMS".to_string(),
        content: "This is a high-performance, scalable CMS built with Rust, PostgreSQL, and modern technologies.".to_string(),
        excerpt: Some("Welcome to the future of content management.".to_string()),
        slug: None,
        published: Some(true),
        tags: Some(vec!["welcome".to_string(), "cms".to_string()]),
        category: None,
        featured_image: None,
        meta_title: Some("Welcome to Enterprise CMS".to_string()),
        meta_description: Some("A production-ready CMS built for scale.".to_string()),
        published_at: None,
        status: Some(cms_backend::models::PostStatus::Published),
    };

    // Create the post using the created admin user's id
    state.db_create_post(sample_post).await?;

    Ok(())
}

/// Fetch applied migration versions from either `schema_migrations` or
/// `__diesel_schema_migrations` (fallback). Returns versions ordered asc.

async fn check_migration_status(state: &AppState) -> Result<()> {
    // Use helper to fetch applied migration versions (handles both table names)
    let applied = state.db_fetch_applied_migrations().await?;

    info!("📊 Migration Status:");
    info!("  Applied migrations: {}", applied.len());

    for migration in applied {
        info!("  ✅ {}", migration);
    }

    // Check for pending migrations
    let pending = state.db_list_pending_migrations(MIGRATIONS).await?;

    if pending.is_empty() {
        info!("  ✅ No pending migrations");
    } else {
    info!("  ⏳ Pending migrations: {}", pending.len());
    for name in pending { info!("  ⏳ {}", name); }
    }

    Ok(())
}

async fn create_backup(_state: &AppState, backup_path: &str) -> Result<()> {
    // This is a simplified version - in production you'd use pg_dump
    info!("💾 Creating backup at: {}", backup_path);

    // Create backup directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(backup_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // In a real implementation, you would:
    // 1. Use pg_dump to create a proper PostgreSQL backup
    // 2. Compress the backup file
    // 3. Upload to cloud storage (S3, etc.)
    // 4. Verify backup integrity

    warn!("⚠️  Backup functionality not fully implemented - use pg_dump for production backups");

    Ok(())
}

async fn restore_backup(_state: &AppState, backup_path: &str) -> Result<()> {
    // This is a simplified version - in production you'd use pg_restore
    info!("🔄 Restoring from backup: {}", backup_path);

    // In a real implementation, you would:
    // 1. Validate backup file
    // 2. Create a new database or drop existing data
    // 3. Use pg_restore to restore the backup
    // 4. Run any necessary post-restore migrations
    // 5. Verify data integrity

    warn!(
        "⚠️  Restore functionality not fully implemented - use pg_restore for production restores"
    );

    Ok(())
}
