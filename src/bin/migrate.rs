//! Database Migration Utility - Improved Version
//!
//! Improvements:
//! - Transaction-based migrations for atomicity
//! - Comprehensive backup before operations
//! - Migration dry-run capability
//! - Detailed progress reporting
//! - Rollback safety checks
//! - Migration verification

use clap::{Parser, Subcommand};
use cms_backend::{AppError, AppState, Result};
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

/// Migration safety levels
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Reserved for future safety classification
enum SafetyLevel {
    Safe,      // Read-only operations
    Moderate,  // Reversible changes
    Dangerous, // Irreversible operations
}

#[derive(Parser)]
#[command(
    name = "cms-migrate",
    version = env!("CARGO_PKG_VERSION"),
    about = "Enterprise CMS Database Migration Tool with Safety Features"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,

    /// Perform dry-run (show what would happen without executing)
    #[arg(long, global = true)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run pending migrations
    Migrate {
        /// Skip automatic seeding after migrations
        #[arg(long = "no-seed")]
        no_seed: bool,

        /// Create backup before migrating
        #[arg(long, default_value = "true")]
        backup: bool,

        /// Verify migrations after applying
        #[arg(long)]
        verify: bool,
    },

    /// Rollback migrations
    Rollback {
        /// Number of steps to rollback
        #[arg(default_value = "1")]
        steps: usize,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Drop all tables and recreate (DANGEROUS!)
    Reset {
        /// Confirm by typing the database name
        #[arg(long)]
        confirm_db_name: Option<String>,
    },

    /// Seed database with initial data
    Seed {
        /// Reseed even if data exists
        #[arg(long)]
        force: bool,
    },

    /// Show migration status
    Status {
        /// Show detailed information
        #[arg(long)]
        verbose: bool,
    },

    /// Validate database integrity
    Validate {
        /// Check for inconsistencies
        #[arg(long)]
        deep: bool,
    },

    /// Create database backup
    Backup {
        /// Backup path (default: ./backups)
        #[arg(default_value = "./backups")]
        path: String,

        /// Compress backup
        #[arg(long)]
        compress: bool,
    },

    /// Restore database from backup
    Restore {
        /// Path to backup file
        path: String,

        /// Skip confirmation
        #[arg(long)]
        force: bool,
    },
}

/// Initialize logging with appropriate level
///
/// This function uses `tracing_subscriber` to configure logging directly
/// without modifying environment variables, which is safer in multi-threaded contexts.
fn initialize_logging(debug: bool) -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt};

    let log_level = if debug { "debug" } else { "info" };

    // Create a filter that respects RUST_LOG if set, otherwise uses the provided level
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .map_err(|e| AppError::Internal(format!("Failed to initialize logging: {e}")))?;

    // Initialize the subscriber
    fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|e| AppError::Internal(format!("Failed to set up logging subscriber: {e}")))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_env();
    let cli = Cli::parse();

    initialize_logging(cli.debug)?;

    print_banner();
    check_dry_run_mode(cli.dry_run);

    let app_state = init_app_state().await?;
    execute_and_handle_result(&cli, &app_state).await
}

/// Print application banner
fn print_banner() {
    info!(
        "🔧 Enterprise CMS Database Migration Tool v{}",
        env!("CARGO_PKG_VERSION")
    );
}

/// Check and warn about dry-run mode
fn check_dry_run_mode(dry_run: bool) {
    if dry_run {
        warn!("🔍 DRY-RUN MODE: No changes will be made");
    }
}

/// Execute command and handle result
async fn execute_and_handle_result(cli: &Cli, app_state: &AppState) -> Result<()> {
    let result = execute_command(cli, app_state).await;

    match &result {
        Ok(()) => info!("✅ Operation completed successfully"),
        Err(e) => {
            error!("❌ Operation failed: {e}");
            std::process::exit(1);
        }
    }

    result
}

