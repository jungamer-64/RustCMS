//! JWT 認証サービス (Phase 5.3 - 新実装)
//!
//! # 役割: 認証 (Authentication)
//! - ユーザーのアイデンティティ検証
//! - セッション管理
//! - トークン生成・検証・リフレッシュ
//!
//! # JWT vs Biscuit の使い分け
//! - **JWT**: "このユーザーは誰か?" (Who are you?)
//! - **Biscuit**: "このユーザーは何ができるか?" (What can you do?)
//!
//! # JWT クレーム構造
//! ```json
//! {
//!   "sub": "<user_id>",           // Subject (UUID)
//!   "username": "<username>",      // ユーザー名
//!   "role": "Admin|Editor|Viewer", // 基本ロール
//!   "session_id": "<uuid>",        // セッション識別子
//!   "exp": 1234567890,             // 有効期限 (Unix timestamp)
//!   "iat": 1234567890,             // 発行時刻
//!   "token_type": "access|refresh" // トークン種別
//! }
//! ```

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::auth::error::AuthError;

#[cfg(feature = "restructure_domain")]
use crate::domain::user::UserRole;

#[cfg(not(feature = "restructure_domain"))]
use crate::models::UserRole;

use crate::common::type_utils::common_types::SessionId;

/// JWT トークン種別
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// アクセストークン (短命、リソースアクセス用)
    Access,
    /// リフレッシュトークン (長命、アクセストークン更新用)
    Refresh,
}

/// JWT クレーム (標準 + カスタム)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject - ユーザーID (標準クレーム)
    pub sub: String,
    /// ユーザー名
    pub username: String,
    /// ユーザーロール
    pub role: UserRole,
    /// セッションID
    pub session_id: String,
    /// 有効期限 (標準クレーム)
    pub exp: i64,
    /// 発行時刻 (標準クレーム)
    pub iat: i64,
    /// トークン種別
    pub token_type: TokenType,
}

impl JwtClaims {
    /// ユーザーID を UUID として取得
    pub fn user_id(&self) -> Uuid {
        Uuid::parse_str(&self.sub).expect("Invalid UUID in JWT sub claim")
    }
}

/// JWT トークンペア (アクセス + リフレッシュ)
#[derive(Debug, Clone)]
pub struct JwtTokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

/// JWT サービス設定
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// JWT 署名用秘密鍵 (HS256) - 互換性のために残す
    pub secret: String,
    /// Ed25519 鍵ペアのファイルパス (EdDSA) - Phase 5.7
    pub key_pair_path: Option<String>,
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
            secret: String::from("default-secret-change-in-production"),
            key_pair_path: Some(String::from("./secrets/ed25519.key")), // Phase 5.7
            access_token_ttl_secs: 900,        // 15分
            refresh_token_ttl_secs: 2_592_000, // 30日
            remember_me_ttl_secs: 86_400,      // 24時間
        }
    }
}

impl JwtConfig {
    /// Config から JwtConfig を作成 (Phase 5.4.2)
    pub fn from_config(config: &crate::config::Config) -> Self {
        use secrecy::ExposeSecret;
        
        Self {
            secret: config.auth.jwt_secret.expose_secret().to_string(),
            key_pair_path: Some(String::from("./secrets/ed25519.key")), // Phase 5.7
            access_token_ttl_secs: config.auth.access_token_ttl_secs,
            refresh_token_ttl_secs: config.auth.refresh_token_ttl_secs,
            remember_me_ttl_secs: config.auth.remember_me_access_ttl_secs,
        }
    }
}

/// JWT 認証サービス
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    config: JwtConfig,
    ed25519_keypair: Option<super::Ed25519KeyPair>, // Phase 5.7
}

impl JwtService {
    /// 新しい JWT サービスを作成
    pub fn new(config: JwtConfig) -> Result<Self, AuthError> {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());
        
