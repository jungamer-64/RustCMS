//! Authentication Service - Biscuit (統一版)
//!
//! 目的: 既存の JWT / Biscuit 併用実装を廃止し、Biscuit トークンのみで
//! アクセス/リフレッシュ (スライディングセッション) を提供する。
//!
//! 提供機能:
//! - Biscuit 署名トークン (access / refresh の2種類)
//! - WebAuthn (未改変・今後拡張用プレースホルダ)
//! - Argon2 パスワード検証
//! - RBAC (role -> permissions マッピング)
//!
//! トークン仕様:
//! - access biscuit: 有効期限 1h (remember_me=false の場合) / 24h (remember_me=true の場合 *従来挙動 24h を保持*)
//! - refresh biscuit: 有効期限 30d, 使用時に refresh_version を +1 し再発行
//! - Biscuit 内に以下の facts を格納:
//!   user("<uuid>", "<username>", "<role>");
//!   token_type("access"|"refresh");
//!   exp(<unix_ts>);            // 失効時刻 (秒)
//!   session("<session_id>", <version>);
//! - refresh 時は version をインクリメントし旧 refresh トークンを無効化
//! - セッション状態はメモリ (HashMap) 管理 (分散構成向けには外部ストアへ差し替え予定)

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use biscuit_auth::{KeyPair, PrivateKey, PublicKey, builder::BiscuitBuilder, error::Format as BiscuitFormat, Algorithm as BiscuitAlgorithm};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    config::AuthConfig,
    database::Database,
    models::{CreateUserRequest, User, UserRole},
    utils::{common_types::UserInfo, password},
    Result,
};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Password hashing error: {0}")]
    PasswordHash(String),
    #[error("Biscuit error: {0}")]
    Biscuit(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("WebAuthn error: {0}")]
    WebAuthn(String),
}

impl From<AuthError> for crate::AppError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidCredentials | AuthError::UserNotFound => {
                crate::AppError::Authentication(err.to_string())
            }
            AuthError::TokenExpired | AuthError::InvalidToken => {
                crate::AppError::Authentication(err.to_string())
            }
            AuthError::InsufficientPermissions => crate::AppError::Authorization(err.to_string()),
            _ => crate::AppError::Authentication(err.to_string()),
        }
    }
}

/// Authentication service
#[allow(dead_code)]
pub struct AuthService {
    /// Biscuit private key for token generation
    biscuit_private_key: PrivateKey,
    /// Biscuit public key for token verification
    biscuit_public_key: PublicKey,
    /// Database reference
    database: Database,
    /// Configuration
    config: AuthConfig,
    /// Active sessions (session_id -> SessionData)
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    /// Password hasher
    argon2: Argon2<'static>,
}

impl Clone for AuthService {
    fn clone(&self) -> Self {
        // Preserve existing keys instead of regenerating them so tokens remain verifiable
        Self {
            biscuit_private_key: self.biscuit_private_key.clone(),
            biscuit_public_key: self.biscuit_public_key,
            database: self.database.clone(),
            config: self.config.clone(),
            sessions: Arc::clone(&self.sessions),
            argon2: Argon2::default(),
        }
    }
}

/// Session data
#[derive(Debug, Clone)]
pub struct SessionData {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    /// 現在有効なリフレッシュトークンのバージョン (1 から開始し、ローテーション毎に +1)
    pub refresh_version: u32,
}

/// Biscuit 解析結果内部表現 (impl 内定義不可のためここで宣言)
struct ParsedBiscuit {
    user_id: Uuid,
    username: String,
    role: UserRole,
    token_type: String,
    // 有効期限は parse 内でチェック後廃棄するため保持しない
    session_id: String,
    version: u32,
}

// JWT 関連構造体は削除 (後方互換保持不要と判断)

/// Login request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

/// Authentication response
#[derive(Debug, Serialize)]
#[derive(ToSchema)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    pub biscuit_token: String,
    pub expires_in: i64,
    pub session_id: String,
}

/// Refresh response (rotated tokens)
#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub session_id: String,
    pub refresh_token: String,
}