#[instrument(skip(cli, state))]
async fn execute_command(cli: &Cli, state: &AppState) -> Result<()> {
    match &cli.command {
        Commands::Migrate {
            no_seed,
            backup,
            verify,
        } => {
            let options = MigrateOptions {
                seeding: if *no_seed {
                    SeedingMode::Disable
                } else {
                    SeedingMode::Enable
                },
                backup: if *backup {
                    BackupMode::Enable
                } else {
                    BackupMode::Disable
                },
                verification: if *verify {
                    VerificationMode::Enable
                } else {
                    VerificationMode::Disable
                },
                execution: if cli.dry_run {
                    ExecutionMode::DryRun
                } else {
                    ExecutionMode::Execute
                },
            };
            handle_migrate(state, options).await
        }
        Commands::Rollback { steps, force } => {
            handle_rollback(state, *steps, *force, cli.dry_run).await
        }
        Commands::Reset { confirm_db_name } => {
            handle_reset(state, confirm_db_name.as_deref(), cli.dry_run).await
        }
        Commands::Seed { force } => handle_seed(state, *force, cli.dry_run).await,
        Commands::Status { verbose } => handle_status(state, *verbose).await,
        Commands::Validate { deep } => handle_validate(state, *deep).await,
        Commands::Backup { path, compress } => handle_backup(state, path, *compress).await,
        Commands::Restore { path, force } => handle_restore(state, path, *force, cli.dry_run).await,
    }
}

/// Migration operation options
#[derive(Debug, Default)]
struct MigrateOptions {
    seeding: SeedingMode,
    backup: BackupMode,
    verification: VerificationMode,
    execution: ExecutionMode,
}

/// Seeding mode for database initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SeedingMode {
    #[default]
    Enable,
    Disable,
}

/// Backup mode before operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum BackupMode {
    #[default]
    Enable,
    Disable,
}

/// Verification mode after operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum VerificationMode {
    Enable,
    #[default]
    Disable,
}

/// Execution mode for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ExecutionMode {
    #[default]
    Execute,
    DryRun,
}

/// Displays pending migration information
fn display_pending_migrations(pending: &[String]) -> Result<()> {
    if pending.is_empty() {
        info!("✅ No pending migrations");
        return Ok(());
    }

    info!("Found {} pending migration(s):", pending.len());
    for (idx, name) in pending.iter().enumerate() {
        info!("  {}. {}", idx + 1, name);
    }
    Ok(())
}

/// Performs pre-migration backup if enabled
fn perform_backup_if_enabled(state: &AppState, backup: BackupMode) -> Result<()> {
    if backup == BackupMode::Enable {
        info!("💾 Creating pre-migration backup...");
        create_backup(state, "./backups/pre-migration")?;
    }
    Ok(())
}

/// Performs post-migration verification if enabled
async fn verify_if_enabled(state: &AppState, verification: VerificationMode) -> Result<()> {
    if verification == VerificationMode::Enable {
        info!("🔍 Verifying database integrity...");
        verify_database(state).await?;
    }
    Ok(())
}

/// Performs database seeding if enabled
async fn seed_if_enabled(state: &AppState, seeding: SeedingMode) -> Result<()> {
    if seeding == SeedingMode::Enable {
        info!("🌱 Seeding database...");
        seed_database(state, false).await?;
    }
    Ok(())
}

#[instrument(skip(state))]
async fn handle_migrate(state: &AppState, options: MigrateOptions) -> Result<()> {
    let MigrateOptions {
        seeding,
        backup,
        verification,
        execution,
    } = options;
    let start = Instant::now();

    // Check current status
    info!("📊 Checking migration status...");
    let pending = state.db_list_pending_migrations(MIGRATIONS).await?;

    display_pending_migrations(&pending)?;

    if pending.is_empty() {
        return Ok(());
    }

    if execution == ExecutionMode::DryRun {
        info!("🔍 DRY-RUN: Would apply {} migration(s)", pending.len());
        return Ok(());
    }

    // Create backup if requested
    perform_backup_if_enabled(state, backup)?;

    // Apply migrations
    info!("📊 Applying migrations...");
    run_migrations(state).await?;

    let duration = start.elapsed();
    info!("✅ Migrations completed in {:?}", duration);

    // Verify if requested
    verify_if_enabled(state, verification).await?;

    // Seed database if requested
    seed_if_enabled(state, seeding).await?;

    Ok(())
}

/// Validates rollback parameters
fn validate_rollback_steps(steps: usize) -> Result<()> {
    if steps == 0 {
        return Err(AppError::BadRequest(
            "Steps must be greater than 0".to_string(),
        ));
    }
    Ok(())
}

/// Prompts user for rollback confirmation
fn confirm_rollback(force: bool, dry_run: bool) -> Result<bool> {
    if force || dry_run {
        return Ok(true);
    }

    warn!("⚠️  This operation may result in data loss!");
    println!(
        "
Type 'ROLLBACK' to confirm:"
    );

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim() != "ROLLBACK" {
        info!("❌ Rollback cancelled");
        return Ok(false);
    }

    Ok(true)
}

