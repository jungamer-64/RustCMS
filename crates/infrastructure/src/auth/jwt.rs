//! JWT 認証サービス (Refactored - EdDSA Only)
//!
//! # 改善点
//! - EdDSA (Ed25519) のみをサポート（HS256廃止）
//! - 統合鍵管理システムを使用
//! - エラーハンドリングの改善
//! - コードの簡素化

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::unified_key_management::UnifiedKeyPair;

#[cfg(feature = "restructure_domain")]
use domain::user::UserRole;

#[cfg(not(feature = "restructure_domain"))]
use domain::user::UserRole;

use shared_core::types::common_types::SessionId;

/// JWT トークン種別
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Refresh => "refresh",
        }
    }
}

/// JWT クレーム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject - ユーザーID
    pub sub: String,
    /// ユーザー名
    pub username: String,
    /// ユーザーロール
    pub role: UserRole,
    /// セッションID
    pub session_id: String,
    /// セッションバージョン
    pub session_version: u32,
    /// 有効期限 (Unix timestamp)
    pub exp: i64,
    /// 発行時刻 (Unix timestamp)
    pub iat: i64,
    /// トークン種別
    pub token_type: TokenType,
}

impl JwtClaims {
    /// ユーザーID を UUID として取得
    pub fn user_id(&self) -> Result<Uuid, AuthError> {
        Uuid::parse_str(&self.sub)
            .map_err(|_| AuthError::TokenParseError("Invalid UUID in sub claim".to_string()))
    }

    /// セッションIDを取得
    pub fn session_id(&self) -> SessionId {
        SessionId::from(self.session_id.clone())
    }

    /// セッションバージョンを取得
    #[must_use]
    pub fn session_version(&self) -> u32 {
        self.session_version
    }
}

/// JWT トークンペア
#[derive(Debug, Clone)]
pub struct JwtTokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

/// JWT サービス設定
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// アクセストークン有効期限 (秒)
    pub access_token_ttl_secs: u64,
    /// リフレッシュトークン有効期限 (秒)
    pub refresh_token_ttl_secs: u64,
    /// "Remember Me" 時のアクセストークン有効期限 (秒)
    pub remember_me_ttl_secs: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            access_token_ttl_secs: 900,        // 15分
            refresh_token_ttl_secs: 2_592_000, // 30日
            remember_me_ttl_secs: 86_400,      // 24時間
        }
    }
}

/// JWT 認証サービス (EdDSA Only)
pub struct JwtService {
    keypair: UnifiedKeyPair,
    config: JwtConfig,
}

impl JwtService {
    /// 新しい JWT サービスを作成
    pub fn new(keypair: UnifiedKeyPair, config: JwtConfig) -> Self {
        Self { keypair, config }
    }

