//! Integration Tests for Event-Driven Architecture
//!
//! These tests verify the complete end-to-end event flow:
//! 1. Handler performs CRUD operation
//! 2. Handler emits event via AppState
//! 3. Event bus broadcasts to listeners
//! 4. Listeners process events (search indexing, cache invalidation)
//!
//! # Test Strategy
//!
//! - Use mock database/search/cache services
//! - Simulate event listeners
//! - Verify complete event flow
//! - Test failure scenarios and resilience

mod mock_services;

use cms_backend::events::{AppEvent, PostEventData, UserEventData};
use mock_services::*;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

/// Test context with mock services and event bus
struct TestContext {
    database: MockDatabase,
    search: MockSearchService,
    cache: MockCacheService,
    event_bus: tokio::sync::broadcast::Sender<AppEvent>,
    receiver: tokio::sync::broadcast::Receiver<AppEvent>,
}

impl TestContext {
    fn new() -> Self {
        let (event_bus, receiver) = tokio::sync::broadcast::channel(16);

        Self {
            database: MockDatabase::new(),
            search: MockSearchService::new(),
            cache: MockCacheService::new(),
            event_bus,
            receiver,
        }
    }

    /// Spawn mock listeners that simulate the real listener behavior
    fn spawn_mock_listeners(&mut self) {
        let search = self.search.clone();
        let cache = self.cache.clone();
        let database = self.database.clone();
        let mut search_receiver = self.event_bus.subscribe();
        let mut cache_receiver = self.event_bus.subscribe();

        // Mock search listener
        tokio::spawn(async move {
            while let Ok(event) = search_receiver.recv().await {
                match event {
                    AppEvent::UserCreated(data) | AppEvent::UserUpdated(data) => {
                        if let Some(user) = database.get_user(data.id).await {
                            let _ = search.index_user(&user).await;
                        }
                    }
                    AppEvent::UserDeleted(user_id) => {
                        let _ = search.remove_document(&user_id.to_string()).await;
                    }
                    AppEvent::PostCreated(data)
                    | AppEvent::PostUpdated(data)
                    | AppEvent::PostPublished(data) => {
                        if let Some(post) = database.get_post(data.id).await {
                            let _ = search.index_post(&post).await;
                        }
                    }
                    AppEvent::PostDeleted(post_id) => {
                        let _ = search.remove_document(&post_id.to_string()).await;
                    }
                    _ => {}
                }
            }
        });

        // Mock cache listener
        tokio::spawn(async move {
            while let Ok(event) = cache_receiver.recv().await {
                match event {
                    AppEvent::UserCreated(data) | AppEvent::UserUpdated(data) => {
                        let _ = cache.delete(&format!("user:{}", data.id)).await;
                        let _ = cache.delete_pattern("users:*").await;
                    }
                    AppEvent::UserDeleted(user_id) => {
                        let _ = cache.delete(&format!("user:{}", user_id)).await;
                        let _ = cache.delete_pattern("users:*").await;
                    }
                    AppEvent::PostCreated(data)
                    | AppEvent::PostUpdated(data)
                    | AppEvent::PostPublished(data) => {
                        let _ = cache.delete(&format!("post:{}", data.id)).await;
                        let _ = cache.delete_pattern("posts:*").await;
                    }
                    AppEvent::PostDeleted(post_id) => {
                        let _ = cache.delete(&format!("post:{}", post_id)).await;
                        let _ = cache.delete_pattern("posts:*").await;
                    }
                    _ => {}
                }
            }
        });
    }
}

