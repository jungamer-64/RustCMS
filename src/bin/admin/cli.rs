// src/bin/admin/cli.rs
use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

/// CLI-specific user role enum (Phase 2: domain UserRole doesn't implement ValueEnum)
#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "kebab_case")]
pub enum UserRoleArg {
    Admin,
    Editor,
    Subscriber,
}

impl fmt::Display for UserRoleArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            UserRoleArg::Admin => "admin",
            UserRoleArg::Editor => "editor",
            UserRoleArg::Subscriber => "subscriber",
        };
        write!(f, "{}", value)
    }
}

impl From<UserRoleArg> for cms_backend::models::UserRole {
    fn from(role: UserRoleArg) -> Self {
        match role {
            UserRoleArg::Admin => cms_backend::models::UserRole::Admin,
            UserRoleArg::Editor => cms_backend::models::UserRole::Editor,
            UserRoleArg::Subscriber => cms_backend::models::UserRole::Subscriber,
        }
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
        let value = match self {
            PostStatus::Draft => "draft",
            PostStatus::Published => "published",
            PostStatus::Archived => "archived",
        };
        write!(f, "{}", value)
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
        let value = match self {
            Period::Day => "day",
            Period::Week => "week",
            Period::Month => "month",
            Period::Year => "year",
        };
        write!(f, "{}", value)
    }
}

#[derive(Parser)]
#[command(name = "cms-admin")]
#[command(about = "CMS Administration Tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
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
pub enum UserAction {
    /// List all users
    List {
        /// Filter by role (admin, editor, subscriber)
        #[arg(long, value_enum)]
        role: Option<UserRoleArg>,
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
        /// User role (admin, editor, subscriber)
        #[arg(short, long, value_enum, default_value_t = UserRoleArg::Subscriber)]
        role: UserRoleArg,
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
        role: Option<UserRoleArg>,
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
pub enum ContentAction {
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
pub enum SystemAction {
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
pub enum CacheAction {
    /// Clear all cache
    Clear,
    /// Show cache statistics
    Stats,
    /// Warm up cache
    Warmup,
}

#[derive(Subcommand)]
pub enum DatabaseAction {
    /// Show database statistics
    Stats,
    /// Optimize database
    Optimize,
    /// Check database integrity
    Check,
}

#[derive(Subcommand)]
pub enum AnalyticsAction {
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
pub enum SecurityAction {
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
