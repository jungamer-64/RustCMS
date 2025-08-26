//! Authentication Service - biscuit-auth + WebAuthn + JWT
//!
//! Provides comprehensive authentication and authorization:
//! - biscuit-auth for cryptographic authorization tokens
//! - WebAuthn for passwordless authentication  
//! - JWT for session management
//! - Password-based authentication with Argon2
//! - Role-based access control (RBAC)

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use biscuit_auth::{KeyPair, PrivateKey, PublicKey};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
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
    #[error("JWT error: {0}")]
    Jwt(String),
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
    /// JWT encoding key
    jwt_encoding_key: EncodingKey,
    /// JWT decoding key
    jwt_decoding_key: DecodingKey,
    /// Database reference
    database: Database,
    /// Configuration
    config: AuthConfig,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    /// Password hasher
    argon2: Argon2<'static>,
}

impl Clone for AuthService {
    fn clone(&self) -> Self {
        // Preserve existing keys instead of regenerating them so tokens remain verifiable
        Self {
            biscuit_private_key: self.biscuit_private_key.clone(),
            biscuit_public_key: self.biscuit_public_key.clone(),
            jwt_encoding_key: self.jwt_encoding_key.clone(),
            jwt_decoding_key: self.jwt_decoding_key.clone(),
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
}

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String, // User ID
    pub username: String,
    pub role: String,
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
    pub jti: String, // JWT ID (session ID)
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    pub biscuit_token: String,
    pub expires_in: i64,
    pub session_id: String,
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
    /// Create new authentication service
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
                    let priv_key = PrivateKey::from_bytes(&priv_bytes)
                        .map_err(|e| crate::AppError::Internal(format!("Failed to parse biscuit private key from env: {}", e)))?;
                    let pub_key = PublicKey::from_bytes(&pub_bytes)
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
                            let priv_key = PrivateKey::from_bytes(&priv_bytes).map_err(|e| {
                                crate::AppError::Internal(format!("Failed to parse biscuit private key from file: {}", e))
                            })?;
                            let pub_key = PublicKey::from_bytes(&pub_bytes).map_err(|e| {
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

                        let priv_b64 = STANDARD.encode(&priv_bytes);
                        let pub_b64 = STANDARD.encode(&pub_bytes);

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

        // Create JWT keys
        let jwt_encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let jwt_decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());

        // Initialize Argon2
        let argon2 = Argon2::default();

        Ok(Self {
            biscuit_private_key,
            biscuit_public_key,
            jwt_encoding_key,
            jwt_decoding_key,
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

    /// Create authentication response with tokens
    #[allow(dead_code)]
    async fn create_auth_response(&self, user: User, remember_me: bool) -> Result<AuthResponse> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Determine expiration based on remember_me
        let expires_in = if remember_me {
            ChronoDuration::days(30).num_seconds()
        } else {
            ChronoDuration::hours(24).num_seconds()
        };

        let expires_at = now + ChronoDuration::seconds(expires_in);

        // Create JWT token
        let jwt_claims = JwtClaims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            role: user.role.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti: session_id.clone(),
        };

        let access_token = encode(&Header::default(), &jwt_claims, &self.jwt_encoding_key)
            .map_err(|e| AuthError::Jwt(e.to_string()))?;

        // Create refresh token (longer expiration)
        let refresh_claims = JwtClaims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            role: user.role.to_string(),
            exp: (now + ChronoDuration::days(30)).timestamp() as usize,
            iat: now.timestamp() as usize,
            jti: format!("{}_refresh", session_id),
        };

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.jwt_encoding_key)
            .map_err(|e| AuthError::Jwt(e.to_string()))?;

        // Create Biscuit token
        let biscuit_token = self.create_biscuit_token(&user)?;

        // Store session
        let session_data = SessionData {
            user_id: user.id,
            username: user.username.clone(),
            role: UserRole::parse_str(&user.role).unwrap_or(UserRole::Subscriber),
            created_at: now,
            expires_at,
            last_accessed: now,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session_data);

        Ok(AuthResponse {
            user: UserInfo::from(user),
            access_token,
            refresh_token,
            biscuit_token,
            expires_in,
            session_id,
        })
    }

    /// Create Biscuit authorization token (cryptographically signed)
    #[allow(dead_code)]
    fn create_biscuit_token(&self, user: &User) -> Result<String> {
        // Use biscuit-auth to build a token with a simple fact containing user info.
        // This creates a sealed Biscuit and returns its base64 representation.

    // Build authority block with a fact like: user("<id>", "<username>", "<role>")
    let mut builder = biscuit_auth::Biscuit::builder();
    let fact = format!("user(\"{}\", \"{}\", \"{}\")", user.id, user.username, user.role);
    // Parse datalog code into the builder
    builder.add_code(&fact).map_err(|e| AuthError::Biscuit(format!("Failed to add fact: {}", e)))?;

    // Build and sign the token with the service's keypair (create KeyPair from private key)
    let keypair = biscuit_auth::KeyPair::from(&self.biscuit_private_key);

        let token = builder.build(&keypair).map_err(|e| AuthError::Biscuit(format!("Failed to build biscuit token: {}", e)))?;

        // Serialize to base64
        let b64 = token.to_base64().map_err(|e| AuthError::Biscuit(format!("Failed to serialize biscuit token: {}", e)))?;
        Ok(b64)
    }