#[tokio::test]
async fn test_user_created_event_flow() {
    let mut ctx = TestContext::new();
    ctx.spawn_mock_listeners();

    // Create user in mock database
    let user = create_test_user("testuser");
    ctx.database.insert_user(user.clone()).await;

    // Simulate user creation event
    let user_data = UserEventData {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
    };

    // Emit event (simulating state.emit_user_created())
    let _ = ctx.event_bus.send(AppEvent::UserCreated(user_data.clone()));

    // Verify event received
    let result = timeout(Duration::from_millis(100), ctx.receiver.recv()).await;
    assert!(result.is_ok(), "Event should be received");

    match result.unwrap() {
        Ok(AppEvent::UserCreated(data)) => {
            assert_eq!(data.id, user.id);
            assert_eq!(data.username, "testuser");
        }
        _ => panic!("Expected UserCreated event"),
    }

    // Wait for listeners to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify search indexing occurred
    assert!(
        ctx.search.verify_user_indexed(user.id).await,
        "User should be indexed in search"
    );

    // Verify cache invalidation occurred
    assert!(
        ctx.cache
            .verify_key_deleted(&format!("user:{}", user.id))
            .await,
        "User cache should be invalidated"
    );
    assert!(
        ctx.cache.verify_pattern_deleted("users:").await,
        "Users pattern should be invalidated"
    );
}

#[tokio::test]
async fn test_post_created_triggers_search_indexing() {
    let mut ctx = TestContext::new();
    ctx.spawn_mock_listeners();

    // Create post in mock database
    let author_id = Uuid::new_v4();
    let post = create_test_post("Test Post", author_id);
    ctx.database.insert_post(post.clone()).await;

    // Emit post created event
    let post_data = PostEventData {
        id: post.id,
        title: post.title.clone(),
        slug: post.slug.clone(),
        author_id: post.author_id,
        published: post.published,
    };

    let _ = ctx.event_bus.send(AppEvent::PostCreated(post_data));

    // Wait for listeners to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify search indexing occurred
    assert!(
        ctx.search.verify_post_indexed(post.id).await,
        "Post should be indexed in search"
    );

    // Verify the indexed post has correct data (came from database)
    let indexed_post = ctx.search.get_indexed_post(post.id).await;
    assert!(indexed_post.is_some());
    assert_eq!(indexed_post.unwrap().title, "Test Post");

    // Verify cache invalidation occurred
    assert!(
        ctx.cache
            .verify_key_deleted(&format!("post:{}", post.id))
            .await
    );
    assert!(ctx.cache.verify_pattern_deleted("posts:").await);
}

#[tokio::test]
async fn test_user_updated_triggers_cache_invalidation() {
    let mut ctx = TestContext::new();
    ctx.spawn_mock_listeners();

    // Create user in mock database
    let user = create_test_user("testuser");
    ctx.database.insert_user(user.clone()).await;

    // Emit user updated event
    let user_data = UserEventData {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: "Admin".to_string(), // Changed role
    };

    let _ = ctx.event_bus.send(AppEvent::UserUpdated(user_data));

    // Wait for listeners to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify cache invalidation occurred
    assert!(
        ctx.cache
            .verify_key_deleted(&format!("user:{}", user.id))
            .await,
        "User cache key should be deleted"
    );
    assert!(
        ctx.cache.verify_pattern_deleted("users:").await,
        "User list cache should be invalidated"
    );

    // Verify search was also updated (users are indexed on update)
    assert!(
        ctx.search.verify_user_indexed(user.id).await,
        "User should be re-indexed in search"
    );
}

#[tokio::test]
async fn test_multiple_listeners_receive_same_event() {
    let mut ctx = TestContext::new();

    // Create additional receivers to simulate multiple independent listeners
    let mut receiver2 = ctx.event_bus.subscribe();
    let mut receiver3 = ctx.event_bus.subscribe();

    // Spawn mock listeners (uses ctx.receiver internally)
    ctx.spawn_mock_listeners();

    let user_id = Uuid::new_v4();
    let event = AppEvent::UserDeleted(user_id);

    // Send event
    let _ = ctx.event_bus.send(event);

    // All receivers should get it
    let r2 = timeout(Duration::from_millis(100), receiver2.recv()).await;
    let r3 = timeout(Duration::from_millis(100), receiver3.recv()).await;

    assert!(r2.is_ok(), "Second receiver should get event");
    assert!(r3.is_ok(), "Third receiver should get event");

    // Verify all are UserDeleted events
    for result in [r2, r3] {
        match result.unwrap() {
            Ok(AppEvent::UserDeleted(id)) => assert_eq!(id, user_id),
            _ => panic!("Expected UserDeleted event"),
        }
    }

    // Wait for mock listeners to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify mock listeners also processed the event
    assert!(
        ctx.cache
            .verify_key_deleted(&format!("user:{}", user_id))
            .await
    );
}

