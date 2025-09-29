//! CMS Administration CLI Tool
//!
//! Comprehensive command-line interface for managing users, content,
//! system settings, and performing administrative tasks.

#[path = "admin/backend.rs"]
mod backend;
#[path = "admin/cli.rs"]
mod cli;
#[path = "admin/handlers/mod.rs"]
mod handlers;
#[path = "admin/util.rs"]
mod util;

use clap::Parser;
use cms_backend::Result;
use tracing::info;

use crate::cli::{Cli, Commands};
use crate::handlers::{
    analytics::handle_analytics_action, content::handle_content_action,
    security::handle_security_action, system::handle_system_action, user::handle_user_action_state,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli).await
}

async fn run(cli: Cli) -> Result<()> {
    let app_state = cms_backend::utils::init::init_app_state_with_verbose(cli.verbose).await?;

    info!("🔧 CMS Administration Tool v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::User { action } => handle_user_action_state(action, &app_state).await?,
        Commands::Content { action } => handle_content_action(action, &app_state)?,
        Commands::System { action } => handle_system_action(action, &app_state).await?,
        Commands::Analytics { action } => handle_analytics_action(action, &app_state)?,
        Commands::Security { action } => handle_security_action(action, &app_state)?,
    }

    Ok(())
}
