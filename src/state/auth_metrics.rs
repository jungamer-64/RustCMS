//! 認証システム用メトリクス
//! 
//! auth_v3.rsから分離して循環参照を避ける

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use dashmap::DashMap;

/// 高性能認証メトリクス
#[derive(Default, Debug)]
pub struct AuthMetrics {
    pub login_attempts: AtomicU64,
    pub login_successes: AtomicU64,
    pub token_validations: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

/// 認証セッション（並行アクセス対応）
#[derive(Clone, Debug)]
pub struct AuthSession {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: Instant,
    pub last_access: Instant,
    pub ip_address: String,
    pub user_agent: String,
}

/// レート制限（ロックフリー）
#[derive(Default, Debug)]
pub struct RateLimiter {
    attempts: DashMap<String, (AtomicU64, Instant)>,
}

impl RateLimiter {
    pub fn check_rate_limit(&self, ip: &str, max_attempts: u64, window: Duration) -> bool {
        let now = Instant::now();
        
        self.attempts
            .entry(ip.to_string())
            .and_modify(|(count, last_reset)| {
                if now.duration_since(*last_reset) > window {
                    count.store(1, Ordering::Relaxed);
                    *last_reset = now;
                } else {
                    count.fetch_add(1, Ordering::Relaxed);
                }
            })
            .or_insert_with(|| (AtomicU64::new(1), now));

        self.attempts
            .get(ip)
            .map(|entry| entry.0.load(Ordering::Relaxed) <= max_attempts)
            .unwrap_or(true)
    }
}