#[tokio::test]
async fn test_listener_error_doesnt_crash_system() {
    // Test the resilience principle:
    // "Listener failures are logged but don't crash the application"

    let mut ctx = TestContext::new();

    // Configure search service to fail intentionally
    ctx.search.set_should_fail(true).await;

    // Spawn listeners (search will fail, cache should succeed)
    ctx.spawn_mock_listeners();

    // Create user in database
    let user = create_test_user("testuser");
    ctx.database.insert_user(user.clone()).await;

    // Emit event
    let user_data = UserEventData {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
    };

    let _ = ctx.event_bus.send(AppEvent::UserCreated(user_data));

    // Wait for listeners to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify search listener attempted (and failed) to index
    assert_eq!(
        ctx.search.index_user_call_count().await,
        1,
        "Search listener should have attempted to index"
    );
    assert!(
        !ctx.search.verify_user_indexed(user.id).await,
        "User should NOT be indexed (search failed)"
    );

    // Verify cache listener STILL processed successfully
    assert!(
        ctx.cache
            .verify_key_deleted(&format!("user:{}", user.id))
            .await,
        "Cache listener should succeed despite search failure"
    );
    assert!(
        ctx.cache.verify_pattern_deleted("users:").await,
        "Cache pattern should be invalidated"
    );

    // Verify system continued operating (no panic, no crash)
    // We can emit another event successfully
    let another_user = create_test_user("another");
    ctx.database.insert_user(another_user.clone()).await;
    let _ = ctx.event_bus.send(AppEvent::UserCreated(UserEventData {
        id: another_user.id,
        username: another_user.username.clone(),
        email: another_user.email.clone(),
        role: another_user.role.clone(),
    }));

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Second event also processed (cache worked even though search still failing)
    assert!(
        ctx.cache
            .verify_key_deleted(&format!("user:{}", another_user.id))
            .await
    );
}

#[tokio::test]
async fn test_listener_lag_handling() {
    // Test the lag tolerance principle:
    // "Listeners warn but continue when lagging"

    // Create event bus with VERY small buffer to trigger overflow quickly
    let (event_bus, mut receiver) = broadcast::channel::<AppEvent>(2);

    let database = MockDatabase::new();
    let search = MockSearchService::new();
    let cache = MockCacheService::new();

    // Create users in database first
    let user_ids: Vec<Uuid> = (0..10)
        .map(|i| {
            let user = create_test_user(&format!("user{}", i));
            let id = user.id;
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    database.insert_user(user).await;
                })
            });
            id
        })
        .collect();

    // Spawn a SLOW listener that takes time to process
    let search_clone = search.clone();
    let database_clone = database.clone();
    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(AppEvent::UserCreated(data)) => {
                    // Simulate slow processing
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    if let Some(user) = database_clone.get_user(data.id).await {
                        let _ = search_clone.index_user(&user).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    // Lag detected! This is expected behavior
                    eprintln!("Listener lagged and skipped {} events", skipped);
                    // Listener continues processing (doesn't crash)
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
                _ => {}
            }
        }
    });

    // Rapidly emit many events (more than buffer can hold)
    for &user_id in &user_ids {
        let user_data = UserEventData {
            id: user_id,
            username: format!("user{}", user_id),
            email: format!("user{}@example.com", user_id),
            role: "User".to_string(),
        };

        // Fire-and-forget (some may be dropped due to lag)
        let _ = event_bus.send(AppEvent::UserCreated(user_data));
    }

    // Give listener time to process what it can
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Key assertion: Listener didn't crash, it continued processing
    // Not all users will be indexed (due to lag), but SOME should be
    let indexed_count = search.index_user_call_count().await;

    println!(
        "Total events emitted: {}, Indexed: {}",
        user_ids.len(),
        indexed_count
    );

    // The listener should have processed at least SOME events (not 0, not all)
    assert!(
        indexed_count > 0,
        "Listener should have processed some events"
    );
    assert!(
        indexed_count < user_ids.len(),
        "Not all events should be processed (lag should have occurred)"
    );
}

