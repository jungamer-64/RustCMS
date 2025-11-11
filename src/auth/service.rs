//! 認証サービス (Refactored)
//!
//! # 改善点
//! - パスワード検証の実装
//! - 統合鍵管理の使用
//! - エラーハンドリングの改善
//! - セッション管理の明確化
//! - JWT/Biscuit役割の明確化

use chrono::{Duration as ChronoDuration, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppError, Result,
    auth::{
        error::AuthError,
        jwt::{JwtConfig, JwtService},
        password_service::PasswordService,
        session::{InMemorySessionStore, SessionData, SessionStore},
        unified_key_management::{KeyLoadConfig, UnifiedKeyPair},
    },
    config::AuthConfig,
};

#[cfg(feature = "restructure_domain")]
use crate::{
    application::ports::repositories::UserRepository,
    common::type_utils::common_types::{AuthResponse, AuthTokens, SessionId, UserInfo},
    domain::user::{Email, User, UserRole},
};

#[cfg(not(feature = "restructure_domain"))]
use crate::{
    models::{User, UserRole},
    repositories::UserRepository,
    utils::common_types::{SessionId, UserInfo},
};

/// ログイン要求
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

/// 認証コンテキスト
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub session_id: SessionId,
    pub permissions: Vec<String>,
}

/// 認証サービス
pub struct AuthService {
    /// 統合鍵ペア
    unified_keypair: Arc<UnifiedKeyPair>,
    /// JWTサービス
    jwt_service: Arc<JwtService>,
    /// パスワードサービス
    password_service: Arc<PasswordService>,
    /// ユーザーリポジトリ
    user_repo: Arc<dyn UserRepository>,
    /// セッションストア
    session_store: Arc<dyn SessionStore>,
    /// 設定
    config: AuthConfig,
}

impl Clone for AuthService {
    fn clone(&self) -> Self {
        Self {
            unified_keypair: Arc::clone(&self.unified_keypair),
            jwt_service: Arc::clone(&self.jwt_service),
            password_service: Arc::clone(&self.password_service),
            user_repo: Arc::clone(&self.user_repo),
            session_store: Arc::clone(&self.session_store),
            config: self.config.clone(),
        }
    }
}

impl AuthService {
    /// 新しい認証サービスを作成
    ///
    /// # Arguments
    /// * `config` - 認証設定
    /// * `user_repo` - ユーザーリポジトリ
    ///
    /// # Errors
    /// 鍵の読み込みや初期化に失敗した場合
    pub fn new_with_repo(config: &AuthConfig, user_repo: Arc<dyn UserRepository>) -> Result<Self> {
        let session_store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        Self::new_with_repo_and_store(config, user_repo, session_store)
    }

    /// セッションストアを差し替えて認証サービスを作成
    pub fn new_with_repo_and_store(
        config: &AuthConfig,
        user_repo: Arc<dyn UserRepository>,
        session_store: Arc<dyn SessionStore>,
    ) -> Result<Self> {
        // 統合鍵ペアの読み込み
        let key_config = KeyLoadConfig {
            key_file_path: "./secrets/unified_ed25519.key".to_string(),
            is_production: config.is_production,
        };

        let unified_keypair = UnifiedKeyPair::load_or_generate(&key_config)
            .map_err(|e| AppError::Internal(format!("Failed to load key pair: {e}")))?;

        info!(
            "Loaded unified Ed25519 key pair (fingerprint: {})",
            unified_keypair.fingerprint()
        );

        // JWTサービスの初期化
        let jwt_config = JwtConfig {
            access_token_ttl_secs: config.access_token_ttl_secs,
            refresh_token_ttl_secs: config.refresh_token_ttl_secs,
            remember_me_ttl_secs: config.remember_me_access_ttl_secs,
        };

        let jwt_service = JwtService::new(unified_keypair.clone(), jwt_config);

        // パスワードサービスの初期化
        let password_service = PasswordService::new();

        Ok(Self {
            unified_keypair: Arc::new(unified_keypair),
            jwt_service: Arc::new(jwt_service),
            password_service: Arc::new(password_service),
            user_repo,
            session_store,
            config: config.clone(),
        })
    }

    /// データベースから認証サービスを作成
    pub fn new(
        config: &AuthConfig,
        database: &crate::infrastructure::database::connection::DatabasePool,
    ) -> Result<Self> {
        #[cfg(all(feature = "database", feature = "restructure_domain"))]
        {
            use crate::infrastructure::DieselUserRepository;
            let repo: Arc<dyn UserRepository> =
                Arc::new(DieselUserRepository::new(database.get_pool()));
            Self::new_with_repo(config, repo)
        }

        #[cfg(all(feature = "database", not(feature = "restructure_domain")))]
        {
            use crate::repositories::UserRepository as LegacyRepo;
            let repo: Arc<dyn UserRepository> = Arc::new(LegacyRepo::new(database.clone()));
            Self::new_with_repo(config, repo)
        }

        #[cfg(not(feature = "database"))]
        compile_error!("auth feature requires database feature");
    }

