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

use argon2::Argon2;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use biscuit_auth::{
    Algorithm as BiscuitAlgorithm, KeyPair, PrivateKey, PublicKey, builder::BiscuitBuilder,
    error::Format as BiscuitFormat,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    Result,
    config::AuthConfig,
    database::Database,
    models::{CreateUserRequest, User, UserRole},
    utils::{common_types::UserInfo, password},
};

// --- Key file helper funcs (extracted for reduced complexity in try_load_dir_keys) ---
fn read_file_string(path: &std::path::Path, label: &str) -> crate::Result<String> {
    std::fs::read_to_string(path).map_err(|e| {
    crate::AppError::Internal(format!("Failed reading biscuit {label} key file: {e}", label = label, e = e))
    })
}
fn decode_key_b64(data: &str, label: &str) -> crate::Result<Vec<u8>> {
    STANDARD.decode(data).map_err(|e| {
    crate::AppError::Internal(format!("Failed to decode biscuit {label} key b64: {e}", label = label, e = e))
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

impl AuthService {
    /// Testing helper: clears all sessions (used by integration tests)
    pub async fn clear_sessions_for_test(&self) {
        self.sessions.write().await.clear();
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
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct AuthResponse {
    pub user: UserInfo,
    /// Canonical nested tokens container (used by handlers to build AuthSuccessResponse)
    pub tokens: crate::utils::auth_response::AuthTokens,
}

// RefreshResponse removed: refresh now returns AuthTokens + User directly at handler layer.

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
    // --- Key loading helpers (duplication reduction) ---
    fn try_load_env_keys() -> Option<(PrivateKey, PublicKey)> {
        let priv_b64 = std::env::var("BISCUIT_PRIVATE_KEY_B64").ok()?;
        let pub_b64 = std::env::var("BISCUIT_PUBLIC_KEY_B64").ok()?;
        let priv_bytes = STANDARD.decode(&priv_b64).ok()?;
        let pub_bytes = STANDARD.decode(&pub_b64).ok()?;
        let priv_key = PrivateKey::from_bytes(&priv_bytes, BiscuitAlgorithm::Ed25519).ok()?;
        let pub_key = PublicKey::from_bytes(&pub_bytes, BiscuitAlgorithm::Ed25519).ok()?;
        Some((priv_key, pub_key))
    }

    fn generate_and_persist(dir: &std::path::Path) -> crate::Result<(PrivateKey, PublicKey)> {
        std::fs::create_dir_all(dir).map_err(|e| {
            crate::AppError::Internal(format!("Failed to create biscuit key dir: {}", e))
        })?;
        let kp = KeyPair::new();
        let priv_b64 = STANDARD.encode(kp.private().to_bytes());
        let pub_b64 = STANDARD.encode(kp.public().to_bytes());
        std::fs::write(dir.join("biscuit_private.b64"), &priv_b64).map_err(|e| {
            crate::AppError::Internal(format!("Failed to write biscuit private key file: {}", e))
        })?;
        std::fs::write(dir.join("biscuit_public.b64"), &pub_b64).map_err(|e| {
            crate::AppError::Internal(format!("Failed to write biscuit public key file: {}", e))
        })?;
        Ok((kp.private(), kp.public()))
    }

    fn generate_ephemeral() -> (PrivateKey, PublicKey) {
        let kp = KeyPair::new();
        (kp.private(), kp.public())
    }
    #[inline]
    fn ensure_active(&self, user: &User) -> Result<()> {
        if !user.is_active {
            return Err(AuthError::InvalidCredentials.into());
        }
        Ok(())
    }
    // ---------------- Internal helpers (duplication reduction) ----------------
    fn compute_expiries(&self, remember_me: bool) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        let access_cfg = self.config.access_token_ttl_secs as i64;
        let refresh_cfg = self.config.refresh_token_ttl_secs as i64;
        // remember_me の場合 access を最大 refresh 長まで (従来: *2 か refresh で min)
        let access_ttl = if remember_me {
            ChronoDuration::seconds(access_cfg * 2).min(ChronoDuration::seconds(refresh_cfg))
        } else {
            ChronoDuration::seconds(access_cfg)
        };
        let refresh_ttl = ChronoDuration::seconds(refresh_cfg);
        (now + access_ttl, now + refresh_ttl)
    }

    async fn insert_session(
        &self,
        user: &User,
        session_id: &str,
        refresh_exp: DateTime<Utc>,
        refresh_version: u32,
    ) {
        let now = Utc::now();
        let data = SessionData {
            user_id: user.id,
            username: user.username.clone(),
            role: UserRole::parse_str(&user.role).unwrap_or(UserRole::Subscriber),
            created_at: now,
            expires_at: refresh_exp,
            last_accessed: now,
            refresh_version,
        };
        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), data);
    }

    fn issue_access_and_refresh(
        &self,
        user: &User,
        session_id: &str,
        version: u32,
        access_exp: DateTime<Utc>,
        refresh_exp: DateTime<Utc>,
    ) -> Result<(String, String, i64)> {
        let access_token =
            self.build_biscuit_token(user, session_id, version, "access", access_exp.timestamp())?;
        let refresh_token = self.build_biscuit_token(
            user,
            session_id,
            version,
            "refresh",
            refresh_exp.timestamp(),
        )?;
        Ok((
            access_token,
            refresh_token,
            (access_exp - Utc::now()).num_seconds(),
        ))
    }

    // ---------------- Biscuit fact query helpers ----------------
    fn biscuit_query_triple(
        &self,
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<(String, String, String)> {
        let v: Vec<(String, String, String)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {}: {}", ctx, e)))?;
        v.into_iter().next().ok_or(AuthError::InvalidToken.into())
    }

    fn biscuit_query_string(
        &self,
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<String> {
        let v: Vec<(String,)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {}: {}", ctx, e)))?;
        v.into_iter()
            .next()
            .map(|t| t.0)
            .ok_or(AuthError::InvalidToken.into())
    }

    fn biscuit_query_i64(
        &self,
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<i64> {
        let v: Vec<(i64,)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {}: {}", ctx, e)))?;
        v.into_iter()
            .next()
            .map(|t| t.0)
            .ok_or(AuthError::InvalidToken.into())
    }

    fn biscuit_query_session(
        &self,
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<(String, u32)> {
        let v: Vec<(String, i64)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {}: {}", ctx, e)))?;
        let (sid, ver_i) = v.into_iter().next().ok_or(AuthError::InvalidToken)?;
        Ok((sid, ver_i as u32))
    }

    /// Create new authentication service (Biscuit 専用)
    pub async fn new(config: &AuthConfig, database: &Database) -> Result<Self> {
        // Attempt to load biscuit keypair from environment (base64 encoded), otherwise generate.
        // Assumption: `PrivateKey` and `PublicKey` provide byte (de)serialization APIs.
        // If the biscuit-auth types differ, adapt loading to the concrete API (e.g. from_pem/from_slice).
        // Determine biscuit keys via env, config directory, or generation and persist when appropriate
        let (biscuit_private_key, biscuit_public_key) =
            if let Some(pair) = Self::try_load_env_keys() {
                pair
            } else {
                let path = std::path::Path::new(&config.biscuit_root_key);
                if !config.biscuit_root_key.is_empty() && path.exists() && path.is_dir() {
                    if path.join("biscuit_private.b64").exists()
                        && path.join("biscuit_public.b64").exists()
                    {
                        let priv_key = read_biscuit_private_key(&path.join("biscuit_private.b64"))?;
                        let pub_key = read_biscuit_public_key(&path.join("biscuit_public.b64"))?;
                        (priv_key, pub_key)
                    } else {
                        Self::generate_and_persist(path)?
                    }
                } else {
                    Self::generate_ephemeral()
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
    pub async fn authenticate_user(
        &self,
        state: &crate::AppState,
        request: LoginRequest,
    ) -> Result<crate::models::User> {
        // Lookup user via AppState DB wrapper
        let user = state
            .db_get_user_by_email(request.email.as_str())
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        self.ensure_active(&user)?;

        // Verify password
        if let Some(password_hash) = &user.password_hash {
            match password::verify_password(&request.password, password_hash) {
                Ok(true) => {}
                Ok(false) => return Err(AuthError::InvalidCredentials.into()),
                Err(e) => return Err(AuthError::PasswordHash(e.to_string()).into()),
            }
        } else {
            return Err(AuthError::InvalidCredentials.into());
        }

        // Update last login via AppState wrapper
        state
            .db_update_last_login(user.id)
            .await
            .map_err(|e| AuthError::Database(e.to_string()))?;

        // Return user (handlers will call create_session via AppState wrapper)
        Ok(user)
    }

    /// Create authentication response with Biscuit access & refresh tokens
    pub async fn create_auth_response(
        &self,
        user: User,
        remember_me: bool,
    ) -> Result<AuthResponse> {
        let session_id = Uuid::new_v4().to_string();
        let (access_exp, refresh_exp) = self.compute_expiries(remember_me);
        let refresh_version = 1u32;
        self.insert_session(&user, &session_id, refresh_exp, refresh_version)
            .await;
        let (access_token, refresh_token, expires_in) = self.issue_access_and_refresh(
            &user,
            &session_id,
            refresh_version,
            access_exp,
            refresh_exp,
        )?;
        let tokens = crate::utils::auth_response::AuthTokens {
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
            biscuit_token: access_token,
            expires_in,
            session_id,
        };
        Ok(AuthResponse {
            user: UserInfo::from(user),
            tokens,
        })
    }

    /// Refresh tokens (access + rotated refresh) using current valid refresh token.
    /// 仕様:
    /// - refresh JWT の jti は "<session_id>_refresh_v<version>" 形式
    /// - セッションに保存している refresh_version と一致した場合のみ有効
    /// - 使用成功時に refresh_version をインクリメントし新しい refresh_token を発行 (旧トークンは無効化)
    /// - アクセストークンは 1h、リフレッシュは都度 30d (スライディング) とする
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<(crate::utils::auth_response::AuthTokens, UserInfo)> {
        let parsed = self.parse_refresh_biscuit(refresh_token)?; // token_type 検証込み
        let (new_version, user) = self.bump_and_load_user(&parsed).await?;
        let (access_exp, refresh_exp) = self.compute_expiries(false);
        let (access_token, new_refresh_token, expires_in) = self.issue_access_and_refresh(
            &user,
            &parsed.session_id,
            new_version,
            access_exp,
            refresh_exp,
        )?;
        let user_info = UserInfo::from(&user);
        let tokens = crate::utils::auth_response::AuthTokens {
            access_token: access_token.clone(),
            refresh_token: new_refresh_token,
            biscuit_token: access_token.clone(),
            expires_in,
            session_id: parsed.session_id,
        };
        Ok((tokens, user_info))
    }

    // セッション version をインクリメントしユーザーを取得 (有効性検査込み)
    async fn bump_and_load_user(
        &self,
        parsed: &ParsedBiscuit,
    ) -> Result<(u32, crate::models::User)> {
        let new_version = {
            let mut sessions = self.sessions.write().await;
            self.validate_and_bump_refresh_session(parsed, &mut sessions)?
        };
        let user = self
            .database
            .get_user_by_id(parsed.user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        self.ensure_active(&user)?;
        Ok((new_version, user))
    }

    fn parse_refresh_biscuit(&self, token: &str) -> Result<ParsedBiscuit> {
        let parsed = self.parse_biscuit(token)?;
        if parsed.token_type != "refresh" {
            return Err(AuthError::InvalidToken.into());
        }
        Ok(parsed)
    }

    fn validate_and_bump_refresh_session(
        &self,
        parsed: &ParsedBiscuit,
        sessions: &mut std::collections::HashMap<String, SessionData>,
    ) -> Result<u32> {
        let session = sessions
            .get_mut(&parsed.session_id)
            .ok_or(AuthError::InvalidToken)?;
        if session.expires_at < Utc::now() {
            return Err(AuthError::TokenExpired.into());
        }
        if session.refresh_version != parsed.version {
            return Err(AuthError::InvalidToken.into());
        }
        session.refresh_version += 1;
        Ok(session.refresh_version)
    }

    /// Build biscuit token with required facts
    fn build_biscuit_token(
        &self,
        user: &User,
        session_id: &str,
        version: u32,
        token_type: &str,
        exp_unix: i64,
    ) -> Result<String> {
        let mut program = String::new();
        program.push_str(&format!(
            "user(\"{id}\", \"{username}\", \"{role}\");\n",
            id = user.id,
            username = user.username,
            role = user.role
        ));
        program.push_str(&format!("token_type(\"{t}\");\n", t = token_type));
        program.push_str(&format!("exp({e});\n", e = exp_unix));
        program.push_str(&format!("session(\"{sid}\", {v});\n", sid = session_id, v = version));

        let builder: BiscuitBuilder = biscuit_auth::Biscuit::builder();
        let builder = builder
            .code(&program)
            .map_err(|e| AuthError::Biscuit(format!("Failed to build biscuit facts: {}", e)))?;
        let keypair = KeyPair::from(&self.biscuit_private_key);
        let token = builder
            .build(&keypair)
            .map_err(|e| AuthError::Biscuit(format!("Failed to sign biscuit: {}", e)))?;
        let b64 = token
            .to_base64()
            .map_err(|e| AuthError::Biscuit(format!("Failed to serialize biscuit token: {}", e)))?;
        Ok(b64)
    }

    /// (旧互換) verify_jwt -> Biscuit access 検証 (deprecated, behind legacy-auth-flat for eventual removal)
    #[cfg(feature = "legacy-auth-flat")]
    #[deprecated(
        note = "Use verify_biscuit(state, token) or auth_middleware injected AuthContext (will be removed in 3.0.0)"
    )]
    pub async fn verify_jwt(&self, token: &str) -> Result<AuthContext> {
        self.verify_biscuit_generic(token, Some("access")).await
    }

    /// Biscuit トークン検証 (AppState 経由でユーザー確認 & メトリクス計測用ラッパーと組み合わせて利用)
    ///
    /// 直接 DB コネクションを取得せず、`AppState` の `db_get_user_by_id` を利用することで
    /// メトリクスと一貫した DB アクセス経路を確保する。
    pub async fn verify_biscuit(
        &self,
        state: &crate::AppState,
        token: &str,
    ) -> Result<AuthContext> {
        let (ctx, _user) = self.verify_biscuit_with_user(state, token).await?;
        Ok(ctx)
    }

    /// Biscuit 検証＋ユーザー取得（有効性検査込み）を一度で行い、DB 取得の重複を避ける
    pub async fn verify_biscuit_with_user(
        &self,
        state: &crate::AppState,
        token: &str,
    ) -> Result<(AuthContext, crate::models::User)> {
        let ctx = self.verify_biscuit_generic(token, None).await?;
        let user = state
            .db_get_user_by_id(ctx.user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        self.ensure_active(&user)?;
        Ok((ctx, user))
    }

    fn parse_biscuit(&self, token: &str) -> Result<ParsedBiscuit> {
        let unverified = biscuit_auth::UnverifiedBiscuit::from_base64(token)
            .map_err(|e| AuthError::Biscuit(format!("Failed to parse biscuit token: {}", e)))?;
        let key_provider =
            |_opt_root_id: Option<u32>| -> std::result::Result<PublicKey, BiscuitFormat> {
                Ok(self.biscuit_public_key)
            };
        let biscuit = unverified.verify(key_provider).map_err(|e| {
            AuthError::Biscuit(format!("Biscuit signature verification failed: {}", e))
        })?;
        let mut authorizer = biscuit
            .authorizer()
            .map_err(|e| AuthError::Biscuit(format!("Failed to create authorizer: {}", e)))?;
        let _ = authorizer
            .authorize()
            .map_err(|e| AuthError::Biscuit(format!("Authorizer run failed: {}", e)))?;
        let (id_s, username, role_s) = self.biscuit_query_triple(
            &mut authorizer,
            "data($id,$u,$r) <- user($id,$u,$r)",
            "user facts",
        )?;
        let user_id = Uuid::parse_str(&id_s).map_err(|_| AuthError::InvalidToken)?;
        let role = UserRole::parse_str(&role_s).map_err(|_| AuthError::InvalidToken)?;
        let token_type = self.biscuit_query_string(
            &mut authorizer,
            "data($t) <- token_type($t)",
            "token_type",
        )?;
    let exp = self.biscuit_query_i64(&mut authorizer, "data($e) <- exp($e)", "exp")?;
        if exp < Utc::now().timestamp() {
            return Err(AuthError::TokenExpired.into());
        }
        let (session_id, version) = self.biscuit_query_session(
            &mut authorizer,
            "data($sid,$v) <- session($sid,$v)",
            "session",
        )?;
        Ok(ParsedBiscuit {
            user_id,
            username,
            role,
            token_type,
            session_id,
            version,
        })
    }

    async fn verify_biscuit_generic(
        &self,
        token: &str,
        expect_type: Option<&str>,
    ) -> Result<AuthContext> {
        let parsed = self.parse_and_check(token, expect_type)?;
        // セッション整合性チェックを専用ヘルパーへ委譲
        self.validate_session_consistency(&parsed).await?;
        Ok(self.build_auth_context(&parsed))
    }

    /// セッション存在・期限・バージョン整合性を検証し、last_accessed を更新
    async fn validate_session_consistency(&self, parsed: &ParsedBiscuit) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let sess = sessions
            .get_mut(&parsed.session_id)
            .ok_or(AuthError::InvalidToken)?;
        if sess.expires_at < now {
            return Err(AuthError::TokenExpired.into());
        }
        // last_accessed 更新
        sess.last_accessed = now;
        // access の場合は version <= stored_version を許可 (新しい refresh で version が進むため)
        if parsed.token_type == "access" && parsed.version > sess.refresh_version {
            return Err(AuthError::InvalidToken.into());
        }
        // refresh の場合は厳密一致を要求 (refresh_access_token で再発行済みなら旧は拒否)
        if parsed.token_type == "refresh" && parsed.version != sess.refresh_version {
            return Err(AuthError::InvalidToken.into());
        }
        Ok(())
    }

    #[inline]
    fn ensure_token_type(&self, expect_type: Option<&str>, actual: &str) -> Result<()> {
        if let Some(t) = expect_type
            && actual != t
        {
            return Err(AuthError::InvalidToken.into());
        }
        Ok(())
    }

    #[inline]
    fn build_auth_context(&self, parsed: &ParsedBiscuit) -> AuthContext {
        let role_clone = parsed.role.clone();
        AuthContext {
            user_id: parsed.user_id,
            username: parsed.username.clone(),
            role: parsed.role.clone(),
            session_id: parsed.session_id.clone(),
            permissions: self.get_role_permissions(role_clone.as_str()),
        }
    }

    #[inline]
    fn parse_and_check(&self, token: &str, expect_type: Option<&str>) -> Result<ParsedBiscuit> {
        let parsed = self.parse_biscuit(token)?;
        self.ensure_token_type(expect_type, &parsed.token_type)?;
        Ok(parsed)
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
    pub async fn create_user(
        &self,
        state: &crate::AppState,
        request: CreateUserRequest,
    ) -> Result<User> {
        state.db_create_user(request).await
    }

    /// Backwards-compatible wrapper kept for call sites that used the old helper
    pub async fn create_user_with_state(
        &self,
        state: &crate::AppState,
        request: CreateUserRequest,
    ) -> Result<User> {
        self.create_user(state, request).await
    }

    /// Validate token (Biscuit access) and return user
    pub async fn validate_token(
        &self,
        state: &crate::AppState,
        token: &str,
    ) -> Result<crate::models::User> {
        let (_ctx, user) = self.verify_biscuit_with_user(state, token).await?;
        Ok(user)
    }

    /// Create session token (Biscuit access) - retained for API 互換, returns access biscuit
    pub async fn create_session(&self, user_id: Uuid, state: &crate::AppState) -> Result<String> {
        let user = state
            .db_get_user_by_id(user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        self.ensure_active(&user)?;
        let session_id = Uuid::new_v4().to_string();
        let (access_exp, refresh_exp) = self.compute_expiries(false);
        self.insert_session(&user, &session_id, refresh_exp, 1)
            .await;
        let token =
            self.build_biscuit_token(&user, &session_id, 1, "access", access_exp.timestamp())?;
        Ok(token)
    }
}

/// Helper function to check if user has admin permissions
pub fn require_admin_permission(auth_context: &AuthContext) -> crate::Result<()> {
    if auth_context.permissions.iter().any(|p| p == "admin") {
        Ok(())
    } else {
        Err(AuthError::InsufficientPermissions.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn require_admin_permission_allows_admin() {
        let ctx = AuthContext {
            user_id: Uuid::new_v4(),
            username: "admin_user".to_string(),
            role: UserRole::Admin,
            session_id: "s1".to_string(),
            permissions: vec!["read".to_string(), "admin".to_string()],
        };
        assert!(require_admin_permission(&ctx).is_ok());
    }

    #[test]
    fn require_admin_permission_denies_non_admin() {
        let ctx = AuthContext {
            user_id: Uuid::new_v4(),
            username: "normal_user".to_string(),
            role: UserRole::Subscriber,
            session_id: "s2".to_string(),
            permissions: vec!["read".to_string()],
        };
        let res = require_admin_permission(&ctx);
        assert!(res.is_err());
        // Ensure the error maps to AppError::Authorization when converted
        let app_err: crate::AppError = res.unwrap_err();
        match app_err {
            crate::AppError::Authorization(_) => {}
            other => panic!("expected Authorization error, got {:?}", other),
        }
    }
}
