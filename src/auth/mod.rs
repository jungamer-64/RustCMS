//! Authentication Service - Biscuit (統一版)
//!
//! リファクタリング概要 (2025-09):
//! - Key 管理: `PrivateKey` / `PublicKey` から毎回 `KeyPair` を再生成していた非効率を解消し、起動時に確定した `KeyPair` を保持
//! - `remember_me` TTL: 旧仕様 ("2倍 or refresh 以下") の曖昧さを廃止し、通常 = `config.access_token_ttl_secs` / remember_me = 24h 固定
//! - セッションストレージ: `HashMap` 直使用を排除し `SessionStore` trait + `InMemorySessionStore` 抽象化 (将来 Redis/Postgres 差替え容易化)
//! - Refresh Token 並行リクエスト: ポリシーを明示 (旧トークン即失効)。version ミスマッチは `InvalidToken`
//! - 有効期限検証: Biscuit の `exp` fact を parse 時に必ず検証 (期限切れは `TokenExpired`)
//! - テスト容易性: セッション操作は trait 経由。全削除は `#[cfg(test)]` のみ公開。
//!
//! 既存の機能説明は下記オリジナルコメントを継承。

//! 目的: 既存の JWT / Biscuit 併用実装を廃止し、Biscuit トークンのみで
//! アクセス/リフレッシュ (スライディングセッション) を提供する。
//!
//! 提供機能:
//! - Biscuit 署名トークン (access / refresh の2種類)
//! - `WebAuthn` (未改変・今後拡張用プレースホルダ)
//! - Argon2 パスワード検証
//! - RBAC (role -> permissions マッピング)
//!
//! トークン仕様 (更新後):
//! - `access biscuit`: 有効期限 `config.access_token_ttl_secs` (`remember_me=false`) / 24h 固定 (`remember_me`=true)
//! - `refresh biscuit`: 有効期限 30d (設定値) / 使用時に `refresh_version` +1 し旧 refresh トークン即失効
//! - Biscuit 内 facts:
//! ```text
//! user("<uuid>", "<username>", "<role>");
//! token_type("access"|"refresh");
//! exp(<unix_ts>);          // 失効時刻 (秒)
//! session("<session_id>", <version>);
//! ```
//! - refresh 使用時: version インクリメント -> 旧トークンは version ミスマッチで無効化 (並行リクエスト対策)
//! - セッション状態: `SessionStore` 抽象 (現状 `InMemory`)。

use argon2::Argon2;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use biscuit_auth::{
    Algorithm as BiscuitAlgorithm, KeyPair, PrivateKey, PublicKey, builder::BiscuitBuilder,
    error::Format as BiscuitFormat,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap; // InMemory 実装でのみ使用
use std::fmt::Write;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    Result,
    config::AuthConfig,
    models::{User, UserRole},
    repositories::UserRepository,
    utils::{common_types::{UserInfo, SessionId}, password},
};
use std::future::Future;
use std::pin::Pin;

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

#[inline]
fn mask_session_id(sid: &SessionId) -> String {
    let s = sid.as_ref();
    if s.len() <= 6 {
        return "***".to_string();
    }
    format!("{}…{}", &s[..3], &s[s.len() - 3..])
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
            AuthError::InsufficientPermissions => Self::Authorization(err.to_string()),
            _ => Self::Authentication(err.to_string()),
        }
    }
}