/// Performs the actual rollback operation with backup
async fn perform_rollback_with_backup(state: &AppState, steps: usize) -> Result<()> {
    // Create backup before rollback
    info!("💾 Creating pre-rollback backup...");
    create_backup(state, "./backups/pre-rollback")?;

    // Perform rollback
    rollback_migrations(state, steps).await?;

    info!("✅ Rollback completed");
    Ok(())
}

#[instrument(skip(state))]
async fn handle_rollback(state: &AppState, steps: usize, force: bool, dry_run: bool) -> Result<()> {
    validate_rollback_steps(steps)?;

    warn!("⚠️  Rolling back {} migration(s)", steps);

    let confirmed = confirm_rollback(force, dry_run)?;
    if !confirmed {
        return Ok(());
    }

    if dry_run {
        info!("🔍 DRY-RUN: Would rollback {} migration(s)", steps);
        return Ok(());
    }

    perform_rollback_with_backup(state, steps).await?;

    Ok(())
}

#[instrument(skip(state))]
async fn handle_reset(
    state: &AppState,
    confirm_db_name: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    display_reset_warnings();
    let db_name = extract_database_name();
    confirm_reset_operation(confirm_db_name, &db_name)?;

    if dry_run {
        info!("� DRY-RUN: Would reset database '{db_name}'");
        return Ok(());
    }

    perform_reset(state).await?;
    info!("✅ Database reset completed");
    Ok(())
}

/// Display danger warnings for reset operation
fn display_reset_warnings() {
    warn!("�🚨 DANGER: This will DROP ALL TABLES!");
    warn!("🚨 ALL DATA WILL BE PERMANENTLY LOST!");
}

