//! Tests for the event-driven architecture
//!
//! This module validates the event bus functionality including:
//! - Event emission and reception
//! - Multiple subscribers
//! - All AppEvent variants

use rust_cms::events::{AppEvent, UserEventData, PostEventData, create_event_bus};
use uuid::Uuid;

#[tokio::test]
async fn test_event_bus_creation() {
    let (sender, _receiver) = create_event_bus(16);
    assert_eq!(sender.len(), 0, "New event bus should have no pending messages");
}

#[tokio::test]
async fn test_user_created_event() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let user_id = Uuid::new_v4();
    let event_data = UserEventData {
        id: user_id,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
    };
    
    // Send event
    let send_result = event_bus.send(AppEvent::UserCreated(event_data.clone()));
    assert!(send_result.is_ok(), "Event should be sent successfully");
    
    // Receive event
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::UserCreated(data) => {
            assert_eq!(data.id, user_id);
            assert_eq!(data.username, "testuser");
            assert_eq!(data.email, "test@example.com");
        }
        _ => panic!("Expected UserCreated event"),
    }
}

#[tokio::test]
async fn test_user_updated_event() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let user_id = Uuid::new_v4();
    let event_data = UserEventData {
        id: user_id,
        username: "updateduser".to_string(),
        email: "updated@example.com".to_string(),
    };
    
    event_bus.send(AppEvent::UserUpdated(event_data.clone())).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::UserUpdated(data) => {
            assert_eq!(data.id, user_id);
            assert_eq!(data.username, "updateduser");
        }
        _ => panic!("Expected UserUpdated event"),
    }
}

#[tokio::test]
async fn test_user_deleted_event() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let user_id = Uuid::new_v4();
    event_bus.send(AppEvent::UserDeleted(user_id)).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::UserDeleted(id) => {
            assert_eq!(id, user_id);
        }
        _ => panic!("Expected UserDeleted event"),
    }
}

#[tokio::test]
async fn test_post_created_event() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let post_id = Uuid::new_v4();
    let author_id = Uuid::new_v4();
    let event_data = PostEventData {
        id: post_id,
        title: "Test Post".to_string(),
        slug: "test-post".to_string(),
        author_id,
        published: false,
    };
    
    event_bus.send(AppEvent::PostCreated(event_data.clone())).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::PostCreated(data) => {
            assert_eq!(data.id, post_id);
            assert_eq!(data.title, "Test Post");
            assert_eq!(data.slug, "test-post");
            assert_eq!(data.author_id, author_id);
            assert!(!data.published);
        }
        _ => panic!("Expected PostCreated event"),
    }
}

#[tokio::test]
async fn test_post_published_event() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let post_id = Uuid::new_v4();
    let author_id = Uuid::new_v4();
    let event_data = PostEventData {
        id: post_id,
        title: "Published Post".to_string(),
        slug: "published-post".to_string(),
        author_id,
        published: true,
    };
    
    event_bus.send(AppEvent::PostPublished(event_data.clone())).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::PostPublished(data) => {
            assert_eq!(data.id, post_id);
            assert!(data.published);
        }
        _ => panic!("Expected PostPublished event"),
    }
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let (event_bus, mut receiver1) = create_event_bus(16);
    let mut receiver2 = event_bus.subscribe();
    let mut receiver3 = event_bus.subscribe();
    
    let user_id = Uuid::new_v4();
    let event_data = UserEventData {
        id: user_id,
        username: "multicast".to_string(),
        email: "multi@example.com".to_string(),
    };
    
    // Send one event
    event_bus.send(AppEvent::UserCreated(event_data.clone())).ok();
    
    // All three receivers should get it
    let r1 = receiver1.recv().await.expect("Receiver 1 should receive");
    let r2 = receiver2.recv().await.expect("Receiver 2 should receive");
    let r3 = receiver3.recv().await.expect("Receiver 3 should receive");
    
    for received in &[r1, r2, r3] {
        match received {
            AppEvent::UserCreated(data) => {
                assert_eq!(data.id, user_id);
                assert_eq!(data.username, "multicast");
            }
            _ => panic!("Expected UserCreated event"),
        }
    }
}

#[tokio::test]
async fn test_no_subscribers_does_not_panic() {
    let (event_bus, receiver) = create_event_bus(16);
    
    // Drop the receiver so there are no subscribers
    drop(receiver);
    
    let user_id = Uuid::new_v4();
    let event_data = UserEventData {
        id: user_id,
        username: "orphan".to_string(),
        email: "orphan@example.com".to_string(),
    };
    
    // This should not panic even with no subscribers
    let result = event_bus.send(AppEvent::UserCreated(event_data));
    
    // send() returns Err when there are no receivers, which is expected
    assert!(result.is_err(), "Should return error when no subscribers");
}

#[tokio::test]
async fn test_event_bus_capacity() {
    let (event_bus, mut receiver) = create_event_bus(4); // Small capacity
    
    // Send more events than capacity
    for i in 0..10 {
        let event_data = UserEventData {
            id: Uuid::new_v4(),
            username: format!("user{}", i),
            email: format!("user{}@example.com", i),
        };
        event_bus.send(AppEvent::UserCreated(event_data)).ok();
    }
    
    // Should be able to receive at least the last few events
    let mut received_count = 0;
    while receiver.try_recv().is_ok() {
        received_count += 1;
    }
    
    assert!(received_count > 0, "Should receive at least some events");
}

#[tokio::test]
async fn test_comment_event_placeholder() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let comment_id = Uuid::new_v4();
    event_bus.send(AppEvent::CommentCreated(comment_id)).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::CommentCreated(id) => {
            assert_eq!(id, comment_id);
        }
        _ => panic!("Expected CommentCreated event"),
    }
}

#[tokio::test]
async fn test_category_event_placeholder() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let category_id = Uuid::new_v4();
    event_bus.send(AppEvent::CategoryUpdated(category_id)).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::CategoryUpdated(id) => {
            assert_eq!(id, category_id);
        }
        _ => panic!("Expected CategoryUpdated event"),
    }
}

#[tokio::test]
async fn test_tag_event_placeholder() {
    let (event_bus, mut receiver) = create_event_bus(16);
    
    let tag_id = Uuid::new_v4();
    event_bus.send(AppEvent::TagDeleted(tag_id)).ok();
    
    let received = receiver.recv().await.expect("Should receive event");
    match received {
        AppEvent::TagDeleted(id) => {
            assert_eq!(id, tag_id);
        }
        _ => panic!("Expected TagDeleted event"),
    }
}