//================ Session Store 抽象化 ==================
#[allow(async_fn_in_trait)]
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub trait SessionStore: Send + Sync {
    fn insert(&self, id: SessionId, data: SessionData) -> BoxFuture<'_, ()>;
    fn remove(&self, id: SessionId) -> BoxFuture<'_, ()>;
    fn count(&self) -> BoxFuture<'_, usize>;
    fn cleanup_expired(&self, now: DateTime<Utc>) -> BoxFuture<'_, ()>;
    fn validate_access(
        &self,
        id: SessionId,
        version: u32,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<()>>;
    fn validate_and_bump_refresh(
        &self,
        id: SessionId,
        expected_version: u32,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<u32>>;
    #[cfg(test)]
    fn clear(&self) -> BoxFuture<'_, ()>;
}

pub struct InMemorySessionStore {
    inner: RwLock<HashMap<SessionId, SessionData>>, 
}
impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}
impl InMemorySessionStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[allow(clippy::significant_drop_tightening)]
impl SessionStore for InMemorySessionStore {
    fn insert(&self, id: SessionId, data: SessionData) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.inner.write().await.insert(id, data);
        })
    }
    fn remove(&self, id: SessionId) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.inner.write().await.remove(&id);
        })
    }
    fn count(&self) -> BoxFuture<'_, usize> {
        Box::pin(async move { self.inner.read().await.len() })
    }
    fn cleanup_expired(&self, now: DateTime<Utc>) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.inner.write().await.retain(|_, s| s.expires_at > now);
        })
    }
    fn validate_access(
        &self,
        id: SessionId,
        version: u32,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let mut map = self.inner.write().await;
            let sess = map.get_mut(&id).ok_or(AuthError::InvalidToken)?;
            if sess.expires_at < now {
                return Err(AuthError::TokenExpired.into());
            }
            if version > sess.refresh_version {
                return Err(AuthError::InvalidToken.into());
            }
            sess.last_accessed = now;
            Ok(())
        })
    }
    fn validate_and_bump_refresh(
        &self,
        id: SessionId,
        expected_version: u32,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<u32>> {
        Box::pin(async move {
            let mut map = self.inner.write().await;
            let sess = map.get_mut(&id).ok_or(AuthError::InvalidToken)?;
            if sess.expires_at < now {
                return Err(AuthError::TokenExpired.into());
            }
            if sess.refresh_version != expected_version {
                return Err(AuthError::InvalidToken.into());
            }
            sess.refresh_version += 1;
            sess.last_accessed = now;
            Ok(sess.refresh_version)
        })
    }
    #[cfg(test)]
    fn clear(&self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.inner.write().await.clear();
        })
    }
}

/// Authentication service
#[cfg(feature = "auth")]
pub struct AuthService {
    /// 起動時に読み込み/生成された KeyPair (再利用でパフォーマンス向上)
    biscuit_keypair: Arc<KeyPair>,
    /// 公開鍵 (検証用) - Copy 可能
    biscuit_public_key: PublicKey,
    user_repo: Arc<dyn UserRepository>,
    config: AuthConfig,
    session_store: Arc<dyn SessionStore>,
    argon2: Arc<Argon2<'static>>,
    access_ttl_secs: i64,
    refresh_ttl_secs: i64,
}

#[cfg(feature = "auth")]
impl Clone for AuthService {
    fn clone(&self) -> Self {
        Self {
            biscuit_keypair: Arc::clone(&self.biscuit_keypair),
            biscuit_public_key: self.biscuit_public_key,
            user_repo: Arc::clone(&self.user_repo),
            config: self.config.clone(),
            session_store: Arc::clone(&self.session_store),
            argon2: Arc::clone(&self.argon2),
            access_ttl_secs: self.access_ttl_secs,
            refresh_ttl_secs: self.refresh_ttl_secs,
        }
    }
}

#[cfg(feature = "auth")]
impl AuthService {
    #[cfg(test)]
    pub async fn clear_sessions_for_test(&self) {
        self.session_store.clear().await;
    }
}

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub refresh_version: u32, // 現在有効な refresh token version
}

struct ParsedBiscuit {
    user_id: Uuid,
    username: String,
    role: UserRole,
    token_type: String,
    session_id: SessionId,
    version: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub tokens: crate::utils::auth_response::AuthTokens,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub session_id: SessionId,
    pub permissions: Vec<String>,
}

#[cfg(feature = "auth")]
impl AuthService {
    // remember_me 用アクセス TTL は config から
    // ---- Key Loading (起動時のみ) ----
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
    fn generate_and_persist(dir: &std::path::Path) -> crate::Result<KeyPair> {
        std::fs::create_dir_all(dir).map_err(|e| {
            crate::AppError::Internal(format!("Failed to create biscuit key dir: {e}"))
        })?;
        let kp = KeyPair::new();
        let priv_b64 = STANDARD.encode(kp.private().to_bytes());
        let pub_b64 = STANDARD.encode(kp.public().to_bytes());
        std::fs::write(dir.join("biscuit_private.b64"), &priv_b64).map_err(|e| {
            crate::AppError::Internal(format!("Failed to write biscuit private key file: {e}"))
        })?;
        std::fs::write(dir.join("biscuit_public.b64"), &pub_b64).map_err(|e| {
            crate::AppError::Internal(format!("Failed to write biscuit public key file: {e}"))
        })?;
        Ok(kp)
    }
    fn generate_ephemeral() -> KeyPair {
        KeyPair::new()
    }