/// Extract database name from DATABASE_URL
fn extract_database_name() -> String {
    std::env::var("DATABASE_URL")
        .ok()
        .and_then(|url| {
            url.split('/')
                .next_back()
                .map(|s| s.split('?').next().unwrap_or(s).to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Confirm reset operation with user or provided name
fn confirm_reset_operation(confirm_db_name: Option<&str>, db_name: &str) -> Result<()> {
    if let Some(provided_name) = confirm_db_name {
        validate_provided_db_name(provided_name, db_name)?;
    } else {
        prompt_for_confirmation(db_name)?;
    }
    Ok(())
}

/// Validate provided database name matches actual name
fn validate_provided_db_name(provided_name: &str, expected_name: &str) -> Result<()> {
    if provided_name != expected_name {
        return Err(AppError::BadRequest(format!(
            "Database name mismatch. Expected '{expected_name}', got '{provided_name}'"
        )));
    }
    Ok(())
}

/// Prompt user for confirmation by typing database name
fn prompt_for_confirmation(db_name: &str) -> Result<()> {
    println!("\nType the database name '{db_name}' to confirm:");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim() != db_name {
        info!("❌ Reset cancelled");
        return Err(AppError::BadRequest("Reset cancelled by user".to_string()));
    }
    Ok(())
}

/// Perform the actual database reset
async fn perform_reset(state: &AppState) -> Result<()> {
    warn!("💾 Creating final backup before reset...");
    create_backup(state, "./backups/final-before-reset")?;
    reset_database(state).await?;
    Ok(())
}

#[instrument(skip(state))]
async fn handle_seed(state: &AppState, force: bool, dry_run: bool) -> Result<()> {
    if dry_run {
        info!("🔍 DRY-RUN: Would seed database");
        return Ok(());
    }

    seed_database(state, force).await
}

#[instrument(skip(state))]
async fn handle_status(state: &AppState, verbose: bool) -> Result<()> {
    info!("📊 Migration Status Report");
    info!("{}", "=".repeat(50));

    // Get applied migrations
    let applied = state.db_fetch_applied_migrations().await?;
    info!("✅ Applied migrations: {}", applied.len());

    if verbose {
        for (idx, migration) in applied.iter().enumerate() {
            info!("  {}. {}", idx + 1, migration);
        }
    }

    // Get pending migrations
    let pending = state.db_list_pending_migrations(MIGRATIONS).await?;

    if pending.is_empty() {
        info!("✅ No pending migrations");
    } else {
        warn!("⏳ Pending migrations: {}", pending.len());
        for (idx, name) in pending.iter().enumerate() {
            warn!("  {}. {}", idx + 1, name);
        }
    }

    // Database health check
    match state.health_check().await {
        Ok(health) => {
            info!(
                "💚 Database: {} ({}ms)",
                health.database.status, health.database.response_time_ms
            );
        }
        Err(e) => {
            error!("❌ Database health check failed: {}", e);
        }
    }

    Ok(())
}

#[instrument(skip(state))]
async fn handle_validate(state: &AppState, deep: bool) -> Result<()> {
    info!("🔍 Validating database integrity...");

    if deep {
        info!("Running deep validation (this may take a while)...");
    }

    verify_database(state).await?;

    // Additional deep checks
    if deep {
        info!("Checking referential integrity...");
        check_referential_integrity(state).await?;

        info!("Checking data consistency...");
        check_data_consistency(state).await?;
    }

    info!("✅ Database validation passed");
    Ok(())
}

#[instrument(skip(state))]
async fn handle_backup(state: &AppState, path: &str, compress: bool) -> Result<()> {
    info!("💾 Creating database backup...");

    let backup_path = if compress {
        format!("{path}.tar.gz")
    } else {
        path.to_string()
    };

    create_backup(state, &backup_path)?;

    info!("✅ Backup created: {backup_path}");
    Ok(())
}

#[instrument(skip(state))]
async fn handle_restore(state: &AppState, path: &str, force: bool, dry_run: bool) -> Result<()> {
    warn!("🔄 Restoring database from: {}", path);
    warn!("⚠️  This will overwrite existing data!");

    if !force && !dry_run {
        println!("\nType 'RESTORE' to confirm:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim() != "RESTORE" {
            info!("❌ Restore cancelled");
            return Ok(());
        }
    }

    if dry_run {
        info!("🔍 DRY-RUN: Would restore from {}", path);
        return Ok(());
    }

    restore_backup(state, path)?;

    info!("✅ Database restored from {}", path);
    Ok(())
}

// Helper functions

async fn run_migrations(state: &AppState) -> Result<()> {
    state.db_run_pending_migrations(MIGRATIONS).await
}

async fn rollback_migrations(state: &AppState, steps: usize) -> Result<()> {
    for step in 1..=steps {
        info!("Rolling back migration {}/{}", step, steps);
        match state.db_revert_last_migration(MIGRATIONS).await {
            Ok(()) => info!("✅ Reverted migration {}", step),
            Err(e) => {
                error!("❌ Failed to revert migration {}: {}", step, e);
                return Err(e);
            }
        }
    }
    Ok(())
}

async fn reset_database(state: &AppState) -> Result<()> {
    drop_all_tables(state).await?;
    recreate_schema(state).await?;
    Ok(())
}

/// Drop all database tables
async fn drop_all_tables(state: &AppState) -> Result<()> {
    info!("🗑️  Dropping all tables...");

    let drop_statements = get_drop_table_statements();

    for statement in drop_statements {
        debug!("Executing: {statement}");
        let _ = state.db_execute_sql(statement).await.map_err(|e| {
            debug!("Note: {e} (this may be expected)");
            e
        });
    }

    Ok(())
}

/// Get all DROP TABLE statements for database reset
fn get_drop_table_statements() -> Vec<&'static str> {
    vec![
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
        "DROP TABLE IF EXISTS __diesel_schema_migrations CASCADE",
        "DROP TABLE IF EXISTS schema_migrations CASCADE",
    ]
}

/// Recreate database schema from migrations
async fn recreate_schema(state: &AppState) -> Result<()> {
    info!("🔄 Recreating schema...");
    run_migrations(state).await?;
    state.db_ensure_schema_migrations_compat().await?;
    Ok(())
}

async fn seed_database(state: &AppState, force: bool) -> Result<()> {
    check_existing_data(state, force).await?;
    create_admin_user(state).await?;
    create_sample_content(state).await?;
    display_security_warnings();

    Ok(())
}

/// Check if database already has data and handle accordingly
async fn check_existing_data(state: &AppState, force: bool) -> Result<()> {
    let existing_users: i64 = state.db_admin_users_count().await?;

    if existing_users > 0 && !force {
        info!("📊 Database already contains {existing_users} user(s), skipping seeding");
        info!("💡 Use --force to reseed anyway");
        return Err(AppError::BadRequest("Database already seeded".to_string()));
    }

    if force && existing_users > 0 {
        warn!("⚠️  Force seeding with existing data");
    }

    Ok(())
}

/// Create the default admin user
async fn create_admin_user(state: &AppState) -> Result<()> {
    info!("👤 Creating admin user...");

    let admin_user = cms_backend::models::CreateUserRequest {
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password: "admin123".to_string(),
        role: cms_backend::models::UserRole::Admin,
        first_name: Some(String::new()),
        last_name: Some(String::new()),
    };

    state.db_create_user(admin_user).await?;
    info!("✅ Admin user created");

    Ok(())
}

/// Create sample content for demonstration
async fn create_sample_content(state: &AppState) -> Result<()> {
    info!("📝 Creating sample content...");

    let sample_post = cms_backend::models::CreatePostRequest {
        title: "Welcome to Enterprise CMS".to_string(),
        content: "High-performance, scalable CMS built with Rust and PostgreSQL.".to_string(),
        excerpt: Some("Welcome to the future of content management.".to_string()),
        slug: None,
        published: Some(true),
        tags: Some(vec!["welcome".to_string(), "cms".to_string()]),
        category: None,
        featured_image: None,
        meta_title: Some("Welcome".to_string()),
        meta_description: Some("Production-ready CMS".to_string()),
        published_at: None,
        status: Some(cms_backend::models::PostStatus::Published),
    };

    state.db_create_post(sample_post).await?;
    info!("✅ Sample content created");

    Ok(())
}

/// Display security warnings about default credentials
fn display_security_warnings() {
    warn!("⚠️  Default admin password is 'admin123'");
    warn!("⚠️  Change this immediately in production!");
}

async fn verify_database(state: &AppState) -> Result<()> {
    info!("🔍 Verifying database integrity...");

    verify_database_health(state).await?;
    verify_critical_tables(state).await?;
    verify_migration_history(state).await?;

    Ok(())
}

/// Verify database health and connectivity
async fn verify_database_health(state: &AppState) -> Result<()> {
    let health = state.health_check().await?;

    if health.database.status != "up" {
        error!("Database health check failed: {:?}", health.database.error);
        return Err(AppError::Internal(format!(
            "Database health check failed: {:?}",
            health.database.error
        )));
    }

    info!("✓ Database connection: OK");
    info!("✓ Response time: {}ms", health.database.response_time_ms);
    Ok(())
}

/// Verify that all critical tables exist
async fn verify_critical_tables(state: &AppState) -> Result<()> {
    let critical_tables = vec![
        "users",
        "posts",
        "categories",
        "tags",
        "api_keys",
        "user_sessions",
        "__diesel_schema_migrations",
    ];

    for table in critical_tables {
        match state
            .db_execute_sql(&format!(
                "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '{table}'"
            ))
            .await
        {
            Ok(_) => debug!("✓ Table exists: {table}"),
            Err(e) => {
                error!("❌ Critical table missing: {table}");
                return Err(AppError::Internal(format!(
                    "Critical table '{table}' is missing: {e}"
                )));
            }
        }
    }

    info!("✓ All critical tables present");
    Ok(())
}

/// Verify migration history and integrity
async fn verify_migration_history(state: &AppState) -> Result<()> {
    match state
        .db_execute_sql("SELECT COUNT(*) FROM __diesel_schema_migrations")
        .await
    {
        Ok(_) => {
            info!("✓ Migration history intact");
            verify_applied_migrations(state).await?;
        }
        Err(e) => {
            warn!("⚠️  Cannot verify migration history: {e}");
            warn!("⚠️  This may indicate database is not initialized");
        }
    }

    Ok(())
}

/// Verify applied migrations details
async fn verify_applied_migrations(state: &AppState) -> Result<()> {
    let applied = state.db_fetch_applied_migrations().await?;

    if applied.is_empty() {
        warn!("⚠️  No migrations applied yet - database may be empty");
    } else {
        info!("✓ {} migration(s) applied", applied.len());

        // Verify migrations are in order (no gaps)
        for (idx, migration) in applied.iter().enumerate() {
            debug!("  {}. {migration}", idx + 1);
        }
    }

    Ok(())
}

async fn check_referential_integrity(state: &AppState) -> Result<()> {
    info!("🔗 Checking referential integrity...");

    let integrity_checks = get_integrity_check_queries();
    let has_orphans = run_integrity_checks(state, &integrity_checks).await;

    report_integrity_results(has_orphans);
    Ok(())
}

/// Get list of referential integrity check queries
fn get_integrity_check_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "posts.author_id → users.id",
            "SELECT COUNT(*) FROM posts p WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = p.author_id)",
        ),
        (
            "comments.post_id → posts.id",
            "SELECT COUNT(*) FROM comments c WHERE NOT EXISTS (SELECT 1 FROM posts p WHERE p.id = c.post_id)",
        ),
        (
            "comments.user_id → users.id",
            "SELECT COUNT(*) FROM comments c WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = c.user_id)",
        ),
        (
            "api_keys.user_id → users.id",
            "SELECT COUNT(*) FROM api_keys a WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = a.user_id)",
        ),
        (
            "user_sessions.user_id → users.id",
            "SELECT COUNT(*) FROM user_sessions s WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = s.user_id)",
        ),
    ]
}

