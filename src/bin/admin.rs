//! CMS Administration CLI Tool
//!
//! Comprehensive command-line interface for managing users, content,
//! system settings, and performing administrative tasks.

use clap::{Parser, Subcommand};
use cms_backend::{
    AppState, Result,
    models::{CreateUserRequest, UpdateUserRequest, UserRole},
};
use std::io::{self, Write};
use tracing::{info, warn};
use comfy_table::{Table, Cell};
use ring::rand::{SecureRandom, SystemRandom};

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

#[derive(Subcommand)]
enum ContentAction {
    /// List posts
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
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
        #[arg(long, default_value = "draft")]
        status: String,
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
        #[arg(long, default_value = "month")]
        period: String,
    },
    /// Show content statistics
    Content {
        /// Time period
        #[arg(long, default_value = "month")]
        period: String,
    },
    /// Show performance metrics
    Performance {
        /// Time period
        #[arg(long, default_value = "day")]
        period: String,
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

    info!(
        "🔧 CMS Administration Tool v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Execute command
    match cli.command {
        Commands::User { action } => handle_user_action(action, &app_state).await?,
        Commands::Content { action } => handle_content_action(action, &app_state),
        Commands::System { action } => handle_system_action(action, &app_state).await?,
        Commands::Analytics { action } => handle_analytics_action(action, &app_state),
        Commands::Security { action } => handle_security_action(action, &app_state),
    }

    Ok(())
}

async fn handle_user_action(action: UserAction, state: &AppState) -> Result<()> {
    match action {
        UserAction::List { role, active_only } => user_list(&role, active_only, state).await?,
        UserAction::Create { username, email, role, generate_password } => {
            user_create(username, email, role, generate_password, state).await?
        }
        UserAction::Update { user, email, role, active } => {
            user_update(user, email, role, active, state).await?
        }
        UserAction::Delete { user, force } => user_delete(user, force, state).await?,
        UserAction::ResetPassword { user, password, generate_password } => {
            user_reset_password(user, password, generate_password, state).await?
        }
    }

    Ok(())
}

async fn user_list(role: &Option<UserRole>, active_only: bool, state: &AppState) -> Result<()> {
    info!("📊 Listing users...");
    let role_filter: Option<&str> = role.as_ref().map(|r| r.as_str());
    let active_filter = if active_only { Some(true) } else { None };
    let users = state.database.list_users(role_filter, active_filter).await?;

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

async fn user_create(
    username: String,
    email: String,
    role: UserRole,
    generate_password: bool,
    state: &AppState,
) -> Result<()> {
    let password = if generate_password {
        generate_random_password()
    } else {
        prompt_password("Enter password for new user: ")?
    };

    let user = CreateUserRequest {
        username: username.clone(),
        email,
        password: password.clone(),
        first_name: Some(String::new()),
        last_name: Some(String::new()),
        role,
    };

    let created_user = state.db_create_user(user).await?;

    info!("✅ User created successfully:");
    println!("  ID: {}", created_user.id);
    println!("  Username: {}", created_user.username);
    println!("  Email: {}", created_user.email);
    println!("  Role: {}", created_user.role);

    if generate_password {
        warn!("🔑 Generated password: {}", password);
        warn!("⚠️  Please save this password securely - it will not be shown again!");
    }

    Ok(())
}

async fn user_update(
    user: String,
    email: Option<String>,
    role: Option<UserRole>,
    active: Option<bool>,
    state: &AppState,
) -> Result<()> {
    let existing_user = find_user_by_id_or_username(state, &user).await?;

    let update = UpdateUserRequest {
        username: None,
        email: email.clone(),
        first_name: None,
        last_name: None,
        role,
        is_active: active,
    };

    let updated_user = state.db_update_user(existing_user.id, update).await?;

    info!("✅ User updated successfully:");
    println!("  ID: {}", updated_user.id);
    println!("  Username: {}", updated_user.username);
    println!("  Email: {}", updated_user.email);
    println!("  Role: {}", updated_user.role);
    println!("  Active: {}", updated_user.is_active);

    Ok(())
}

async fn user_delete(user: String, force: bool, state: &AppState) -> Result<()> {
    let existing_user = find_user_by_id_or_username(state, &user).await?;

    if !force {
        warn!(
            "⚠️  You are about to delete user: {} ({})",
            existing_user.username, existing_user.email
        );
        warn!("⚠️  This action cannot be undone!");
        print!("Type 'DELETE' to confirm: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim() != "DELETE" {
            info!("❌ User deletion cancelled");
            return Ok(());
        }
    }

    state.db_delete_user(existing_user.id).await?;
    info!("✅ User deleted successfully");

    Ok(())
}

async fn user_reset_password(
    user: String,
    password: Option<String>,
    generate_password: bool,
    state: &AppState,
) -> Result<()> {
    let existing_user = find_user_by_id_or_username(state, &user).await?;

    // Clear, explicit intent handling
    let new_password = match (password, generate_password) {
        (Some(p), false) => p,
        (None, true) => generate_random_password(),
        (None, false) => prompt_password("Enter new password: ")?,
        (Some(_), true) => unreachable!(),
    };

    state
        .db_reset_user_password(existing_user.id, &new_password)
        .await?;

    info!(
        "✅ Password reset successfully for user: {}",
        existing_user.username
    );
    if generate_password {
        warn!("🔑 Generated password: {}", new_password);
    }

    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn handle_content_action(_action: ContentAction, _state: &AppState) {
    // Indicate that content commands are not yet implemented. This makes
    // running the CLI in development obvious when these commands are used.
    warn!("'Content' command invoked but not implemented.");
    // For stronger developer-time visibility you can replace the above
    // with a `todo!()` when you want the process to panic until implemented:
    // todo!("Implement content action: {:?}", action);
}

#[allow(clippy::cognitive_complexity)]
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
        (&health.database.status, health.database.response_time_ms, health.database.error.as_deref()),
        (&health.cache.status, health.cache.response_time_ms, health.cache.error.as_deref()),
        (&health.search.status, health.search.response_time_ms, health.search.error.as_deref()),
        (&health.auth.status, health.auth.response_time_ms, health.auth.error.as_deref()),
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

    let table = cms_backend::utils::bin_utils::render_health_table_components(overall, db, cache, search, auth);
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

#[allow(clippy::cognitive_complexity)]
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

fn generate_random_password() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let charset_len = CHARSET.len() as u16;
    let threshold: u16 = 256u16 - (256u16 % charset_len);
    let rng = SystemRandom::new();

    let mut password = String::with_capacity(16);
    let mut byte = [0u8; 1];
    while password.len() < 16 {
        // Fail fast if RNG can't produce bytes
        rng.fill(&mut byte).expect("Failed to read from system's entropy source");
        let v = byte[0] as u16;
        if v < threshold {
            let idx = (v % charset_len) as usize;
            password.push(CHARSET[idx] as char);
        }
    }
    password
}

fn prompt_password(prompt: &str) -> Result<String> {
    // Use rpassword to securely read password without echoing to the terminal
    let password = rpassword::prompt_password(prompt)
    .map_err(|e| cms_backend::AppError::Internal(e.to_string()))?;

    if password.is_empty() {
        return Err(cms_backend::AppError::BadRequest(
            "Password cannot be empty".to_string(),
        ));
    }

    Ok(password)
}

/// Find user by UUID or username and return a NotFound AppError if missing
async fn find_user_by_id_or_username(state: &AppState, identifier: &str) -> Result<cms_backend::models::User> {
    let result = if let Ok(id) = uuid::Uuid::parse_str(identifier) {
        state.db_get_user_by_id(id).await
    } else {
        state.db_get_user_by_username(identifier).await
    };
    result.map_err(|_| cms_backend::AppError::NotFound(format!("User '{}' not found", identifier)))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_password_length_and_charset() {
        // Generate multiple passwords to reduce flakiness
        for _ in 0..8 {
            let pw = generate_random_password();
            // length
            assert_eq!(pw.len(), 16);

            // allowed chars
            for ch in pw.chars() {
                let bytes = ch as u8;
                // must be ASCII printable and in our CHARSET
                let found = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*"
                    .iter()
                    .any(|&c| c == bytes);
                assert!(found, "password contains invalid char: {}", ch);
            }
        }
    }
}