/// Authentication context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub session_id: String,
    pub permissions: Vec<String>,
}

impl AuthService {
    /// Create new authentication service (Biscuit 専用)
    pub async fn new(config: &AuthConfig, database: &Database) -> Result<Self> {
        // Attempt to load biscuit keypair from environment (base64 encoded), otherwise generate.
        // Assumption: `PrivateKey` and `PublicKey` provide byte (de)serialization APIs.
        // If the biscuit-auth types differ, adapt loading to the concrete API (e.g. from_pem/from_slice).
        // Determine biscuit keys via env, config directory, or generation and persist when appropriate
        let (biscuit_private_key, biscuit_public_key) = {
            if let (Ok(priv_b64), Ok(pub_b64)) = (
                std::env::var("BISCUIT_PRIVATE_KEY_B64"),
                std::env::var("BISCUIT_PUBLIC_KEY_B64"),
            ) {
                if let (Ok(priv_bytes), Ok(pub_bytes)) = (STANDARD.decode(&priv_b64), STANDARD.decode(&pub_b64)) {
                    // biscuit-auth 6.0: from_bytes now requires Algorithm
                    let priv_key = PrivateKey::from_bytes(&priv_bytes, BiscuitAlgorithm::Ed25519)
                        .map_err(|e| crate::AppError::Internal(format!("Failed to parse biscuit private key from env: {}", e)))?;
                    let pub_key = PublicKey::from_bytes(&pub_bytes, BiscuitAlgorithm::Ed25519)
                        .map_err(|e| crate::AppError::Internal(format!("Failed to parse biscuit public key from env: {}", e)))?;
                    (priv_key, pub_key)
                } else {
                    let keypair = KeyPair::new();
                    (keypair.private(), keypair.public())
                }
            } else {
                // 2) If config.biscuit_root_key is a directory path, try to read keys from files there
                let biscuit_key_path = std::path::Path::new(&config.biscuit_root_key);
                if !config.biscuit_root_key.is_empty() && biscuit_key_path.exists() && biscuit_key_path.is_dir() {
                    let priv_file = biscuit_key_path.join("biscuit_private.b64");
                    let pub_file = biscuit_key_path.join("biscuit_public.b64");

                    if priv_file.exists() && pub_file.exists() {
                        let priv_b64 = std::fs::read_to_string(&priv_file).map_err(|e| {
                            crate::AppError::Internal(format!("Failed reading biscuit private key file: {}", e))
                        })?;
                        let pub_b64 = std::fs::read_to_string(&pub_file).map_err(|e| {
                            crate::AppError::Internal(format!("Failed reading biscuit public key file: {}", e))
                        })?;

                        if let (Ok(priv_bytes), Ok(pub_bytes)) = (STANDARD.decode(&priv_b64), STANDARD.decode(&pub_b64)) {
                            let priv_key = PrivateKey::from_bytes(&priv_bytes, BiscuitAlgorithm::Ed25519).map_err(|e| {
                                crate::AppError::Internal(format!("Failed to parse biscuit private key from file: {}", e))
                            })?;
                            let pub_key = PublicKey::from_bytes(&pub_bytes, BiscuitAlgorithm::Ed25519).map_err(|e| {
                                crate::AppError::Internal(format!("Failed to parse biscuit public key from file: {}", e))
                            })?;
                            (priv_key, pub_key)
                        } else {
                            let keypair = KeyPair::new();
                            (keypair.private(), keypair.public())
                        }
                    } else {
                        // Files not present: generate and persist
                        std::fs::create_dir_all(biscuit_key_path).map_err(|e| {
                            crate::AppError::Internal(format!("Failed to create biscuit key dir: {}", e))
                        })?;

                        let keypair = KeyPair::new();
                        // Try to serialize keys to bytes and write base64; if serialization API differs,
                        // this may need adaptation to the biscuit-auth API.
                        let priv_bytes = keypair.private().to_bytes();
                        let pub_bytes = keypair.public().to_bytes();

                        let priv_b64 = STANDARD.encode(priv_bytes);
                        let pub_b64 = STANDARD.encode(pub_bytes);

                        std::fs::write(biscuit_key_path.join("biscuit_private.b64"), priv_b64).map_err(|e| {
                            crate::AppError::Internal(format!("Failed to write biscuit private key file: {}", e))
                        })?;
                        std::fs::write(biscuit_key_path.join("biscuit_public.b64"), pub_b64).map_err(|e| {
                            crate::AppError::Internal(format!("Failed to write biscuit public key file: {}", e))
                        })?;

                        (keypair.private(), keypair.public())
                    }
                } else {
                    // 3) Fallback: generate an ephemeral keypair
                    let keypair = KeyPair::new();
                    (keypair.private(), keypair.public())
                }
            }
        };

        // Initialize Argon2
        let argon2 = Argon2::default();

        Ok(Self {
            biscuit_private_key,
            biscuit_public_key,
            database: database.clone(),
            config: config.clone(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            argon2,
        })
    }

    /// Authenticate user with username/password using AppState for DB access
    pub async fn authenticate_user(&self, state: &crate::AppState, request: LoginRequest) -> Result<crate::models::User> {
        // Lookup user via AppState DB wrapper
        let user = state.db_get_user_by_email(request.email.as_str()).await
            .map_err(|_| AuthError::UserNotFound)?;

        if !user.is_active {
            return Err(AuthError::InvalidCredentials.into());
        }

        // Verify password
        if let Some(password_hash) = &user.password_hash {
            let parsed_hash = PasswordHash::new(password_hash)
                .map_err(|e| AuthError::PasswordHash(e.to_string()))?;

            self.argon2
                .verify_password(request.password.as_bytes(), &parsed_hash)
                .map_err(|_| AuthError::InvalidCredentials)?;
        } else {
            return Err(AuthError::InvalidCredentials.into());
        }

        // Update last login via AppState wrapper
        state.db_update_last_login(user.id).await.map_err(|e| AuthError::Database(e.to_string()))?;

        // Return user (handlers will call create_session via AppState wrapper)
        Ok(user)
    }

    /// Create authentication response with Biscuit access & refresh tokens
    pub async fn create_auth_response(&self, user: User, remember_me: bool) -> Result<AuthResponse> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // TTL は設定値を使用 (remember_me なら refresh TTL の 1/2 を access にする等のポリシーも可能)
    let access_secs_cfg = self.config.access_token_ttl_secs as i64;
    let refresh_secs_cfg = self.config.refresh_token_ttl_secs as i64;
        let access_ttl = if remember_me {
            ChronoDuration::seconds(access_secs_cfg * 2).min(ChronoDuration::seconds(refresh_secs_cfg))
        } else {
            ChronoDuration::seconds(access_secs_cfg)
        };
        let refresh_ttl = ChronoDuration::seconds(refresh_secs_cfg);

        let access_exp = now + access_ttl;
        let refresh_exp = now + refresh_ttl;
        let refresh_version: u32 = 1;

        // セッション: refresh の最長寿命 (30d) を expires_at に採用
        let session_data = SessionData {
            user_id: user.id,
            username: user.username.clone(),
            role: UserRole::parse_str(&user.role).unwrap_or(UserRole::Subscriber),
            created_at: now,
            expires_at: refresh_exp,
            last_accessed: now,
            refresh_version,
        };
        self.sessions.write().await.insert(session_id.clone(), session_data);

        // Biscuit access & refresh 発行
        let access_token = self.build_biscuit_token(&user, &session_id, refresh_version, "access", access_exp.timestamp())?;
        let refresh_token = self.build_biscuit_token(&user, &session_id, refresh_version, "refresh", refresh_exp.timestamp())?;

        Ok(AuthResponse {
            user: UserInfo::from(user),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
            biscuit_token: access_token, // 後方互換: biscuit_token = access_token
            expires_in: (access_exp - now).num_seconds(),
            session_id,
        })
    }

