use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use uuid::Uuid;
use validator::Validate;

// エラー型
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Unauthorized(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string()),
        };

        let body = Json(serde_json::json!({
            "error": message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // User ID
    pub username: String,
    pub role: String,
    pub exp: i64,         // Expiration time
    pub iat: i64,         // Issued at
}

// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: chrono::DateTime<Utc>,
}

// Login request
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 3))]
    pub username: String,
    #[validate(length(min = 6))]
    pub password: String,
}

// Auth service
#[derive(Clone)]
pub struct AuthService {
    jwt_secret: String,
}

impl AuthService {
    pub fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }

    pub fn hash_password(&self, password: &str) -> Result<String, AppError> {
        hash(password, DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        verify(password, hash)
            .map_err(|e| AppError::Internal(format!("Password verification failed: {}", e)))
    }

    pub fn generate_token(&self, user: &User) -> Result<String, AppError> {
        let now = Utc::now();
        let exp = now + Duration::hours(24);

        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            role: user.role.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.jwt_secret.as_bytes());

        encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        let decoding_key = DecodingKey::from_secret(self.jwt_secret.as_bytes());
        let validation = Validation::new(Algorithm::HS256);

        decode::<Claims>(token, &decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))
    }
}

// App state
#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
}

// Handlers
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now()
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // バリデーション
    request.validate()
        .map_err(|e| AppError::Validation(format!("Validation failed: {}", e)))?;

    // デモ用のユーザー（本来はデータベースから取得）
    let demo_user = if request.username == "admin" {
        let password_hash = state.auth_service.hash_password("password")?;
        User {
            id: Uuid::new_v4(),
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            password_hash,
            role: "admin".to_string(),
            created_at: Utc::now(),
        }
    } else {
        return Err(AppError::Unauthorized("User not found".to_string()));
    };

    // パスワード検証
    let is_valid = state.auth_service.verify_password(&request.password, &demo_user.password_hash)?;
    if !is_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // トークン生成
    let token = state.auth_service.generate_token(&demo_user)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "token": token,
        "user": {
            "id": demo_user.id,
            "username": demo_user.username,
            "role": demo_user.role
        }
    })))
}

pub async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "Logged out successfully"
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ログ設定
    tracing_subscriber::fmt::init();

    // アプリケーション状態
    let auth_service = AuthService::new("your-secret-key".to_string());
    let state = AppState { auth_service };

    // ルーター
    let app = Router::new()
        .route("/", get(|| async { "Rust CMS Simple Backend" }))
        .route("/health", get(health))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // サーバー起動
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("🚀 Simple CMS Server starting on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
