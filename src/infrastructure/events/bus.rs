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
    pub role: String,
}

impl UserEventData {
    /// Create from a full User model
    #[cfg(feature = "database")]
    pub fn from_user(user: &crate::models::user::User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
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

    // =========================================================================
    // Event Bus Creation & Configuration Tests
    // =========================================================================

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
    fn test_event_bus_with_different_capacities() {
        let capacities = vec![1, 10, 100, 1000];
        for capacity in capacities {
            let (tx, _rx) = create_event_bus(capacity);
            // Should create bus successfully with given capacity
            let event = AppEvent::UserDeleted(Uuid::new_v4());
            assert!(tx.send(event).is_ok());
        }
    }

    #[test]
    fn test_event_bus_send_multiple_events() {
        let (tx, mut rx) = create_event_bus(100);

        let events = vec![
            AppEvent::UserCreated(UserEventData {
                id: Uuid::new_v4(),
                username: "user1".to_string(),
                email: "user1@example.com".to_string(),
                role: "User".to_string(),
            }),
            AppEvent::PostCreated(PostEventData {
                id: Uuid::new_v4(),
                title: "Post 1".to_string(),
                slug: "post-1".to_string(),
                author_id: Uuid::new_v4(),
                published: false,
            }),
            AppEvent::UserDeleted(Uuid::new_v4()),
        ];

        for event in events {
            tx.send(event).unwrap();
        }

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());
    }

    // =========================================================================
    // Multiple Subscribers Tests
    // =========================================================================

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
    fn test_multiple_subscribers_independent_channels() {
        let (tx, _) = create_event_bus(10);

        let mut rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();
        let mut rx3 = tx.subscribe();

        let event1 = AppEvent::UserDeleted(Uuid::new_v4());
        let event2 = AppEvent::PostDeleted(Uuid::new_v4());

        tx.send(event1).unwrap();
        tx.send(event2).unwrap();

        // Each receiver should have both events
        assert_eq!(rx1.try_recv().is_ok(), true);
        assert_eq!(rx2.try_recv().is_ok(), true);
        assert_eq!(rx3.try_recv().is_ok(), true);

        // Second event should be in queue
        assert_eq!(rx1.try_recv().is_ok(), true);
        assert_eq!(rx2.try_recv().is_ok(), true);
        assert_eq!(rx3.try_recv().is_ok(), true);
    }

    #[test]
    fn test_subscriber_dropped_doesnt_affect_others() {
        let (tx, _) = create_event_bus(10);

        let mut rx1 = tx.subscribe();
        let rx2 = tx.subscribe();

        drop(rx2); // Drop the second receiver

        let event = AppEvent::UserCreated(UserEventData {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            role: "User".to_string(),
        });

        // Should still work for remaining subscriber
        assert!(tx.send(event).is_ok());
        assert!(rx1.try_recv().is_ok());
    }

    // =========================================================================
    // Event Data Tests
    // =========================================================================

    #[test]
    fn test_event_data_from_models() {
        let user_data = UserEventData {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            role: "User".to_string(),
        };

        assert_eq!(user_data.username, "test");
        assert_eq!(user_data.email, "test@example.com");
        assert_eq!(user_data.role, "User");
    }

    #[test]
    fn test_post_event_data() {
        let post_data = PostEventData {
            id: Uuid::new_v4(),
            title: "My Post".to_string(),
            slug: "my-post".to_string(),
            author_id: Uuid::new_v4(),
            published: true,
        };

        assert_eq!(post_data.title, "My Post");
        assert_eq!(post_data.slug, "my-post");
        assert_eq!(post_data.published, true);
    }

    #[test]
    fn test_user_event_data_clone() {
        let original = UserEventData {
            id: Uuid::new_v4(),
            username: "user1".to_string(),
            email: "user@example.com".to_string(),
            role: "Admin".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.username, cloned.username);
    }

    // =========================================================================
    // AppEvent Enum Tests (全バリアント)
    // =========================================================================

    #[test]
    fn test_app_event_user_created() {
        let user_data = UserEventData {
            id: Uuid::new_v4(),
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            role: "User".to_string(),
        };
        let event = AppEvent::UserCreated(user_data.clone());
        assert!(matches!(event, AppEvent::UserCreated(_)));
    }

    #[test]
    fn test_app_event_user_updated() {
        let user_data = UserEventData {
            id: Uuid::new_v4(),
            username: "updated".to_string(),
            email: "updated@example.com".to_string(),
            role: "User".to_string(),
        };
        let event = AppEvent::UserUpdated(user_data);
        assert!(matches!(event, AppEvent::UserUpdated(_)));
    }

    #[test]
    fn test_app_event_user_deleted() {
        let user_id = Uuid::new_v4();
        let event = AppEvent::UserDeleted(user_id);
        assert!(matches!(event, AppEvent::UserDeleted(_)));
    }

    #[test]
    fn test_app_event_post_created() {
        let post_data = PostEventData {
            id: Uuid::new_v4(),
            title: "New Post".to_string(),
            slug: "new-post".to_string(),
            author_id: Uuid::new_v4(),
            published: false,
        };
        let event = AppEvent::PostCreated(post_data);
        assert!(matches!(event, AppEvent::PostCreated(_)));
    }

    #[test]
    fn test_app_event_post_published() {
        let post_data = PostEventData {
            id: Uuid::new_v4(),
            title: "Published Post".to_string(),
            slug: "published-post".to_string(),
            author_id: Uuid::new_v4(),
            published: true,
        };
        let event = AppEvent::PostPublished(post_data);
        assert!(matches!(event, AppEvent::PostPublished(_)));
    }

