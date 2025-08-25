//! Legacy password hashing utilities (SHA256-based)
//! 
//! DEPRECATED: This module provides legacy SHA256-based hashing.
//! New code should use the `password` module which provides Argon2-based hashing.
//! 
//! This module is kept for backward compatibility only.

use sha2::{Sha256, Digest};
use hex;

/// Hash password using SHA256 (DEPRECATED - use utils::password::hash_password instead)
#[deprecated(note = "Use utils::password::hash_password for secure Argon2 hashing")]
pub fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify password using SHA256 (DEPRECATED - use utils::password::verify_password instead)
#[deprecated(note = "Use utils::password::verify_password for secure Argon2 verification")]
pub fn verify_password(password: &str, salt: &str, hash: &str) -> bool {
    hash_password(password, salt) == hash
}