#[tokio::test]
async fn test_listener_fetches_fresh_database_data() {
    // Test the "single source of truth" principle:
    // "Database is authoritative. Listeners always fetch fresh data."

    let mut ctx = TestContext::new();
    ctx.spawn_mock_listeners();

    // Step 1: Create initial user in database
    let mut user = create_test_user("original_username");
    let original_email = user.email.clone();
    ctx.database.insert_user(user.clone()).await;

    // Step 2: Update user DIRECTLY in database (bypass event system)
    user.username = "updated_username".to_string();
    user.email = "updated@example.com".to_string();
    user.role = "Admin".to_string(); // Changed role
    ctx.database.update_user(user.id, user.clone()).await;

    // Step 3: Emit event with OLD data (as if handler had stale data)
    let stale_event_data = UserEventData {
        id: user.id,
        username: "original_username".to_string(), // OLD username
        email: original_email.clone(),             // OLD email
        role: "User".to_string(),                  // OLD role
    };

    let _ = ctx.event_bus.send(AppEvent::UserCreated(stale_event_data));

    // Wait for listener to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Step 4 & 5: Verify listener fetched FRESH data from database
    assert!(
        ctx.search.verify_user_indexed(user.id).await,
        "User should be indexed"
    );

    // Get the indexed user and verify it has UPDATED data, not stale event data
    let indexed_user = ctx.search.get_indexed_user(user.id).await;
    assert!(indexed_user.is_some(), "Indexed user should exist");

    let indexed_user = indexed_user.unwrap();
    assert_eq!(
        indexed_user.username, "updated_username",
        "Listener should have fetched NEW username from database, not old event data"
    );
    assert_eq!(
        indexed_user.email, "updated@example.com",
        "Listener should have fetched NEW email from database, not old event data"
    );
    assert_eq!(
        indexed_user.role, "Admin",
        "Listener should have fetched NEW role from database, not old event data"
    );

    // This proves listeners don't trust event payloads - they always fetch from DB
}

#[tokio::test]
async fn test_fire_and_forget_no_subscribers() {
    // Test that emitting events with no subscribers doesn't panic
    let ctx = TestContext::new();

    // Drop the receiver so there are no subscribers
    drop(ctx.receiver);

    let user_id = Uuid::new_v4();
    let event = AppEvent::UserDeleted(user_id);

    // This should not panic (fire-and-forget principle)
    let result = ctx.event_bus.send(event);

    // send() returns Err when no subscribers (expected)
    assert!(result.is_err(), "Expected error when no subscribers");

    // But the system continues operating normally
    // (in real code, we use `let _ = event_bus.send()` to ignore this)

    // Verify no panics occurred and we can still use the event bus
    let another_event = AppEvent::UserDeleted(Uuid::new_v4());
    let _ = ctx.event_bus.send(another_event); // Still doesn't panic
}

/// Integration test helper: Wait for listener to process event
async fn wait_for_listener_processing() {
    // In real tests, you'd have a mechanism to know when listeners finished
    // For now, just a simple delay
    tokio::time::sleep(Duration::from_millis(50)).await;
}

/// Integration test helper: Verify search index contains expected document
#[cfg(feature = "search")]
async fn verify_search_indexed(_state: &AppState, _id: Uuid) {
    // In real tests, query search service to verify document exists
    // assert!(state.search.get_document(id).await.is_ok());
}

/// Integration test helper: Verify cache key was invalidated
#[cfg(feature = "cache")]
async fn verify_cache_invalidated(_state: &AppState, _key: &str) {
    // In real tests, query cache service to verify key doesn't exist
    // assert!(state.cache.get::<String>(key).await.unwrap().is_none());
}

