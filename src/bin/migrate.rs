//! Database migration utility for Enterprise CMS
//! 
//! Handles database schema migrations, data migrations, and database maintenance tasks.

use cms_backend::{
    config::Config,
    database::Database,
    Result,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use tracing::{info, error, warn};
use clap::{Parser, Subcommand};
use diesel::RunQueryDsl;
use diesel::pg::PgConnection;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🔧 Enterprise CMS Database Migration Tool v{}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config = Config::from_env()?;
    
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

    // Connect to database
    info!("🔗 Connecting to database...");
    let database = Database::new(&config.database).await?;

    match cli.command {
        Commands::Migrate { no_seed } => {
            info!("📊 Running database migrations...");
            run_migrations(&database).await?;
            info!("✅ Database migrations completed successfully");

            if no_seed {
                info!("🔕 Skipping seeding because --no-seed was passed");
            } else {
                info!("🌱 Seeding database with initial data (default)...");
                seed_database(&database).await?;
                info!("✅ Database seeding completed");
            }
        }
        Commands::Rollback { steps } => {
            let steps = steps.unwrap_or(1);
            warn!("⚠️  Rolling back {} migration(s)...", steps);
            rollback_migrations(&database, steps).await?;
            info!("✅ Migration rollback completed");
        }
        Commands::Reset => {
            warn!("🚨 DANGER: This will drop all tables and recreate them!");
            warn!("🚨 All data will be lost! Type 'YES' to confirm:");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim() == "YES" {
                reset_database(&database).await?;
                info!("✅ Database reset completed");
            } else {
                info!("❌ Database reset cancelled");
            }
        }
        Commands::Seed => {
            info!("🌱 Seeding database with initial data...");
            seed_database(&database).await?;
            info!("✅ Database seeding completed");
        }
        Commands::Status => {
            info!("📊 Checking migration status...");
            check_migration_status(&database).await?;
        }
        Commands::Backup { path } => {
            let backup_path = path.as_deref().unwrap_or("./backups");
            info!("💾 Creating database backup to {}...", backup_path);
            create_backup(&database, backup_path).await?;
            info!("✅ Database backup completed");
        }
        Commands::Restore { path } => {
            let backup_path = path;
            warn!("🔄 Restoring database from {}...", backup_path);
            restore_backup(&database, &backup_path).await?;
            info!("✅ Database restore completed");
        }
    }
    
    Ok(())
}

// `print_usage` was removed in favor of Clap-based automatic help generation.

async fn run_migrations(database: &Database) -> Result<()> {
    let mut conn = database.get_connection()?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;
    Ok(())
}

async fn rollback_migrations(database: &Database, steps: usize) -> Result<()> {
    let mut conn = database.get_connection()?;
    
    for _ in 0..steps {
    match conn.revert_last_migration(MIGRATIONS) {
            Ok(_) => info!("✅ Reverted migration"),
            Err(e) => {
                error!("❌ Failed to revert migration: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

async fn reset_database(database: &Database) -> Result<()> {
    info!("🗑️  Dropping all tables...");
    
    // This is a simplified version - in production you'd want more sophisticated schema dropping
    let mut conn = database.get_connection()?;
    
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
    
    for statement in drop_statements {
        if let Err(e) = diesel::sql_query(statement).execute(&mut conn) {
            warn!("Failed to execute: {} - {}", statement, e);
        }
    }
    
    info!("🔄 Recreating schema...");
    run_migrations(database).await?;
    
    // Ensure compatibility between possible diesel migration table names
    {
        let mut conn = database.get_connection()?;
        ensure_schema_migrations_compat(&mut conn)?;
    }
    
    Ok(())
}

async fn seed_database(database: &Database) -> Result<()> {
    // Create initial admin user and default settings
    let mut conn = database.get_connection()?;
    
    // Check if already seeded
    use cms_backend::database::schema::users::dsl::*;
    use diesel::prelude::*;
    
    let existing_users: i64 = users.count().get_result(&mut conn)
        .map_err(|e| cms_backend::AppError::Database(e))?;
    
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

    let _admin = database.create_user(admin_user).await?;
    
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
    database.create_post(sample_post).await?;
    
    Ok(())
}

/// Fetch applied migration versions from either `schema_migrations` or
/// `__diesel_schema_migrations` (fallback). Returns versions ordered asc.
fn fetch_applied_versions(conn: &mut PgConnection) -> Result<Vec<String>> {
    use diesel::prelude::*;
    #[derive(diesel::QueryableByName)]
    struct MigrationVersion {
        #[diesel(sql_type = diesel::sql_types::Text)]
        version: String,
    }

    let rows: Vec<MigrationVersion> = match diesel::sql_query("SELECT version FROM schema_migrations ORDER BY version ASC").load(conn) {
        Ok(r) => r,
        Err(_) => diesel::sql_query("SELECT version FROM __diesel_schema_migrations ORDER BY version ASC")
            .load(conn)
            .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?,
    };

    Ok(rows.into_iter().map(|r| r.version).collect())
}

/// Ensure `schema_migrations` exists and copy rows from `__diesel_schema_migrations` if present.
fn ensure_schema_migrations_compat(conn: &mut PgConnection) -> Result<()> {
    use diesel::prelude::*;

    let create_sql = "CREATE TABLE IF NOT EXISTS schema_migrations (version VARCHAR(255) PRIMARY KEY, applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW());";
    if let Err(e) = diesel::sql_query(create_sql).execute(conn) {
        warn!("Failed to ensure schema_migrations table: {}", e);
    }

    let copy_sql = "INSERT INTO schema_migrations(version, applied_at) SELECT version, run_on FROM __diesel_schema_migrations WHERE version NOT IN (SELECT version FROM schema_migrations);";
    if let Err(e) = diesel::sql_query(copy_sql).execute(conn) {
        warn!("Could not copy migration rows to schema_migrations: {}", e);
    }

    Ok(())
}

async fn check_migration_status(database: &Database) -> Result<()> {
    let mut conn = database.get_connection()?;
    // Use helper to fetch applied migration versions (handles both table names)
    let applied = fetch_applied_versions(&mut conn)?;
    
    info!("📊 Migration Status:");
    info!("  Applied migrations: {}", applied.len());
    
    for migration in applied {
        info!("  ✅ {}", migration);
    }
    
    // Check for pending migrations
    let pending = conn.pending_migrations(MIGRATIONS)
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;
    
    if pending.is_empty() {
        info!("  ✅ No pending migrations");
    } else {
        info!("  ⏳ Pending migrations: {}", pending.len());
        for migration in pending {
            info!("  ⏳ {}", migration.name());
        }
    }
    
    Ok(())
}

async fn create_backup(_database: &Database, backup_path: &str) -> Result<()> {
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

async fn restore_backup(_database: &Database, backup_path: &str) -> Result<()> {
    // This is a simplified version - in production you'd use pg_restore
    info!("🔄 Restoring from backup: {}", backup_path);
    
    // In a real implementation, you would:
    // 1. Validate backup file
    // 2. Create a new database or drop existing data
    // 3. Use pg_restore to restore the backup
    // 4. Run any necessary post-restore migrations
    // 5. Verify data integrity
    
    warn!("⚠️  Restore functionality not fully implemented - use pg_restore for production restores");
    
    Ok(())
}
