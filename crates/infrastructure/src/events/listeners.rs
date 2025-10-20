//! Infrastructure event listeners.
//!
//! Currently provides lightweight placeholders that can be extended with
//! feature-specific behaviour (cache invalidation, search indexing, etc.).

use crate::events::{AppEvent, EventBus};
use crate::AppState;
use tracing::info;

/// Spawn event listener tasks.
///
/// For now this simply logs that listeners are active. Future work can attach
/// concrete background jobs for cache invalidation, search indexing, and more.
pub fn spawn_event_listeners(_state: AppState, _event_bus: EventBus) {
    info!("Event listeners placeholder started");
}

/// Placeholder handler illustrating where cache invalidation logic could live.
pub async fn handle_cache_event(_state: &AppState, _event: AppEvent) {
    // no-op for now
}
