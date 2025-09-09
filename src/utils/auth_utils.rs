//! Authentication utilities
//!
//! Common authentication helper functions

#![allow(deprecated)] // internal deprecations intentionally retained under feature gate

#[cfg(feature = "legacy-admin-token")]
use std::env;

/// Check if the provided admin token is valid
/// This is a simple token-based authentication for admin endpoints
/// 
/// # Deprecated
/// This function is deprecated in favor of Biscuit-based authentication with role permissions.
/// Use the unified authentication system with "admin" permission checking instead.
#[cfg(feature = "legacy-admin-token")]
#[deprecated(note = "Use Biscuit authentication with admin permission checking instead (will be removed in 3.0.0)")]
pub fn check_admin_token(req_token: &str) -> bool {
    env::var("ADMIN_TOKEN")
        .map(|t| t == req_token)
        .unwrap_or(false)
}

/// Get the admin token from environment
/// 
/// # Deprecated
/// This function is deprecated in favor of Biscuit-based authentication with role permissions.
/// Use the unified authentication system with "admin" permission checking instead.
#[cfg(feature = "legacy-admin-token")]
#[deprecated(note = "Use Biscuit authentication with admin permission checking instead (will be removed in 3.0.0)")]
pub fn get_admin_token() -> Option<String> {
    env::var("ADMIN_TOKEN").ok()
}

#[cfg(all(test, feature = "legacy-admin-token"))]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Mutex to ensure tests run sequentially to avoid env var conflicts
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[cfg(feature = "legacy-admin-token")]
    #[test]
    fn test_check_admin_token() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // Save original value
        let original = env::var("ADMIN_TOKEN").ok();

        // Test with no environment variable
        env::remove_var("ADMIN_TOKEN");
        assert!(!check_admin_token("any_token"));

        // Test with matching token
        env::set_var("ADMIN_TOKEN", "test_token");
        assert!(check_admin_token("test_token"));

        // Test with non-matching token
        assert!(!check_admin_token("wrong_token"));

        // Restore original value
        match original {
            Some(val) => env::set_var("ADMIN_TOKEN", val),
            None => env::remove_var("ADMIN_TOKEN"),
        }
    }

    #[cfg(feature = "legacy-admin-token")]
    #[test]
    fn test_check_admin_token_empty() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // Save original value
        let original = env::var("ADMIN_TOKEN").ok();

        // Test with empty token in environment
        env::set_var("ADMIN_TOKEN", "");
        assert!(check_admin_token(""));
        assert!(!check_admin_token("not_empty"));

        // Restore original value
        match original {
            Some(val) => env::set_var("ADMIN_TOKEN", val),
            None => env::remove_var("ADMIN_TOKEN"),
        }
    }

    #[cfg(feature = "legacy-admin-token")]
    #[test]
    fn test_get_admin_token() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // Save original value
        let original = env::var("ADMIN_TOKEN").ok();

        // Test with no environment variable
        env::remove_var("ADMIN_TOKEN");
        assert!(get_admin_token().is_none());

        // Test with set token
        env::set_var("ADMIN_TOKEN", "test_admin_token");
        assert_eq!(get_admin_token(), Some("test_admin_token".to_string()));

        // Test with empty token
        env::set_var("ADMIN_TOKEN", "");
        assert_eq!(get_admin_token(), Some("".to_string()));

        // Restore original value
        match original {
            Some(val) => env::set_var("ADMIN_TOKEN", val),
            None => env::remove_var("ADMIN_TOKEN"),
        }
    }

    #[cfg(feature = "legacy-admin-token")]
    #[test]
    fn test_check_admin_token_special_characters() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // Save original value
        let original = env::var("ADMIN_TOKEN").ok();

        let special_token = "token!@#$%^&*(){}[]|\\:;\"'<>,.?/~`";
        env::set_var("ADMIN_TOKEN", special_token);

        assert!(check_admin_token(special_token));
        assert!(!check_admin_token("different_token"));

        // Restore original value
        match original {
            Some(val) => env::set_var("ADMIN_TOKEN", val),
            None => env::remove_var("ADMIN_TOKEN"),
        }
    }

    #[cfg(feature = "legacy-admin-token")]
    #[test]
    fn test_check_admin_token_unicode() {
        let _lock = TEST_MUTEX.lock().unwrap();

        // Save original value
        let original = env::var("ADMIN_TOKEN").ok();

        let unicode_token = "トークン123🔑";
        env::set_var("ADMIN_TOKEN", unicode_token);

        assert!(check_admin_token(unicode_token));
        assert!(!check_admin_token("regular_token"));

        // Restore original value
        match original {
            Some(val) => env::set_var("ADMIN_TOKEN", val),
            None => env::remove_var("ADMIN_TOKEN"),
        }
    }
}
