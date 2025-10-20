//! 統合Ed25519鍵管理 (Refactored)
//!
//! JWT, Biscuit両方で使用する単一のEd25519鍵ペア管理システム
//!
//! # セキュリティ原則
//! - 単一の真実の源（Single Source of Truth）
//! - 環境変数 > ファイル の優先順位
//! - 自動鍵生成は開発環境のみ（本番では明示的な鍵必須）

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use biscuit_auth::Algorithm as BiscuitAlgorithm;
use biscuit_auth::{
    KeyPair as BiscuitKeyPair, PrivateKey as BiscuitPrivateKey, PublicKey as BiscuitPublicKey,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::sync::Arc;
use std::{fs, path::Path};
use tracing::{debug, error, info, warn};

use crate::auth::AuthError;

/// 統合Ed25519鍵ペア
///
/// JWTとBiscuitの両方で使用可能な単一の鍵ペア
#[derive(Clone)]
pub struct UnifiedKeyPair {
    /// Ed25519署名鍵（JWT用）
    signing_key: SigningKey,
    /// Ed25519検証鍵（JWT用）
    verifying_key: VerifyingKey,
    /// Biscuit鍵ペア（Cloneを実装していないためArcで囲む）
    biscuit_keypair: Arc<BiscuitKeyPair>,
}

/// 鍵ロード設定
pub struct KeyLoadConfig {
    /// 鍵ファイルのパス（存在しない場合は生成）
    pub key_file_path: String,
    /// 本番環境フラグ（trueの場合、鍵が存在しない場合はエラー）
    pub is_production: bool,
}

impl Default for KeyLoadConfig {
    fn default() -> Self {
        Self {
            key_file_path: "./secrets/unified_ed25519.key".to_string(),
            is_production: false,
        }
    }
}

impl UnifiedKeyPair {
    const SEED_LENGTH: usize = 32;

    /// 秘密鍵シードから鍵ペアを構築
    fn from_seed(seed: &[u8]) -> Result<Self, AuthError> {
        if seed.len() != Self::SEED_LENGTH {
            return Err(AuthError::KeyManagementError(format!(
                "Invalid Ed25519 seed length: expected {}, got {}",
                Self::SEED_LENGTH,
                seed.len()
            )));
        }

        let seed_array: [u8; Self::SEED_LENGTH] = seed.try_into().map_err(|_| {
            AuthError::KeyManagementError("Failed to convert seed to fixed array".into())
        })?;

        let signing_key = SigningKey::from_bytes(&seed_array);
        let verifying_key = signing_key.verifying_key();

        let biscuit_private = BiscuitPrivateKey::from_bytes(&seed_array, BiscuitAlgorithm::Ed25519)
            .map_err(|e| {
                AuthError::KeyManagementError(format!("Failed to create Biscuit private key: {e}"))
            })?;
        let biscuit_keypair = Arc::new(BiscuitKeyPair::from(&biscuit_private));

        Ok(Self {
            signing_key,
            verifying_key,
            biscuit_keypair,
        })
    }

    /// 新しい鍵ペアを生成
    pub fn generate() -> Result<Self, AuthError> {
        use rand::RngCore;
        let mut csprng = rand::rng();
        let mut secret_bytes = [0u8; Self::SEED_LENGTH];
        csprng.fill_bytes(&mut secret_bytes);

        Self::from_seed(&secret_bytes)
    }

    /// 環境変数から鍵をロード
    ///
    /// 環境変数: ED25519_PRIVATE_KEY_B64
    fn try_load_from_env() -> Result<Option<Self>, AuthError> {
        let key_b64 = match std::env::var("ED25519_PRIVATE_KEY_B64") {
            Ok(k) => k,
            Err(_) => return Ok(None),
        };

        debug!("Loading Ed25519 key from environment variable");

        let bytes = BASE64.decode(key_b64.trim()).map_err(|e| {
            AuthError::KeyManagementError(format!("Invalid base64 in ED25519_PRIVATE_KEY_B64: {e}"))
        })?;

        info!("Ed25519 key loaded from environment variable");
        Ok(Some(Self::from_seed(&bytes)?))
    }

    /// ファイルから鍵をロード
    fn load_from_file(path: &str) -> Result<Self, AuthError> {
        let data = fs::read_to_string(path).map_err(|e| {
            AuthError::BiscuitError(format!("Failed to read key file {}: {}", path, e))
        })?;

        let bytes = BASE64.decode(data.trim()).map_err(|e| {
            AuthError::KeyManagementError(format!("Invalid base64 in key file: {e}"))
        })?;

        info!("Ed25519 key loaded from file: {}", path);
        Self::from_seed(&bytes)
    }

    /// 鍵をファイルに保存
    fn save_to_file(&self, path: &str) -> Result<(), AuthError> {
        // ディレクトリを作成
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AuthError::BiscuitError(format!("Failed to create key directory: {e}"))
            })?;
        }

        let encoded = BASE64.encode(self.signing_key.to_bytes());
        fs::write(path, encoded)
            .map_err(|e| AuthError::BiscuitError(format!("Failed to write key file: {e}")))?;

        // Unix系OSでパーミッション設定（600）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .map_err(|e| AuthError::BiscuitError(format!("Failed to read file metadata: {e}")))?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms).map_err(|e| {
                AuthError::BiscuitError(format!("Failed to set file permissions: {e}"))
            })?;
        }

        Ok(())
    }

    /// 設定に基づいて鍵をロードまたは生成
    ///
    /// 優先順位:
    /// 1. 環境変数 (ED25519_PRIVATE_KEY_B64)
    /// 2. ファイル
    /// 3. 新規生成（開発環境のみ）
    pub fn load_or_generate(config: &KeyLoadConfig) -> Result<Self, AuthError> {
        // 1. 環境変数から試行
        if let Some(keypair) = Self::try_load_from_env()? {
            return Ok(keypair);
        }

        // 2. ファイルから試行
        let path = Path::new(&config.key_file_path);
        if path.exists() {
            return Self::load_from_file(&config.key_file_path);
        }

        // 3. 本番環境では鍵が必須
        if config.is_production {
            error!("Ed25519 key not found in production environment");
            return Err(AuthError::BiscuitError(
                "Ed25519 key must be provided in production (via environment variable or file)"
                    .to_string(),
            ));
        }

        // 4. 開発環境でのみ新規生成
        warn!("Ed25519 key not found, generating new one (development mode only)");
        let keypair = Self::generate()?;

        if let Err(e) = keypair.save_to_file(&config.key_file_path) {
            warn!("Failed to save generated key: {}", e);
        } else {
            info!("Generated Ed25519 key saved to: {}", config.key_file_path);
        }

        Ok(keypair)
    }

    // === JWT用メソッド ===

    /// メッセージに署名（JWT用）
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// 署名を検証（JWT用）
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), AuthError> {
        self.verifying_key
            .verify(message, signature)
            .map_err(|_| AuthError::InvalidTokenSignature)
    }

    /// 署名鍵への参照を取得
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// 検証鍵への参照を取得
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    // === Biscuit用メソッド ===

    /// Biscuit鍵ペアへの参照を取得
    pub fn biscuit_keypair(&self) -> &BiscuitKeyPair {
        &self.biscuit_keypair
    }

    /// Biscuit公開鍵を取得
    pub fn biscuit_public_key(&self) -> BiscuitPublicKey {
        self.biscuit_keypair.public()
    }

    // === 鍵情報取得 ===

    /// 公開鍵をBase64エンコードで取得（配布用）
    pub fn public_key_base64(&self) -> String {
        BASE64.encode(self.verifying_key.as_bytes())
    }

    /// 秘密鍵のフィンガープリント（ログ用、実際の鍵は含まない）
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.verifying_key.as_bytes());
        let result = hasher.finalize();
        format!("sha256:{}", hex::encode(&result[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = UnifiedKeyPair::generate().expect("Failed to generate keypair");
        let message = b"test message";

        // JWT署名と検証
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_biscuit_keypair_compatibility() {
        let keypair = UnifiedKeyPair::generate().expect("Failed to generate keypair");

        // Biscuit鍵ペアが正しく生成されていることを確認
        let biscuit_kp = keypair.biscuit_keypair();
        let public_key = biscuit_kp.public();

        // 公開鍵が取得できることを確認
        assert_eq!(public_key.to_bytes().len(), 32);
    }

    #[test]
    fn test_save_and_load() {
        let temp_path = "./test_secrets/test_unified.key";

        // 生成して保存
        let original = UnifiedKeyPair::generate().expect("Failed to generate");
        original.save_to_file(temp_path).expect("Failed to save");

        // ロードして検証
        let loaded = UnifiedKeyPair::load_from_file(temp_path).expect("Failed to load");

        // 同じ鍵であることを確認
        assert_eq!(original.public_key_base64(), loaded.public_key_base64());

        // クリーンアップ
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_fingerprint() {
        let keypair = UnifiedKeyPair::generate().expect("Failed to generate");
        let fp = keypair.fingerprint();

        assert!(fp.starts_with("sha256:"));
        assert_eq!(fp.len(), 23); // "sha256:" + 16 hex chars
    }
}
