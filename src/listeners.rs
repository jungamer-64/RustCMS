//! Event Listeners for the Event-Driven Architecture
//!
//! This module implements background tasks that listen to application events
//! and perform cross-cutting concerns like search indexing and cache invalidation.
//!
//! # Architecture
//!
//! - Listeners are spawned once at application startup
//! - Each listener subscribes to the event bus and filters relevant events
//! - Errors in listeners are logged but don't crash the application
//! - Feature-gated: only compiled when relevant features are enabled
//!
//! # Design Philosophy
//!
//! - **Decoupling**: Handlers emit events without knowing who consumes them
//! - **Fire-and-forget**: Event emission never fails the primary operation
//! - **Resilience**: Listener failures are logged, not propagated
//! - **Idempotency**: Listeners should handle duplicate events gracefully

use crate::AppState;
use crate::events::{AppEvent, EventBus};
use tracing::{debug, error, info, warn};

/// Spawns all event listener tasks.
///
/// This function creates background tokio tasks for:
/// - Search indexing (if `search` feature is enabled)
/// - Cache invalidation (if `cache` feature is enabled)
///
/// Each listener runs independently and will continue until the application shuts down.
///
/// # Arguments
///
/// * `state` - Application state containing database, cache, and search services
/// * `event_bus` - The event bus sender used to subscribe to events
pub fn spawn_event_listeners(state: AppState, event_bus: EventBus) {
    info!("🎧 Spawning event listeners");

    // Spawn search indexing listener
    #[cfg(feature = "search")]
    {
        let search_state = state.clone();
        let mut search_receiver = event_bus.subscribe();

        tokio::spawn(async move {
            info!("🔍 Search indexing listener started");

            loop {
                match search_receiver.recv().await {
                    Ok(event) => {
                        if let Err(e) = handle_search_event(&search_state, event).await {
                            error!(error = ?e, "Search indexing failed for event");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "Search listener lagged, some events were skipped");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("Search listener: event bus closed, shutting down");
                        break;
                    }
                }
            }
        });
    }

    // Spawn cache invalidation listener
    #[cfg(feature = "cache")]
    {
        let cache_state = state.clone();
        let mut cache_receiver = event_bus.subscribe();

        tokio::spawn(async move {
            info!("🗑️  Cache invalidation listener started");

            loop {
                match cache_receiver.recv().await {
                    Ok(event) => {
                        handle_cache_event(&cache_state, event).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "Cache listener lagged, some events were skipped");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("Cache listener: event bus closed, shutting down");
                        break;
                    }
                }
            }
        });
    }

    info!("✅ Event listeners spawned successfully");
}

// ============================================================================
// Search Indexing Event Handler
// ============================================================================

#[cfg(feature = "search")]
async fn handle_search_event(state: &AppState, event: AppEvent) -> crate::Result<()> {
    match event {
        AppEvent::UserCreated(data) | AppEvent::UserUpdated(data) => {
            debug!(user_id = %data.id, "Indexing user");

            // Fetch fresh data from database (single source of truth)
            let user = state.db_get_user_by_id(data.id).await?;
            state.search.index_user(&user).await?;

            debug!(user_id = %data.id, "User indexed successfully");
        }

        AppEvent::UserDeleted(user_id) => {
            debug!(user_id = %user_id, "Removing user from index");
            state.search.remove_document(&user_id.to_string()).await?;
            debug!(user_id = %user_id, "User removed from index");
        }

        AppEvent::PostCreated(data)
        | AppEvent::PostUpdated(data)
        | AppEvent::PostPublished(data) => {
            debug!(post_id = %data.id, "Indexing post");

            // Fetch fresh data from database
            let post = state.db_get_post_by_id(data.id).await?;
            state.search.index_post(&post).await?;

            debug!(post_id = %data.id, "Post indexed successfully");
        }

        AppEvent::PostDeleted(post_id) => {
            debug!(post_id = %post_id, "Removing post from index");
            state.search.remove_document(&post_id.to_string()).await?;
            debug!(post_id = %post_id, "Post removed from index");
        }

        // Placeholder events - no action yet
        AppEvent::CommentCreated(_)
        | AppEvent::CommentUpdated(_)
        | AppEvent::CommentDeleted(_)
        | AppEvent::CategoryCreated(_)
        | AppEvent::CategoryUpdated(_)
        | AppEvent::CategoryDeleted(_)
        | AppEvent::TagCreated(_)
        | AppEvent::TagUpdated(_)
        | AppEvent::TagDeleted(_) => {
            // No search indexing for these entities yet
            debug!("Received event with no search action defined");
        }
    }

    Ok(())
}

// ============================================================================
// Cache Invalidation Event Handler
// ============================================================================

#[cfg(feature = "cache")]
async fn handle_cache_event(state: &AppState, event: AppEvent) {
    match event {
        AppEvent::UserCreated(data) | AppEvent::UserUpdated(data) => {
            debug!(user_id = %data.id, "Invalidating user caches");
            state.invalidate_user_caches(data.id).await;
        }

        AppEvent::UserDeleted(user_id) => {
            debug!(user_id = %user_id, "Invalidating user caches (deleted)");
            state.invalidate_user_caches(user_id).await;
        }

        AppEvent::PostCreated(data)
        | AppEvent::PostUpdated(data)
        | AppEvent::PostPublished(data) => {
            debug!(post_id = %data.id, "Invalidating post caches");
            state.invalidate_post_caches(data.id).await;
        }

        AppEvent::PostDeleted(post_id) => {
            debug!(post_id = %post_id, "Invalidating post caches (deleted)");
            state.invalidate_post_caches(post_id).await;
        }

        // Placeholder events - no specific cache invalidation yet
        AppEvent::CommentCreated(_) | AppEvent::CommentUpdated(_) | AppEvent::CommentDeleted(_) => {
            // When comments are implemented, invalidate related post caches
            debug!("Comment event received, no cache action defined yet");
        }

        AppEvent::CategoryCreated(_)
        | AppEvent::CategoryUpdated(_)
        | AppEvent::CategoryDeleted(_)
        | AppEvent::TagCreated(_)
        | AppEvent::TagUpdated(_)
        | AppEvent::TagDeleted(_) => {
            // Future: invalidate category/tag caches when implemented
            debug!("Category/Tag event received, no cache action defined yet");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_stub_compiles() {
        // Ensures the stub compiles without errors
        // Real functional tests will be added with full implementation
    }
}