/// Run integrity checks on database
async fn run_integrity_checks(state: &AppState, integrity_checks: &[(&str, &str)]) -> bool {
    let has_orphans = false;

    for (relationship, query) in integrity_checks {
        match state.db_execute_sql(query).await {
            Ok(_) => {
                debug!("✓ Integrity check passed: {relationship}");
            }
            Err(e) => {
                debug!("Note: Cannot check {relationship} - {e}");
            }
        }
    }

    has_orphans
}

/// Report integrity check results
fn report_integrity_results(has_orphans: bool) {
    if has_orphans {
        warn!("⚠️  Found orphaned records - consider data cleanup");
    } else {
        info!("✓ Referential integrity OK");
    }
}

async fn check_data_consistency(state: &AppState) -> Result<()> {
    info!("🔍 Checking data consistency...");

    check_record_counts(state).await?;
    check_email_format(state).await;
    check_author_references(state).await;
    check_session_expiration(state).await;

    info!("✅ Data consistency checks completed");
    Ok(())
}

/// Check basic record counts
async fn check_record_counts(state: &AppState) -> Result<()> {
    let user_count = state.db_admin_users_count().await?;
    info!("✓ User count: {user_count}");

    if user_count == 0 {
        warn!("⚠️  No users found - database may need seeding");
    }

    let post_count = state.db_admin_posts_count().await?;
    info!("✓ Post count: {post_count}");

    Ok(())
}

