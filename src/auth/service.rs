use std::sync::Arc;

use argon2::Argon2;
use biscuit_auth::{KeyPair, PublicKey};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    Result,
    auth::{
        biscuit::{self, ParsedBiscuit},
        error::AuthError,
        key_management::load_or_generate_keypair,
        mask_session_id,
        session::{InMemorySessionStore, SessionData, SessionStore},
    },
    config::AuthConfig,
    models::{User, UserRole},
    repositories::UserRepository,
    utils::{
        common_types::{SessionId, UserInfo},
        password,
    },
};

/// Authentication service
#[cfg(feature = "auth")]
pub struct AuthService {
    /// 起動時に読み込み/生成された `KeyPair` (再利用でパフォーマンス向上)
    pub(super) biscuit_keypair: Arc<KeyPair>,
    /// 公開鍵 (検証用) - Copy 可能
    pub(super) biscuit_public_key: PublicKey,
    pub(super) user_repo: Arc<dyn UserRepository>,
    pub(super) config: AuthConfig,
    pub(super) session_store: Arc<InMemorySessionStore>,
    pub(super) argon2: Arc<Argon2<'static>>,
    pub(super) access_ttl_secs: i64,
    pub(super) refresh_ttl_secs: i64,
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
    #[cfg(test)]
    pub async fn clear_sessions_for_test(&self) {
        self.session_store.clear().await;
    }

