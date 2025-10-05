# Event-Driven Architecture Documentation

## Overview

This document describes the event-driven architecture implemented in RustCMS to decouple cross-cutting concerns from business logic.

## Motivation

**Before**: Direct coupling between handlers and side effects
```rust
let user = state.db_create_user(request).await?;
#[cfg(feature = "search")]
state.search.index_user(&user).await?;
#[cfg(feature = "cache")]
state.invalidate_user_caches(user.id).await;
Ok(ApiOk(user))
```

**Problems:**
- Handler code cluttered with feature gates
- Search and cache logic scattered across handlers
- Difficult to test handlers independently
- Hard to add new side effects without modifying handlers

**After**: Event-driven pattern
```rust
let user = state.db_create_user(request).await?;
state.emit_user_created(&user);
Ok(ApiOk(user))
```

**Benefits:**
- Clean separation of concerns
- Feature gates handled by event system
- Easy to add new listeners without changing handlers
- Testable in isolation

## Architecture Components

### 1. Event Definitions (`src/events.rs`)

**AppEvent Enum**: Single comprehensive enum for all domain events
```rust
pub enum AppEvent {
    // User events
    UserCreated(UserEventData),
    UserUpdated(UserEventData),
    UserDeleted(Uuid),
    
    // Post events
    PostCreated(PostEventData),
    PostUpdated(PostEventData),
    PostDeleted(Uuid),
    PostPublished(PostEventData),
    
    // ... more events
}
```

**Event Data Structures**: Lightweight wrappers containing only essential data
```rust
pub struct UserEventData {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

pub struct PostEventData {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub author_id: Uuid,
    pub published: bool,
}
```

**EventBus Type**: Type alias for broadcast channel
```rust
pub type EventBus = broadcast::Sender<AppEvent>;
```

### 2. Event Emission (`src/app.rs`)

**Helper Methods on AppState**: Encapsulate event creation and sending
```rust
impl AppState {
    /// Emit a UserCreated event
    #[cfg(feature = "database")]
    pub fn emit_user_created(&self, user: &crate::models::User) {
        let event_data = crate::events::UserEventData::from_user(user);
        let _ = self.event_bus.send(crate::events::AppEvent::UserCreated(event_data));
    }
    
    // ... more emit_* methods
}
```

**Fire-and-Forget Pattern**: Event emission never fails the primary operation
- Returns `Err` when no subscribers (expected, ignored with `let _`)
- Listeners handle errors independently
- Primary operation always succeeds or fails based on its own logic

### 3. Event Listeners (`src/listeners.rs`)

**Listener Tasks**: Background tokio tasks that process events

**Search Indexing Listener** (feature-gated):
```rust
#[cfg(feature = "search")]
async fn handle_search_event(state: &AppState, event: AppEvent) -> Result<()> {
    match event {
        AppEvent::UserCreated(data) | AppEvent::UserUpdated(data) => {
            // Fetch fresh data from database (single source of truth)
            let user = state.db_get_user_by_id(data.id).await?;
            state.search.index_user(&user).await?;
        }
        AppEvent::UserDeleted(user_id) => {
            state.search.remove_document(&user_id.to_string()).await?;
        }
        // ... more events
    }
    Ok(())
}
```

**Cache Invalidation Listener** (feature-gated):
```rust
#[cfg(feature = "cache")]
async fn handle_cache_event(state: &AppState, event: AppEvent) {
    match event {
        AppEvent::UserCreated(data) | AppEvent::UserUpdated(data) => {
            state.invalidate_user_caches(data.id).await;
        }
        AppEvent::UserDeleted(user_id) => {
            state.invalidate_user_caches(user_id).await;
        }
        // ... more events
    }
}
```

**Key Design Decisions**:
- **Fetch Fresh Data**: Always query database for latest state (data integrity)
- **Resilient Error Handling**: Listener errors are logged, not propagated
- **Lag Handling**: Warns when listener falls behind, continues processing
- **Graceful Shutdown**: Closes when event bus is dropped

### 4. Integration Points

**AppState Initialization** (`src/app.rs`):
```rust
pub struct AppState {
    // ... other fields
    pub event_bus: crate::events::EventBus,
}

// In builder
event_bus: crate::events::create_event_bus(1024).0,
```

**Listener Spawning** (`spawn_background_tasks` in `src/app.rs`):
```rust
fn spawn_background_tasks(state: &AppState) {
    #[cfg(feature = "auth")]
    spawn_auth_cleanup_task(state);
    
    spawn_csrf_cleanup_task(state);
    
    // Spawn event listeners
    crate::listeners::spawn_event_listeners(state.clone(), state.event_bus.clone());
}
```

## Usage Guide

### Adding a New Event

1. **Define the event in `src/events.rs`**:
```rust
pub enum AppEvent {
    // ... existing events
    
    CommentCreated(CommentEventData),
}

pub struct CommentEventData {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
}
```

2. **Add emit helper to `AppState`**:
```rust
impl AppState {
    pub fn emit_comment_created(&self, comment: &Comment) {
        let event_data = CommentEventData::from_comment(comment);
        let _ = self.event_bus.send(AppEvent::CommentCreated(event_data));
    }
}
```

3. **Update listeners** (if needed):
```rust
match event {
    AppEvent::CommentCreated(data) => {
        // Handle comment creation
        let comment = state.db_get_comment(data.id).await?;
        state.cache.invalidate(&format!("post:{}:comments", data.post_id)).await?;
    }
    // ...
}
```

4. **Use in handlers**:
```rust
let comment = state.db_create_comment(request).await?;
state.emit_comment_created(&comment);
Ok(ApiOk(comment))
```

