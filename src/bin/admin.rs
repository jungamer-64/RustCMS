//! CMS Administration CLI Tool
//!
//! Comprehensive command-line interface for managing users, content,
//! system settings, and performing administrative tasks.

use clap::{Parser, Subcommand, ValueEnum};
use cms_backend::{
    AppState, Result,
    models::{CreateUserRequest, UpdateUserRequest, UserRole},
};
use async_trait::async_trait;
use comfy_table::{Cell, Table};
use ring::rand::{SecureRandom, SystemRandom};
use secrecy::{ExposeSecret, SecretString};
use std::io::{self, Write};
use tokio::task;
use std::fmt;
use tracing::{info, warn};

/// Small trait to abstract AppState DB operations for easier testing of CLI logic.
#[async_trait]
pub trait AdminBackend: Sync + Send {
    async fn create_user(
        &self,
        req: CreateUserRequest,
    ) -> cms_backend::Result<cms_backend::models::User>;

    async fn reset_user_password(
        &self,
        user_id: uuid::Uuid,
        new_password: &str,
    ) -> cms_backend::Result<()>;

    async fn get_user_by_id(&self, id: uuid::Uuid) -> cms_backend::Result<cms_backend::models::User>;

    async fn get_user_by_username(&self, username: &str) -> cms_backend::Result<cms_backend::models::User>;

    async fn update_user(
        &self,
        id: uuid::Uuid,
        req: UpdateUserRequest,
    ) -> cms_backend::Result<cms_backend::models::User>;

    async fn delete_user(&self, id: uuid::Uuid) -> cms_backend::Result<()>;
}

#[async_trait]
impl AdminBackend for AppState {
    async fn create_user(
        &self,
        req: CreateUserRequest,
    ) -> cms_backend::Result<cms_backend::models::User> {
        self.db_create_user(req).await
    }

    async fn reset_user_password(&self, user_id: uuid::Uuid, new_password: &str) -> cms_backend::Result<()> {
        self.db_reset_user_password(user_id, new_password).await
    }

    async fn get_user_by_id(&self, id: uuid::Uuid) -> cms_backend::Result<cms_backend::models::User> {
        self.db_get_user_by_id(id).await
    }

    async fn get_user_by_username(&self, username: &str) -> cms_backend::Result<cms_backend::models::User> {
        self.db_get_user_by_username(username).await
    }

    async fn update_user(&self, id: uuid::Uuid, req: UpdateUserRequest) -> cms_backend::Result<cms_backend::models::User> {
        self.db_update_user(id, req).await
    }

    async fn delete_user(&self, id: uuid::Uuid) -> cms_backend::Result<()> {
        self.db_delete_user(id).await
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab_case")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

impl fmt::Display for PostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PostStatus::Draft => "draft",
            PostStatus::Published => "published",
            PostStatus::Archived => "archived",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab_case")]
pub enum Period {
    Day,
    Week,
    Month,
    Year,
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Period::Day => "day",
            Period::Week => "week",
            Period::Month => "month",
            Period::Year => "year",
        };
        write!(f, "{}", s)
    }
}

#[derive(Parser)]
#[command(name = "cms-admin")]
#[command(about = "CMS Administration Tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// User management commands
    User {
        #[command(subcommand)]
        action: UserAction,
    },
    /// Content management commands
    Content {
        #[command(subcommand)]
        action: ContentAction,
    },
    /// System administration commands
    System {
        #[command(subcommand)]
        action: SystemAction,
    },
    /// Analytics and reporting
    Analytics {
        #[command(subcommand)]
        action: AnalyticsAction,
    },
    /// Security and audit commands
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },
}