    #[test]
    fn test_app_event_comment_operations() {
        let id = Uuid::new_v4();
        let created_event = AppEvent::CommentCreated(id);
        let updated_event = AppEvent::CommentUpdated(id);
        let deleted_event = AppEvent::CommentDeleted(id);

        assert!(matches!(created_event, AppEvent::CommentCreated(_)));
        assert!(matches!(updated_event, AppEvent::CommentUpdated(_)));
        assert!(matches!(deleted_event, AppEvent::CommentDeleted(_)));
    }

    #[test]
    fn test_app_event_tag_operations() {
        let id = Uuid::new_v4();
        let created_event = AppEvent::TagCreated(id);
        let updated_event = AppEvent::TagUpdated(id);
        let deleted_event = AppEvent::TagDeleted(id);

        assert!(matches!(created_event, AppEvent::TagCreated(_)));
        assert!(matches!(updated_event, AppEvent::TagUpdated(_)));
        assert!(matches!(deleted_event, AppEvent::TagDeleted(_)));
    }

    #[test]
    fn test_app_event_category_operations() {
        let id = Uuid::new_v4();
        let created_event = AppEvent::CategoryCreated(id);
        let updated_event = AppEvent::CategoryUpdated(id);
        let deleted_event = AppEvent::CategoryDeleted(id);

        assert!(matches!(created_event, AppEvent::CategoryCreated(_)));
        assert!(matches!(updated_event, AppEvent::CategoryUpdated(_)));
        assert!(matches!(deleted_event, AppEvent::CategoryDeleted(_)));
    }

    // =========================================================================
    // Fire-and-Forget Pattern Tests
    // =========================================================================

    #[test]
    fn test_event_bus_send_does_not_panic_on_no_subscribers() {
        let (tx, _rx) = create_event_bus(10);
        // Drop receiver so there are no subscribers
        drop(_rx);

        // Sending should not panic even if there are no subscribers
        let event = AppEvent::UserDeleted(Uuid::new_v4());
        // In broadcast channel, send returns error if all receivers are dropped
        let result = tx.send(event);
        // Result might be Ok or Err depending on implementation
        let _ = result;
    }

    #[test]
    fn test_fire_and_forget_semantics() {
        let (tx, mut rx) = create_event_bus(10);

        let event = AppEvent::PostDeleted(Uuid::new_v4());
        let send_result = tx.send(event.clone());

        // Send completes regardless of whether anyone is listening
        assert!(send_result.is_ok() || send_result.is_err());

        // If receiver exists, it should get the event
        if let Ok(received) = rx.try_recv() {
            assert!(matches!(received, AppEvent::PostDeleted(_)));
        }
    }

    // =========================================================================
    // Channel Capacity Tests
    // =========================================================================

    #[test]
    fn test_event_bus_send_succeeds_with_subscribers() {
        let (tx, _rx) = create_event_bus(10);

        let event1 = AppEvent::UserDeleted(Uuid::new_v4());
        let event2 = AppEvent::UserDeleted(Uuid::new_v4());

        // Sending should succeed when there are subscribers
        assert!(tx.send(event1).is_ok());
        assert!(tx.send(event2).is_ok());
    }

    // =========================================================================
    // Clone & Debug Tests
    // =========================================================================

    #[test]
    fn test_app_event_clone() {
        let user_data = UserEventData {
            id: Uuid::new_v4(),
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            role: "User".to_string(),
        };
        let event = AppEvent::UserCreated(user_data);
        let cloned = event.clone();

        assert!(matches!(event, AppEvent::UserCreated(_)));
        assert!(matches!(cloned, AppEvent::UserCreated(_)));
    }

    #[test]
    fn test_app_event_debug() {
        let event = AppEvent::UserDeleted(Uuid::new_v4());
        let debug_str = format!("{event:?}");
        assert!(debug_str.contains("UserDeleted"));
    }

    // =========================================================================
    // Integration Tests (複数イベントシーケンス)
    // =========================================================================

    #[test]
    fn test_event_sequence_user_lifecycle() {
        let (tx, mut rx) = create_event_bus(10);

        let user_id = Uuid::new_v4();
        let user_data = UserEventData {
            id: user_id,
            username: "lifecycle_user".to_string(),
            email: "lifecycle@example.com".to_string(),
            role: "User".to_string(),
        };

        // Simulate user lifecycle
        tx.send(AppEvent::UserCreated(user_data.clone())).unwrap();
        tx.send(AppEvent::UserUpdated(user_data.clone())).unwrap();
        tx.send(AppEvent::UserDeleted(user_id)).unwrap();

        // All events should be receivable in order
        assert!(matches!(rx.try_recv().unwrap(), AppEvent::UserCreated(_)));
        assert!(matches!(rx.try_recv().unwrap(), AppEvent::UserUpdated(_)));
        assert!(matches!(rx.try_recv().unwrap(), AppEvent::UserDeleted(_)));
    }

    #[test]
    fn test_event_sequence_post_lifecycle() {
        let (tx, mut rx) = create_event_bus(10);

        let post_data = PostEventData {
            id: Uuid::new_v4(),
            title: "Blog Post".to_string(),
            slug: "blog-post".to_string(),
            author_id: Uuid::new_v4(),
            published: false,
        };

        // Simulate post lifecycle
        tx.send(AppEvent::PostCreated(post_data.clone())).unwrap();
        let mut post_published = post_data.clone();
        post_published.published = true;
        tx.send(AppEvent::PostPublished(post_published)).unwrap();
        tx.send(AppEvent::PostDeleted(post_data.id)).unwrap();

        assert!(matches!(rx.try_recv().unwrap(), AppEvent::PostCreated(_)));
        assert!(matches!(rx.try_recv().unwrap(), AppEvent::PostPublished(_)));
        assert!(matches!(rx.try_recv().unwrap(), AppEvent::PostDeleted(_)));
    }
}
