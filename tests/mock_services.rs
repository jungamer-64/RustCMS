//! Mock Services for Integration Testing
//!
//! This module provides mock implementations of database, search, and cache
//! services for testing the event-driven architecture without external dependencies.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

// ============================================================================
// Mock Database Service
// ============================================================================

/// Mock database that stores users and posts in memory
#[derive(Clone)]
pub struct MockDatabase {
    users: Arc<Mutex<HashMap<Uuid, MockUser>>>,
    posts: Arc<Mutex<HashMap<Uuid, MockPost>>>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            posts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn insert_user(&self, user: MockUser) {
        self.users.lock().await.insert(user.id, user);
    }
    
    pub async fn get_user(&self, id: Uuid) -> Option<MockUser> {
        self.users.lock().await.get(&id).cloned()
    }
    
    pub async fn update_user(&self, id: Uuid, user: MockUser) {
        self.users.lock().await.insert(id, user);
    }
    
    pub async fn delete_user(&self, id: Uuid) -> bool {
        self.users.lock().await.remove(&id).is_some()
    }
    
    pub async fn insert_post(&self, post: MockPost) {
        self.posts.lock().await.insert(post.id, post);
    }
    
    pub async fn get_post(&self, id: Uuid) -> Option<MockPost> {
        self.posts.lock().await.get(&id).cloned()
    }
    
    pub async fn update_post(&self, id: Uuid, post: MockPost) {
        self.posts.lock().await.insert(id, post);
    }
    
    pub async fn delete_post(&self, id: Uuid) -> bool {
        self.posts.lock().await.remove(&id).is_some()
    }
}

#[derive(Clone, Debug)]
pub struct MockUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
}

#[derive(Clone, Debug)]
pub struct MockPost {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub author_id: Uuid,
    pub published: bool,
}

// ============================================================================
// Mock Search Service
// ============================================================================

/// Mock search service that tracks indexing operations
#[derive(Clone)]
pub struct MockSearchService {
    indexed_users: Arc<Mutex<HashMap<Uuid, MockUser>>>,
    indexed_posts: Arc<Mutex<HashMap<Uuid, MockPost>>>,
    removed_documents: Arc<Mutex<Vec<String>>>,
    index_user_calls: Arc<Mutex<usize>>,
    index_post_calls: Arc<Mutex<usize>>,
    remove_calls: Arc<Mutex<usize>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockSearchService {
    pub fn new() -> Self {
        Self {
            indexed_users: Arc::new(Mutex::new(HashMap::new())),
            indexed_posts: Arc::new(Mutex::new(HashMap::new())),
            removed_documents: Arc::new(Mutex::new(Vec::new())),
            index_user_calls: Arc::new(Mutex::new(0)),
            index_post_calls: Arc::new(Mutex::new(0)),
            remove_calls: Arc::new(Mutex::new(0)),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Index a user (stores in mock index)
    pub async fn index_user(&self, user: &MockUser) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Mock search service configured to fail".to_string());
        }
        
        *self.index_user_calls.lock().await += 1;
        self.indexed_users.lock().await.insert(user.id, user.clone());
        Ok(())
    }
    
    /// Index a post (stores in mock index)
    pub async fn index_post(&self, post: &MockPost) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Mock search service configured to fail".to_string());
        }
        
        *self.index_post_calls.lock().await += 1;
        self.indexed_posts.lock().await.insert(post.id, post.clone());
        Ok(())
    }
    
    /// Remove document from index
    pub async fn remove_document(&self, id: &str) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Mock search service configured to fail".to_string());
        }
        
        *self.remove_calls.lock().await += 1;
        self.removed_documents.lock().await.push(id.to_string());
        Ok(())
    }
    
    // ========== Verification Methods ==========
    
    /// Check if user was indexed
    pub async fn verify_user_indexed(&self, id: Uuid) -> bool {
        self.indexed_users.lock().await.contains_key(&id)
    }
    
    /// Check if post was indexed
    pub async fn verify_post_indexed(&self, id: Uuid) -> bool {
        self.indexed_posts.lock().await.contains_key(&id)
    }
    
    /// Check if document was removed
    pub async fn verify_document_removed(&self, id: &str) -> bool {
        self.removed_documents.lock().await.contains(&id.to_string())
    }
    
    /// Get number of index_user calls
    pub async fn index_user_call_count(&self) -> usize {
        *self.index_user_calls.lock().await
    }
    
    /// Get number of index_post calls
    pub async fn index_post_call_count(&self) -> usize {
        *self.index_post_calls.lock().await
    }
    
    /// Get number of remove_document calls
    pub async fn remove_document_call_count(&self) -> usize {
        *self.remove_calls.lock().await
    }
    
    /// Configure service to fail on next operation
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }
    
    /// Get indexed user (for verification)
    pub async fn get_indexed_user(&self, id: Uuid) -> Option<MockUser> {
        self.indexed_users.lock().await.get(&id).cloned()
    }
    
    /// Get indexed post (for verification)
    pub async fn get_indexed_post(&self, id: Uuid) -> Option<MockPost> {
        self.indexed_posts.lock().await.get(&id).cloned()
    }
}

// ============================================================================
// Mock Cache Service
// ============================================================================

