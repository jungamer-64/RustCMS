//! Ed25519 鍵管理モジュール (Phase 5.7)
//!
//! Azure Key Vault などのクラウドサービスに依存せず、
//! ファイルベースで安全に Ed25519 鍵ペアを管理します。
//!
//! # セキュリティ機能
//! - 秘密鍵のファイルシステム保存（permissions 600推奨）
//! - 初回起動時の自動鍵生成
//! - base64エンコーディングでの鍵保存
//! - 公開鍵の安全な配布

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use std::{fs, path::Path};
use tracing::{info, warn};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::auth::AuthError;

/// Ed25519 鍵ペア
#[derive(Clone)]
pub struct Ed25519KeyPair {
    /// 署名用秘密鍵
    signing_key: SigningKey,
    /// 検証用公開鍵
    verifying_key: VerifyingKey,
}

impl Ed25519KeyPair {
    /// 新しい鍵ペアを生成
    pub fn generate() -> Self {
        // ed25519-dalek 2.x では from_bytes でランダム生成
        use rand::RngCore;
        let mut csprng = rand::thread_rng();
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// ファイルから鍵ペアを読み込む、存在しない場合は新規生成
    ///
    /// # Arguments
    /// * `private_key_path` - 秘密鍵ファイルのパス
    ///
    /// # Security
    /// 秘密鍵ファイルは chmod 600 に設定することを推奨
    pub fn load_or_generate(private_key_path: &str) -> Result<Self, AuthError> {
        let path = Path::new(private_key_path);
        
        if path.exists() {
            info!("Loading existing Ed25519 key pair from: {}", private_key_path);
            Self::load_from_file(private_key_path)
        } else {
            warn!("Ed25519 key pair not found, generating new one");
            let keypair = Self::generate();
            keypair.save_to_file(private_key_path)?;
            info!("New Ed25519 key pair generated and saved to: {}", private_key_path);
            Ok(keypair)
        }
    }

    /// ファイルから鍵ペアを読み込む
    fn load_from_file(path: &str) -> Result<Self, AuthError> {
        let data = fs::read_to_string(path)
            .map_err(|_| AuthError::InvalidToken)?;
        
        let bytes = BASE64.decode(data.trim())
            .map_err(|_| AuthError::InvalidToken)?;
        
        if bytes.len() != 32 {
            return Err(AuthError::InvalidToken);
        }
        
        let signing_key = SigningKey::from_bytes(
            bytes.as_slice().try_into().map_err(|_| AuthError::InvalidToken)?
        );
        let verifying_key = signing_key.verifying_key();
        
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// 鍵ペアをファイルに保存
    fn save_to_file(&self, path: &str) -> Result<(), AuthError> {
        // ディレクトリが存在しない場合は作成
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|_| AuthError::InvalidToken)?;
        }
        
        let encoded = BASE64.encode(self.signing_key.to_bytes());
        fs::write(path, encoded)
            .map_err(|_| AuthError::InvalidToken)?;
        
        // Unix系OSでパーミッション設定（600）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)
                .map_err(|_| AuthError::InvalidToken)?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)
                .map_err(|_| AuthError::InvalidToken)?;
        }
        
        Ok(())
    }

    /// メッセージに署名
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// 署名を検証
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), AuthError> {
        self.verifying_key
            .verify(message, signature)
            .map_err(|_| AuthError::InvalidToken)
    }

    /// 公開鍵をバイト列で取得
    pub fn public_key_bytes(&self) -> &[u8; 32] {
        self.verifying_key.as_bytes()
    }

    /// 公開鍵をBase64エンコードで取得(配布用)
    pub fn public_key_base64(&self) -> String {
        BASE64.encode(self.public_key_bytes())
    }

    /// 署名鍵への参照を取得（JWT署名用）
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// 検証鍵への参照を取得（JWT検証用）
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = Ed25519KeyPair::generate();
        let message = b"test message";
        
        // 署名と検証
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_public_key_base64() {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let keypair = Ed25519KeyPair::generate();
        let base64_key = keypair.public_key_base64();
        
        // Base64デコードできることを確認
        assert!(BASE64.decode(&base64_key).is_ok());
        assert_eq!(BASE64.decode(&base64_key).unwrap().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = Ed25519KeyPair::generate();
        let message = b"Hello, Ed25519!";
        
        let signature = keypair.sign(message);
        
        // 正しい署名の検証
        assert!(keypair.verify(message, &signature).is_ok());
        
        // 不正なメッセージの検証
        let wrong_message = b"Wrong message";
        assert!(keypair.verify(wrong_message, &signature).is_err());
    }
}
