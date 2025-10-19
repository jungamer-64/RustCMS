//! Application initialization utilities for bin/ files
//! Phase 5: Updated for new AppState implementation

use crate::{AppError, Config, Result};
use std::sync::Arc;

/// Initialize AppState with all services
///
/// Phase 5: 新AppState実装（infrastructure/app_state.rs）を使用
/// Note: サービス初期化は今後実装予定。現在はconfig+eventのみ
#[cfg(feature = "restructure_domain")]
pub async fn init_app_state() -> Result<Arc<crate::AppState>> {
    use crate::infrastructure::app_state::AppState;
    
    let config = Config::from_env()?;
    let builder = AppState::builder(config);
    
    // Phase 5: サービス初期化は後で実装
    // TODO: database, cache, search, auth の初期化を追加
    
    Ok(Arc::new(builder.build()?))
}

/// Initialize AppState with verbose logging
#[cfg(feature = "restructure_domain")]
pub async fn init_app_state_with_verbose(_verbose: bool) -> Result<Arc<crate::AppState>> {
    init_app_state().await
}

// ============================================================================
// Legacy removed - app.rs deleted in Phase 5
// ============================================================================

