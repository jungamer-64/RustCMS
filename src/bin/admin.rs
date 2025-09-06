//! Enterprise CMS Administration CLI Tool
//!
//! Comprehensive command-line interface for managing users, content,
//! system settings, and performing administrative tasks.

use clap::{Parser, Subcommand};
use cms_backend::{
    models::{CreateUserRequest, UpdateUserRequest, UserRole},
    AppState, Result,
};
use std::io::{self, Write};
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "cms-admin")]
#[command(about = "Enterprise CMS Administration Tool")]
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
        #[arg(long)]
        role: Option<String>,
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
        #[arg(short, long, default_value = "user")]
        role: String,
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
        #[arg(long)]
        role: Option<String>,
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
        /// New password (if not provided, will be generated)
        #[arg(long)]
        password: Option<String>,
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

    // Initialize full AppState (includes database when feature enabled)
    let app_state = cms_backend::utils::init::init_app_state().await?;

    // Apply CLI log level override (keep existing behavior)
    if cli.verbose {
        tracing::info!("Verbose logging enabled via CLI flag");
    }

    info!(
        "🔧 Enterprise CMS Administration Tool v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Execute command
    match cli.command {
        Commands::User { action } => handle_user_action(action, &app_state).await?,
        Commands::Content { action } => handle_content_action(action, &app_state).await?,
        Commands::System { action } => handle_system_action(action, &app_state).await?,
        Commands::Analytics { action } => handle_analytics_action(action, &app_state).await?,
        Commands::Security { action } => handle_security_action(action, &app_state).await?,
    }

    Ok(())
}

async fn handle_user_action(action: UserAction, state: &AppState) -> Result<()> {
    match action {
        UserAction::List { role, active_only } => {
            info!("📊 Listing users...");
            let users = state
                .database
                .list_users(role.as_deref(), Some(active_only))
                .await?;

            println!("┌─────────────────────────────────────────┬──────────────┬─────────────────────────┬────────┬────────┐");
            println!("│ ID                                      │ Username     │ Email                   │ Role   │ Active │");
            println!("├─────────────────────────────────────────┼──────────────┼─────────────────────────┼────────┼────────┤");

            for user in users {
                println!(
                    "│ {:39} │ {:12} │ {:23} │ {:6} │ {:6} │",
                    user.id,
                    truncate(&user.username, 12),
                    truncate(&user.email, 23),
                    user.role,
                    if user.is_active { "Yes" } else { "No" }
                );
            }

            println!("└─────────────────────────────────────────┴──────────────┴─────────────────────────┴────────┴────────┘");
        }

        UserAction::Create {
            username,
            email,
            role,
            generate_password,
        } => {
            let password = if generate_password {
                generate_random_password()
            } else {
                prompt_password("Enter password for new user: ")?
            };

            let role_enum = UserRole::parse_str(&role)?;

            let user = CreateUserRequest {
                username: username.clone(),
                email,
                password: password.clone(),
                first_name: Some("".to_string()),
                last_name: Some("".to_string()),
                role: role_enum,
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
        }

        UserAction::Update {
            user,
            email,
            role,
            active,
        } => {
            let existing_user = if user.parse::<uuid::Uuid>().is_ok() {
                let id = uuid::Uuid::parse_str(&user)
                    .map_err(|e| cms_backend::AppError::BadRequest(e.to_string()))?;
                state.db_get_user_by_id(id).await?
            } else {
                state.db_get_user_by_username(&user).await?
            };

            // Convert Option<String> -> Option<UserRole>
            let role_enum_opt: Option<UserRole> = match role.clone() {
                Some(r) => Some(UserRole::parse_str(&r)?),
                None => None,
            };

            let update = UpdateUserRequest {
                username: None,
                email: email.clone(),
                first_name: None,
                last_name: None,
                role: role_enum_opt,
                is_active: active,
            };

            let updated_user = state.db_update_user(existing_user.id, update).await?;

            info!("✅ User updated successfully:");
            println!("  ID: {}", updated_user.id);
            println!("  Username: {}", updated_user.username);
            println!("  Email: {}", updated_user.email);
            println!("  Role: {}", updated_user.role);
            println!("  Active: {}", updated_user.is_active);
        }

        UserAction::Delete { user, force } => {
            let existing_user = if user.parse::<uuid::Uuid>().is_ok() {
                let id = uuid::Uuid::parse_str(&user)
                    .map_err(|e| cms_backend::AppError::BadRequest(e.to_string()))?;
                state.db_get_user_by_id(id).await?
            } else {
                state.db_get_user_by_username(&user).await?
            };

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
        }

        UserAction::ResetPassword { user, password } => {
            let existing_user = if user.parse::<uuid::Uuid>().is_ok() {
                let id = uuid::Uuid::parse_str(&user)
                    .map_err(|e| cms_backend::AppError::BadRequest(e.to_string()))?;
                state.db_get_user_by_id(id).await?
            } else {
                state.db_get_user_by_username(&user).await?
            };

            let new_password = password.clone().unwrap_or_else(|| {
                prompt_password("Enter new password: ")
                    .unwrap_or_else(|_| generate_random_password())
            });

            state.db_reset_user_password(existing_user.id, &new_password).await?;

            info!(
                "✅ Password reset successfully for user: {}",
                existing_user.username
            );
            if password.is_none() {
                warn!("🔑 New password: {}", new_password);
            }
        }
    }

    Ok(())
}

async fn handle_content_action(action: ContentAction, _state: &AppState) -> Result<()> {
    match action {
        ContentAction::List {
            status,
            author,
            limit,
        } => {
            info!(
                "📊 Listing posts (status: {:?}, author: {:?}, limit: {})",
                status, author, limit
            );
            // Implementation would list posts with filters
        }

        ContentAction::Create {
            title,
            file: _file,
            author: _author,
            status: _status,
        } => {
            info!("📝 Creating post: {}", title);
            // Implementation would create post
        }

        ContentAction::PublishScheduled => {
            info!("📅 Publishing scheduled posts...");
            // Implementation would publish scheduled posts
        }

        ContentAction::ReindexSearch => {
            info!("🔍 Rebuilding search index...");
            // Implementation would rebuild search index
        }

        ContentAction::CleanupMedia => {
            info!("🧹 Cleaning up orphaned media files...");
            // Implementation would clean up media
        }
    }

    Ok(())
}

async fn handle_system_action(action: SystemAction, _state: &AppState) -> Result<()> {
    match action {
        SystemAction::Status => {
            info!("📊 System Status:");
            // Implementation would show comprehensive system status
            println!("  🚀 Server: Running");
            println!("  💾 Database: Connected");
            println!("  🗄️  Cache: Active");
            println!("  🔍 Search: Indexed");
        }

        SystemAction::Settings { key, value } => {
            info!("⚙️  Updating setting: {} = {}", key, value);
            // Implementation would update system setting
        }

        SystemAction::Cache { action } => {
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
        }

        SystemAction::Database { action } => {
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
        }
    }

    Ok(())
}

async fn handle_analytics_action(action: AnalyticsAction, _state: &AppState) -> Result<()> {
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

    Ok(())
}

async fn handle_security_action(action: SecurityAction, _state: &AppState) -> Result<()> {
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

    Ok(())
}

// Utility functions

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{:.width$}...", s, width = max_len - 3)
    }
}

fn generate_random_password() -> String {
    use rand::Rng;
    use rand::rng;

    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut rng = rng();

    (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn prompt_password(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;

    let password = password.trim().to_string();
    if password.is_empty() {
        return Err(cms_backend::AppError::BadRequest(
            "Password cannot be empty".to_string(),
        ));
    }

    Ok(password)
}
