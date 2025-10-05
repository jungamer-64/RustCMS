// src/cache/mod.rs
//! Cache Service - Redis + In-memory caching
//!
//! Provides high-performance caching with:
//! - Redis for distributed caching
//! - In-memory cache for ultra-fast access
//! - Automatic cache invalidation
//! - Cache warming and prefetching
//! - Cache statistics and monitoring

use redis::{AsyncCommands, Client};
use serde::{Serialize, de::DeserializeOwned};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::{Result, config::RedisConfig};

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Cache miss for key: {0}")]
    Miss(String),
    #[error("Cache connection error")]
    Connection,
    #[error("Invalid TTL: {0}")]
    InvalidTtl(u64),
}

impl From<CacheError> for crate::AppError {
    fn from(err: CacheError) -> Self {
        Self::Internal(err.to_string())
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    created_at: Instant,
    expires_at: Option<Instant>,
    hits: u64,
}

/// Cache statistics
#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub redis_hits: u64,
    pub redis_misses: u64,
    pub memory_hits: u64,
    pub memory_misses: u64,
    pub total_operations: u64,
    pub cache_size: usize,
    pub hit_ratio: f64,
}

/// Cache service with Redis and in-memory tiers
#[derive(Clone)]
pub struct CacheService {
    /// Redis client for distributed caching
    redis_client: Client,
    /// In-memory cache for ultra-fast access
    memory_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<u8>>>>>,
    /// Configuration
    config: RedisConfig,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Maximum memory cache size
    max_memory_size: usize,
}

impl CacheService {
    /// Create new cache service
    ///
    /// # Errors
    /// - Redis への接続確立に失敗した場合。
    /// - シリアライズ/デシリアライズに失敗した場合。
    pub async fn new(config: &RedisConfig) -> Result<Self> {
        let redis_client = Client::open(config.url.as_str())?;

        // Test connection (use multiplexed connection API)
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        let _: String = conn.set("test", "test").await?;
        let _: bool = conn.del("test").await?;

        Ok(Self {
            redis_client,
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_memory_size: 1000, // Max 1000 items in memory
        })
    }

    /// Set value in cache with TTL
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    /// - 値のシリアライズに失敗した場合。
    pub async fn set<T>(&self, key: String, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Sync,
    {
        let serialized = serde_json::to_vec(value)?;
        let prefix = &self.config.key_prefix;
        let full_key = format!("{prefix}{key}");

        // Set in Redis (multiplexed)
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        if let Some(ttl) = ttl {
            let secs = ttl.as_secs();
            let _: () = conn.set_ex(&full_key, &serialized, secs).await?;
        } else {
            let _: () = conn.set(&full_key, &serialized).await?;
        }

        // Set in memory cache
        self.memory_insert(full_key, serialized, ttl).await;

        // Update stats (limit guard lifetime)
        {
            let mut stats = self.stats.write().await;
            stats.total_operations += 1;
        }

        Ok(())
    }

    #[inline]
    async fn memory_insert(&self, full_key: String, data: Vec<u8>, ttl: Option<Duration>) {
        let expires_at = ttl.map(|t| Instant::now() + t);
        let entry = CacheEntry {
            value: data,
            created_at: Instant::now(),
            expires_at,
            hits: 0,
        };

        let mut memory_cache = self.memory_cache.write().await;
        if memory_cache.len() >= self.max_memory_size {
            self.evict_lru(&mut memory_cache).await;
        }
        memory_cache.insert(full_key, entry);
    }

    /// Get value from cache
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    /// - 取得したデータのデシリアライズに失敗した場合。
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let prefix = &self.config.key_prefix;
        let full_key = format!("{prefix}{key}");

        // Try memory cache first
        let mut bytes: Option<Vec<u8>> = None;
        #[allow(clippy::significant_drop_tightening)]
        {
            let mut memory_cache = self.memory_cache.write().await;
            if let Some(entry) = memory_cache.get_mut(&full_key) {
                // Check if expired
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() > expires_at {
                        memory_cache.remove(&full_key);
                        drop(memory_cache);
                    } else {
                        // Cache hit: clone data and increment hit counter under lock
                        entry.hits += 1;
                        bytes = Some(entry.value.clone());
                    }
                } else {
                    // No expiration, cache hit
                    entry.hits += 1;
                    bytes = Some(entry.value.clone());
                }
            }
        }
        if let Some(bytes) = bytes {
            let value: T = serde_json::from_slice(&bytes)?;
            self.record_memory_hit().await;
            return Ok(Some(value));
        }
        self.record_memory_miss().await;

        // Try Redis cache
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let result: Option<Vec<u8>> = conn.get(&full_key).await?;

