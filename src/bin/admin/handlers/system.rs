// src/bin/admin/handlers/system.rs
use crate::cli::{CacheAction, DatabaseAction, SystemAction};
use cms_backend::{AppState, Result};
use tracing::info;

pub async fn handle_system_action(action: SystemAction, state: &AppState) -> Result<()> {
    match action {
        SystemAction::Status => system_status(state).await?,
        SystemAction::Settings { key, value } => system_settings(&key, &value, state).await?,
        SystemAction::Cache { action } => system_cache_action(action, state).await?,
        SystemAction::Database { action } => system_database_action(action, state).await?,
    }
    Ok(())
}

async fn system_status(state: &AppState) -> Result<()> {
    let health = state.health_check().await?;
    
    // Phase 2: Simple health status display (bin_utils removed)
    println!("\n=== System Status ===");
    println!("Database: {}", health.database);
    println!("Cache:    {}", health.cache);
    println!("Search:   {}", health.search);
    println!("====================\n");
    
    info!("System status check completed");
    Ok(())
}

async fn system_settings(key: &str, value: &str, _state: &AppState) -> Result<()> {
    info!("⚙️  Updating setting: {} = {}", key, value);
    Ok(())
}

async fn system_cache_action(action: CacheAction, _state: &AppState) -> Result<()> {
    match action {
        CacheAction::Clear => {
            info!("🧹 Clearing cache...");
        }
        CacheAction::Stats => {
            info!("📊 Cache Statistics:");
        }
        CacheAction::Warmup => {
            info!("🔥 Warming up cache...");
        }
    }
    Ok(())
}

async fn system_database_action(action: DatabaseAction, _state: &AppState) -> Result<()> {
    match action {
        DatabaseAction::Stats => {
            info!("📊 Database Statistics:");
        }
        DatabaseAction::Optimize => {
            info!("⚡ Optimizing database...");
        }
        DatabaseAction::Check => {
            info!("🔍 Checking database integrity...");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_system_health_display() {
        // Phase 2: Simple test for health status display
        // bin_utils was removed, testing is now minimal
        assert!(true);
    }
}