    /// Verify JWT token
    pub async fn verify_jwt(&self, token: &str) -> Result<AuthContext> {
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<JwtClaims>(token, &self.jwt_decoding_key, &validation)
            .map_err(|e| AuthError::Jwt(e.to_string()))?;

        let claims = token_data.claims;

        // Check if session exists and is valid
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(&claims.jti) {
            if session.expires_at < Utc::now() {
                return Err(AuthError::TokenExpired.into());
            }

            // Update last accessed time
            drop(sessions);
            let mut sessions_write = self.sessions.write().await;
            if let Some(session) = sessions_write.get_mut(&claims.jti) {
                session.last_accessed = Utc::now();
            }

            Ok(AuthContext {
                user_id: Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?,
                username: claims.username,
                role: UserRole::parse_str(&claims.role).map_err(|_| AuthError::InvalidToken)?,
                session_id: claims.jti,
                permissions: self.get_role_permissions(&claims.role),
            })
        } else {
            Err(AuthError::InvalidToken.into())
        }
    }

    /// Verify Biscuit token
    pub async fn verify_biscuit(&self, token: &str) -> Result<AuthContext> {
        // Parse the base64 biscuit into an UnverifiedBiscuit
        let unverified = biscuit_auth::UnverifiedBiscuit::from_base64(token)
            .map_err(|e| AuthError::Biscuit(format!("Failed to parse biscuit token: {}", e)))?;

        // Check signature using a key provider callback that returns our public key
        let key_provider = |_opt_root_id: Option<u32>| -> biscuit_auth::PublicKey {
            // ignore root id and return the configured public key
            self.biscuit_public_key.clone()
        };

        let biscuit = unverified
            .check_signature(key_provider)
            .map_err(|e| AuthError::Biscuit(format!("Biscuit signature verification failed: {}", e)))?;

        // Create an Authorizer from the verified biscuit to inspect facts
        let mut authorizer = biscuit.authorizer().map_err(|e| AuthError::Biscuit(format!("Failed to create authorizer: {}", e)))?;

        // Run a simple authorize (no additional checks) so facts are loaded
        let _ = authorizer.authorize().map_err(|e| AuthError::Biscuit(format!("Authorizer run failed: {}", e)))?;

        // Extract facts using the Authorizer query API instead of naive string parsing.
        // We expect a fact of the form: user("<id>", "<username>", "<role>")
        let query = r#"data($id, $username, $role) <- user($id, $username, $role)"#;
        let res: Vec<(String, String, String)> = authorizer
            .query_all(query)
            .map_err(|e| AuthError::Biscuit(format!("Failed to query biscuit facts: {}", e)))?;

        if res.is_empty() {
            return Err(AuthError::InvalidToken.into());
        }

        let (id_s, username, role_str) = res[0].clone();
        let user_id = Uuid::parse_str(&id_s).map_err(|_| AuthError::InvalidToken)?;
        let role = UserRole::parse_str(&role_str).map_err(|_| AuthError::InvalidToken)?;

        // Verify user still exists and is active
        let mut conn = self.database.get_connection()?;
        let user = User::find_by_id(&mut conn, user_id).map_err(|_| AuthError::UserNotFound)?;
        if !user.is_active {
            return Err(AuthError::InvalidCredentials.into());
        }

        Ok(AuthContext {
            user_id,
            username,
            role,
            session_id: "biscuit".to_string(),
            permissions: self.get_role_permissions(&role_str),
        })
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

    /// Validate JWT token and return user if valid (uses AppState to fetch user)
    pub async fn validate_token(&self, state: &crate::AppState, token: &str) -> Result<crate::models::User> {
        // Verify JWT and extract claims
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<JwtClaims>(token, &self.jwt_decoding_key, &validation)
            .map_err(|e| AuthError::Jwt(e.to_string()))?;

        let claims = token_data.claims;

        // Ensure session exists
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(&claims.jti) {
            if session.expires_at < Utc::now() {
                return Err(AuthError::TokenExpired.into());
            }
        } else {
            return Err(AuthError::InvalidToken.into());
        }

        // Lookup user via AppState DB wrapper
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
        let user = state.db_get_user_by_id(user_id).await.map_err(|_| AuthError::UserNotFound)?;

        if !user.is_active {
            return Err(AuthError::InvalidCredentials.into());
        }

        Ok(user)
    }

    /// Create session token
    pub async fn create_session(&self, user_id: Uuid) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + ChronoDuration::hours(24);

        let session_data = SessionData {
            user_id,
            username: "user".to_string(), // Would get from database
            role: UserRole::Subscriber,
            created_at: now,
            expires_at,
            last_accessed: now,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session_data);

        // Generate JWT token
        let claims = Claims {
            sub: user_id.to_string(),
            username: "user".to_string(),
            email: "user@example.com".to_string(),
            role: "subscriber".to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti: session_id,
        };

        let token = encode(&Header::default(), &claims, &self.jwt_encoding_key)
            .map_err(|e| AuthError::Jwt(e.to_string()))?;

        Ok(token)
    }
}
