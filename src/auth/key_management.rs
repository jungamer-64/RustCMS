use crate::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use biscuit_auth::{Algorithm as BiscuitAlgorithm, KeyPair, PrivateKey, PublicKey};
use secrecy::ExposeSecret;

// --- Key file helper funcs (共通読込ユーティリティ) ---
fn read_file_string(path: &std::path::Path, label: &str) -> crate::Result<String> {
    std::fs::read_to_string(path).map_err(|e| {
        crate::AppError::Internal(format!("Failed reading biscuit {label} key file: {e}"))
    })
}
fn decode_key_b64(data: &str, label: &str) -> crate::Result<Vec<u8>> {
    STANDARD.decode(data).map_err(|e| {
        crate::AppError::Internal(format!("Failed to decode biscuit {label} key b64: {e}"))
    })
}
fn read_biscuit_private_key(path: &std::path::Path) -> crate::Result<PrivateKey> {
    let b64 = read_file_string(path, "private")?;
    let bytes = decode_key_b64(&b64, "private")?;
    PrivateKey::from_bytes(&bytes, BiscuitAlgorithm::Ed25519)
        .map_err(|e| crate::AppError::Internal(format!("Failed to parse biscuit private key: {e}")))
}
fn read_biscuit_public_key(path: &std::path::Path) -> crate::Result<PublicKey> {
    let b64 = read_file_string(path, "public")?;
    let bytes = decode_key_b64(&b64, "public")?;
    PublicKey::from_bytes(&bytes, BiscuitAlgorithm::Ed25519)
        .map_err(|e| crate::AppError::Internal(format!("Failed to parse biscuit public key: {e}")))
}

fn try_load_env_keys() -> Option<KeyPair> {
    let priv_b64 = std::env::var("BISCUIT_PRIVATE_KEY_B64").ok()?;
    let pub_b64 = std::env::var("BISCUIT_PUBLIC_KEY_B64").ok()?;
    let priv_bytes = STANDARD.decode(&priv_b64).ok()?;
    let pub_bytes = STANDARD.decode(&pub_b64).ok()?;
    let priv_key = PrivateKey::from_bytes(&priv_bytes, BiscuitAlgorithm::Ed25519).ok()?;
    let pub_key = PublicKey::from_bytes(&pub_bytes, BiscuitAlgorithm::Ed25519).ok()?;
    let kp = KeyPair::from(&priv_key);
    if kp.public().to_bytes() != pub_key.to_bytes() {
        return None;
    }
    Some(kp)
}

pub(crate) fn load_or_generate_keypair(config: &crate::config::AuthConfig) -> Result<KeyPair> {
    if let Some(kp) = try_load_env_keys() {
        return Ok(kp);
    }
    let key_str = config.biscuit_root_key.expose_secret();
    let path = std::path::Path::new(key_str);
    if !key_str.is_empty() && path.exists() && path.is_dir() {
        let priv_key = read_biscuit_private_key(&path.join("biscuit_private.b64"))?;
        let pub_key = read_biscuit_public_key(&path.join("biscuit_public.b64"))?;
        let kp = KeyPair::from(&priv_key);
        if kp.public().to_bytes() != pub_key.to_bytes() {
            return Err(crate::AppError::Internal(
                "Mismatched biscuit key pair (public key differs from private)".to_string(),
            ));
        }
        Ok(kp)
    } else {
        Ok(KeyPair::new())
    }
}
