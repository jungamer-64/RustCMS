//! Authentication Handlers
//!
//! Handles user authentication, registration, and session management

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
#[cfg(all(feature = "legacy-auth-flat", feature = "monitoring"))]
use metrics::counter;
use serde::Deserialize;
#[cfg(feature = "legacy-auth-flat")]
use serde::Serialize;
use serde_json::json;
use utoipa::ToSchema;

#[cfg(feature = "legacy-auth-flat")]
use crate::utils::auth_response::AuthSuccessResponse;

#[cfg(feature = "legacy-auth-flat")]
use crate::utils::common_types::UserInfo;
use crate::utils::response_ext::ApiOk;
use crate::{AppState, Result, auth::LoginRequest, models::CreateUserRequest};

/// Registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[cfg(feature = "legacy-auth-flat")]
/// 旧 `LoginResponse` 型 (feature = `legacy-auth-flat` 有効時のみ公開) - 新規コードは `AuthSuccessResponse` を使用してください
#[allow(dead_code)]
#[allow(deprecated)]
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub success: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub biscuit_token: String,
    pub user: UserInfo,
    pub expires_in: i64,
    pub session_id: String,
    pub token: String,
}

#[cfg(feature = "legacy-auth-flat")]
#[allow(deprecated)]
impl From<AuthSuccessResponse> for LoginResponse {
    fn from(a: AuthSuccessResponse) -> Self {
        // Emit one-time runtime warning when legacy LoginResponse mapping is exercised.
        #[cfg(feature = "legacy-auth-flat")]
        {
            use crate::utils::deprecation::warn_once;
            warn_once(
                "legacy_login_response",
                "LoginResponse conversion invoked; migrate consumers to AuthSuccessResponse (flattened fields removed in 3.0.0).",
            );
            // Metrics: track remaining legacy LoginResponse conversions
            #[cfg(feature = "monitoring")]
            counter!("legacy_login_response_conversion_total").increment(1);
        }
        // Avoid referencing deprecated flattened fields directly; derive from unified tokens.
        let AuthSuccessResponse {
            success,
            tokens,
            user,
            ..
        } = a;
        // access_token は token と重複して返すため、一度だけ clone/コピーを行う
        let access_token = tokens.access_token;
        Self {
            success,
            token: access_token.clone(),
            access_token,
            refresh_token: tokens.refresh_token,
            biscuit_token: tokens.biscuit_token,
            user,
            expires_in: tokens.expires_in,
            session_id: tokens.session_id,
        }
    }
}

/// Register a new user
/// 新規ユーザーを登録します。
///
/// # Errors
/// - 入力の検証に失敗した場合。
/// - 既存ユーザーとの一意制約に違反した場合。
/// - 内部エラーが発生した場合。
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = crate::utils::auth_response::AuthSuccessResponse,
            examples((
                "Registered" = (
                    summary = "登録成功 (unified tokens; flat legacy fields may appear when feature auth-flat-fields is enabled)",
                    value = json!({
                        "success": true,
                        "tokens": {
                            "access_token": "ACCESS_TOKEN_SAMPLE",
                            "refresh_token": "REFRESH_TOKEN_SAMPLE",
                            "biscuit_token": "BISCUIT_TOKEN_SAMPLE",
                            "expires_in": 3600,
                            "session_id": "sess_123"
                        },
                        "user": {"id": "1d2e3f40-1111-2222-3333-444455556666", "username": "alice", "email": "alice@example.com", "role": "subscriber"}
                        // NOTE: When feature auth-flat-fields is ON the response also repeats
                        // access_token / refresh_token / biscuit_token / expires_in / session_id / token at top-level (deprecated)
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
        eprintln!("Failed to index user for search: {e}");
    }

    // Build full unified auth success response via new convenience wrapper
    let unified = state.auth_build_success_response(user, false).await?;
    Ok((StatusCode::CREATED, ApiOk(unified)))
}

/// Login user
/// 認証を行います。
///
/// # Errors
/// - 資格情報が不正な場合。
/// - 内部エラーが発生した場合。
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login success", body = crate::utils::auth_response::AuthSuccessResponse,
            examples((
                "LoggedIn" = (
                    summary = "ログイン成功 (unified tokens; flat legacy fields may appear when feature auth-flat-fields is enabled)",
                    value = json!({
                        "success": true,
                        "tokens": {
                            "access_token": "ACCESS_TOKEN_SAMPLE",
                            "refresh_token": "REFRESH_TOKEN_SAMPLE",
                            "biscuit_token": "BISCUIT_TOKEN_SAMPLE",
                            "expires_in": 3600,
                            "session_id": "sess_123"
                        },
                        "user": {"id": "1d2e3f40-1111-2222-3333-444455556666", "username": "alice", "email": "alice@example.com", "role": "subscriber"}
                        // NOTE: When feature auth-flat-fields is ON the response also repeats deprecated flat fields
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
    // Build full unified auth success response via convenience wrapper
    let unified = state.auth_build_success_response(user, remember).await?;
    Ok(ApiOk(unified))
}

/// Logout user
/// 認証セッションを無効化します。
///
/// # Errors
/// - 認証情報が欠如/無効な場合。
/// - 内部エラーが発生した場合。
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
/// 現在のユーザープロファイルを取得します（ダミー実装）。
///
/// # Errors
/// - 未認証の場合。
/// - 内部エラーが発生した場合。
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
pub struct RefreshRequest {
    pub refresh_token: String,
}

// Legacy RefreshResponse removed: handler now always returns AuthSuccessResponse

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = crate::utils::auth_response::AuthSuccessResponse,
            examples((
                "Refreshed" = (
                    summary = "トークン更新成功",
                    value = json!({
                        "success": true,
                        "tokens": {"access_token": "NEW_ACCESS_TOKEN_SAMPLE", "refresh_token": "NEW_REFRESH_TOKEN_SAMPLE", "biscuit_token": "", "expires_in": 3600, "session_id": "sess_123"},
                        "user": {"id": "uuid", "username": "user", "email": "user@example.com", "first_name": null, "last_name": null, "role": "Subscriber", "is_active": true, "email_verified": true, "last_login": null, "created_at": "2025-01-01T00:00:00Z", "updated_at": "2025-01-01T00:00:00Z"},
                        "access_token": "NEW_ACCESS_TOKEN_SAMPLE",
                        "refresh_token": "NEW_REFRESH_TOKEN_SAMPLE",
                        "biscuit_token": "",
                        "expires_in": 3600,
                        "session_id": "sess_123",
                        "token": "NEW_ACCESS_TOKEN_SAMPLE"
                    })
                )
            ))
        ),
        (status = 401, description = "Invalid or expired refresh token"),
        (status = 500, description = "Server error")
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<impl IntoResponse> {
    let unified = state
        .auth_refresh_success_response(&body.refresh_token)
        .await?;
    Ok(ApiOk(unified))
}