    /// Refresh tokens (access + rotated refresh) using current valid refresh token.
    /// 仕様:
    /// - refresh JWT の jti は "<session_id>_refresh_v<version>" 形式
    /// - セッションに保存している refresh_version と一致した場合のみ有効
    /// - 使用成功時に refresh_version をインクリメントし新しい refresh_token を発行 (旧トークンは無効化)
    /// - アクセストークンは 1h、リフレッシュは都度 30d (スライディング) とする
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<RefreshResponse> {
        // refresh biscuit を検証し、version/期限/セッション整合性を確認後、
        // 新しい access & refresh biscuit を発行する。
        let parsed = self.parse_biscuit(refresh_token)?;
        if parsed.token_type != "refresh" { return Err(AuthError::InvalidToken.into()); }

        // セッション確認 & バージョン一致
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&parsed.session_id).ok_or(AuthError::InvalidToken)?;
        if session.expires_at < Utc::now() { return Err(AuthError::TokenExpired.into()); }
        if session.refresh_version != parsed.version { return Err(AuthError::InvalidToken.into()); }

        // バージョン更新
        session.refresh_version += 1;
        let new_version = session.refresh_version;

    let now = Utc::now();
    let access_exp = now + ChronoDuration::seconds(self.config.access_token_ttl_secs as i64);
    let refresh_exp = now + ChronoDuration::seconds(self.config.refresh_token_ttl_secs as i64);