        // Phase 5.7: EdDSA鍵ペアをロード（存在する場合）
        let ed25519_keypair = if let Some(ref path) = config.key_pair_path {
            match super::Ed25519KeyPair::load_or_generate(path) {
                Ok(kp) => {
                    debug!("EdDSA keypair loaded from: {}", path);
                    Some(kp)
                }
                Err(e) => {
                    warn!("Failed to load EdDSA keypair, falling back to HS256: {:?}", e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            encoding_key,
            decoding_key,
            config,
            ed25519_keypair,
        })
    }

    /// EdDSA (Ed25519) でJWTトークンを署名 (Phase 5.7)
    ///
    /// jsonwebtoken 9.3 は EdDSA をサポートしていないため、手動で JWT を構築
    fn sign_with_eddsa(&self, claims: &JwtClaims) -> Result<String, AuthError> {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        
        let keypair = self.ed25519_keypair.as_ref()
            .ok_or(AuthError::InvalidToken)?;
        
        // JWT ヘッダー (EdDSA = Ed25519)
        let header = serde_json::json!({
            "alg": "EdDSA",
            "typ": "JWT"
        });
        
        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header)
            .map_err(|_| AuthError::InvalidToken)?);
        let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(claims)
            .map_err(|_| AuthError::InvalidToken)?);
        
        let message = format!("{}.{}", header_b64, claims_b64);
        let signature = keypair.sign(message.as_bytes());
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        
        Ok(format!("{}.{}", message, signature_b64))
    }

    /// EdDSA (Ed25519) でJWTトークンを検証 (Phase 5.7)
    fn verify_with_eddsa(&self, token: &str) -> Result<JwtClaims, AuthError> {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        
        let keypair = self.ed25519_keypair.as_ref()
            .ok_or(AuthError::InvalidToken)?;
        
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }
        
        let message = format!("{}.{}", parts[0], parts[1]);
        let signature_bytes = URL_SAFE_NO_PAD.decode(parts[2])
            .map_err(|_| AuthError::InvalidToken)?;
        
        if signature_bytes.len() != 64 {
            return Err(AuthError::InvalidToken);
        }
        
        let signature = ed25519_dalek::Signature::from_bytes(
            signature_bytes.as_slice().try_into().map_err(|_| AuthError::InvalidToken)?
        );
        
        keypair.verify(message.as_bytes(), &signature)?;
        
        // クレームをデコード
        let claims_json = URL_SAFE_NO_PAD.decode(parts[1])
            .map_err(|_| AuthError::InvalidToken)?;
        let claims: JwtClaims = serde_json::from_slice(&claims_json)
            .map_err(|_| AuthError::InvalidToken)?;
        
        Ok(claims)
    }

    /// JWT トークンペアを生成 (アクセス + リフレッシュ)
    ///
    /// # Arguments
    /// * `user_id` - ユーザーID
    /// * `username` - ユーザー名
    /// * `role` - ユーザーロール
    /// * `session_id` - セッションID
    /// * `remember_me` - "Remember Me" オプション
    ///
    /// # Errors
    /// トークン生成に失敗した場合
    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        username: String,
        role: UserRole,
        session_id: SessionId,
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
            role: role.clone(),
            session_id: session_id.to_string(),
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
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
            token_type: TokenType::Refresh,
        };

        // Phase 5.7: EdDSA優先、フォールバックとしてHS256を使用
        let access_token = if self.ed25519_keypair.is_some() {
            self.sign_with_eddsa(&access_claims)?
        } else {
            encode(&Header::default(), &access_claims, &self.encoding_key)
                .map_err(|e| {
                    warn!("Failed to encode access token: {}", e);
                    AuthError::InvalidToken
                })?
        };

        let refresh_token = if self.ed25519_keypair.is_some() {
            self.sign_with_eddsa(&refresh_claims)?
        } else {
            encode(&Header::default(), &refresh_claims, &self.encoding_key)
                .map_err(|e| {
                    warn!("Failed to encode refresh token: {}", e);
                    AuthError::InvalidToken
                })?
        };

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

    /// JWT トークンを検証してクレームを取得
    ///
    /// # Arguments
    /// * `token` - JWT トークン文字列
    /// * `expected_type` - 期待するトークン種別
    ///
    /// # Errors
    /// トークンが無効、期限切れ、または種別が不一致の場合
    pub fn verify_token(
        &self,
        token: &str,
        expected_type: TokenType,
    ) -> Result<JwtClaims, AuthError> {
        // Phase 5.7: EdDSA優先、HS256フォールバック
        let claims = if self.ed25519_keypair.is_some() {
            // EdDSA検証を試みる
            match self.verify_with_eddsa(token) {
                Ok(c) => c,
                Err(_) => {
                    // EdDSA検証失敗時はHS256にフォールバック（移行期間用）
                    let mut validation = Validation::new(Algorithm::HS256);
                    validation.validate_exp = true;
                    
                    let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
                        .map_err(|e| {
                            match e.kind() {
                                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                                    debug!("JWT token expired");
                                    AuthError::TokenExpired
                                }
                                _ => {
                                    warn!("JWT verification failed (EdDSA and HS256): {}", e);
                                    AuthError::InvalidToken
                                }
                            }
                        })?;
                    token_data.claims
                }
            }
        } else {
            // EdDSA鍵がない場合はHS256のみ
            let mut validation = Validation::new(Algorithm::HS256);
            validation.validate_exp = true;
            
            let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
                .map_err(|e| {
                    match e.kind() {
                        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                            debug!("JWT token expired");
                            AuthError::TokenExpired
                        }
                        _ => {
                            warn!("JWT verification failed: {}", e);
                            AuthError::InvalidToken
                        }
                    }
                })?;
            token_data.claims
        };

        // 有効期限の検証（EdDSA検証の場合は手動で確認）
        if self.ed25519_keypair.is_some() {
            let now = Utc::now().timestamp();
            if claims.exp < now {
                debug!("JWT token expired (EdDSA)");
                return Err(AuthError::TokenExpired);
            }
        }

        // トークン種別の検証
        if claims.token_type != expected_type {
            warn!(
                "Token type mismatch: expected {:?}, got {:?}",
                expected_type, claims.token_type
            );
            return Err(AuthError::InvalidToken);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> JwtService {
        JwtService::new(JwtConfig {
            secret: "test-secret-key".to_string(),
            key_pair_path: None, // テスト環境ではHS256を使用
            access_token_ttl_secs: 900,
            refresh_token_ttl_secs: 2592000,
            remember_me_ttl_secs: 86400,
        }).expect("Failed to create JwtService")
    }

    #[test]
    fn test_generate_and_verify_token_pair() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();

        let token_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Editor,
                session_id.clone(),
                false,
            )
            .expect("Failed to generate token pair");

        // アクセストークンの検証
        let access_claims = service
            .verify_access_token(&token_pair.access_token)
            .expect("Failed to verify access token");

        assert_eq!(access_claims.sub, user_id.to_string());
        assert_eq!(access_claims.username, "testuser");
        assert_eq!(access_claims.session_id, session_id.as_ref());  // to_string() → as_ref()
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

        let token_pair = service
            .generate_token_pair(
                user_id,
                "testuser".to_string(),
                UserRole::Subscriber,  // Viewer → Subscriber に修正
                session_id,
                false,
            )
            .expect("Failed to generate token pair");

        // アクセストークンをリフレッシュトークンとして検証 (エラーになるべき)
        let result = service.verify_refresh_token(&token_pair.access_token);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_invalid_token() {
        let service = create_test_service();
        let result = service.verify_access_token("invalid.token.here");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    // ===== Phase 5.7: EdDSA テスト =====

    fn create_eddsa_test_service() -> JwtService {
        JwtService::new(JwtConfig {
            secret: "fallback-secret".to_string(),
            key_pair_path: Some("./test_secrets/test_ed25519.key".to_string()),
            access_token_ttl_secs: 900,
            refresh_token_ttl_secs: 2592000,
            remember_me_ttl_secs: 86400,
        }).expect("Failed to create EdDSA JwtService")
    }

    #[test]
    fn test_eddsa_generate_and_verify() {
        let service = create_eddsa_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();

        let token_pair = service
            .generate_token_pair(
                user_id,
                "eddsa_user".to_string(),
                UserRole::Admin,
                session_id.clone(),
                false,
            )
            .expect("Failed to generate EdDSA token pair");

        // EdDSA署名されたトークンを検証
        let access_claims = service
            .verify_access_token(&token_pair.access_token)
            .expect("Failed to verify EdDSA access token");

        assert_eq!(access_claims.sub, user_id.to_string());
        assert_eq!(access_claims.username, "eddsa_user");
        assert_eq!(access_claims.role, UserRole::Admin);
        assert_eq!(access_claims.session_id, session_id.to_string());
        assert_eq!(access_claims.token_type, TokenType::Access);

        // リフレッシュトークンも検証
        let refresh_claims = service
            .verify_refresh_token(&token_pair.refresh_token)
            .expect("Failed to verify EdDSA refresh token");

        assert_eq!(refresh_claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_eddsa_invalid_signature() {
        let service = create_eddsa_test_service();
        
        // 別のEdDSA鍵で署名されたトークン（不正）
        let another_service = create_eddsa_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();

        let token_pair = another_service
            .generate_token_pair(
                user_id,
                "attacker".to_string(),
                UserRole::Admin,
                session_id,
                false,
            )
            .expect("Failed to generate token");

        // 異なる鍵でトークンを改ざん
        let mut tampered_token = token_pair.access_token.clone();
        tampered_token.push_str("tampered");

        let result = service.verify_access_token(&tampered_token);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn test_eddsa_token_format() {
        let service = create_eddsa_test_service();
        let user_id = Uuid::new_v4();
        let session_id = SessionId::new();

        let token_pair = service
            .generate_token_pair(
                user_id,
                "format_test".to_string(),
                UserRole::Editor,
                session_id,
                false,
            )
            .expect("Failed to generate token");

        // JWT形式確認: "header.payload.signature"
        let parts: Vec<&str> = token_pair.access_token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts");

        // ヘッダーをデコードしてEdDSAアルゴリズム確認
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let header_json = URL_SAFE_NO_PAD.decode(parts[0])
            .expect("Failed to decode header");
        let header: serde_json::Value = serde_json::from_slice(&header_json)
            .expect("Failed to parse header");
        
        assert_eq!(header["alg"], "EdDSA");
        assert_eq!(header["typ"], "JWT");
    }
}