    /// Convert domain User to API UserInfo
    ///
    /// Maps the domain `User` entity to the API `UserInfo` response type.
    /// Note: The domain User model does not currently track first_name, last_name,
    /// or email_verified, so these fields are set to default values.
    fn user_to_info(user: &User) -> UserInfo {
        UserInfo {
            id: user.id().to_string(),
            username: user.username().as_str().to_string(),
            email: user.email().as_str().to_string(),
            first_name: None, // Not tracked in domain::User yet
            last_name: None,  // Not tracked in domain::User yet
            role: format!("{:?}", user.role()), // Convert UserRole enum to string
            is_active: user.is_active(),
            email_verified: false, // Not tracked in domain::User yet
            last_login: user.last_login(),
            created_at: user.created_at(),
            updated_at: user.updated_at(),
        }
    }

    /// ユーザーを認証
    ///
    /// # Arguments
    /// * `request` - ログイン要求
    ///
    /// # Errors
    /// - メールアドレスが無効
    /// - ユーザーが見つからない
    /// - パスワードが不正
    /// - アカウントが無効
    pub async fn authenticate_user(&self, request: LoginRequest) -> Result<User> {
        info!(target: "auth", email = %request.email, "Login attempt");

        // メールアドレスの検証
        let email = Email::new(request.email.clone()).map_err(|e| {
            warn!(target: "auth", "Invalid email format: {}", e);
            AuthError::InvalidCredentials
        })?;

        // ユーザーの取得
        let user = self
            .user_repo
            .find_by_email(&email)
            .await
            .map_err(|e| {
                error!(target: "auth", "Database error during login: {}", e);
                AuthError::DatabaseError(e.to_string())
            })?
            .ok_or_else(|| {
                warn!(target: "auth", email = %request.email, "User not found");
                AuthError::InvalidCredentials
            })?;

        // アカウントの状態確認
        if !user.is_active() {
            warn!(target: "auth", user_id = %user.id(), "Inactive account login attempt");
            return Err(AuthError::UserInactive.into());
        }

        // パスワード検証
        self.verify_user_password(&user, &request.password).await?;

        // ログイン時刻の更新
        self.update_user_last_login(user.id()).await?;

        info!(target: "auth", user_id = %user.id(), "Login successful");
        Ok(user)
    }

    /// Phase 9: パスワードを検証（完全実装）
    async fn verify_user_password(&self, user: &User, password: &str) -> Result<()> {
        // パスワードハッシュを取得
        let hash = user.password_hash().ok_or_else(|| {
            warn!(target: "auth", user_id = %user.id(), "User has no password hash");
            AuthError::InvalidCredentials
        })?;

        // パスワードを検証
        self.password_service
            .verify_password(password, hash)
            .map_err(|e| {
                warn!(target: "auth", user_id = %user.id(), error = ?e, "Password verification failed");
                AuthError::InvalidPassword
            })?;

        debug!(target: "auth", user_id = %user.id(), "Password verified successfully");
        Ok(())
    }

    /// Phase 9: 最終ログイン日時を更新
    pub async fn update_user_last_login(&self, user_id: crate::domain::user::UserId) -> Result<()> {
        self.user_repo
            .update_last_login(user_id)
            .await
            .map_err(|e| {
                error!(target: "auth", user_id = %user_id, error = ?e, "Failed to update last login");
                AppError::Internal(format!("Failed to update last login: {e}"))
            })?;

        debug!(target: "auth", user_id = %user_id, "Last login updated");
        Ok(())
    }

    /// 認証レスポンスを作成
    ///
    /// # Arguments
    /// * `user` - 認証されたユーザー
    /// * `remember_me` - Remember Meオプション
    ///
    /// # Errors
    /// トークン生成に失敗した場合
    pub async fn create_auth_response(
        &self,
        user: User,
        remember_me: bool,
    ) -> Result<AuthResponse> {
        let session_id = SessionId::new();
        let session_version = 1;

        // JWTトークンペアの生成
        let token_pair = self.jwt_service.generate_token_pair(
            user.id().into(),
            user.username().as_str().to_string(),
            user.role(),
            session_id.clone(),
            session_version,
            remember_me,
        )?;

        // セッションデータの作成
        let session_data = SessionData {
            user_id: user.id().into(),
            username: user.username().as_str().to_string(),
            role: user.role(),
            created_at: Utc::now(),
            expires_at: token_pair.expires_at
                + ChronoDuration::seconds(self.config.refresh_token_ttl_secs as i64),
            last_accessed: Utc::now(),
            refresh_version: session_version,
        };

        // セッションを保存
        self.session_store
            .insert(session_id.clone(), session_data)
            .await;

        info!(
            target: "auth",
            user_id = %user.id(),
            session_id = %mask_session_id(&session_id),
            remember_me = remember_me,
            "Session created"
        );

        Ok(AuthResponse {
            user: Self::user_to_info(&user),
            tokens: AuthTokens {
                access_token: token_pair.access_token,
                refresh_token: token_pair.refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: 3600, // 1 hour default
            },
        })
    }