        if let Some(data) = result {
            let value: T = serde_json::from_slice(&data)?;

            // Store in memory cache for next time
            self.memory_insert(full_key, data, None).await;

            // Update stats
            self.record_redis_hit().await;

            Ok(Some(value))
        } else {
            // Cache miss
            self.record_redis_miss().await;

            Ok(None)
        }
    }

    /// Delete value from cache
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    pub async fn delete(&self, key: &str) -> Result<()> {
        let prefix = &self.config.key_prefix;
        let full_key = format!("{prefix}{key}");

        // Remove from Redis
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let _: () = conn.del(&full_key).await?;

        // Remove from memory cache
        self.memory_cache.write().await.remove(&full_key);

        Ok(())
    }

    /// Check if key exists in cache
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let prefix = &self.config.key_prefix;
        let full_key = format!("{prefix}{key}");

        // Check memory cache first
        {
            let memory_cache = self.memory_cache.read().await;
            if let Some(entry) = memory_cache.get(&full_key) {
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() <= expires_at {
                        return Ok(true);
                    }
                } else {
                    return Ok(true);
                }
            }
        }

        // Check Redis
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let exists: bool = conn.exists(&full_key).await?;

        Ok(exists)
    }

    /// Set TTL for existing key
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    pub async fn expire(&self, key: &str, ttl: Duration) -> Result<()> {
        let full_key = format!("{}{key}", self.config.key_prefix);

        // Set TTL in Redis
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let secs = ttl.as_secs();
        let secs_i64 = i64::try_from(secs).map_err(|_| CacheError::InvalidTtl(secs))?;
        let _: () = conn.expire(&full_key, secs_i64).await?;

        // Update memory cache entry
        {
            let mut memory_cache = self.memory_cache.write().await;
            if let Some(entry) = memory_cache.get_mut(&full_key) {
                entry.expires_at = Some(Instant::now() + ttl);
            }
        }

        Ok(())
    }

    /// Clear all cache entries
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    pub async fn clear(&self) -> Result<()> {
        // Clear Redis with pattern
        let prefix = &self.config.key_prefix;
        let pattern = format!("{prefix}*");
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let keys: Vec<String> = conn.keys(&pattern).await?;
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await?;
        }

        // Clear memory cache
        self.memory_cache.write().await.clear();

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        // Read stats under short-lived lock
        let (redis_hits, redis_misses, memory_hits, memory_misses, total_operations) = {
            let stats = self.stats.read().await;
            (
                stats.redis_hits,
                stats.redis_misses,
                stats.memory_hits,
                stats.memory_misses,
                stats.total_operations,
            )
        };
        let cache_size = self.memory_cache.read().await.len();

        let total_hits = redis_hits + memory_hits;
        let total_misses = redis_misses + memory_misses;
        let denom = total_hits + total_misses;
        // f64 precision is acceptable for ratio in [0,1]; convert explicitly
        #[allow(clippy::cast_precision_loss)]
        let hit_ratio = if denom > 0 {
            (total_hits as f64) / (denom as f64)
        } else {
            0.0
        };

        CacheStats {
            redis_hits,
            redis_misses,
            memory_hits,
            memory_misses,
            total_operations,
            cache_size,
            hit_ratio,
        }
    }

    /// Health check
    ///
    /// # Errors
    /// - Redis との通信に失敗した場合。
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let _: String = conn.set("health_check", "ok").await?;
        let _: bool = conn.del("health_check").await?;
        Ok(())
    }

    /// Evict least recently used entries from memory cache
    #[allow(clippy::unused_async)]
    async fn evict_lru(&self, memory_cache: &mut HashMap<String, CacheEntry<Vec<u8>>>) {
        if memory_cache.is_empty() {
            return;
        }

        // Find entry with lowest hit count and oldest creation time
        let mut lru_key = String::new();
        let mut min_score = f64::MAX;

        for (key, entry) in memory_cache.iter() {
            // Score based on hits and age (lower is more likely to be evicted)
            let age_seconds = entry.created_at.elapsed().as_secs_f64();
            #[allow(clippy::cast_precision_loss)]
            let score = (entry.hits as f64) / (age_seconds + 1.0);

            if score < min_score {
                min_score = score;
                lru_key = key.clone();
            }
        }

        if !lru_key.is_empty() {
            memory_cache.remove(&lru_key);
        }
    }

    /// Warm cache with frequently accessed data
    ///
    /// # Errors
    /// - 内部処理でエラーが発生した場合。
    #[allow(clippy::unused_async)]
    pub async fn warm_cache(&self, keys: Vec<String>) -> Result<()> {
        for key in keys {
            // Try to load from database or other source
            // This is a placeholder - implement according to your needs
            tracing::info!("Warming cache for key: {key}");
        }
        Ok(())
    }

    /// Get cache memory usage
    pub async fn get_memory_usage(&self) -> usize {
        let memory_cache = self.memory_cache.read().await;
        memory_cache.len() * std::mem::size_of::<CacheEntry<Vec<u8>>>()
    }
}

impl CacheService {
    #[inline]
    async fn record_memory_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.memory_misses += 1;
        stats.total_operations += 1;
    }

    #[inline]
    async fn record_redis_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.redis_hits += 1;
    }

    #[inline]
    async fn record_redis_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.redis_misses += 1;
    }

    #[inline]
    async fn record_memory_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.memory_hits += 1;
        stats.total_operations += 1;
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            redis_hits: 0,
            redis_misses: 0,
            memory_hits: 0,
            memory_misses: 0,
            total_operations: 0,
            cache_size: 0,
            hit_ratio: 0.0,
        }
    }
}

/// Cache key generation utilities
pub struct CacheKey;

impl CacheKey {
    #[must_use]
    pub fn user(id: &str) -> String {
        format!("user:{id}")
    }

    #[must_use]
    pub fn post(id: &str) -> String {
        format!("post:{id}")
    }

    #[must_use]
    pub fn session(id: &str) -> String {
        format!("session:{id}")
    }

    #[must_use]
    pub fn search_results(query: &str, page: u32) -> String {
        format!("search:{query}:{page}")
    }

    #[must_use]
    pub fn api_response(endpoint: &str, params: &str) -> String {
        format!("api:{endpoint}:{params}")
    }
}
