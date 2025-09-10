//! Hash utilities (SHA-256 helpers)

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// Compute SHA-256 digest and return as lowercase hex string
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    hex::encode(digest)
}

/// Compute SHA-256 digest and return as base64url (no padding)
pub fn sha256_b64url(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}
