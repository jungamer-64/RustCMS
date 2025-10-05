//! Application Event System
//!
//! This module defines the event-driven architecture for decoupling cross-cutting concerns
//! like search indexing and cache invalidation from core business logic.
//!
//! # Architecture
//!
//! - **Single AppEvent Enum**: All application events are defined in one place
//! - **Broadcast Channel**: Uses tokio::sync::broadcast for pub-sub pattern
//! - **Feature-Aware Consumers**: Event listeners are conditionally compiled
//! - **Fire-and-Forget**: Event emission doesn't block or fail the primary operation
//!
//! # Example
//!
//! ```rust,ignore
//! // In a CRUD handler
//! let user = state.db.create_user(payload).await?;
//! let _ = state.event_bus.send(AppEvent::UserCreated(user.clone()));
//! Ok(Json(user))
//! ```

use tokio::sync::broadcast;
use uuid::Uuid;

/// Type alias for the broadcast sender.
///
/// This is added to AppState and cloned for each request.
pub type EventBus = broadcast::Sender<AppEvent>;

/// Defines all possible events that can occur in the application.
///
/// This enum is the single source of truth for events. Events are cloned
/// for each broadcast subscriber, so they should be lightweight or use Arc
/// for large payloads.
///
/// # Design Decision
///
/// We use a single comprehensive enum rather than domain-specific enums because:
/// - Simplifies channel management (one channel vs many)
/// - Makes adding new listeners trivial
/// - Minimal performance overhead (pattern matching is fast)
/// - Easier to maintain and understand
#[derive(Debug, Clone)]
pub enum AppEvent {
    // ============================================================================
    // User Events
    // ============================================================================
    /// A new user was created
    UserCreated(UserEventData),
    
    /// An existing user was updated
    UserUpdated(UserEventData),
    
    /// A user was deleted (soft or hard delete)
    UserDeleted(Uuid),

    // ============================================================================
    // Post Events
    // ============================================================================
    /// A new post was created
    PostCreated(PostEventData),
    
    /// An existing post was updated
    PostUpdated(PostEventData),
    
    /// A post was deleted
    PostDeleted(Uuid),
    
    /// A post was published (status changed to published)
    PostPublished(PostEventData),

    // ============================================================================
    // Comment Events (Placeholder - using Uuid until Comment model is ready)
    // ============================================================================
    /// A new comment was created
    CommentCreated(Uuid),
    
    /// An existing comment was updated
    CommentUpdated(Uuid),
    
    /// A comment was deleted
    CommentDeleted(Uuid),

    // ============================================================================
    // Category Events (Placeholder - using Uuid until Category model is ready)
    // ============================================================================
    /// A new category was created
    CategoryCreated(Uuid),
    
    /// An existing category was updated
    CategoryUpdated(Uuid),
    
    /// A category was deleted
    CategoryDeleted(Uuid),

    // ============================================================================
    // Tag Events (Placeholder - using Uuid until Tag model is ready)
    // ============================================================================
    /// A new tag was created
    TagCreated(Uuid),
    
    /// An existing tag was updated
    TagUpdated(Uuid),
    
    /// A tag was deleted
    TagDeleted(Uuid),
}

// ============================================================================
// Event Data Structures
// ============================================================================
//
// These are lightweight wrappers around the actual domain models.
// We clone them for event broadcasting, so we keep only essential data.

/// Event data for user-related events
#[derive(Debug, Clone)]
pub struct UserEventData {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

impl UserEventData {
    /// Create from a full User model
    #[cfg(feature = "database")]
    pub fn from_user(user: &crate::models::user::User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
        }
    }
}

/// Event data for post-related events
#[derive(Debug, Clone)]
pub struct PostEventData {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub author_id: Uuid,
    pub published: bool,
}

impl PostEventData {
    /// Create from a full Post model
    #[cfg(feature = "database")]
    pub fn from_post(post: &crate::models::post::Post) -> Self {
        Self {
            id: post.id,
            title: post.title.clone(),
            slug: post.slug.clone(),
            author_id: post.author_id,
            published: post.status == "published",
        }
    }
}

// Note: Comment, Category, and Tag events use Uuid directly for now
// When these models are ready, we can create dedicated event data structs

// ============================================================================
// Event Bus Factory
// ============================================================================

/// Create a new event bus with the specified channel capacity.
///
/// The capacity determines how many events can be queued if receivers are slow.
/// A good default is 1024 for most applications.
///
/// # Returns
///
/// Returns a tuple of (sender, receiver). The sender is cloned into AppState,
/// and receivers are created by calling `sender.subscribe()`.
pub fn create_event_bus(capacity: usize) -> (EventBus, broadcast::Receiver<AppEvent>) {
    broadcast::channel(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_creation() {
        let (tx, mut rx) = create_event_bus(10);
        
        // Test sending and receiving
        let test_event = AppEvent::UserDeleted(Uuid::new_v4());
        tx.send(test_event.clone()).unwrap();
        
        let received = rx.try_recv().unwrap();
        matches!(received, AppEvent::UserDeleted(_));
    }

    #[test]
    fn test_multiple_subscribers() {
        let (tx, _) = create_event_bus(10);
        
        let mut rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();
        
        let user_id = Uuid::new_v4();
        tx.send(AppEvent::UserDeleted(user_id)).unwrap();
        
        // Both receivers should get the event
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_event_data_from_models() {
        // This would need actual model instances in real tests
        // Just testing the structure compiles
        let _user_data = UserEventData {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            email: "test@example.com".to_string(),
        };
    }
}