// ============================================================================
// Test Fixtures and Mocks (for future implementation)
// ============================================================================

/// Mock search service that tracks index_* calls
#[cfg(feature = "search")]
#[derive(Clone)]
struct MockSearchService {
    indexed_users: std::sync::Arc<tokio::sync::Mutex<Vec<Uuid>>>,
    indexed_posts: std::sync::Arc<tokio::sync::Mutex<Vec<Uuid>>>,
}

#[cfg(feature = "search")]
impl MockSearchService {
    fn new() -> Self {
        Self {
            indexed_users: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            indexed_posts: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    async fn verify_user_indexed(&self, id: Uuid) -> bool {
        self.indexed_users.lock().await.contains(&id)
    }

    async fn verify_post_indexed(&self, id: Uuid) -> bool {
        self.indexed_posts.lock().await.contains(&id)
    }

    async fn index_user_call_count(&self) -> usize {
        self.indexed_users.lock().await.len()
    }

    async fn get_indexed_user(&self, id: Uuid) -> Option<Uuid> {
        let users = self.indexed_users.lock().await;
        users.iter().find(|&&uid| uid == id).copied()
    }
}

/// Mock cache service that tracks delete calls
#[cfg(feature = "cache")]
struct MockCacheService {
    deleted_keys: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
}

#[cfg(feature = "cache")]
impl MockCacheService {
    fn new() -> Self {
        Self {
            deleted_keys: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    async fn verify_key_deleted(&self, key: &str) -> bool {
        self.deleted_keys.lock().await.contains(&key.to_string())
    }

    async fn verify_pattern_deleted(&self, pattern: &str) -> bool {
        let keys = self.deleted_keys.lock().await;
        keys.iter().any(|k| k.starts_with(pattern))
    }
}

// ============================================================================
// Example of Full Integration Test (requires setup)
// ============================================================================

/// Full end-to-end test: User registration → Event emission → Search indexing
///
/// This test would require:
/// 1. Test database with schema
/// 2. Real AppState with all services
/// 3. Event listeners spawned
/// 4. HTTP handler invocation
#[tokio::test]
#[ignore = "requires full test environment setup"]
async fn test_full_user_registration_flow() {
    // Setup test environment
    // let db = setup_test_database().await;
    // let state = AppState::from_test_config(db).await;
    // spawn_event_listeners(state.clone(), state.event_bus.clone());

    // Simulate registration handler
    // let request = CreateUserRequest { ... };
    // let user = state.auth_create_user(request).await.unwrap();
    // state.emit_user_created(&user);

    // Wait for listener processing
    // wait_for_listener_processing().await;

    // Verify search indexing occurred
    // assert!(verify_search_indexed(&state, user.id).await);

    // Verify cache invalidation occurred
    // assert!(verify_cache_invalidated(&state, &format!("users:*")).await);
}

#[cfg(test)]
mod integration_test_plan {
    //! Integration Test Implementation Plan
    //!
    //! ## Phase 1: Basic Event Flow
    //! - [x] Test event emission and reception
    //! - [x] Test multiple subscribers
    //! - [x] Test fire-and-forget with no subscribers
    //!
    //! ## Phase 2: Listener Processing (TODO)
    //! - [ ] Mock search service and verify index_user() calls
    //! - [ ] Mock cache service and verify delete() calls
    //! - [ ] Test database fetch in listeners
    //! - [ ] Test error handling in listeners
    //!
    //! ## Phase 3: Handler Integration (TODO)
    //! - [ ] Test auth handler with real event emission
    //! - [ ] Test CRUD handlers with event emission
    //! - [ ] Verify old direct calls are removed
    //!
    //! ## Phase 4: Resilience (TODO)
    //! - [ ] Test listener crashes don't affect handlers
    //! - [ ] Test channel overflow / lagging
    //! - [ ] Test graceful shutdown
    //!
    //! ## Phase 5: Performance (TODO)
    //! - [ ] Benchmark event throughput
    //! - [ ] Measure listener processing latency
    //! - [ ] Test under high concurrency
}
