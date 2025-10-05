//! Unified rate limiting service (fixed window)
//!
//! Consolidates scattered IP based rate limiting logic into a single
//! implementation owned by `AppState` so that all middleware / handlers
//! share consistent counters and configuration.
//!
//! Strategy: simple fixed window (reset after window elapses). For most
//! CMS style APIs this is sufficient; can be extended to sliding window
//! or token bucket later (trait extraction point noted below).
//!
//! Optional future extension ideas:
//! - Redis backend (feature = "cache") for multi-instance deployments
//! - Token bucket or leaky bucket variants
//! - Per-route / per-API-key overrides
//!
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Core in-memory fixed window rate limiter.
#[derive(Debug)]
pub struct FixedWindowLimiter {
    max_requests: u32,
    window: Duration,
    // key => (count, window_start)
    entries: Mutex<HashMap<String, (u32, Instant)>>,
}

impl FixedWindowLimiter {
    #[must_use]
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs.max(1)),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if the action is allowed (i.e., still under limit).
    pub fn allow(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.entries.lock();

        // Opportunistic cleanup of expired windows
        if map.len() > 1024 {
            // only bother when moderately large
            let win = self.window;
            map.retain(|_, (_c, start)| now.duration_since(*start) < win);
        }

        let entry = map.entry(key.to_string()).or_insert_with(|| (0, now));
        if now.duration_since(entry.1) >= self.window {
            entry.0 = 0;
            entry.1 = now;
        }
        entry.0 += 1;
        let allowed = entry.0 <= self.max_requests;
        // drop the lock before returning to tighten guard lifetime
        drop(map);
        allowed
    }

    pub fn tracked_len(&self) -> usize {
        self.entries.lock().len()
    }
    pub const fn window_secs(&self) -> u64 {
        self.window.as_secs()
    }
    pub const fn max_requests(&self) -> u32 {
        self.max_requests
    }
}

/// 汎用レートリミット判定結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed,
    Blocked { retry_after_secs: u64 },
}

/// 統一インターフェイス: IP / APIキー など複数ディメンションのレートリミット用
pub trait GenericRateLimiter: Send + Sync {
    fn check(&self, key: &str) -> RateLimitDecision;
    fn clear(&self, _key: &str) {}
    fn tracked_len(&self) -> usize {
        0
    }
    fn name(&self) -> &'static str {
        "generic"
    }
}

impl GenericRateLimiter for FixedWindowLimiter {
    fn check(&self, key: &str) -> RateLimitDecision {
        if self.allow(key) {
            RateLimitDecision::Allowed
        } else {
            RateLimitDecision::Blocked {
                retry_after_secs: self.window_secs(),
            }
        }
    }
    fn tracked_len(&self) -> usize {
        self.tracked_len()
    }
    fn name(&self) -> &'static str {
        "ip_fixed_window"
    }
}

// ---- APIキー失敗レートリミット用アダプタ (現在の実装は failure カウントベース) ----
#[cfg(feature = "auth")]
pub mod adapters {
    use super::{GenericRateLimiter, RateLimitDecision};
    use crate::middleware::rate_limit_backend::get_rate_limiter;

    /// APIキー失敗回数レート制限を `GenericRateLimiter` に適合させる薄いラッパ。
    /// 既存 backend は `record_failure()` 呼び出しでカウンタを +1 し、閾値超過で true を返すため
    /// `check()` 内で increment + 判定をまとめて行う。
    pub struct ApiKeyFailureLimiterAdapter {
        window_secs: u64,
    }

    impl ApiKeyFailureLimiterAdapter {
        /// Creates a new adapter with explicit window duration.
        #[must_use]
        pub fn new(window_secs: u64) -> Self {
            Self {
                window_secs: window_secs.max(1),
            }
        }

        #[must_use]
        pub fn from_env() -> Self {
            let window = std::env::var("API_KEY_FAIL_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);
            Self {
                window_secs: window.max(1),
            }
        }
    }

    impl GenericRateLimiter for ApiKeyFailureLimiterAdapter {
        fn check(&self, key: &str) -> RateLimitDecision {
            let backend = get_rate_limiter();
            if backend.record_failure(key) {
                RateLimitDecision::Blocked {
                    retry_after_secs: self.window_secs,
                }
            } else {
                RateLimitDecision::Allowed
            }
        }
        fn clear(&self, key: &str) {
            get_rate_limiter().clear(key);
        }
        fn tracked_len(&self) -> usize {
            get_rate_limiter().tracked_len()
        }
        fn name(&self) -> &'static str {
            "api_key_failure_window"
        }
    }
}
