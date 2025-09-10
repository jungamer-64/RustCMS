//! Tests for unified rate limiting components
use cms_backend::limiter::{FixedWindowLimiter, GenericRateLimiter, RateLimitDecision};
use std::thread::sleep;
use std::time::Duration;

#[test]
fn fixed_window_allows_within_limit() {
    let limiter = FixedWindowLimiter::new(3, 60); // 3 per 60s
    assert!(limiter.allow("ip1"));
    assert!(limiter.allow("ip1"));
    assert!(limiter.allow("ip1"));
    // 4th should fail
    assert!(!limiter.allow("ip1"));
}

#[test]
fn fixed_window_resets_after_window() {
    let limiter = FixedWindowLimiter::new(2, 1); // 2 per 1s
    assert!(limiter.allow("k"));
    assert!(limiter.allow("k"));
    assert!(!limiter.allow("k")); // blocked now
    sleep(Duration::from_millis(1050));
    // window elapsed, should allow again
    assert!(limiter.allow("k"));
}

#[test]
fn fixed_window_tracks_multiple_keys() {
    let limiter = FixedWindowLimiter::new(1, 60);
    assert!(limiter.allow("a"));
    assert!(limiter.allow("b"));
    assert!(!limiter.allow("a")); // a exceeded
    assert!(!limiter.allow("b")); // b exceeded
    assert!(limiter.tracked_len() >= 2);
}

#[cfg(feature = "auth")]
mod api_key_adapter_tests {
    use super::*;
    use cms_backend::limiter::adapters::ApiKeyFailureLimiterAdapter;

    #[tokio::test]
    async fn adapter_blocks_after_threshold() {
        // Ensure env threshold determinism; backend reads on construction.
        unsafe {
            std::env::set_var("API_KEY_FAIL_WINDOW_SECS", "60");
            std::env::set_var("API_KEY_FAIL_THRESHOLD", "2"); // block after >2 failures
        }
        let adapter = ApiKeyFailureLimiterAdapter::from_env();
        // First 3 checks correspond to 3 failures (record_failure increments internally)
        assert_eq!(adapter.check("k1"), RateLimitDecision::Allowed);
        assert_eq!(adapter.check("k1"), RateLimitDecision::Allowed);
        match adapter.check("k1") {
            RateLimitDecision::Blocked { .. } => {}
            _ => panic!("expected blocked"),
        }
        // Clear and ensure it allows again
        adapter.clear("k1");
        assert_eq!(adapter.check("k1"), RateLimitDecision::Allowed);
    }
}