#[derive(Subcommand)]
enum UserAction {
    /// List all users
    List {
        /// Filter by role (admin, editor, user)
        #[arg(long, value_enum)]
        role: Option<UserRole>,
        /// Show only active users
        #[arg(long)]
        active_only: bool,
    },
    /// Create a new user
    Create {
        /// Username
        #[arg(short, long)]
        username: String,
        /// Email address
        #[arg(short, long)]
        email: String,
        /// User role (admin, editor, user)
        #[arg(short, long, value_enum, default_value_t = UserRole::Subscriber)]
        role: UserRole,
        /// Generate random password
        #[arg(long)]
        generate_password: bool,
    },
    /// Update user information
    Update {
        /// User ID or username
        user: String,
        /// New email
        #[arg(long)]
        email: Option<String>,
        /// New role
        #[arg(long, value_enum)]
        role: Option<UserRole>,
        /// Activate/deactivate user
        #[arg(long)]
        active: Option<bool>,
    },
    /// Delete a user
    Delete {
        /// User ID or username
        user: String,
        /// Force deletion without confirmation
        #[arg(long)]
        force: bool,
    },
    /// Reset user password
    ResetPassword {
        /// User ID or username
        user: String,
        /// New password (omit to be prompted)
        #[arg(long, conflicts_with = "generate_password")]
        password: Option<String>,
        /// Generate a random password
        #[arg(long)]
        generate_password: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ContentAction {
    /// List posts
    List {
        /// Filter by status
        #[arg(long, value_enum)]
        status: Option<PostStatus>,
        /// Filter by author
        #[arg(long)]
        author: Option<String>,
        /// Limit number of results
        #[arg(long, default_value = "10")]
        limit: i64,
    },
    /// Create a new post
    Create {
        /// Post title
        #[arg(short, long)]
        title: String,
        /// Content file path
        #[arg(short, long)]
        file: Option<String>,
        /// Author username
        #[arg(short, long)]
        author: String,
        /// Post status
        #[arg(long, value_enum, default_value_t = PostStatus::Draft)]
        status: PostStatus,
    },
    /// Publish scheduled posts
    PublishScheduled,
    /// Rebuild search index
    ReindexSearch,
    /// Clean up orphaned media files
    CleanupMedia,
}

#[derive(Subcommand)]
enum SystemAction {
    /// Show system status
    Status,
    /// Update system settings
    Settings {
        /// Setting key
        key: String,
        /// Setting value (JSON)
        value: String,
    },
    /// Cache management
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Database maintenance
    Database {
        #[command(subcommand)]
        action: DatabaseAction,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Clear all cache
    Clear,
    /// Show cache statistics
    Stats,
    /// Warm up cache
    Warmup,
}

#[derive(Subcommand)]
enum DatabaseAction {
    /// Show database statistics
    Stats,
    /// Optimize database
    Optimize,
    /// Check database integrity
    Check,
}

#[derive(Subcommand)]
enum AnalyticsAction {
    /// Show user statistics
    Users {
        /// Time period (day, week, month, year)
        #[arg(long, default_value_t = Period::Month)]
        period: Period,
    },
    /// Show content statistics
    Content {
        /// Time period
        #[arg(long, default_value_t = Period::Month)]
        period: Period,
    },
    /// Show performance metrics
    Performance {
        /// Time period
        #[arg(long, default_value_t = Period::Day)]
        period: Period,
    },
}

#[derive(Subcommand)]
enum SecurityAction {
    /// Show security audit log
    AuditLog {
        /// Number of recent entries
        #[arg(long, default_value = "50")]
        limit: i64,
        /// Filter by user
        #[arg(long)]
        user: Option<String>,
        /// Filter by action
        #[arg(long)]
        action: Option<String>,
    },
    /// List active sessions
    Sessions,
    /// Revoke user sessions
    RevokeSessions {
        /// User ID or username
        user: String,
    },
    /// List API keys
    ApiKeys {
        /// Show only active keys
        #[arg(long)]
        active_only: bool,
    },
    /// Revoke API key
    RevokeApiKey {
        /// API key ID or prefix
        key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli).await?;
    Ok(())
}

async fn run(cli: Cli) -> Result<()> {
    // Initialize full AppState, honoring the verbose flag
    let app_state = cms_backend::utils::init::init_app_state_with_verbose(cli.verbose).await?;

    info!("🔧 CMS Administration Tool v{}", env!("CARGO_PKG_VERSION"));

    // Execute command
    match cli.command {
        Commands::User { action } => handle_user_action(action, &app_state).await?,
        Commands::Content { action } => handle_content_action(action, &app_state)?,
        Commands::System { action } => handle_system_action(action, &app_state).await?,
        Commands::Analytics { action } => handle_analytics_action(action, &app_state),
        Commands::Security { action } => handle_security_action(action, &app_state),
    }

    Ok(())
}

async fn handle_user_action(action: UserAction, state: &AppState) -> Result<()> {
    match action {
        UserAction::List { role, active_only } => user_list(&role, active_only, state).await?,
        UserAction::Create {
            username,
            email,
            role,
            generate_password,
        } => user_create(username, email, role, generate_password, state).await?,
        UserAction::Update {
            user,
            email,
            role,
            active,
        } => user_update(user, email, role, active, state).await?,
        UserAction::Delete { user, force } => user_delete(user, force, state).await?,
        UserAction::ResetPassword {
            user,
            password,
            generate_password,
        } => user_reset_password(user, password, generate_password, state).await?,
    }

    Ok(())
}

async fn user_list(role: &Option<UserRole>, active_only: bool, state: &AppState) -> Result<()> {
    info!("📊 Listing users...");
    let role_filter: Option<&str> = role.as_ref().map(|r| r.as_str());
    let active_filter = if active_only { Some(true) } else { None };
    let users = state
        .database
        .list_users(role_filter, active_filter)
        .await?;

    if users.is_empty() {
        println!("No users found matching the criteria.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["ID", "Username", "Email", "Role", "Active"]);

    for user in users {
        table.add_row(vec![
            Cell::new(user.id.to_string()),
            Cell::new(user.username),
            Cell::new(user.email),
            Cell::new(user.role),
            Cell::new(if user.is_active { "Yes" } else { "No" }),
        ]);
    }

    println!("{table}");

    Ok(())
}

async fn user_create<B: AdminBackend + ?Sized>(
    username: String,
    email: String,
    role: UserRole,
    generate_password: bool,
    backend: &B,
) -> Result<()> {
    let password = if generate_password {
        // Allow optional override from env var ADMIN_PW_LENGTH
        let len = std::env::var("ADMIN_PW_LENGTH")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .map(|v| v.clamp(8, 128))
            .unwrap_or(16);
        generate_random_password_with_len(len)?
    } else {
        // prompt_password is blocking: run it in a blocking task
    prompt_password_async("Enter password for new user: ".to_string()).await?
    };

    let password_for_request = password.expose_secret().to_owned();

    let user = CreateUserRequest {
        username: username.clone(),
        email,
        password: password_for_request,
        first_name: Some(String::new()),
        last_name: Some(String::new()),
        role,
    };

    let created_user = backend.create_user(user).await?;

    info!("✅ User created successfully:");
    println!("  ID: {}", created_user.id);
    println!("  Username: {}", created_user.username);
    println!("  Email: {}", created_user.email);
    println!("  Role: {}", created_user.role);

    if generate_password {
        // Do not log the password itself.
        warn!("🔑 A new random password has been generated.");
        warn!("⚠️  Please save this password securely - it will not be shown again!");
        // Forcing the user to see the password is a better UX than logging it.
        println!("Generated password: {}", password.expose_secret());
    }

    Ok(())
}

async fn user_update<B: AdminBackend + ?Sized>(
    user: String,
    email: Option<String>,
    role: Option<UserRole>,
    active: Option<bool>,
    backend: &B,
) -> Result<()> {
    let existing_user = find_user_by_id_or_username(backend, &user).await?;

    let update = UpdateUserRequest {
        username: None,
        email: email.clone(),
        first_name: None,
        last_name: None,
        role,
        is_active: active,
    };

    let updated_user = backend.update_user(existing_user.id, update).await?;

    info!("✅ User updated successfully:");
    println!("  ID: {}", updated_user.id);
    println!("  Username: {}", updated_user.username);
    println!("  Email: {}", updated_user.email);
    println!("  Role: {}", updated_user.role);
    println!("  Active: {}", updated_user.is_active);

    Ok(())
}

async fn user_delete<B: AdminBackend + ?Sized>(user: String, force: bool, backend: &B) -> Result<()> {
    let existing_user = find_user_by_id_or_username(backend, &user).await?;

    if !force {
        warn!(
            "⚠️  You are about to delete user: {} ({})",
            existing_user.username, existing_user.email
        );
        warn!("⚠️  This action cannot be undone!");

        // Run blocking terminal I/O in a blocking task to avoid stalling the async runtime.
        let confirmed = task::spawn_blocking(move || -> Result<bool> {
            print!("Type 'DELETE' to confirm: ");
            io::stdout().flush().map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;
            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;
            Ok(input.trim().eq_ignore_ascii_case("DELETE"))
        })
        .await
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))??;

        if !confirmed {
            info!("❌ User deletion cancelled");
            return Ok(());
        }
    }

    backend.delete_user(existing_user.id).await?;
    info!("✅ User deleted successfully");

    Ok(())
}

async fn user_reset_password<B: AdminBackend + ?Sized>(
    user: String,
    password: Option<String>,
    generate_password: bool,
    backend: &B,
) -> Result<()> {
    let existing_user = find_user_by_id_or_username(backend, &user).await?;

    // Clear, explicit intent handling
    let new_password = match (password, generate_password) {
        (Some(p), false) => Ok(SecretString::new(p.into_boxed_str())),
        (None, true) => {
            let len = std::env::var("ADMIN_PW_LENGTH")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .map(|v| v.clamp(8, 128))
                .unwrap_or(16);
            generate_random_password_with_len(len)
        }
    (None, false) => prompt_password_async("Enter new password: ".to_string()).await,
        (Some(_), true) => unreachable!(),
    }?;

    backend
        .reset_user_password(existing_user.id, new_password.expose_secret())
        .await?;

    info!(
        "✅ Password reset successfully for user: {}",
        existing_user.username
    );
    if generate_password {
        warn!(
            "🔑 A new random password has been generated for user: {}",
            existing_user.username
        );
        println!("Generated password: {}", new_password.expose_secret());
    }

    Ok(())
}

fn handle_content_action(action: ContentAction, _state: &AppState) -> Result<()> {
    warn!(
        "'Content' command invoked but not implemented: {:?}. Returning NotImplemented.",
        action
    );
    println!(
        "Content commands are not yet available in this CLI build. Refer to CLI.md for the roadmap."
    );
    Err(cms_backend::AppError::NotImplemented(
        "Content commands are not implemented in this build".into(),
    ))
}

async fn handle_system_action(action: SystemAction, state: &AppState) -> Result<()> {
    match action {
        SystemAction::Status => system_status(state).await?,
        SystemAction::Settings { key, value } => system_settings(&key, &value, state).await?,
        SystemAction::Cache { action } => system_cache_action(action, state).await?,
        SystemAction::Database { action } => system_database_action(action, state).await?,
    }

    Ok(())
}

/// Retrieve the application's aggregated health status from `AppState`
/// and render it using the shared `render_health_table_components` helper.
async fn system_status(state: &AppState) -> Result<()> {
    let health = state.health_check().await?;
    let table = cms_backend::utils::bin_utils::render_health_table_components(
        &health.status,
        (
            &health.database.status,
            health.database.response_time_ms,
            health.database.error.as_deref(),
        ),
        (
            &health.cache.status,
            health.cache.response_time_ms,
            health.cache.error.as_deref(),
        ),
        (
            &health.search.status,
            health.search.response_time_ms,
            health.search.error.as_deref(),
        ),
        (
            &health.auth.status,
            health.auth.response_time_ms,
            health.auth.error.as_deref(),
        ),
    );
    println!("{table}");

    Ok(())
}

/// Render a HealthStatus into a comfy_table::Table and return its string form.
// Leverage utils::bin_utils::render_health_table_components for rendering.
#[cfg(test)]
mod system_status_tests {
    // no super imports required for this test

    #[test]
    fn render_health_table_basic() {
        let overall = "healthy";
        let db = ("up", 12.34_f64, None::<&str>);
        let cache = ("up", 5.67_f64, None::<&str>);
        let search = ("up", 7.89_f64, None::<&str>);
        let auth = ("up", 3.21_f64, None::<&str>);

        let table = cms_backend::utils::bin_utils::render_health_table_components(
            overall, db, cache, search, auth,
        );
        let s = format!("{table}");

        // Basic assertions: header and a few component names
        assert!(s.contains("Component"));
        assert!(s.contains("Overall"));
        assert!(s.contains("Database"));
        assert!(s.contains("Cache"));
        assert!(s.contains("Search"));
        assert!(s.contains("Auth"));
        // Check response times formatted
        assert!(s.contains("12.34") || s.contains("12.3"));
        assert!(s.contains("5.67") || s.contains("5.7"));
    }
}

async fn system_settings(key: &str, value: &str, _state: &AppState) -> Result<()> {
    info!("⚙️  Updating setting: {} = {}", key, value);
    // Implementation would update system setting
    Ok(())
}

async fn system_cache_action(action: CacheAction, _state: &AppState) -> Result<()> {
    match action {
        CacheAction::Clear => {
            info!("🧹 Clearing cache...");
            // Implementation would clear cache
        }
        CacheAction::Stats => {
            info!("📊 Cache Statistics:");
            // Implementation would show cache stats
        }
        CacheAction::Warmup => {
            info!("🔥 Warming up cache...");
            // Implementation would warm up cache
        }
    }
    Ok(())
}

async fn system_database_action(action: DatabaseAction, _state: &AppState) -> Result<()> {
    match action {
        DatabaseAction::Stats => {
            info!("📊 Database Statistics:");
            // Implementation would show database stats
        }
        DatabaseAction::Optimize => {
            info!("⚡ Optimizing database...");
            // Implementation would optimize database
        }
        DatabaseAction::Check => {
            info!("🔍 Checking database integrity...");
            // Implementation would check database
        }
    }
    Ok(())
}

fn handle_analytics_action(action: AnalyticsAction, _state: &AppState) {
    match action {
        AnalyticsAction::Users { period } => {
            info!("📊 User Analytics ({})", period);
            // Implementation would show user analytics
        }

        AnalyticsAction::Content { period } => {
            info!("📊 Content Analytics ({})", period);
            // Implementation would show content analytics
        }

        AnalyticsAction::Performance { period } => {
            info!("📊 Performance Metrics ({})", period);
            // Implementation would show performance metrics
        }
    }
}

fn handle_security_action(action: SecurityAction, _state: &AppState) {
    match action {
        SecurityAction::AuditLog {
            limit,
            user,
            action,
        } => {
            info!(
                "🔒 Security Audit Log (limit: {}, user: {:?}, action: {:?})",
                limit, user, action
            );
            // Implementation would show audit log
        }

        SecurityAction::Sessions => {
            info!("🔓 Active Sessions:");
            // Implementation would list active sessions
        }

        SecurityAction::RevokeSessions { user } => {
            info!("🔒 Revoking sessions for user: {}", user);
            // Implementation would revoke sessions
        }

        SecurityAction::ApiKeys { active_only } => {
            info!("🔑 API Keys (active only: {})", active_only);
            // Implementation would list API keys
        }

        SecurityAction::RevokeApiKey { key } => {
            info!("🔒 Revoking API key: {}", key);
            // Implementation would revoke API key
        }
    }
}

// Utility functions

// utility helpers

fn generate_random_password() -> Result<SecretString> {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let charset_len = CHARSET.len() as u16;
    let threshold: u16 = 256u16 - (256u16 % charset_len);
    let rng = SystemRandom::new();

    let mut password = String::with_capacity(16);
    let mut byte = [0u8; 1];
    while password.len() < 16 {
        // Fail fast if RNG can't produce bytes
        rng.fill(&mut byte).map_err(|_| {
            cms_backend::AppError::Internal(
                "Failed to read from system's entropy source".to_string(),
            )
        })?;
        let v = byte[0] as u16;
        if v < threshold {
            let idx = (v % charset_len) as usize;
            password.push(CHARSET[idx] as char);
        }
    }
    Ok(SecretString::new(password.into_boxed_str()))
}

fn prompt_password(prompt: &str) -> Result<SecretString> {
    // Use rpassword to securely read password without echoing to the terminal
    let password = rpassword::prompt_password(prompt)
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;

    if password.is_empty() {
        return Err(cms_backend::AppError::BadRequest(
            "Password cannot be empty".to_string(),
        ));
    }

    Ok(SecretString::new(password.into_boxed_str()))
}

/// Async-friendly wrapper around blocking password prompt.
async fn prompt_password_async(prompt: String) -> Result<SecretString> {
    task::spawn_blocking(move || prompt_password(&prompt))
        .await
        .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?
}

fn generate_random_password_with_len(len: usize) -> Result<SecretString> {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let charset_len = CHARSET.len() as u16;
    let threshold: u16 = 256u16 - (256u16 % charset_len);
    let rng = SystemRandom::new();

    let mut password = String::with_capacity(len);
    let mut byte = [0u8; 1];
    while password.len() < len {
        rng.fill(&mut byte).map_err(|_| {
            cms_backend::AppError::Internal(
                "Failed to read from system's entropy source".to_string(),
            )
        })?;
        let v = byte[0] as u16;
        if v < threshold {
            let idx = (v % charset_len) as usize;
            password.push(CHARSET[idx] as char);
        }
    }
    Ok(SecretString::new(password.into_boxed_str()))
}

/// Find user by UUID or username and return a NotFound AppError if missing
async fn find_user_by_id_or_username<B: AdminBackend + ?Sized>(
    backend: &B,
    identifier: &str,
) -> Result<cms_backend::models::User> {
    // Attempt to parse as UUID first; otherwise treat as username.
    let result = if let Ok(id) = uuid::Uuid::parse_str(identifier) {
        backend.get_user_by_id(id).await
    } else {
        backend.get_user_by_username(identifier).await
    };

    match result {
        Ok(user) => Ok(user),
        Err(e) => {
            // Log the original backend error for diagnostics.
            tracing::debug!(identifier = %identifier, backend_error = %format!("{e}"), "User lookup failed");

            // Prefer matching on the concrete AppError variant rather than
            // relying on error message text. This is more robust if underlying
            // DB libraries change their Display text.
            match e {
                cms_backend::AppError::NotFound(_) => {
                    Err(cms_backend::AppError::NotFound(format!("User '{}' not found", identifier)))
                }
                other => Err(other),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use cms_backend::models::{User as BackendUser, UserRole as BackendUserRole};

    #[test]
    fn test_generate_random_password_length_and_charset() -> Result<()> {
        // Generate multiple passwords to reduce flakiness
        for _ in 0..8 {
            let pw = generate_random_password()?;
            // length
            let secret = pw.expose_secret();
            assert_eq!(secret.len(), 16);

            // allowed chars
            for ch in secret.chars() {
                let bytes = ch as u8;
                // must be ASCII printable and in our CHARSET
                let found = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*".contains(&bytes);
                assert!(found, "password contains invalid char: {}", ch);
            }
        }
        Ok(())
    }

    // Configurable MockBackend for testing AdminBackend consumer logic.
    #[derive(Clone)]
    struct MockBackend {
        user_to_return: Option<BackendUser>,
    // Closure generator to produce an AppError when needed. Using a closure
    // avoids requiring AppError: Clone on the backend error type.
    error_to_return: Option<std::sync::Arc<dyn Fn() -> cms_backend::AppError + Send + Sync>>,
    }

    impl MockBackend {
        fn ok() -> Self {
            Self {
                user_to_return: Some(BackendUser::new(
                    "byname_user".to_string(),
                    "byname@example.com".to_string(),
                    Some("hash".to_string()),
                    None,
                    None,
                    BackendUserRole::Subscriber,
                )),
                error_to_return: None,
            }
        }

        fn not_found() -> Self {
            Self {
                user_to_return: None,
                error_to_return: Some(std::sync::Arc::new(|| cms_backend::AppError::NotFound("mock: not found".into()))),
            }
        }
    }

    #[async_trait]
    impl AdminBackend for MockBackend {
        async fn create_user(
            &self,
            req: CreateUserRequest,
        ) -> cms_backend::Result<cms_backend::models::User> {
            if let Some(f) = &self.error_to_return {
                return Err((f)());
            }
            Ok(BackendUser::new(
                req.username,
                req.email,
                Some("hashedpw".to_string()),
                req.first_name,
                req.last_name,
                req.role,
            ))
        }

        async fn reset_user_password(
            &self,
            _user_id: uuid::Uuid,
            _new_password: &str,
        ) -> cms_backend::Result<()> {
            if let Some(f) = &self.error_to_return {
                return Err((f)());
            }
            Ok(())
        }

        async fn get_user_by_id(&self, _id: uuid::Uuid) -> cms_backend::Result<cms_backend::models::User> {
            if let Some(f) = &self.error_to_return {
                return Err((f)());
            }
            if let Some(u) = &self.user_to_return {
                return Ok(u.clone());
            }
            Err(cms_backend::AppError::NotFound("mock: not found".into()))
        }

        async fn get_user_by_username(&self, _username: &str) -> cms_backend::Result<cms_backend::models::User> {
            if let Some(f) = &self.error_to_return {
                return Err((f)());
            }
            if let Some(u) = &self.user_to_return {
                return Ok(u.clone());
            }
            Err(cms_backend::AppError::NotFound("mock: not found".into()))
        }

        async fn update_user(
            &self,
            _id: uuid::Uuid,
            _req: UpdateUserRequest,
        ) -> cms_backend::Result<cms_backend::models::User> {
            if let Some(f) = &self.error_to_return {
                return Err((f)());
            }
            Ok(BackendUser::new(
                "updated_user".to_string(),
                "updated@example.com".to_string(),
                Some("hash".to_string()),
                None,
                None,
                BackendUserRole::Subscriber,
            ))
        }

        async fn delete_user(&self, _id: uuid::Uuid) -> cms_backend::Result<()> {
            if let Some(f) = &self.error_to_return {
                return Err((f)());
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_user_create_with_mock_backend() -> Result<()> {
    let backend = MockBackend::ok();
        let res = user_create(
            "testuser".to_string(),
            "test@example.com".to_string(),
            BackendUserRole::Subscriber,
            true,
            &backend,
        )
        .await;
        assert!(res.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_user_reset_password_with_mock_backend() -> Result<()> {
    let backend = MockBackend::ok();
        let res = user_reset_password(
            "someuser".to_string(),
            None,
            true,
            &backend,
        )
        .await;
        assert!(res.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_user_update_and_delete_with_mock_backend() -> Result<()> {
    let backend = MockBackend::ok();

        let update_res = user_update(
            "someuser".to_string(),
            Some("new@example.com".to_string()),
            None,
            Some(true),
            &backend,
        )
        .await;
        assert!(update_res.is_ok());

        let delete_res = user_delete("someuser".to_string(), true, &backend).await;
        assert!(delete_res.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_user_lookup_not_found_with_mock_backend() {
        let backend = MockBackend::not_found();
        // When the backend reports not found for lookups, the helper should
        // return an AppError::NotFound variant.
        let id = uuid::Uuid::new_v4();
        let res = find_user_by_id_or_username::<MockBackend>(&backend, &id.to_string()).await;
        match res {
            Err(cms_backend::AppError::NotFound(msg)) => {
                assert!(msg.contains("mock: not found") || msg.contains("User"));
            }
            other => panic!("expected NotFound error, got: {:?}", other),
        }
    }
}
