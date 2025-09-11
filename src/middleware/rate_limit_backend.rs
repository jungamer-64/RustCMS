#[cfg(feature = "monitoring")]
use metrics::gauge;
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
#[cfg(feature = "cache")]
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Once;
use std::time::{Duration, Instant};
use tokio::time::interval;

/// Trait abstraction for API Key failure-based rate limiting backends.
/// record_failure() returns true if the caller should now be rate limited (i.e. block request).
pub trait ApiKeyRateLimiter: Send + Sync + 'static {
    fn record_failure(&self, key: &str) -> bool;
    fn clear(&self, key: &str);
    fn tracked_len(&self) -> usize;
}

/// In-memory fixed-window implementation (process local).
pub struct InMemoryRateLimiter {
    window: Duration,
    threshold: u32,
    max_tracked: usize,
    disabled: bool,
    // map: lookup_hash => (fail_count, window_start)
    map: Mutex<HashMap<String, (u32, Instant)>>,
    // background cleanup spawn guard
    cleaner_started: Once,
}

impl InMemoryRateLimiter {
    pub fn from_env() -> Self {
        let window = std::env::var("API_KEY_FAIL_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60);
        let threshold = std::env::var("API_KEY_FAIL_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);
        let max_tracked = std::env::var("API_KEY_FAIL_MAX_TRACKED")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5000);
        let disabled = std::env::var("API_KEY_FAIL_DISABLE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let inst = Self {
            window: Duration::from_secs(window),
            threshold,
            max_tracked,
            disabled,
            map: Mutex::new(HashMap::new()),
            cleaner_started: Once::new(),
        };
        #[cfg(feature = "monitoring")]
        {
            gauge!("api_key_rate_limit_window_seconds").set(window as f64);
            gauge!("api_key_rate_limit_threshold").set(threshold as f64);
            gauge!("api_key_rate_limit_max_tracked").set(max_tracked as f64);
            gauge!("api_key_rate_limit_enabled").set(if disabled { 0.0 } else { 1.0 });
        }
        inst
    }

    fn spawn_background_cleaner(&self) {
        // Launch a lightweight periodic cleanup task only once.
        self.cleaner_started.call_once(|| {
            let win = self.window;
            tokio::spawn(async move {
                let mut ticker = interval(win.min(Duration::from_secs(30))); // at most every 30s or window
                loop {
                    ticker.tick().await;
                    let this = &*GLOBAL_RATE_LIMITER; // static reference
                    if this.disabled {
                        continue;
                    }
                    let now = Instant::now();
                    let mut map = this.map.lock();
                    if map.is_empty() {
                        continue;
                    }
                    let win = this.window;
                    let _before = map.len(); // monitoring only
                    map.retain(|_, (_c, ts)| now.duration_since(*ts) <= win);
                    // If still over max_tracked (pathological), drop oldest excess
                    if map.len() > this.max_tracked {
                        let mut entries: Vec<_> =
                            map.iter().map(|(k, (_c, ts))| (k.clone(), *ts)).collect();
                        entries.sort_by_key(|(_, ts)| *ts); // oldest first
                        for (k, _) in entries.into_iter().take(map.len() - this.max_tracked) {
                            map.remove(&k);
                        }
                    }
                    #[cfg(feature = "monitoring")]
                    {
                        if _before != map.len() {
                            gauge!("api_key_rate_limit_tracked_keys").set(map.len() as f64);
                        }
                    }
                }
            });
        });
    }
}

impl ApiKeyRateLimiter for InMemoryRateLimiter {
    fn record_failure(&self, key: &str) -> bool {
        if self.disabled {
            return false;
        }
        // lazily start background cleaner
        self.spawn_background_cleaner();
        let mut map = self.map.lock();
        let now = Instant::now();
        // opportunistic cleanup
        if map.len() > (self.max_tracked * 9 / 10) {
            let win = self.window;
            map.retain(|_, (_c, ts)| now.duration_since(*ts) <= win);
        }
        if map.len() >= self.max_tracked {
            // drop oldest
            if let Some(old) = map
                .iter()
                .min_by_key(|(_, (_c, ts))| *ts)
                .map(|(k, _)| k.clone())
            {
                map.remove(&old);
            }
        }
        let entry = map.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) > self.window {
            entry.0 = 0;
            entry.1 = now;
        }
        entry.0 += 1;
        entry.0 > self.threshold
    }
    fn clear(&self, key: &str) {
        let mut map = self.map.lock();
        map.remove(key);
    }
    fn tracked_len(&self) -> usize {
        self.map.lock().len()
    }
}

pub static GLOBAL_RATE_LIMITER: Lazy<InMemoryRateLimiter> =
    Lazy::new(InMemoryRateLimiter::from_env);

#[cfg(feature = "cache")]
static REDIS_RATE_LIMITER: OnceCell<RedisRateLimiter> = OnceCell::new();

