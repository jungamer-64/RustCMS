# Phase 6 - Database Layer Implementation Plan

**Start Date**: 2025年10月17日  
**Expected Completion**: 2025年10月25日  
**Status**: 🚀 IN PROGRESS

---

## Overview

Phase 6 transitions from repository stubs to fully functional database implementations using Diesel ORM. All 5 repositories (User, Post, Comment, Tag, Category) will have actual CRUD operations backed by PostgreSQL.

## Architecture Overview

### Three-Tier Database Architecture

```
Domain Layer (Entities + Value Objects)
    ↓
Application Layer (Repository Ports)
    ↓
Infrastructure Layer (Diesel Adapters) ← Phase 6 Focus
    ↓
Database (PostgreSQL + Migrations)
```

### Database Stack

- **ORM**: Diesel 2.x with async support
- **Database**: PostgreSQL 14+
- **Connection Pool**: Diesel built-in connection pooling
- **Migrations**: Diesel migration system
- **Testing**: testcontainers for integration tests

---

## Phase 6 Objectives

### Primary Goals

1. ✅ Implement actual CRUD for all 5 repositories
2. ✅ Database schema alignment with domain entities
3. ✅ Type-safe SQL queries through Diesel
4. ✅ Transaction support for multi-entity operations
5. ✅ Error handling and recovery strategies

### Success Criteria

- [ ] All 5 repositories have working CRUD implementation
- [ ] 100+ new integration tests with real database
- [ ] Zero SQL injection vulnerabilities
- [ ] Response time < 100ms for typical queries
- [ ] Database schema migrations automated
- [ ] Connection pooling optimized
- [ ] Error recovery tested and documented

---

## Implementation Strategy

### Step 1: Database Schema Analysis (Today)
- Review existing migrations
- Map domain entities to tables
- Identify missing migrations
- Plan schema updates

### Step 2: User Repository (Immediate)
- Implement find_by_id with Diesel
- Implement find_by_email with Diesel
- Implement create with transaction
- Implement update with optimistic locking
- Implement delete with cascade handling

### Step 3: Post Repository (Sequential)
- Implement paginated queries
- Implement filtering and sorting
- Implement post_count tracking
- Implement published_at timestamp handling

### Step 4: Comment Repository (Sequential)
- Implement threading queries
- Implement depth-limited queries
- Implement author filtering
- Implement post association

### Step 5: Tag/Category Repositories (Feature-gated)
- Implement usage counting
- Implement slug uniqueness validation
- Implement active status filtering
- Implement relationship queries

### Step 6: Integration & Testing
- Create testcontainers setup
- Write comprehensive integration tests
- Performance benchmarking
- Error recovery testing

---

## Database Schema Strategy

### Existing Migrations

Current migrations directory: `migrations/`

Expected tables:
- `users` - User entities with email/username uniqueness
- `posts` - Post entities with author relationships
- `comments` - Comment entities with threading support
- `tags` - Tag entities with usage counters
- `categories` - Category entities with hierarchy

### Schema Mapping

**users table**:
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
);
```

**posts table**:
```sql
CREATE TABLE posts (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    status VARCHAR(20) NOT NULL, -- 'draft' or 'published'
    published_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
);
```

**comments table**:
```sql
CREATE TABLE comments (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id),
    author_id UUID NOT NULL REFERENCES users(id),
    content TEXT NOT NULL,
    parent_id UUID REFERENCES comments(id),
    depth INT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
);
```

**tags table**:
```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    usage_count INT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
);
```

**categories table**:
```sql
CREATE TABLE categories (
    id UUID PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    post_count INT DEFAULT 0,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
);
```

### Schema Validation

- [ ] Review actual schema in project
- [ ] Identify missing columns
- [ ] Plan migration for schema alignment
- [ ] Validate indexes for performance

---

## Diesel Implementation Pattern

### User Repository Example (Template)

```rust
use diesel::prelude::*;
use crate::domain::entities::user::{User, UserId, Email, Username};
use crate::application::ports::user_repository::{UserRepository, RepositoryError};

