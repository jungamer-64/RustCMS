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

    const TEST_API_KEY: &str = "test_key_1";

    #[tokio::test]
    #[serial_test::serial]
    async fn adapter_blocks_after_threshold() {
        // Configure adapter with explicit window time for testing.
        // Note: The actual threshold is controlled by the global rate limiter backend,
        // which reads API_KEY_FAIL_THRESHOLD from environment at initialization.
        // This test relies on the default threshold of 10 (blocks when count > 10).
        let adapter = ApiKeyFailureLimiterAdapter::new(60);

        // Record failures repeatedly to reach threshold (default is 10)
        // The implementation blocks when failure_count > threshold, so we need 10 checks
        // to reach count=10, and the 11th will be blocked.
        for _ in 0..10 {
            assert_eq!(adapter.check(TEST_API_KEY), RateLimitDecision::Allowed);
        }

        // 11th check should trigger the block (count becomes 11, which is > 10)
        match adapter.check(TEST_API_KEY) {
            RateLimitDecision::Blocked { .. } => {}
            RateLimitDecision::Allowed => panic!("expected blocked after threshold"),
        }

        // Clear and ensure it allows again
        adapter.clear(TEST_API_KEY);
        assert_eq!(adapter.check(TEST_API_KEY), RateLimitDecision::Allowed);
    }
}