        let user = self.database.get_user_by_id(parsed.user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        if !user.is_active { return Err(AuthError::InvalidCredentials.into()); }

        let access_token = self.build_biscuit_token(&user, &parsed.session_id, new_version, "access", access_exp.timestamp())?;
        let new_refresh_token = self.build_biscuit_token(&user, &parsed.session_id, new_version, "refresh", refresh_exp.timestamp())?;

        Ok(RefreshResponse { access_token, expires_in: (access_exp - now).num_seconds(), session_id: parsed.session_id, refresh_token: new_refresh_token })
    }

    /// Build biscuit token with required facts
    fn build_biscuit_token(&self, user: &User, session_id: &str, version: u32, token_type: &str, exp_unix: i64) -> Result<String> {
        let mut program = String::new();
        program.push_str(&format!("user(\"{}\", \"{}\", \"{}\");\n", user.id, user.username, user.role));
        program.push_str(&format!("token_type(\"{}\");\n", token_type));
        program.push_str(&format!("exp({});\n", exp_unix));
        program.push_str(&format!("session(\"{}\", {});\n", session_id, version));

        let builder: BiscuitBuilder = biscuit_auth::Biscuit::builder();
        let builder = builder.code(&program).map_err(|e| AuthError::Biscuit(format!("Failed to build biscuit facts: {}", e)))?;
        let keypair = KeyPair::from(&self.biscuit_private_key);
        let token = builder.build(&keypair).map_err(|e| AuthError::Biscuit(format!("Failed to sign biscuit: {}", e)))?;
        let b64 = token.to_base64().map_err(|e| AuthError::Biscuit(format!("Failed to serialize biscuit token: {}", e)))?;
        Ok(b64)
    }

    /// (旧互換) verify_jwt -> Biscuit access 検証
    pub async fn verify_jwt(&self, token: &str) -> Result<AuthContext> { self.verify_biscuit_generic(token, Some("access")).await }

    /// Biscuit トークン検証 (AppState 経由でユーザー確認 & メトリクス計測用ラッパーと組み合わせて利用)
    ///
    /// 直接 DB コネクションを取得せず、`AppState` の `db_get_user_by_id` を利用することで
    /// メトリクスと一貫した DB アクセス経路を確保する。
    pub async fn verify_biscuit(&self, state: &crate::AppState, token: &str) -> Result<AuthContext> {
        self.verify_biscuit_generic(token, None).await?; // 署名 & exp など基本検証
        // 既存仕様: user fact 取得 (verify_biscuit_generic 内で抽出できるよう統合)
        let parsed = self.parse_biscuit(token)?;
        // Verify user (DB)
        let user = state.db_get_user_by_id(parsed.user_id).await.map_err(|_| AuthError::UserNotFound)?;
        if !user.is_active { return Err(AuthError::InvalidCredentials.into()); }
    let role_clone = parsed.role.clone();
    Ok(AuthContext { user_id: parsed.user_id, username: parsed.username, role: parsed.role, session_id: parsed.session_id, permissions: self.get_role_permissions(role_clone.as_str()) })
    }