/// Mock cache service that tracks invalidation operations
#[derive(Clone)]
pub struct MockCacheService {
    deleted_keys: Arc<Mutex<Vec<String>>>,
    delete_calls: Arc<Mutex<usize>>,
    should_fail: Arc<Mutex<bool>>,
}

impl MockCacheService {
    pub fn new() -> Self {
        Self {
            deleted_keys: Arc::new(Mutex::new(Vec::new())),
            delete_calls: Arc::new(Mutex::new(0)),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Delete a cache key
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Mock cache service configured to fail".to_string());
        }
        
        *self.delete_calls.lock().await += 1;
        self.deleted_keys.lock().await.push(key.to_string());
        Ok(())
    }
    
    /// Delete keys matching a pattern
    pub async fn delete_pattern(&self, pattern: &str) -> Result<(), String> {
        if *self.should_fail.lock().await {
            return Err("Mock cache service configured to fail".to_string());
        }
        
        *self.delete_calls.lock().await += 1;
        self.deleted_keys.lock().await.push(pattern.to_string());
        Ok(())
    }
    
    // ========== Verification Methods ==========
    
    /// Check if key was deleted
    pub async fn verify_key_deleted(&self, key: &str) -> bool {
        self.deleted_keys.lock().await.contains(&key.to_string())
    }
    
    /// Check if pattern was deleted
    pub async fn verify_pattern_deleted(&self, pattern: &str) -> bool {
        self.deleted_keys.lock().await.iter().any(|k| k.contains(pattern))
    }
    
    /// Get number of delete calls
    pub async fn delete_call_count(&self) -> usize {
        *self.delete_calls.lock().await
    }
    
    /// Get all deleted keys (for detailed verification)
    pub async fn get_deleted_keys(&self) -> Vec<String> {
        self.deleted_keys.lock().await.clone()
    }
    
    /// Configure service to fail on next operation
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }
    
    /// Clear all tracking data (for test cleanup)
    pub async fn clear(&self) {
        self.deleted_keys.lock().await.clear();
        *self.delete_calls.lock().await = 0;
    }
}

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a test user with random ID
pub fn create_test_user(username: &str) -> MockUser {
    MockUser {
        id: Uuid::new_v4(),
        username: username.to_string(),
        email: format!("{}@example.com", username),
        role: "User".to_string(),
    }
}

/// Create a test post with random ID
pub fn create_test_post(title: &str, author_id: Uuid) -> MockPost {
    MockPost {
        id: Uuid::new_v4(),
        title: title.to_string(),
        slug: title.to_lowercase().replace(' ', "-"),
        author_id,
        published: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_database_operations() {
        let db = MockDatabase::new();
        let user = create_test_user("testuser");
        let user_id = user.id;
        
        // Insert
        db.insert_user(user.clone()).await;
        
        // Get
        let retrieved = db.get_user(user_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().username, "testuser");
        
        // Update
        let mut updated_user = user.clone();
        updated_user.email = "newemail@example.com".to_string();
        db.update_user(user_id, updated_user.clone()).await;
        
        let retrieved = db.get_user(user_id).await.unwrap();
        assert_eq!(retrieved.email, "newemail@example.com");
        
        // Delete
        assert!(db.delete_user(user_id).await);
        assert!(db.get_user(user_id).await.is_none());
    }
    
    #[tokio::test]
    async fn test_mock_search_service() {
        let search = MockSearchService::new();
        let user = create_test_user("testuser");
        
        // Index user
        assert!(search.index_user(&user).await.is_ok());
        
        // Verify indexed
        assert!(search.verify_user_indexed(user.id).await);
        assert_eq!(search.index_user_call_count().await, 1);
        
        // Get indexed user
        let indexed = search.get_indexed_user(user.id).await;
        assert!(indexed.is_some());
        assert_eq!(indexed.unwrap().username, "testuser");
    }
    
    #[tokio::test]
    async fn test_mock_search_service_failure() {
        let search = MockSearchService::new();
        let user = create_test_user("testuser");
        
        // Configure to fail
        search.set_should_fail(true).await;
        
        // Index should fail
        let result = search.index_user(&user).await;
        assert!(result.is_err());
        
        // No user should be indexed
        assert!(!search.verify_user_indexed(user.id).await);
    }
    
    #[tokio::test]
    async fn test_mock_cache_service() {
        let cache = MockCacheService::new();
        
        // Delete key
        assert!(cache.delete("user:123").await.is_ok());
        
        // Verify deleted
        assert!(cache.verify_key_deleted("user:123"));
        assert_eq!(cache.delete_call_count().await, 1);
        
        // Delete pattern
        assert!(cache.delete_pattern("users:*").await.is_ok());
        assert!(cache.verify_pattern_deleted("users:"));
        assert_eq!(cache.delete_call_count().await, 2);
    }
    
    #[tokio::test]
    async fn test_mock_cache_service_failure() {
        let cache = MockCacheService::new();
        
        // Configure to fail
        cache.set_should_fail(true).await;
        
        // Delete should fail
        let result = cache.delete("user:123").await;
        assert!(result.is_err());
        
        // No key should be tracked
        assert!(!cache.verify_key_deleted("user:123"));
    }
}