/// Obtain the active rate limiter (memory default, redis if opted in).
pub fn get_rate_limiter() -> &'static dyn ApiKeyRateLimiter {
    // Fast path: check backend selection env (cached after first read via static)
    static BACKEND: Lazy<String> =
        Lazy::new(|| std::env::var("API_KEY_FAIL_BACKEND").unwrap_or_else(|_| "memory".into()));
    if BACKEND.as_str() == "redis" {
        #[cfg(feature = "cache")]
        {
            return REDIS_RATE_LIMITER
                .get_or_init(RedisRateLimiter::from_env)
                .as_dyn();
        }
        // If redis requested but not compiled with cache, fall back to memory
    }
    &*GLOBAL_RATE_LIMITER
}

// ---------------- Redis backend (optional) -----------------
#[cfg(feature = "cache")]
pub struct RedisRateLimiter {
    client: redis::Client,
    window: Duration,
    threshold: u32,
    disabled: bool,
    key_prefix: String,
    // last scan for tracked_len (timestamp, cached_len)
    tracked_cache: Mutex<(Instant, usize)>,
}

#[cfg(feature = "cache")]
impl RedisRateLimiter {
    fn from_env() -> Self {
        let url = match std::env::var("REDIS_URL") {
            Ok(v) => v,
            Err(_) => {
                // Fallback to disabled limiter with dummy client; avoid panic to improve robustness
                tracing::warn!("REDIS_URL not set; Redis rate limiter will be disabled");
                // Use localhost URL that will fail to connect but keep object constructed
                "redis://127.0.0.1/".to_string()
            }
        };
        let window = std::env::var("API_KEY_FAIL_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60);
        let threshold = std::env::var("API_KEY_FAIL_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);
        let mut disabled = std::env::var("API_KEY_FAIL_DISABLE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let key_prefix =
            std::env::var("API_KEY_FAIL_REDIS_PREFIX").unwrap_or_else(|_| "rk:".into());
        #[cfg(feature = "monitoring")]
        {
            gauge!("api_key_rate_limit_window_seconds").set(window as f64);
            gauge!("api_key_rate_limit_threshold").set(threshold as f64);
            gauge!("api_key_rate_limit_enabled").set(if disabled { 0.0 } else { 1.0 });
        }
        let client = match redis::Client::open(url) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to create Redis client for rate limiter; disabling limiter");
                // Construct a dummy client with localhost; subsequent ops will no-op via disabled flag
                disabled = true;
                // best-effort fallback; unwrap is safe as URL literal is valid
                redis::Client::open("redis://127.0.0.1/").unwrap()
            }
        };
        Self {
            client,
            window: Duration::from_secs(window),
            threshold,
            disabled,
            key_prefix,
            tracked_cache: Mutex::new((Instant::now(), 0)),
        }
    }
    fn key(&self, k: &str) -> String {
        format!("{prefix}{k}", prefix = self.key_prefix, k = k)
    }
    fn as_dyn(&self) -> &dyn ApiKeyRateLimiter {
        self
    }
}

#[cfg(feature = "cache")]
impl ApiKeyRateLimiter for RedisRateLimiter {
    fn record_failure(&self, key: &str) -> bool {
        if self.disabled {
            return false;
        }
        let k = self.key(key);
        let window_secs = self.window.as_secs() as usize;
        // Redis atomic pattern: INCR then set EXPIRE if first
        // Using block_on not acceptable; function is sync. We use a dedicated runtime handle via tokio::runtime context.
        // SAFETY: This function called within async context (middleware) but trait signature is sync; we use block_in_place.
        let threshold = self.threshold;
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                if let Ok(mut conn) = self.client.get_multiplexed_async_connection().await {
                    let cnt: i64 = match conn.incr(&k, 1).await {
                        Ok(v) => v,
                        Err(_) => return false,
                    }; // on error do not block
                    if cnt == 1 {
                        let _: Result<(), _> = conn.expire(&k, window_secs as i64).await;
                    }
                    return cnt as u32 > threshold;
                }
                false
            })
        })
    }
    fn clear(&self, key: &str) {
        let k = self.key(key);
    tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                if let Ok(mut conn) = self.client.get_multiplexed_async_connection().await {
                    let _: Result<(), _> = conn.del(&k).await;
                }
            })
    });
    }
    fn tracked_len(&self) -> usize {
        let mut guard = self.tracked_cache.lock();
        let (last, cached) = *guard;
        if last.elapsed() < Duration::from_secs(15) {
            return cached;
        }
        // refresh
        let new_len = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                if let Ok(mut conn) = self.client.get_multiplexed_async_connection().await {
                    let mut cursor: u64 = 0;
                    let mut total = 0usize;
                    loop {
                                    let pattern = format!("{prefix}*", prefix = self.key_prefix);
                        let res: redis::RedisResult<(u64, Vec<String>)> = redis::cmd("SCAN")
                            .arg(cursor)
                            .arg("MATCH")
                            .arg(&pattern)
                            .arg("COUNT")
                            .arg(100usize)
                            .query_async(&mut conn)
                            .await;
                        match res {
                            Ok((next, keys)) => {
                                total += keys.len();
                                if next == 0 {
                                    break total;
                                } else {
                                    cursor = next;
                                }
                            }
                            Err(_) => break 0,
                        };
                    }
                } else {
                    0
                }
            })
        });
        *guard = (Instant::now(), new_len);
        new_len
    }
}