#[async_trait]
impl UserRepository for DieselUserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let db = self.db.clone();
        
        tokio::task::spawn_blocking(move || {
            let mut conn = db.get_connection()?;
            
            users::table
                .find(id.as_uuid())
                .first::<UserModel>(&mut conn)
                .optional()
                .map_err(|e| RepositoryError::Unexpected(e.to_string()))?
                .map(|model| model.to_domain())
        })
        .await
        .map_err(|e| RepositoryError::Unexpected(e.to_string()))?
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError> {
        // Similar pattern with email filtering
    }

    async fn create(&self, user: User) -> Result<(), RepositoryError> {
        // Insert transaction with validation
    }

    async fn update(&self, user: User) -> Result<(), RepositoryError> {
        // Update with version checking (optimistic locking)
    }

    async fn delete(&self, id: UserId) -> Result<(), RepositoryError> {
        // Soft delete or cascade handling
    }
}
```

### Key Patterns

1. **Async Blocking**: Use `tokio::task::spawn_blocking` for sync Diesel operations
2. **Connection Pooling**: Get connection from pool, handle pool exhaustion
3. **Error Mapping**: Map Diesel errors to repository errors
4. **Transaction Support**: Use `connection.transaction()` for multi-step operations
5. **Type Mapping**: Create `UserModel` (DB) ↔ `User` (Domain) converters

---

## Testing Strategy

### Integration Test Structure

```rust
#[tokio::test]
async fn test_user_repository_create_and_find() {
    // Setup: Start testcontainer with PostgreSQL
    let db = setup_test_db().await;
    let repo = DieselUserRepository::new(db);
    
    // Create user
    let user = User::new(...)?;
    repo.create(user.clone()).await?;
    
    // Find by ID
    let found = repo.find_by_id(user.id()).await?;
    assert_eq!(found, Some(user));
}
```

### Test Coverage Goals

- ✅ CRUD operations for all entities
- ✅ Error handling (NotFound, Conflict, Unexpected)
- ✅ Pagination and filtering
- ✅ Transaction rollback scenarios
- ✅ Connection pool exhaustion handling
- ✅ Concurrent access patterns
- ✅ Performance benchmarks

---

## Risk Mitigation

### Known Risks

1. **Connection Pool Exhaustion**
   - Mitigation: Connection pool size tuning, connection timeout handling
   - Test: Simulate concurrent access

2. **N+1 Query Problem**
   - Mitigation: Use Diesel's eager loading (load_inner, load_option)
   - Test: Query count verification in tests

3. **Transaction Deadlocks**
   - Mitigation: Consistent ordering of resource access
   - Test: Concurrent transaction test scenarios

4. **Type Mismatches**
   - Mitigation: Comprehensive model mapping tests
   - Test: Property-based testing with proptest

5. **SQL Injection** (Diesel prevents this, but verify)
   - Mitigation: All queries use Diesel type safety
   - Test: Malformed input validation

---

## Rollout Plan

### Week 1 (Phase 6.1)
- Day 1: Schema review and migration planning
- Day 2-3: User Repository implementation
- Day 4-5: Post Repository implementation

### Week 2 (Phase 6.2)
- Day 1-2: Comment Repository implementation
- Day 3-4: Tag Repository implementation
- Day 5: Category Repository implementation

### Week 3 (Phase 6.3)
- Day 1-2: Integration tests and testcontainers setup
- Day 3-4: Performance testing and optimization
- Day 5: Documentation and Phase 6 completion

---

## Success Metrics

| Metric | Target | Status |
|--------|--------|--------|
| CRUD Completion | 100% | ⏳ In Progress |
| Query Response Time | < 100ms | ⏳ Pending |
| Test Coverage | 100% | ⏳ Pending |
| CVE Count | 0 | ✅ Verified |
| Integration Tests | 50+ | ⏳ Pending |
| Documentation | Complete | ⏳ In Progress |

---

## Deliverables

### Phase 6 Completion Checklist

- [ ] All 5 repository implementations with Diesel
- [ ] 50+ integration tests with testcontainers
- [ ] Database schema aligned with domain model
- [ ] Performance benchmarks documented
- [ ] Error recovery strategies tested
- [ ] Connection pooling optimized
- [ ] Phase 6 completion report
- [ ] Ready for Phase 7 (API Layer)

---

## Next Steps

1. ✅ Review existing database schema
2. ⏳ Implement User Repository with Diesel
3. ⏳ Add integration tests with PostgreSQL
4. ⏳ Implement remaining repositories sequentially
5. ⏳ Performance optimization and tuning

**Current Task**: Schema analysis and migration review

---

**Estimated Time**: 5-7 days  
**Complexity**: High  
**Dependencies**: PostgreSQL, testcontainers, Diesel 2.x