    /// コンフィグと DB からサービスを初期化する。
    ///
    /// # Errors
    /// キーロード/生成、または設定値変換で失敗した場合 `AppError` を返す。
    pub fn new_with_repo(config: &AuthConfig, user_repo: Arc<dyn UserRepository>) -> Result<Self> {
        let keypair = load_or_generate_keypair(config)?;
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
            session_store: Arc::new(InMemorySessionStore::new()),
            argon2,
            access_ttl_secs,
            refresh_ttl_secs,
        })
    }

    /// Backward-compatible constructor from Database
    ///
    /// # Errors
    /// Returns error if key pair initialization or repository setup fails.
    pub fn new(config: &AuthConfig, database: &crate::database::Database) -> Result<Self> {
        let repo: Arc<dyn UserRepository> = Arc::new(database.clone()) as Arc<dyn UserRepository>;
        Self::new_with_repo(config, repo)
    }

    /// メールアドレスとパスワードでユーザを認証し `User` を返す。
    ///
    /// # Errors
    /// - 資格情報が不正 / ユーザが存在しない / パスワードハッシュ検証失敗時。
    pub async fn authenticate_user(&self, request: LoginRequest) -> Result<User> {
        info!(target: "auth", "login_attempt");
        
        let user = self.fetch_user_by_email(&request.email).await?;
        Self::ensure_active(&user)?;
        Self::verify_user_password(&user, &request.password)?;
        self.update_login_timestamp(&user).await?;
        
        info!(target: "auth", user_id=%user.id, "login_success");
        Ok(user)
    }

    /// Fetch user by email address
    async fn fetch_user_by_email(&self, email: &str) -> Result<User> {
        self.user_repo
            .get_user_by_email(email)
            .await
            .map_err(|_| {
                warn!(target: "auth", "login_failed: user_not_found");
                AuthError::UserNotFound.into()
            })
    }

    /// Verify user's password against stored hash
    fn verify_user_password(user: &User, password: &str) -> Result<()> {
        let password_hash = user.password_hash.as_ref().ok_or_else(|| {
            warn!(target: "auth", user_id=%user.id, "login_failed: no_password_hash");
            AuthError::InvalidCredentials
        })?;

        match password::verify_password(password, password_hash) {
            Ok(true) => Ok(()),
            Ok(false) => {
                warn!(target: "auth", user_id=%user.id, "login_failed: invalid_credentials");
                Err(AuthError::InvalidCredentials.into())
            }
            Err(e) => {
                error!(target: "auth", user_id=%user.id, err=%e, "password_verify_error");
                Err(AuthError::PasswordHash(e.to_string()).into())
            }
        }
    }

    /// Update user's last login timestamp
    async fn update_login_timestamp(&self, user: &User) -> Result<()> {
        self.user_repo
            .update_last_login(user.id)
            .await
            .map_err(|e| AuthError::Database(e.to_string()).into())
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
        let (access_exp, refresh_exp) = self.compute_expiries(remember_me)?;
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
        let parsed = biscuit::parse_refresh_biscuit(refresh_token, &self.biscuit_public_key)?;
        let (new_version, user) = self.bump_and_load_user(&parsed).await?; // version bump -> 旧トークン失効
        let (access_exp, refresh_exp) = self.compute_expiries(false)?;
        let (access_token, new_refresh_token, expires_in) = self.issue_access_and_refresh(
            &user,
            &parsed.session_id,
            new_version,
            access_exp,
            refresh_exp,
        )?;
        info!(target: "auth", user_id=%user.id, session=%mask_session_id(&parsed.session_id), new_version, "refresh_success");
        let user_info = UserInfo::from(user);
        let tokens = crate::utils::auth_response::AuthTokens {
            access_token: access_token.clone(),
            refresh_token: new_refresh_token,
            biscuit_token: access_token,
            expires_in,
            session_id: parsed.session_id.0.clone(),
        };
        Ok((tokens, user_info))
    }

    /// access biscuit を検証し `AuthContext` を返す。
    ///
    /// # Errors
    /// - トークン不正 / 期限切れ / セッション不整合。
    pub async fn verify_biscuit(&self, token: &str) -> Result<AuthContext> {
        let (ctx, _user) = self.verify_biscuit_with_user(token).await?;
        debug!(target: "auth", user_id=%ctx.user_id, session=%mask_session_id(&ctx.session_id), role=%ctx.role.as_str(), "access_token_validated");
        Ok(ctx)
    }

    /// biscuit を検証しユーザ情報も同時取得する。
    ///
    /// # Errors
    /// - トークン不正 / 期限切れ / ユーザ不在 / セッション不整合。
    pub async fn verify_biscuit_with_user(&self, token: &str) -> Result<(AuthContext, User)> {
        let ctx = self.verify_biscuit_generic(token, None).await?;
        let user = self
            .user_repo
            .get_user_by_id(ctx.user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        Self::ensure_active(&user)?;
        Ok((ctx, user))
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
    ///
    /// # Errors
    /// Returns error if session store operations fail.
    pub async fn health_check(&self) -> Result<()> {
        // For now, just attempt an in-memory session operation
        let _ = self.get_active_session_count().await;
        Ok(())
    }

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
        let (access_exp, refresh_exp) = self.compute_expiries(false)?;
        self.insert_session(&user, &session_id, refresh_exp, 1)
            .await;
        let token = biscuit::build_token(
            &self.biscuit_keypair,
            &user,
            &session_id,
            1,
            "access",
            access_exp.timestamp(),
        )?;
        Ok(token)
    }

    // --- Private helpers ---

    #[inline]
    fn ensure_active(user: &User) -> Result<()> {
        if !user.is_active {
            return Err(AuthError::InvalidCredentials.into());
        }
        Ok(())
    }

    fn compute_expiries(&self, remember_me: bool) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
        let now = Utc::now();
        let access_ttl = if remember_me {
            ChronoDuration::seconds(
                i64::try_from(self.config.remember_me_access_ttl_secs).map_err(|_| {
                    crate::AppError::ConfigValidationError(
                        "auth.remember_me_access_ttl_secs is too large for i64".to_string(),
                    )
                })?,
            )
        } else {
            ChronoDuration::seconds(self.access_ttl_secs)
        };
        let refresh_ttl = ChronoDuration::seconds(self.refresh_ttl_secs);
        Ok((now + access_ttl, now + refresh_ttl))
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
        self.session_store.insert(session_id.clone(), data).await;
    }

    fn issue_access_and_refresh(
        &self,
        user: &User,
        session_id: &SessionId,
        version: u32,
        access_exp: DateTime<Utc>,
        refresh_exp: DateTime<Utc>,
    ) -> Result<(String, String, i64)> {
        let access_token = biscuit::build_token(
            &self.biscuit_keypair,
            user,
            session_id,
            version,
            "access",
            access_exp.timestamp(),
        )?;
        let refresh_token = biscuit::build_token(
            &self.biscuit_keypair,
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

    async fn verify_biscuit_generic(
        &self,
        token: &str,
        expect_type: Option<&str>,
    ) -> Result<AuthContext> {
        let parsed = biscuit::parse_and_check(token, expect_type, &self.biscuit_public_key)?;
        self.validate_session_consistency(&parsed).await?;
        Ok(self.build_auth_context(&parsed))
    }

    async fn validate_session_consistency(&self, parsed: &ParsedBiscuit) -> Result<()> {
        let now = Utc::now();
        // For refresh tokens, we only need to check if the session is valid.
        // The version bump happens in `validate_and_bump_refresh`.
        self.session_store
            .validate_access(parsed.session_id.clone(), parsed.version, now)
            .await?;
        Ok(())
    }

    #[inline]
    fn build_auth_context(&self, parsed: &ParsedBiscuit) -> AuthContext {
        AuthContext {
            user_id: parsed.user_id,
            username: parsed.username.clone(),
            role: parsed.role,
            session_id: parsed.session_id.clone(),
            permissions: self.get_role_permissions(parsed.role.as_str()),
        }
    }

    fn get_role_permissions(&self, role: &str) -> Vec<String> {
        self.config
            .role_permissions
            .get(role)
            .cloned()
            .unwrap_or_else(|| {
                warn!(role = %role, "role not found in config, falling back to default permissions");
                vec!["read".to_string()]
            })
    }

    /// API Key に基づいて短時間有効な Biscuit トークンを生成する。
    /// 
    /// API Key 認証を経由してリクエストが来た場合、API Key に関連付けられたユーザ情報を元に
    /// Biscuit トークンを生成して、システム内では統一的に Biscuit ベースの認証コンテキストを使用します。
    /// 
    /// # Arguments
    /// * `user_id` - API Key に関連付けられたユーザID
    /// * `permissions` - API Key に付与された権限のリスト
    /// 
    /// # Errors
    /// - ユーザが見つからない場合
    /// - Biscuit トークンの生成に失敗した場合
    pub async fn create_biscuit_from_api_key(
        &self,
        user_id: Uuid,
        permissions: Vec<String>,
    ) -> Result<AuthContext> {
        let user = self
            .user_repo
            .get_user_by_id(user_id)
            .await
            .map_err(|_| AuthError::UserNotFound)?;
        Self::ensure_active(&user)?;
        
        // API Key用の一時的なセッションIDを生成
        let session_id = SessionId::new();
        
        // API Key 認証の場合、パーミッションは API Key 自身に付与されたものを使用
        let role = UserRole::parse_str(&user.role).unwrap_or(UserRole::Subscriber);
        
        debug!(
            target: "auth",
            user_id=%user.id,
            session=%mask_session_id(&session_id),
            role=%role.as_str(),
            permissions=?permissions,
            "biscuit_token_created_from_api_key"
        );
        
        Ok(AuthContext {
            user_id: user.id,
            username: user.username,
            role,
            session_id,
            permissions,
        })
    }
}
