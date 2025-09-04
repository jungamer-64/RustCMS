//! 認証システム用メトリクス
//! 
//! auth_v3.rsから分離して循環参照を避ける

use std::sync::atomic::AtomicU64;
use std::time::Instant;

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

// NOTE: Legacy RateLimiter removed (now unified via AppState.rate_limiter and middleware).
