//! Authentication Handlers
//!
//! Handles user authentication, registration, and session management

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use serde_json::json;

use crate::utils::{common_types::UserInfo};
use crate::utils::response_ext::ApiOk;
use crate::{auth::LoginRequest, models::CreateUserRequest, AppState, Result};

/// Registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Login response
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub success: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub biscuit_token: String,
    pub user: UserInfo,
    pub expires_in: i64,
    pub session_id: String,
    /// 後方互換: 旧クライアントが `token` を参照している可能性があるため複製
    pub token: String,
}

impl From<crate::auth::AuthResponse> for LoginResponse {
    fn from(a: crate::auth::AuthResponse) -> Self {
        let access = a.access_token.clone();
        LoginResponse {
            success: true,
            access_token: access.clone(),
            refresh_token: a.refresh_token,
            biscuit_token: a.biscuit_token,
            user: a.user,
            expires_in: a.expires_in,
            session_id: a.session_id,
            token: access,
        }
    }
}

/// Register a new user
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = LoginResponse,
            examples((
                "Registered" = (
                    summary = "登録成功",
                    value = json!({
                        "success": true,
                        "access_token": "ACCESS_TOKEN_SAMPLE",
                        "refresh_token": "REFRESH_TOKEN_SAMPLE",
                        "biscuit_token": "BISCUIT_TOKEN_SAMPLE",
                        "user": {"id": "1d2e3f40-1111-2222-3333-444455556666", "username": "alice", "email": "alice@example.com", "role": "subscriber"},
                        "expires_in": 3600,
                        "session_id": "sess_123",
                        "token": "ACCESS_TOKEN_SAMPLE"
                    })
                )
            ))
        ),
        (status = 400, description = "Validation error", body = crate::utils::api_types::ApiResponse<serde_json::Value>),
        (status = 500, description = "Server error")
    )
)]
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

    // Create user through AppState auth wrapper (records auth & DB metrics centrally)
    let user = state.auth_create_user(create_request).await?;

    // Index user for search (optional feature)
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_user(&user).await {
        // Log error but don't fail the registration
        eprintln!("Failed to index user for search: {}", e);
    }

    // Build full auth response (access/refresh/biscuit + session) via AppState
    let auth = state.auth_build_auth_response(user, false).await?;
    Ok((StatusCode::CREATED, ApiOk(LoginResponse::from(auth))))
}

/// Login user
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login success", body = LoginResponse,
            examples((
                "LoggedIn" = (
                    summary = "ログイン成功",
                    value = json!({
                        "success": true,
                        "access_token": "ACCESS_TOKEN_SAMPLE",
                        "refresh_token": "REFRESH_TOKEN_SAMPLE",
                        "biscuit_token": "BISCUIT_TOKEN_SAMPLE",
                        "user": {"id": "1d2e3f40-1111-2222-3333-444455556666", "username": "alice", "email": "alice@example.com", "role": "subscriber"},
                        "expires_in": 3600,
                        "session_id": "sess_123",
                        "token": "ACCESS_TOKEN_SAMPLE"
                    })
                )
            ))
        ),
        (status = 400, description = "Validation error", body = crate::utils::api_types::ApiResponse<serde_json::Value>),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Server error")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    let remember = request.remember_me.unwrap_or(false);
    // Authenticate user via AppState wrapper (records auth & DB metrics centrally)
    let user = state.auth_authenticate(request).await?;

    // remember_me を先に取り出してムーブを防止
    // Build full auth response
    let auth = state.auth_build_auth_response(user, remember).await?;
    Ok(ApiOk(LoginResponse::from(auth)))
}

/// Logout user
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Auth",
    security(("BearerAuth" = [])),
    responses(
        (status = 200, description = "Logout success", examples((
            "LoggedOut" = (
                summary="ログアウト成功",
                value = json!({"success": true, "message": "Successfully logged out"})
            )
        ))),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn logout(
    State(_state): State<AppState>,
    // Extract token from Authorization header in middleware
) -> Result<impl IntoResponse> {
    // In a real implementation, you'd extract the token from the Authorization header
    // and invalidate it in the auth service

    Ok(ApiOk(json!({
        "success": true,
        "message": "Successfully logged out"
    })))
}

/// Get current user profile
#[utoipa::path(
    get,
    path = "/api/v1/auth/profile",
    tag = "Auth",
    security(("BearerAuth" = [])),
    responses(
        (status = 200, description = "Profile info placeholder", examples((
            "Profile" = (
                summary = "プロファイル例",
                value = json!({"success": true, "message": "Profile endpoint - requires authentication middleware"})
            )
        ))),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn profile(
    State(_state): State<AppState>,
    // User would be extracted from middleware after token validation
) -> Result<impl IntoResponse> {
    // Placeholder - in real implementation, user ID would come from validated token
    Ok(ApiOk(json!({
        "success": true,
        "message": "Profile endpoint - requires authentication middleware"
    })))
}

/// Refresh token
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest { pub refresh_token: String }

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse { pub success: bool, pub access_token: String, pub expires_in: i64, pub session_id: String, pub refresh_token: String }

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = RefreshResponse,
            examples((
                "Refreshed" = (
                    summary = "トークン更新成功",
                    value = json!({
                        "success": true,
                        "access_token": "NEW_ACCESS_TOKEN_SAMPLE",
                        "expires_in": 3600,
                        "session_id": "sess_123",
                        "refresh_token": "NEW_REFRESH_TOKEN_SAMPLE"
                    })
                )
            ))
        ),
        (status = 401, description = "Invalid or expired refresh token"),
        (status = 500, description = "Server error")
    )
)]
pub async fn refresh_token(State(state): State<AppState>, Json(body): Json<RefreshRequest>) -> Result<impl IntoResponse> {
    let refreshed = state.auth_refresh_access_token(&body.refresh_token).await?;
    let resp = RefreshResponse {
        success: true,
        access_token: refreshed.access_token,
        expires_in: refreshed.expires_in,
        session_id: refreshed.session_id,
        refresh_token: refreshed.refresh_token,
    };
    Ok(ApiOk(resp))
}