    /// リフレッシュトークンで新しいトークンペアを取得
    ///
    /// # Arguments
    /// * `refresh_token` - リフレッシュトークン
    ///
    /// # Errors
    /// - トークンが無効
    /// - セッションが無効
    /// - ユーザーが見つからない
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<(AuthTokens, UserInfo)> {
        // リフレッシュトークンを検証
        let claims = self.jwt_service.verify_refresh_token(refresh_token)?;

        let session_id = claims.session_id();
        let user_id = claims.user_id()?;

        // セッションのバージョンを確認・更新
        let new_version = self
            .session_store
            .validate_and_bump_refresh(session_id.clone(), claims.session_version(), Utc::now())
            .await?;

        // ユーザー情報を取得
        let user_id_vo = crate::domain::user::UserId::from_uuid(user_id);
        let user = self
            .user_repo
            .find_by_id(user_id_vo)
            .await
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?
            .ok_or(AuthError::UserNotFound)?;

        // アカウントの状態確認
        if !user.is_active() {
            return Err(AuthError::UserInactive.into());
        }

        // 新しいトークンペアを生成
        let token_pair = self.jwt_service.generate_token_pair(
            user.id().into(),
            user.username().as_str().to_string(),
            user.role(),
            session_id.clone(),
            new_version,
            false, // リフレッシュ時はremember_meを引き継がない
        )?;

        info!(
            target: "auth",
            user_id = %user.id(),
            session_id = %mask_session_id(&session_id),
            new_version = new_version,
            "Token refreshed"
        );

        Ok((
            AuthTokens {
                access_token: token_pair.access_token,
                refresh_token: token_pair.refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: 3600, // 1 hour default
            },
            Self::user_to_info(&user),
        ))
    }

    /// アクセストークンを検証して認証コンテキストを取得
    ///
    /// # Arguments
    /// * `token` - アクセストークン
    ///
    /// # Errors
    /// - トークンが無効
    /// - セッションが無効
    pub async fn verify_access_token(&self, token: &str) -> Result<AuthContext> {
        // トークンを検証
        let claims = self.jwt_service.verify_access_token(token)?;

        let session_id = claims.session_id();
        let user_id = claims.user_id()?;

        // セッションの有効性を確認
        self.session_store
            .validate_access(session_id.clone(), claims.session_version(), Utc::now())
            .await?;

        // 権限を取得
        let permissions = self.get_role_permissions(claims.role.as_str());

        debug!(
            target: "auth",
            user_id = %user_id,
            session_id = %mask_session_id(&session_id),
            "Access token validated"
        );

        Ok(AuthContext {
            user_id,
            username: claims.username,
            role: claims.role,
            session_id,
            permissions,
        })
    }

    /// ログアウト
    ///
    /// # Arguments
    /// * `session_id` - セッションID
    pub async fn logout(&self, session_id: &str) -> Result<()> {
        let sid = SessionId::from(session_id.to_string());
        self.session_store.remove(sid.clone()).await;

        info!(
            target: "auth",
            session_id = %mask_session_id(&sid),
            "Logout successful"
        );

        Ok(())
    }

    /// パスワードをハッシュ化
    pub fn hash_password(&self, password: &str) -> Result<String> {
        self.password_service
            .hash_password(password)
            .map_err(|e| e.into())
    }