    #[inline]
    fn ensure_active(user: &User) -> Result<()> {
        if !user.is_active {
            return Err(AuthError::InvalidCredentials.into());
        }
        Ok(())
    }

    fn compute_expiries(&self, remember_me: bool) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        let access_ttl = if remember_me {
            // Safe due to config validation at startup
            ChronoDuration::seconds(i64::try_from(self.config.remember_me_access_ttl_secs).expect("validated remember_me_access_ttl_secs"))
        } else {
            ChronoDuration::seconds(self.access_ttl_secs)
        };
        let refresh_ttl = ChronoDuration::seconds(self.refresh_ttl_secs);
        (now + access_ttl, now + refresh_ttl)
    }

    async fn insert_session(
        &self,
        user: &User,
        session_id: &SessionId,
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
        self.session_store
            .insert(session_id.clone(), data)
            .await;
    }

    fn issue_access_and_refresh(
        &self,
        user: &User,
        session_id: &SessionId,
        version: u32,
        access_exp: DateTime<Utc>,
        refresh_exp: DateTime<Utc>,
    ) -> Result<(String, String, i64)> {
        let access_token = self.build_biscuit_token(
            user,
            session_id,
            version,
            "access",
            access_exp.timestamp(),
        )?;
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

    // ---- Biscuit Query Helpers ----
    fn biscuit_query_triple(
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<(String, String, String)> {
        let v: Vec<(String, String, String)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
        v.into_iter()
            .next()
            .ok_or(AuthError::InvalidToken)
            .map_err(Into::into)
    }
    fn biscuit_query_string(
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<String> {
        let v: Vec<(String,)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
        v.into_iter()
            .next()
            .map(|t| t.0)
            .ok_or(AuthError::InvalidToken)
            .map_err(Into::into)
    }
    fn biscuit_query_i64(
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<i64> {
        let v: Vec<(i64,)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
        v.into_iter()
            .next()
            .map(|t| t.0)
            .ok_or(AuthError::InvalidToken)
            .map_err(Into::into)
    }
    fn biscuit_query_session(
        authz: &mut biscuit_auth::Authorizer,
        dsl: &str,
        ctx: &str,
    ) -> Result<(SessionId, u32)> {
        let v: Vec<(String, i64)> = authz
            .query_all(dsl)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
        let (sid, ver_i) = v.into_iter().next().ok_or(AuthError::InvalidToken)?;
        let ver_u32 = u32::try_from(ver_i).map_err(|_| AuthError::InvalidToken)?;
        Ok((SessionId::from(sid), ver_u32))
    }

    /// コンフィグと DB からサービスを初期化する。
    ///
    /// # Errors
    /// キーロード/生成、または設定値変換で失敗した場合 `AppError` を返す。
    pub fn new_with_repo(config: &AuthConfig, user_repo: Arc<dyn UserRepository>) -> Result<Self> {
        let keypair = Self::load_or_generate_keypair(config)?; // complexity: 分離済み
        let argon2 = Arc::new(Argon2::default());
        let access_ttl_secs = i64::try_from(config.access_token_ttl_secs).map_err(|_| {
            crate::AppError::ConfigValidationError(
                "auth.access_token_ttl_secs is too large for i64".to_string(),
            )
        })?;
        let refresh_ttl_secs = i64::try_from(config.refresh_token_ttl_secs).map_err(|_| {
            crate::AppError::ConfigValidationError(
                "auth.refresh_token_ttl_secs is too large for i64".to_string(),
            )
        })?;
        let public_key = keypair.public();
    Ok(Self {
            biscuit_public_key: public_key,
            biscuit_keypair: Arc::new(keypair),
            user_repo,
            config: config.clone(),
            session_store: Arc::new(InMemorySessionStore::new()) as Arc<dyn SessionStore>,
            argon2,
            access_ttl_secs,
            refresh_ttl_secs,
        })
    }

    /// Backward-compatible constructor from Database
    pub fn new(config: &AuthConfig, database: &crate::database::Database) -> Result<Self> {
        let repo: Arc<dyn UserRepository> = Arc::new(database.clone()) as Arc<dyn UserRepository>;
        Self::new_with_repo(config, repo)
    }

    // keypair ロードロジックを分離し cyclomatic complexity を低減
    fn load_or_generate_keypair(config: &AuthConfig) -> Result<KeyPair> {
        if let Some(kp) = Self::try_load_env_keys() {
            return Ok(kp);
        }
        let key_str = config.biscuit_root_key.expose_secret();
        let path = std::path::Path::new(key_str);
        if !key_str.is_empty() && path.exists() && path.is_dir() {
            if path.join("biscuit_private.b64").exists() && path.join("biscuit_public.b64").exists()
            {
                let priv_key = read_biscuit_private_key(&path.join("biscuit_private.b64"))?;
                let pub_key = read_biscuit_public_key(&path.join("biscuit_public.b64"))?;
                let kp = KeyPair::from(&priv_key);
                if kp.public().to_bytes() != pub_key.to_bytes() {
                    return Err(crate::AppError::Internal(
                        "Mismatched biscuit key pair (public key differs from private)".into(),
                    ));
                }
                Ok(kp)
            } else {
                Self::generate_and_persist(path)
            }
        } else {
            Ok(Self::generate_ephemeral())
        }
    }

    /// メールアドレスとパスワードでユーザを認証し `User` を返す。
    ///
    /// # Errors
    /// - 資格情報が不正 / ユーザが存在しない / パスワードハッシュ検証失敗時。
    pub async fn authenticate_user(
        &self,
        request: LoginRequest,
    ) -> Result<User> {
        info!(target: "auth", "login_attempt");
        let user = self
            .user_repo
            .get_user_by_email(request.email.as_str())
            .await
            .map_err(|_| {
                warn!(target: "auth", "login_failed: user_not_found");
                AuthError::UserNotFound
            })?;
        Self::ensure_active(&user)?;
        if let Some(password_hash) = &user.password_hash {
            match password::verify_password(&request.password, password_hash) {
                Ok(true) => {}
                Ok(false) => {
                    warn!(target: "auth", user_id=%user.id, "login_failed: invalid_credentials");
                    return Err(AuthError::InvalidCredentials.into());
                }
                Err(e) => {
                    error!(target: "auth", user_id=%user.id, err=%e, "password_verify_error");
                    return Err(AuthError::PasswordHash(e.to_string()).into());
                }
            }
        } else {
            warn!(target: "auth", user_id=%user.id, "login_failed: no_password_hash");
            return Err(AuthError::InvalidCredentials.into());
        }
        self
            .user_repo
            .update_last_login(user.id)
            .await
            .map_err(|e| AuthError::Database(e.to_string()))?;
        info!(target: "auth", user_id=%user.id, "login_success");
        Ok(user)
    }

    /// 認証後にセッションとトークンを生成して `AuthResponse` を返す。
    ///
    /// # Errors
    /// Biscuit 署名やキー不整合などで失敗した場合。
    pub async fn create_auth_response(
        &self,
        user: User,
        remember_me: bool,
    ) -> Result<AuthResponse> {
        let session_id = SessionId::new();
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
        info!(target: "auth", user_id=%user.id, session=%mask_session_id(&session_id), remember_me, "session_created_and_tokens_issued");
        let tokens = crate::utils::auth_response::AuthTokens {
            access_token: access_token.clone(),
            refresh_token,
            biscuit_token: access_token,
            expires_in,
            session_id: session_id.0.clone(),
        };
        Ok(AuthResponse {
            user: UserInfo::from(user),
            tokens,
        })
    }

    /// refresh トークンを検証し新しい access/refresh トークンを返す。
    ///
    /// # Errors
    /// - トークン不正 / 期限切れ / セッション不一致 / ユーザ不在。
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<(crate::utils::auth_response::AuthTokens, UserInfo)> {
        let parsed = self.parse_refresh_biscuit(refresh_token)?;
        let (new_version, user) = self.bump_and_load_user(&parsed).await?; // version bump -> 旧トークン失効
        let (access_exp, refresh_exp) = self.compute_expiries(false);
        let (access_token, new_refresh_token, expires_in) = self.issue_access_and_refresh(
            &user,
            &parsed.session_id,
            new_version,
            access_exp,
            refresh_exp,
        )?;
        info!(target: "auth", user_id=%user.id, session=%mask_session_id(&parsed.session_id), new_version, "refresh_success");
        let user_info = UserInfo::from(&user);
        let tokens = crate::utils::auth_response::AuthTokens {
            access_token: access_token.clone(),
            refresh_token: new_refresh_token,
            biscuit_token: access_token,
            expires_in,
            session_id: parsed.session_id.0.clone(),
        };
        Ok((tokens, user_info))
    }

    async fn bump_and_load_user(&self, parsed: &ParsedBiscuit) -> Result<(u32, User)> {
        let new_version = self
            .session_store
            .validate_and_bump_refresh(parsed.session_id.clone(), parsed.version, Utc::now())
            .await?;
        let user = self
            .user_repo
            .get_user_by_id(parsed.user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        Self::ensure_active(&user)?;
        Ok((new_version, user))
    }

    fn parse_refresh_biscuit(&self, token: &str) -> Result<ParsedBiscuit> {
        let parsed = self.parse_biscuit(token)?;
        if parsed.token_type != "refresh" {
            return Err(AuthError::InvalidToken.into());
        }
        Ok(parsed)
    }

    fn build_biscuit_token(
        &self,
        user: &User,
        session_id: &SessionId,
        version: u32,
        token_type: &str,
        exp_unix: i64,
    ) -> Result<String> {
        let mut program = String::with_capacity(256);
        writeln!(
            &mut program,
            "user(\"{}\", \"{}\", \"{}\");",
            user.id, user.username, user.role
        )
        .map_err(|e| AuthError::Biscuit(format!("Failed to generate biscuit facts: {e}")))?;
        writeln!(&mut program, "token_type(\"{token_type}\");")
            .map_err(|e| AuthError::Biscuit(format!("Failed to write token_type: {e}")))?;
        writeln!(&mut program, "exp({exp_unix});")
            .map_err(|e| AuthError::Biscuit(format!("Failed to write exp: {e}")))?;
        writeln!(&mut program, "session(\"{}\", {version});", session_id.as_ref())
            .map_err(|e| AuthError::Biscuit(format!("Failed to write session: {e}")))?;
        let builder: BiscuitBuilder = biscuit_auth::Biscuit::builder();
        let builder = builder
            .code(&program)
            .map_err(|e| AuthError::Biscuit(format!("Failed to build biscuit facts: {e}")))?;
        // 改善: 起動時 keypair 再利用 (前は都度 PrivateKey -> KeyPair 化)
        let token = builder
            .build(&self.biscuit_keypair)
            .map_err(|e| AuthError::Biscuit(format!("Failed to sign biscuit: {e}")))?;
        let b64 = token
            .to_base64()
            .map_err(|e| AuthError::Biscuit(format!("Failed to serialize biscuit token: {e}")))?;
        Ok(b64)
    }

    #[cfg(feature = "legacy-auth-flat")]
    #[deprecated(note = "Use verify_biscuit(state, token) or auth_middleware (removal planned)")]
    /// (後方互換) access biscuit を検証する。
    ///
    /// # Errors
    /// - トークン不正 / 期限切れ / セッション不整合。
    pub async fn verify_jwt(&self, token: &str) -> Result<AuthContext> {
        self.verify_biscuit_generic(token, Some("access")).await
    }

    /// access biscuit を検証し `AuthContext` を返す。
    ///
    /// # Errors
    /// - トークン不正 / 期限切れ / セッション不整合。
    pub async fn verify_biscuit(
        &self,
        token: &str,
    ) -> Result<AuthContext> {
        let (ctx, _user) = self.verify_biscuit_with_user(token).await?;
        debug!(target: "auth", user_id=%ctx.user_id, session=%mask_session_id(&ctx.session_id), role=%ctx.role.as_str(), "access_token_validated");
        Ok(ctx)
    }

    /// biscuit を検証しユーザ情報も同時取得する。
    ///
    /// # Errors
    /// - トークン不正 / 期限切れ / ユーザ不在 / セッション不整合。
    pub async fn verify_biscuit_with_user(
        &self,
        token: &str,
    ) -> Result<(AuthContext, User)> {
        let ctx = self.verify_biscuit_generic(token, None).await?;
        let user = self
            .user_repo
            .get_user_by_id(ctx.user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        Self::ensure_active(&user)?;
        Ok((ctx, user))
    }

    fn create_authorizer(&self, token: &str) -> Result<biscuit_auth::Authorizer> {
        let unverified = biscuit_auth::UnverifiedBiscuit::from_base64(token)
            .map_err(|e| AuthError::Biscuit(format!("Failed to parse biscuit token: {e}")))?;
        let key_provider = |_opt_root: Option<u32>| -> std::result::Result<PublicKey, BiscuitFormat> {
            Ok(self.biscuit_public_key)
        };
        let biscuit = unverified
            .verify(key_provider)
            .map_err(|e| AuthError::Biscuit(format!("Biscuit signature verification failed: {e}")))?;
        let mut authorizer = biscuit
            .authorizer()
            .map_err(|e| AuthError::Biscuit(format!("Failed to create authorizer: {e}")))?;
        authorizer
            .authorize()
            .map_err(|e| AuthError::Biscuit(format!("Authorizer run failed: {e}")))?;
        Ok(authorizer)
    }

    fn parse_biscuit(&self, token: &str) -> Result<ParsedBiscuit> {
        let mut authorizer = self.create_authorizer(token)?;
        let (user_id, username, role) = Self::biscuit_get_user(&mut authorizer)?;
        let token_type = Self::biscuit_get_token_type(&mut authorizer)?;
        Self::biscuit_validate_exp(&mut authorizer)?;
        let (session_id, version) = Self::biscuit_get_session(&mut authorizer)?;
        Ok(ParsedBiscuit { user_id, username, role, token_type, session_id, version })
    }

    fn biscuit_get_user(
        authorizer: &mut biscuit_auth::Authorizer,
    ) -> Result<(Uuid, String, UserRole)> {
        let (id_s, username, role_s) = Self::biscuit_query_triple(
            authorizer,
            "data($id,$u,$r) <- user($id,$u,$r)",
            "user facts",
        )?;
        let user_id = Uuid::parse_str(&id_s).map_err(|_| AuthError::InvalidToken)?;
        let role = UserRole::parse_str(&role_s).map_err(|_| AuthError::InvalidToken)?;
        Ok((user_id, username, role))
    }

    fn biscuit_get_token_type(authorizer: &mut biscuit_auth::Authorizer) -> Result<String> {
        Self::biscuit_query_string(authorizer, "data($t) <- token_type($t)", "token_type")
    }

    fn biscuit_validate_exp(authorizer: &mut biscuit_auth::Authorizer) -> Result<()> {
        let exp = Self::biscuit_query_i64(authorizer, "data($e) <- exp($e)", "exp")?;
        let now_ts = Utc::now().timestamp();
        if exp < now_ts {
            warn!(target: "auth", "token_expired");
            return Err(AuthError::TokenExpired.into());
        }
        Ok(())
    }

    fn biscuit_get_session(
        authorizer: &mut biscuit_auth::Authorizer,
    ) -> Result<(SessionId, u32)> {
        Self::biscuit_query_session(authorizer, "data($sid,$v) <- session($sid,$v)", "session")
    }

    async fn verify_biscuit_generic(
        &self,
        token: &str,
        expect_type: Option<&str>,
    ) -> Result<AuthContext> {
        let parsed = self.parse_and_check(token, expect_type)?;
        self.validate_session_consistency(&parsed).await?;
        Ok(Self::build_auth_context(&parsed))
    }

    async fn validate_session_consistency(&self, parsed: &ParsedBiscuit) -> Result<()> {
        let now = Utc::now();
        match parsed.token_type.as_str() {
            "access" => {
                self.session_store
                    .validate_access(parsed.session_id.clone(), parsed.version, now)
                    .await?
            }
            "refresh" => {
                self.session_store
                    .validate_access(parsed.session_id.clone(), parsed.version, now)
                    .await?
            }
            _ => return Err(AuthError::InvalidToken.into()),
        }
        Ok(())
    }

    #[inline]
    fn ensure_token_type(expect_type: Option<&str>, actual: &str) -> Result<()> {
        if let Some(t) = expect_type {
            if actual != t {
                return Err(AuthError::InvalidToken.into());
            }
        }
        Ok(())
    }
    #[inline]
    fn build_auth_context(parsed: &ParsedBiscuit) -> AuthContext {
        AuthContext {
            user_id: parsed.user_id,
            username: parsed.username.clone(),
            role: parsed.role,
            session_id: parsed.session_id.clone(),
            permissions: Self::get_role_permissions(parsed.role.as_str()),
        }
    }
    #[inline]
    fn parse_and_check(&self, token: &str, expect_type: Option<&str>) -> Result<ParsedBiscuit> {
        let parsed = self.parse_biscuit(token)?;
        Self::ensure_token_type(expect_type, &parsed.token_type)?;
        Ok(parsed)
    }

    fn get_role_permissions(role: &str) -> Vec<String> {
        match role {
            "SuperAdmin" => ["admin", "read", "write", "delete"]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            "Admin" => ["read", "write", "delete"]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            "Editor" => ["read", "write"]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            "Author" => ["read", "write_own"]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            _ => vec!["read".to_string()],
        }
    }

    /// パスワードをハッシュ化する。
    ///
    /// # Errors
    /// ハッシュ化処理で失敗した場合。
    pub fn hash_password(&self, password: &str) -> Result<String> {
        password::hash_password(password)
    }

    /// パスワードとハッシュを検証する。
    ///
    /// # Errors
    /// 検証処理で失敗した場合。
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        password::verify_password(password, hash)
    }

    /// セッションID を無効化 (削除) する。
    ///
    /// # Errors
    /// 常に Ok。将来ストレージ層エラー発生時に拡張予定。
    pub async fn logout(&self, session_id: &str) -> Result<()> {
        let sid = SessionId(session_id.to_string());
        self.session_store.remove(sid.clone()).await;
        info!(target: "auth", session=%mask_session_id(&sid), "logout_success");
        Ok(())
    }
    pub async fn cleanup_expired_sessions(&self) {
        self.session_store.cleanup_expired(Utc::now()).await;
    }
    pub async fn get_active_session_count(&self) -> usize {
        self.session_store.count().await
    }

    /// ヘルスチェック: セッションストア健全性確認（DB 非依存）。
    pub async fn health_check(&self) -> Result<()> {
        // For now, just attempt an in-memory session operation
        let _ = self.get_active_session_count().await;
        Ok(())
    }

    /// ユーザを作成する。
    ///
    /// # Errors
    /// DB 操作に失敗した場合。
    // Note: User creation is handled by AppState/database layer; AuthService provides authentication and token logic only.
    /// トークンを検証しユーザを返す。
    ///
    /// # Errors
    /// トークン不正/期限切れ/ユーザ不在。
    pub async fn validate_token(&self, token: &str) -> Result<User> {
        let (_ctx, user) = self.verify_biscuit_with_user(token).await?;
        Ok(user)
    }

    /// 明示的にセッションを生成し access biscuit を返す。
    ///
    /// # Errors
    /// ユーザ取得/トークン生成失敗時。
    pub async fn create_session(&self, user_id: Uuid) -> Result<String> {
        let user = self
            .user_repo
            .get_user_by_id(user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        Self::ensure_active(&user)?;
        let session_id = SessionId::new();
        let (access_exp, refresh_exp) = self.compute_expiries(false);
        self.insert_session(&user, &session_id, refresh_exp, 1)
            .await;
        let token = self.build_biscuit_token(&user, &session_id, 1, "access", access_exp.timestamp())?;
        Ok(token)
    }
}

/// 管理者権限 (admin) を要求する。
///
/// # Errors
/// 権限不足の場合。
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
            session_id: SessionId("s1".to_string()),
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
            session_id: SessionId("s2".to_string()),
            permissions: vec!["read".to_string()],
        };
        let res = require_admin_permission(&ctx);
        assert!(res.is_err());
        let app_err: crate::AppError = res.unwrap_err();
        match app_err {
            crate::AppError::Authorization(_) => {}
            other => panic!("expected Authorization error, got {other:?}"),
        }
    }
}