    fn parse_biscuit(&self, token: &str) -> Result<ParsedBiscuit> {
        let unverified = biscuit_auth::UnverifiedBiscuit::from_base64(token)
            .map_err(|e| AuthError::Biscuit(format!("Failed to parse biscuit token: {}", e)))?;
        let key_provider = |_opt_root_id: Option<u32>| -> std::result::Result<PublicKey, BiscuitFormat> { Ok(self.biscuit_public_key) };
        let biscuit = unverified.verify(key_provider)
            .map_err(|e| AuthError::Biscuit(format!("Biscuit signature verification failed: {}", e)))?;
        let mut authorizer = biscuit.authorizer().map_err(|e| AuthError::Biscuit(format!("Failed to create authorizer: {}", e)))?;
        let _ = authorizer.authorize().map_err(|e| AuthError::Biscuit(format!("Authorizer run failed: {}", e)))?;

        // user
        let user_q = r#"data($id,$u,$r) <- user($id,$u,$r)"#;
        let usr: Vec<(String,String,String)> = authorizer.query_all(user_q)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query user facts: {}", e)))?;
        if usr.is_empty() { return Err(AuthError::InvalidToken.into()); }
        let (id_s, username, role_s) = usr[0].clone();
        let user_id = Uuid::parse_str(&id_s).map_err(|_| AuthError::InvalidToken)?;
        let role = UserRole::parse_str(&role_s).map_err(|_| AuthError::InvalidToken)?;

        // token_type
        let type_q = r#"data($t) <- token_type($t)"#;
        let types: Vec<(String,)> = authorizer.query_all(type_q)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query token_type: {}", e)))?;
        if types.is_empty() { return Err(AuthError::InvalidToken.into()); }
        let token_type = types[0].0.clone();

        // exp
        let exp_q = r#"data($e) <- exp($e)"#;
        let exps: Vec<(i64,)> = authorizer.query_all(exp_q)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query exp: {}", e)))?;
        if exps.is_empty() { return Err(AuthError::InvalidToken.into()); }
    let exp = exps[0].0;
    if exp < Utc::now().timestamp() { return Err(AuthError::TokenExpired.into()); }

        // session
        let sess_q = r#"data($sid,$v) <- session($sid,$v)"#;
        let sess: Vec<(String,i64)> = authorizer.query_all(sess_q)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query session: {}", e)))?;
        if sess.is_empty() { return Err(AuthError::InvalidToken.into()); }
        let (session_id, version_i) = sess[0].clone();
        let version: u32 = version_i as u32;

    Ok(ParsedBiscuit { user_id, username, role, token_type, session_id, version })
    }

    async fn verify_biscuit_generic(&self, token: &str, expect_type: Option<&str>) -> Result<AuthContext> {
        let parsed = self.parse_biscuit(token)?;
        if let Some(t) = expect_type { if parsed.token_type != t { return Err(AuthError::InvalidToken.into()); } }
        // セッション整合性 (存在 / 期限 / version は access では version 一致のみ任意だが整合性優先で確認)
        let mut sessions = self.sessions.write().await;
        if let Some(sess) = sessions.get_mut(&parsed.session_id) {
            if sess.expires_at < Utc::now() { return Err(AuthError::TokenExpired.into()); }
            // last_accessed 更新
            sess.last_accessed = Utc::now();
            // access の場合は version <= stored_version を許可 (新しい refresh で version が進むため)
            if parsed.token_type == "access" && parsed.version > sess.refresh_version { return Err(AuthError::InvalidToken.into()); }
            // refresh の場合は厳密一致を要求 (refresh_access_token で再発行済みなら旧は拒否)
            if parsed.token_type == "refresh" && parsed.version != sess.refresh_version { return Err(AuthError::InvalidToken.into()); }
        } else {
            return Err(AuthError::InvalidToken.into());
        }
    let role_clone = parsed.role.clone();
    Ok(AuthContext { user_id: parsed.user_id, username: parsed.username, role: parsed.role, session_id: parsed.session_id, permissions: self.get_role_permissions(role_clone.as_str()) })
    }