    /// JWT トークンペアを生成
    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        username: String,
        role: UserRole,
        session_id: SessionId,
        session_version: u32,
        remember_me: bool,
    ) -> Result<JwtTokenPair, AuthError> {
        let now = Utc::now();

        // アクセストークンの有効期限
        let access_ttl = if remember_me {
            self.config.remember_me_ttl_secs
        } else {
            self.config.access_token_ttl_secs
        };

        let access_exp = now + Duration::seconds(access_ttl as i64);
        let refresh_exp = now + Duration::seconds(self.config.refresh_token_ttl_secs as i64);

        // アクセストークン
        let access_claims = JwtClaims {
            sub: user_id.to_string(),
            username: username.clone(),
            role,
            session_id: session_id.to_string(),
            session_version,
            exp: access_exp.timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Access,
        };

        // リフレッシュトークン
        let refresh_claims = JwtClaims {
            sub: user_id.to_string(),
            username,
            role,
            session_id: session_id.to_string(),
            session_version,
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Refresh,
        };

        let access_token = self.sign_token(&access_claims)?;
        let refresh_token = self.sign_token(&refresh_claims)?;

        debug!(
            user_id = %user_id,
            session_id = %session_id,
            remember_me = remember_me,
            "Generated JWT token pair"
        );

        Ok(JwtTokenPair {
            access_token,
            refresh_token,
            expires_at: access_exp,
        })
    }

    /// JWT トークンを検証
    pub fn verify_token(
        &self,
        token: &str,
        expected_type: TokenType,
    ) -> Result<JwtClaims, AuthError> {
        let claims = self.verify_signature(token)?;

        // 有効期限の検証
        let now = Utc::now().timestamp();
        if claims.exp < now {
            debug!("JWT token expired: exp={}, now={}", claims.exp, now);
            return Err(AuthError::TokenExpired);
        }

        // トークン種別の検証
        if claims.token_type != expected_type {
            warn!(
                "Token type mismatch: expected {:?}, got {:?}",
                expected_type, claims.token_type
            );
            return Err(AuthError::TokenTypeMismatch {
                expected: expected_type.as_str().to_string(),
                actual: claims.token_type.as_str().to_string(),
            });
        }

        debug!(
            user_id = %claims.sub,
            session_id = %claims.session_id,
            token_type = ?claims.token_type,
            "JWT token verified successfully"
        );

        Ok(claims)
    }

    /// アクセストークンを検証
    pub fn verify_access_token(&self, token: &str) -> Result<JwtClaims, AuthError> {
        self.verify_token(token, TokenType::Access)
    }

    /// リフレッシュトークンを検証
    pub fn verify_refresh_token(&self, token: &str) -> Result<JwtClaims, AuthError> {
        self.verify_token(token, TokenType::Refresh)
    }

    // === Private methods ===

    /// EdDSA でJWTトークンを署名
    fn sign_token(&self, claims: &JwtClaims) -> Result<String, AuthError> {
        // JWT ヘッダー
        let header = serde_json::json!({
            "alg": "EdDSA",
            "typ": "JWT"
        });

        let header_b64 =
            URL_SAFE_NO_PAD
                .encode(serde_json::to_string(&header).map_err(|e| {
                    AuthError::JwtError(format!("Failed to serialize header: {e}"))
                })?);

        let claims_b64 =
            URL_SAFE_NO_PAD
                .encode(serde_json::to_string(claims).map_err(|e| {
                    AuthError::JwtError(format!("Failed to serialize claims: {e}"))
                })?);

        let message = format!("{}.{}", header_b64, claims_b64);
        let signature = self.keypair.sign(message.as_bytes());
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{}.{}", message, signature_b64))
    }

    /// EdDSA でJWTトークンの署名を検証
    fn verify_signature(&self, token: &str) -> Result<JwtClaims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidTokenFormat);
        }

        // 署名検証
        let message = format!("{}.{}", parts[0], parts[1]);
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| AuthError::TokenParseError(format!("Invalid signature base64: {e}")))?;

        if signature_bytes.len() != 64 {
            return Err(AuthError::InvalidTokenSignature);
        }

        let signature = ed25519_dalek::Signature::from_bytes(
            signature_bytes
                .as_slice()
                .try_into()
                .map_err(|_| AuthError::InvalidTokenSignature)?,
        );

        self.keypair.verify(message.as_bytes(), &signature)?;

        // クレームをデコード
        let claims_json = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| AuthError::TokenParseError(format!("Invalid claims base64: {e}")))?;

        let claims: JwtClaims = serde_json::from_slice(&claims_json)
            .map_err(|e| AuthError::TokenParseError(format!("Invalid claims JSON: {e}")))?;

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::unified_key_management::UnifiedKeyPair;

    fn create_test_service() -> JwtService {
        let keypair = UnifiedKeyPair::generate().expect("Failed to generate keypair");
        let config = JwtConfig::default();
        JwtService::new(keypair, config)
    }

    #[test]
    fn test_generate_and_verify_token_pair() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();
        let session_version = 1;

        let token_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Editor,
                session_id.clone(),
                session_version,
                false,
            )
            .expect("Failed to generate token pair");

        // アクセストークンの検証
        let access_claims = service
            .verify_access_token(&token_pair.access_token)
            .expect("Failed to verify access token");

        assert_eq!(access_claims.sub, user_id.to_string());
        assert_eq!(access_claims.username, "testuser");
        assert_eq!(access_claims.session_id, session_id.as_ref());
        assert_eq!(access_claims.session_version, session_version);
        assert_eq!(access_claims.token_type, TokenType::Access);

        // リフレッシュトークンの検証
        let refresh_claims = service
            .verify_refresh_token(&token_pair.refresh_token)
            .expect("Failed to verify refresh token");

        assert_eq!(refresh_claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_token_type_mismatch() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();
        let session_version = 1;

        let token_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Subscriber,
                session_id,
                session_version,
                false,
            )
            .expect("Failed to generate token pair");

        // アクセストークンをリフレッシュトークンとして検証 (エラーになるべき)
        let result = service.verify_refresh_token(&token_pair.access_token);
        assert!(matches!(result, Err(AuthError::TokenTypeMismatch { .. })));
    }

    #[test]
    fn test_invalid_token_format() {
        let service = create_test_service();

        // 不正な形式
        let result = service.verify_access_token("invalid.token");
        assert!(matches!(result, Err(AuthError::InvalidTokenFormat)));
    }

    #[test]
    fn test_tampered_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();
        let session_version = 1;

        let token_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Editor,
                session_id,
                session_version,
                false,
            )
            .expect("Failed to generate token pair");

        // トークンを改ざん
        let mut tampered = token_pair.access_token.clone();
        tampered.push_str("tampered");

        let result = service.verify_access_token(&tampered);
        assert!(result.is_err());
    }

    #[test]
    fn test_remember_me_expiry() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();
        let session_version = 1;

        // Remember Me なし
        let normal_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Editor,
                session_id.clone(),
                session_version,
                false,
            )
            .expect("Failed to generate token pair");

        // Remember Me あり
        let remember_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Editor,
                session_id,
                session_version,
                true,
            )
            .expect("Failed to generate token pair");

        // Remember Me の方が有効期限が長いことを確認
        assert!(remember_pair.expires_at > normal_pair.expires_at);
    }
}
