//! Password hashing utilities
//!
//! Provides secure password hashing using Argon2 (recommended standard)
//! Replaces multiple hash implementations across the project

use crate::AppError;
use crate::Result;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

/// Hash a password using Argon2 (secure, recommended)
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))?;

    Ok(password_hash.to_string())
}

/// Verify a password against its hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash format: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Hash should succeed");

        // Verify correct password
        assert!(verify_password(password, &hash).expect("Verification should succeed"));

        // Verify incorrect password
        assert!(!verify_password("wrong_password", &hash).expect("Verification should succeed"));
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let password1 = "password1";
        let password2 = "password2";

        let hash1 = hash_password(password1).expect("Hash should succeed");
        let hash2 = hash_password(password2).expect("Hash should succeed");

        assert_ne!(
            hash1, hash2,
            "Different passwords should produce different hashes"
        );
    }

    #[test]
    fn test_same_password_different_hashes() {
        let password = "same_password";

        let hash1 = hash_password(password).expect("Hash should succeed");
        let hash2 = hash_password(password).expect("Hash should succeed");

        // Due to salt, even same passwords should produce different hashes
        assert_ne!(
            hash1, hash2,
            "Same password should produce different hashes due to salt"
        );

        // But both should verify correctly
        assert!(verify_password(password, &hash1).expect("Verification should succeed"));
        assert!(verify_password(password, &hash2).expect("Verification should succeed"));
    }

    #[test]
    fn test_empty_password() {
        let password = "";
        let hash = hash_password(password).expect("Hash should succeed even for empty password");

        assert!(verify_password(password, &hash).expect("Verification should succeed"));
        assert!(!verify_password("not_empty", &hash).expect("Verification should succeed"));
    }

    #[test]
    fn test_long_password() {
        let password = "a".repeat(1000); // Very long password
        let hash = hash_password(&password).expect("Hash should succeed for long password");

        assert!(verify_password(&password, &hash).expect("Verification should succeed"));
        assert!(!verify_password("different", &hash).expect("Verification should succeed"));
    }

    #[test]
    fn test_unicode_password() {
        let password = "パスワード123🔐";
        let hash = hash_password(password).expect("Hash should succeed for unicode password");

        assert!(verify_password(password, &hash).expect("Verification should succeed"));
        assert!(!verify_password("password123", &hash).expect("Verification should succeed"));
    }

    #[test]
    fn test_invalid_hash_format() {
        let password = "test_password";
        let invalid_hash = "not_a_valid_hash";

        // Should return error for invalid hash format
        assert!(verify_password(password, invalid_hash).is_err());
    }

    #[test]
    fn test_argon2_hash_format() {
        let password = "test_password";
        let hash = hash_password(password).expect("Hash should succeed");

        // Argon2 hash should start with $argon2
        assert!(
            hash.starts_with("$argon2"),
            "Hash should be in Argon2 format"
        );

        // Should contain proper sections separated by $
        let parts: Vec<&str> = hash.split('$').collect();
        assert!(parts.len() >= 5, "Argon2 hash should have at least 5 parts");
    }

    #[test]
    fn test_hash_consistency() {
        let password = "consistency_test";

        // Generate multiple hashes and verify all work
        for _ in 0..10 {
            let hash = hash_password(password).expect("Hash should succeed");
            assert!(verify_password(password, &hash).expect("Verification should succeed"));
        }
    }
}