    /// Get permissions for a role
    fn get_role_permissions(&self, role: &str) -> Vec<String> {
        match role {
            "SuperAdmin" => vec![
                "admin".to_string(),
                "read".to_string(),
                "write".to_string(),
                "delete".to_string(),
            ],
            "Admin" => vec![
                "read".to_string(),
                "write".to_string(),
                "delete".to_string(),
            ],
            "Editor" => vec!["read".to_string(), "write".to_string()],
            "Author" => vec!["read".to_string(), "write_own".to_string()],
            _ => vec!["read".to_string()],
        }
    }

    /// Hash password using Argon2
    pub fn hash_password(&self, password: &str) -> Result<String> {
        password::hash_password(password)
    }

    /// Verify password against hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        password::verify_password(password, hash)
    }

    /// Logout user (invalidate session)
    pub async fn logout(&self, session_id: &str) -> Result<()> {
        self.sessions.write().await.remove(session_id);
        Ok(())
    }

    /// Clean expired sessions
    pub async fn cleanup_expired_sessions(&self) {
        let now = Utc::now();
        let mut sessions = self.sessions.write().await;

        sessions.retain(|_, session| session.expires_at > now);
    }

    /// Get active session count
    pub async fn get_active_session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Health check for auth service
    pub async fn health_check(&self) -> Result<()> {
        // Check database connection
        let _conn = self.get_conn()?;
        Ok(())
    }

    /// Convenience helper to get a pooled DB connection and map errors to AuthError
    fn get_conn(&self) -> std::result::Result<crate::database::PooledConnection, AuthError> {
        self.database
            .get_connection()
            .map_err(|e| AuthError::Database(e.to_string()))
    }

    /// Create user using AppState so metrics are recorded centrally
    pub async fn create_user(&self, state: &crate::AppState, request: CreateUserRequest) -> Result<User> {
        state.db_create_user(request).await
    }

    /// Backwards-compatible wrapper kept for call sites that used the old helper
    pub async fn create_user_with_state(&self, state: &crate::AppState, request: CreateUserRequest) -> Result<User> {
        self.create_user(state, request).await
    }

    /// Validate token (Biscuit access) and return user
    pub async fn validate_token(&self, state: &crate::AppState, token: &str) -> Result<crate::models::User> {
        let parsed = self.verify_biscuit_generic(token, Some("access")).await?;
        let user = state.db_get_user_by_id(parsed.user_id).await.map_err(|_| AuthError::UserNotFound)?;
        if !user.is_active { return Err(AuthError::InvalidCredentials.into()); }
        Ok(user)
    }

    /// Create session token (Biscuit access) - retained for API 互換, returns access biscuit
    pub async fn create_session(&self, user_id: Uuid, state: &crate::AppState) -> Result<String> {
        let user = state.db_get_user_by_id(user_id).await.map_err(|_| AuthError::UserNotFound)?;
        if !user.is_active { return Err(AuthError::InvalidCredentials.into()); }
        // シンプル: create_auth_response 相当を再利用する代わりにミニマル access biscuit を返す
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let access_exp = now + ChronoDuration::hours(1);
        let session_data = SessionData { user_id, username: user.username.clone(), role: UserRole::parse_str(&user.role).unwrap_or(UserRole::Subscriber), created_at: now, expires_at: now + ChronoDuration::days(30), last_accessed: now, refresh_version: 1 };
        self.sessions.write().await.insert(session_id.clone(), session_data);
        let token = self.build_biscuit_token(&user, &session_id, 1, "access", access_exp.timestamp())?;
        Ok(token)
    }
}
