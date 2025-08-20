//! Authentication Handlers
//! 
//! Handles user authentication, registration, and session management

use axum::{
    response::{IntoResponse, Json},
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState, Result,
    models::{User, CreateUserRequest},
    auth::LoginRequest,
};

/// Registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: String,
    pub user: UserInfo,
    pub expires_in: i64,
}

/// User info for responses
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
}

impl From<&User> for UserInfo {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: user.role.clone(),
            is_active: user.is_active,
        }
    }
}

/// Register a new user
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    // Create user request
    let create_request = CreateUserRequest {
        username: request.username,
        email: request.email,
        password: request.password,
        first_name: request.first_name,
        last_name: request.last_name,
        role: crate::models::UserRole::Subscriber, // Default role
    };

    // Create user through auth service
    let user = state.auth.create_user(create_request).await?;
    
    // Index user for search (optional feature)
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_user(&user).await {
        // Log error but don't fail the registration
        eprintln!("Failed to index user for search: {}", e);
    }

    // Generate session token
    let token = state.auth.create_session(user.id).await?;

    let response = LoginResponse {
        success: true,
        token,
        user: UserInfo::from(&user),
        expires_in: 3600, // 1 hour
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Login user
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    // Authenticate user
    let user = state.auth.authenticate_user(request).await?;
    
    // Generate session token
    let token = state.auth.create_session(user.id).await?;

    let response = LoginResponse {
        success: true,
        token,
        user: UserInfo::from(&user),
        expires_in: 3600, // 1 hour
    };

    Ok(Json(response))
}

/// Logout user
pub async fn logout(
    State(_state): State<AppState>,
    // Extract token from Authorization header in middleware
) -> Result<impl IntoResponse> {
    // In a real implementation, you'd extract the token from the Authorization header
    // and invalidate it in the auth service
    
    Ok(Json(json!({
        "success": true,
        "message": "Successfully logged out"
    })))
}

/// Get current user profile
pub async fn profile(
    State(_state): State<AppState>,
    // User would be extracted from middleware after token validation
) -> Result<impl IntoResponse> {
    // Placeholder - in real implementation, user ID would come from validated token
    Ok(Json(json!({
        "success": true,
        "message": "Profile endpoint - requires authentication middleware"
    })))
}

/// Refresh token
pub async fn refresh_token(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse> {
    // Placeholder for token refresh logic
    Ok(Json(json!({
        "success": true,
        "message": "Token refresh endpoint"
    })))
}
