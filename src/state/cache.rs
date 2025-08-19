//! 高性能リクエストキャッシュシステム
//! 
//! - LRU方式のメモリキャッシュ
//! - 並行アクセス最適化
//! - TTL（Time To Live）サポート

use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use bytes::Bytes;
use super::FastHasher;

/// キャッシュエントリ
#[derive(Clone, Debug)]
struct CacheEntry {
    data: Bytes,
    created_at: Instant,
    ttl: Duration,
    access_count: u64,
}

impl CacheEntry {
    fn new(data: Bytes, ttl: Duration) -> Self {
        Self {
            data,
            created_at: Instant::now(),
            ttl,
            access_count: 0,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    fn access(&mut self) -> &Bytes {
        self.access_count += 1;
        &self.data
    }
}

/// 高性能リクエストキャッシュ
#[derive(Debug)]
pub struct RequestCache {
    cache: DashMap<String, CacheEntry, FastHasher>,
    max_size: usize,
    default_ttl: Duration,
    stats: RwLock<CacheStats>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_requests: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_requests as f64
        }
    }
}

impl RequestCache {
    pub fn new() -> Self {
        Self::with_capacity(1000) // デフォルト1000エントリ
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            cache: DashMap::with_hasher(FastHasher::default()),
            max_size,
            default_ttl: Duration::from_secs(300), // 5分
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// キャッシュに値を設定
    pub fn set<K, V>(&self, key: K, value: V, ttl: Option<Duration>)
    where
        K: Into<String>,
        V: Serialize,
    {
        let key = key.into();
        let ttl = ttl.unwrap_or(self.default_ttl);
        
        if let Ok(serialized) = bincode::serialize(&value) {
            let entry = CacheEntry::new(Bytes::from(serialized), ttl);
            
            // キャッシュサイズ管理
            if self.cache.len() >= self.max_size {
                self.evict_lru();
            }
            
            self.cache.insert(key, entry);
        }
    }

    /// キャッシュから値を取得
    pub fn get<K, V>(&self, key: &K) -> Option<V>
    where
        K: AsRef<str>,
        V: for<'de> Deserialize<'de>,
    {
        let key_str = key.as_ref();
        
        // 統計更新
        {
            let mut stats = self.stats.write();
            stats.total_requests += 1;
        }

        if let Some(mut entry) = self.cache.get_mut(key_str) {
            if entry.is_expired() {
                drop(entry);
                self.cache.remove(key_str);
                self.record_miss();
                return None;
            }

            let data = entry.access().clone();
            self.record_hit();
            
            // デシリアライゼーション
            if let Ok(value) = bincode::deserialize(&data) {
                Some(value)
            } else {
                // 破損したデータを削除
                drop(entry);
                self.cache.remove(key_str);
                None
            }
        } else {
            self.record_miss();
            None
        }
    }

    /// キーが存在するかチェック
    pub fn contains_key<K>(&self, key: &K) -> bool
    where
        K: AsRef<str>,
    {
        if let Some(entry) = self.cache.get(key.as_ref()) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// キャッシュから削除
    pub fn remove<K>(&self, key: &K) -> bool
    where
        K: AsRef<str>,
    {
        self.cache.remove(key.as_ref()).is_some()
    }

    /// 期限切れエントリをクリーンアップ
    pub fn cleanup_expired(&self) {
        self.cache.retain(|_, entry| !entry.is_expired());
    }

    /// LRU方式で最も古いエントリを削除
    fn evict_lru(&self) {
        if let Some((oldest_key, _)) = self.cache
            .iter()
            .min_by_key(|entry| entry.value().created_at)
            .map(|entry| (entry.key().clone(), entry.value().clone()))
        {
            self.cache.remove(&oldest_key);
            let mut stats = self.stats.write();
            stats.evictions += 1;
        }
    }

    fn record_hit(&self) {
        let mut stats = self.stats.write();
        stats.hits += 1;
    }

    fn record_miss(&self) {
        let mut stats = self.stats.write();
        stats.misses += 1;
    }

    /// キャッシュ統計を取得
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// キャッシュをクリア
    pub fn clear(&self) {
        self.cache.clear();
        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }
    
    /// ユーザーキャッシュに挿入（auth_v3用）
    pub async fn insert_user_cache(&self, email: &str, user: &crate::models::user::User) {
        let cache_key = format!("user:{}", email);
        self.set(cache_key, user, None);
    }
    
    /// ユーザーキャッシュから取得（auth_v3用）
    pub async fn get_user_cache(&self, email: &str) -> Option<crate::models::user::User> {
        let cache_key = format!("user:{}", email);
        self.get(&cache_key)
    }

    /// キャッシュサイズ
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// キャッシュが空かどうか
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// 投稿キャッシュメソッド
    pub async fn get_post_cache(&self, post_id: &str) -> Option<crate::models::post::Post> {
        let key = format!("post:{}", post_id);
        self.get(&key)
    }
    
    pub async fn insert_post_cache(&self, post_id: &str, post: &crate::models::post::Post) {
        let key = format!("post:{}", post_id);
        self.set(&key, post, Some(Duration::from_secs(600))); // 10分間キャッシュ
    }
}

/// キャッシュキー生成ヘルパー
pub fn cache_key_for_user_posts(user_id: &str, page: u32, limit: u32) -> String {
    format!("user:{}:posts:{}:{}", user_id, page, limit)
}

pub fn cache_key_for_post(post_id: &str) -> String {
    format!("post:{}", post_id)
}

pub fn cache_key_for_user_profile(user_id: &str) -> String {
    format!("user:{}:profile", user_id)
}

/// キャッシュ無効化ヘルパー
pub fn invalidate_user_cache(cache: &RequestCache, user_id: &str) {
    // ユーザー関連のキャッシュを無効化
    let pattern = format!("user:{}:", user_id);
    
    let keys_to_remove: Vec<String> = cache.cache
        .iter()
        .filter(|entry| entry.key().starts_with(&pattern))
        .map(|entry| entry.key().clone())
        .collect();
    
    for key in keys_to_remove {
        cache.remove(&key);
    }
}

pub fn invalidate_post_cache(cache: &RequestCache, post_id: &str) {
    cache.remove(&cache_key_for_post(post_id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestData {
        id: u32,
        name: String,
    }

    #[test]
    fn test_cache_basic_operations() {
        let cache = RequestCache::new();
        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };

        // Set and get
        cache.set("test_key", &data, None);
        let retrieved: Option<TestData> = cache.get(&"test_key");
        assert_eq!(retrieved, Some(data));

        // Key existence
        assert!(cache.contains_key(&"test_key"));
        assert!(!cache.contains_key(&"nonexistent"));

        // Remove
        assert!(cache.remove(&"test_key"));
        assert!(!cache.contains_key(&"test_key"));
    }

    #[test]
    fn test_cache_ttl() {
        let cache = RequestCache::new();
        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };

        // Set with very short TTL
        cache.set("test_key", &data, Some(Duration::from_millis(1)));
        
        // Wait for expiry
        std::thread::sleep(Duration::from_millis(10));
        
        let retrieved: Option<TestData> = cache.get(&"test_key");
        assert_eq!(retrieved, None);
    }
}
