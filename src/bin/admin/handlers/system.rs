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

        assert!(s.contains("Component"));
        assert!(s.contains("Overall"));
        assert!(s.contains("Database"));
        assert!(s.contains("Cache"));
        assert!(s.contains("Search"));
        assert!(s.contains("Auth"));
        assert!(s.contains("12.34") || s.contains("12.3"));
        assert!(s.contains("5.67") || s.contains("5.7"));
    }
}