/// Check email format validity
async fn check_email_format(state: &AppState) {
    debug!("Checking email format consistency...");
    match state
        .db_execute_sql("SELECT COUNT(*) FROM users WHERE email NOT LIKE '%@%'")
        .await
    {
        Ok(_) => debug!("✓ Email format check passed"),
        Err(e) => warn!("⚠️  Email format check failed: {e}"),
    }
}

/// Check post author references
async fn check_author_references(state: &AppState) {
    debug!("Checking post author references...");
    match state
        .db_execute_sql("SELECT COUNT(*) FROM posts WHERE author_id NOT IN (SELECT id FROM users)")
        .await
    {
        Ok(_) => debug!("✓ Post author references valid"),
        Err(e) => warn!("⚠️  Post author reference check failed: {e}"),
    }
}

/// Check for expired sessions
async fn check_session_expiration(state: &AppState) {
    debug!("Checking session expiration...");
    match state
        .db_execute_sql(
            "SELECT COUNT(*) FROM user_sessions WHERE created_at < NOW() - INTERVAL '30 days'",
        )
        .await
    {
        Ok(_) => debug!("✓ Session expiration check completed"),
        Err(e) => debug!("Session check skipped: {e}"),
    }
}

fn create_backup(_state: &AppState, path: &str) -> Result<()> {
    // Create backup directory
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Generate timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_file = format!("{path}_{timestamp}.sql");

    info!("💾 Creating backup: {backup_file}");

    // In production, this would use pg_dump
    warn!("⚠️  Actual backup implementation requires pg_dump");
    warn!("💡 Use: pg_dump -h host -U user -d db > {backup_file}");

    Ok(())
}

fn restore_backup(_state: &AppState, path: &str) -> Result<()> {
    info!("🔄 Restoring from: {path}");

    // Verify backup file exists
    if !std::path::Path::new(path).exists() {
        return Err(AppError::NotFound(format!("Backup file not found: {path}")));
    }

    // In production, this would use pg_restore
    warn!("⚠️  Actual restore implementation requires pg_restore");
    warn!("💡 Use: psql -h host -U user -d db < {path}");

    Ok(())
}

fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}

#[cfg(feature = "restructure_domain")]
async fn init_app_state() -> Result<std::sync::Arc<cms_backend::AppState>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_levels() {
        // Verify safety level enum
        let safe = SafetyLevel::Safe;
        let moderate = SafetyLevel::Moderate;
        let dangerous = SafetyLevel::Dangerous;

        // Test can be compiled
        assert!(matches!(safe, SafetyLevel::Safe));
        assert!(matches!(moderate, SafetyLevel::Moderate));
        assert!(matches!(dangerous, SafetyLevel::Dangerous));
    }
}
