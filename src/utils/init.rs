//! Application initialization utilities for bin/ files
//! Phase 4: Provides init_app_state() helper

use crate::{AppError, Config, Result};
use std::sync::Arc;

#[cfg(not(feature = "restructure_domain"))]
pub async fn init_app_state() -> Result<Arc<crate::app::AppState>> {
    use crate::app::AppState;
    let config = Config::from_env()?;
    let mut builder = AppState::builder(config);
    
    #[cfg(feature = "database")]
    {
        builder = builder.with_database().await?;
    }
    
    #[cfg(feature = "cache")]
    {
        builder = builder.with_cache().await?;
    }
    
    #[cfg(feature = "search")]
    {
        builder = builder.with_search().await?;
    }
    
    #[cfg(feature = "auth")]
    {
        builder = builder.with_auth().await?;
    }
    
    Ok(Arc::new(builder.build()?))
}

#[cfg(not(feature = "restructure_domain"))]
pub async fn init_app_state_with_verbose(_verbose: bool) -> Result<Arc<crate::app::AppState>> {
    init_app_state().await
}

#[cfg(feature = "restructure_domain")]
pub async fn init_app_state() -> Result<Arc<()>> {
    Err(AppError::Config("init_app_state not implemented".to_string()))
}

#[cfg(feature = "restructure_domain")]
pub async fn init_app_state_with_verbose(_verbose: bool) -> Result<Arc<()>> {
    init_app_state().await
}