    /// パスワードを検証
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        match self.password_service.verify_password(password, hash) {
            Ok(()) => Ok(true),
            Err(AuthError::InvalidPassword) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// パスワードポリシーを検証
    pub fn validate_password_policy(&self, password: &str) -> Result<()> {
        self.password_service
            .validate_password_policy(password)
            .map_err(|e| e.into())
    }

    /// パスワード強度を計算
    pub fn calculate_password_strength(&self, password: &str) -> u8 {
        self.password_service.calculate_password_strength(password)
    }

    /// 期限切れセッションのクリーンアップ
    pub async fn cleanup_expired_sessions(&self) {
        let before_count = self.session_store.count().await;
        self.session_store.cleanup_expired(Utc::now()).await;
        let after_count = self.session_store.count().await;

        if before_count > after_count {
            info!(
                target: "auth",
                removed = before_count - after_count,
                remaining = after_count,
                "Expired sessions cleaned up"
            );
        }
    }

    /// アクティブなセッション数を取得
    pub async fn get_active_session_count(&self) -> usize {
        self.session_store.count().await
    }

    /// ヘルスチェック
    pub async fn health_check(&self) -> Result<()> {
        // セッションストアの動作確認
        let _ = self.get_active_session_count().await;
        Ok(())
    }

    // === Private methods ===

    /// ロールから権限を取得
    fn get_role_permissions(&self, role: &str) -> Vec<String> {
        self.config
            .role_permissions
            .get(role)
            .cloned()
            .unwrap_or_else(|| {
                warn!(
                    target: "auth",
                    role = %role,
                    "Role not found in config, using default permissions"
                );
                vec!["read".to_string()]
            })
    }

    #[cfg(test)]
    pub async fn clear_sessions_for_test(&self) {
        self.session_store.clear().await;
    }
}

/// セッションIDをマスク（ログ用）
#[inline]
fn mask_session_id(sid: &SessionId) -> String {
    let s: &str = sid.as_ref();
    if s.len() <= 6 {
        return "***".to_string();
    }
    format!("{}…{}", &s[..3], &s[s.len() - 3..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // モックユーザーリポジトリ
    struct MockUserRepository {
        users: HashMap<Uuid, User>,
    }

    #[async_trait::async_trait]
    impl UserRepository for MockUserRepository {
        async fn save(
            &self,
            _user: User,
        ) -> std::result::Result<(), crate::application::ports::repositories::RepositoryError>
        {
            unimplemented!("MockUserRepository::save not implemented")
        }

        async fn find_by_id(
            &self,
            id: crate::domain::user::UserId,
        ) -> std::result::Result<
            Option<User>,
            crate::application::ports::repositories::RepositoryError,
        > {
            Ok(self.users.get(&id.into()).cloned())
        }

        async fn find_by_email(
            &self,
            email: &Email,
        ) -> std::result::Result<
            Option<User>,
            crate::application::ports::repositories::RepositoryError,
        > {
            Ok(self
                .users
                .values()
                .find(|u| u.email().as_str() == email.as_str())
                .cloned())
        }

        async fn find_by_username(
            &self,
            _username: &crate::domain::user::Username,
        ) -> std::result::Result<
            Option<User>,
            crate::application::ports::repositories::RepositoryError,
        > {
            Ok(None)
        }

        async fn delete(
            &self,
            _id: crate::domain::user::UserId,
        ) -> std::result::Result<(), crate::application::ports::repositories::RepositoryError>
        {
            unimplemented!("MockUserRepository::delete not implemented")
        }

        async fn list_all(
            &self,
            _limit: i64,
            _offset: i64,
        ) -> std::result::Result<Vec<User>, crate::application::ports::repositories::RepositoryError>
        {
            Ok(self.users.values().cloned().collect())
        }

        async fn update_password_hash(
            &self,
            _user_id: crate::domain::user::UserId,
            _password_hash: String,
        ) -> std::result::Result<(), crate::application::ports::repositories::RepositoryError>
        {
            Ok(())
        }

        async fn update_last_login(
            &self,
            _user_id: crate::domain::user::UserId,
        ) -> std::result::Result<(), crate::application::ports::repositories::RepositoryError>
        {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_password_hashing() {
        let config = AuthConfig::default();
        let repo = Arc::new(MockUserRepository {
            users: HashMap::new(),
        });

        let service = AuthService::new_with_repo(&config, repo).expect("Failed to create service");

        let password = "TestPassword123";
        let hash = service
            .hash_password(password)
            .expect("Failed to hash password");

        assert!(service.verify_password(password, &hash).unwrap());
        assert!(!service.verify_password("WrongPassword", &hash).unwrap());
    }

    #[tokio::test]
    async fn test_password_policy() {
        let config = AuthConfig::default();
        let repo = Arc::new(MockUserRepository {
            users: HashMap::new(),
        });

        let service = AuthService::new_with_repo(&config, repo).expect("Failed to create service");

        // 有効なパスワード
        assert!(service.validate_password_policy("SecurePass123").is_ok());

        // 無効なパスワード
        assert!(service.validate_password_policy("short").is_err());
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let config = AuthConfig::default();
        let repo = Arc::new(MockUserRepository {
            users: HashMap::new(),
        });

        let service = AuthService::new_with_repo(&config, repo).expect("Failed to create service");

        // 期限切れセッションのクリーンアップ
        service.cleanup_expired_sessions().await;

        let count = service.get_active_session_count().await;
        assert_eq!(count, 0);
    }
}