### Migrating Existing Handlers

**Before**:
```rust
let post = state.db_create_post(request).await?;
#[cfg(feature = "search")]
state.search.index_post(&post).await?;
#[cfg(feature = "cache")]
state.invalidate_post_caches(post.id).await;
Ok(ApiOk(post))
```

**After**:
```rust
let post = state.db_create_post(request).await?;
state.emit_post_created(&post);
Ok(ApiOk(post))
```

**Steps**:
1. Remove direct search/cache calls
2. Remove `#[cfg(feature)]` guards
3. Add single `state.emit_*()` call
4. Verify tests still pass

## Design Principles

### 1. Single Source of Truth

**Database is authoritative**. Listeners always fetch fresh data rather than trusting event payloads:

```rust
// ✅ Correct: Fetch from database
let user = state.db_get_user_by_id(data.id).await?;
state.search.index_user(&user).await?;

// ❌ Wrong: Use event data directly
state.search.index_user_partial(data).await?;
```

**Rationale**: Event data is lightweight and may be stale or partial.

### 2. Fire-and-Forget

**Event emission never fails the primary operation**:

```rust
// Use `let _` to ignore send() result
let _ = self.event_bus.send(event);
```

**Rationale**: 
- No subscribers is a valid state (e.g., search feature disabled)
- Listener failures shouldn't affect user-facing operations
- Errors are logged within listeners

### 3. Feature-Gate at Boundaries

**Feature gates only in two places**:
1. On emit helper methods (if they need database models)
2. On listener spawn code

**Handlers have NO feature gates**:
```rust
// ✅ Clean handler
state.emit_user_created(&user);

// ❌ Don't do this
#[cfg(feature = "search")]
state.emit_user_created(&user);
```

### 4. Idempotent Listeners

**Listeners should handle duplicate events gracefully**:
- Search reindexing same document is safe
- Cache invalidation is idempotent
- Use database queries to verify state

### 5. Lagging Tolerance

**Listeners warn but continue when lagging**:
```rust
Err(RecvError::Lagged(skipped)) => {
    warn!(skipped, "Listener lagged, some events were skipped");
}
```

**Rationale**: Dropping events is better than crashing the listener.

## Testing Strategy

### Unit Tests

**Event System** (`tests/event_system_tests.rs`):
- Event creation and broadcasting
- Multiple subscribers receive same event
- All event variants can be sent/received
- No subscribers doesn't panic

**Emit Helpers**:
```rust
#[test]
fn test_emit_user_created() {
    let (tx, mut rx) = create_event_bus(10);
    let state = AppState { event_bus: tx, /* ... */ };
    
    state.emit_user_created(&user);
    
    let event = rx.try_recv().unwrap();
    assert!(matches!(event, AppEvent::UserCreated(_)));
}
```

### Integration Tests

**Listener Behavior**:
- Event triggers search indexing
- Event triggers cache invalidation
- Listener errors don't crash system
- Database fetch occurs after event

**Handler Migration**:
- Handlers still work after removing direct calls
- Search still updates after event migration
- Cache still invalidates after event migration

## Performance Considerations

### Channel Capacity

**Default: 1024 events**
```rust
event_bus: crate::events::create_event_bus(1024).0,
```

**Tuning**:
- Small (128): Low-traffic applications
- Medium (1024): Default, good for most use cases
- Large (10000): High-traffic, slow listeners

**Trade-offs**:
- Larger capacity: More memory, better lag tolerance
- Smaller capacity: Less memory, faster lag detection

### Listener Performance

**Database Fetches**: Each event triggers a DB query
- **Cost**: ~1-10ms per fetch (pooled connections)
- **Benefit**: Data integrity, simplicity
- **Alternative**: Event data caching (future optimization)

**Concurrency**: Listeners run concurrently with request handlers
- **Benefit**: Zero latency impact on user requests
- **Trade-off**: Eventual consistency (search/cache lag)

## Troubleshooting

### Events Not Processing

**Symptoms**: Search not updating, cache not invalidating

**Checks**:
1. Are listeners spawned? (Check logs for "🎧 Spawning event listeners")
2. Are features enabled? (`#[cfg(feature = "search")]`)
3. Are events being emitted? (Add debug logging in emit methods)
4. Are listeners crashing? (Check error logs)

### Listener Lagging

**Symptoms**: Warning logs "Listener lagged, N events skipped"

**Solutions**:
1. Increase channel capacity
2. Optimize listener logic (batch operations)
3. Add more listener instances (future: multiple consumers)

### Memory Usage

**Symptoms**: High memory consumption

**Checks**:
1. Channel capacity too large?
2. Event data structures too heavy? (Use references, not clones)
3. Listeners not consuming fast enough?

## Future Enhancements

### Planned

1. **Persistent Event Log**: Replay events on startup
2. **Event Metrics**: Track event throughput, listener lag
3. **Multiple Listener Instances**: Scale listeners horizontally
4. **Priority Events**: Critical events processed first

### Under Consideration

1. **Event Batching**: Group events for bulk operations
2. **Dead Letter Queue**: Failed events for retry
3. **Event Filtering**: Listeners subscribe to specific events only
4. **External Event Bus**: Redis pub/sub, Kafka integration

## References

- [Tokio broadcast documentation](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [Event-Driven Architecture patterns](https://martinfowler.com/articles/201701-event-driven.html)
- [Domain Events](https://martinfowler.com/eaaDev/DomainEvent.html)

## Change Log

- **2025-10-05**: Initial implementation
  - Added AppEvent enum with User/Post events
  - Implemented search indexing and cache invalidation listeners
  - Migrated auth.rs and users.rs handlers
  - Created comprehensive test suite
