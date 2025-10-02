//! Authentication Handlers
//!
//! ユーザーの登録/認証/トークン再発行など、認可前提のパブリック API を提供します。
//! 各ハンドラは `AppState` のラッパ経由で DB/キャッシュ/トークン発行を行い、
//! 共通のメトリクスやロギング、エラー整形を集中管理します。
//!
//! 小さな契約（コントラクト）:
//! - 入力: `axum` によって JSON ボディが `serde` で検証/デシリアライズされる。
//! - 出力: 成功時は `ApiOk(T)` ラッパで JSON を返す。エラー時は `AppError` に集約され HTTP ステータスへ変換。
//! - セキュリティ: 認証済みが必要なエンドポイントは `BearerAuth` 等のセキュリティ定義と、実際のミドルウェア検証を前提にする。
//! - トークン: 成功時の認証系レスポンスは「統一トークン表現」(`AuthSuccessResponse`) を返す（`tokens.access_token` など）。
//!
//! トークンのライフサイクル概要:
//! - `login`/`register` でアクセストークン・リフレッシュトークンを発行。
//! - `refresh` でリフレッシュトークンを検証し、成功時に両トークンを回転（ローテーション）する。
//! - `logout` はトークン失効のためのフック（実装によりブラックリストやセッション無効化を行う）。
//!
//! フィーチャーフラグ:
//! - `monitoring`: 特定イベントのカウンタを発火。
//! - `search`: 登録時にユーザーを検索インデックスに反映（失敗しても本体処理は継続）。

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;

use crate::utils::response_ext::ApiOk;
use crate::{
    AppState, Result,
    auth::{AuthContext, LoginRequest},
    models::CreateUserRequest,
};

/// Registration request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Register a new user
/// 新規ユーザーを登録します。
/// - 認証ハンドラ: Biscuit認証専用
/// - デフォルトのロールは `Subscriber`。
/// - 必要に応じてメール検証や追加プロファイル設定は上位層で実施してください。
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = crate::utils::auth_response::AuthSuccessResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    // Create user request (入力値は事前に axum/serde で検証済み)
    let create_request = CreateUserRequest {
        username: request.username,
        email: request.email,
        password: request.password,
        first_name: request.first_name,
        last_name: request.last_name,
        role: crate::models::UserRole::Subscriber, // Default role
    };

    // AppState の auth ラッパを利用してユーザー作成（認証/DB メトリクスを一元的に記録）
    let user = state.auth_create_user(create_request).await?;

    // 検索統合が有効な場合、ユーザーを検索インデックスに登録（失敗しても本処理は継続）
    #[cfg(feature = "search")]
    if let Err(e) = state.search.index_user(&user).await {
        // Log error but don't fail the registration
        eprintln!("Failed to index user for search: {e}");
    }

    // 新しい統一レスポンスの生成（旧フラット形式は feature ベースで互換提供）
    let unified = state.auth_build_success_response(user, false).await?;
    Ok((StatusCode::CREATED, ApiOk(unified)))
}

/// Login user
/// 認証を行います。
///
/// - `remember_me` が `true` の場合は長めの有効期限を採用する設定（実装側のポリシーに依存）。
/// - レート制限や CAPTCHA 等の追加防御はミドルウェア/フロントで行ってください。
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
                    summary = "ログイン成功 (unified Biscuit authentication)",
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
/// アクセストークンの検証に成功した場合、セッションストアから該当セッションを
/// 削除し、以後のトークン利用を無効化します。
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
    State(state): State<AppState>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<impl IntoResponse> {
    state.auth_logout(auth_ctx.session_id.as_ref()).await?;

    Ok(ApiOk(json!({
        "success": true,
        "message": "Successfully logged out"
    })))
}

/// Get current user profile
/// 現在のユーザープロファイルを取得します（ダミー実装）。
///
/// 実運用では、検証済みのアクセストークンからユーザー ID を取得し、
/// DB もしくはキャッシュからプロファイルを引いて返却します。
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
/// Refresh access/refresh tokens.
///
/// リフレッシュトークンの検証に成功した場合、アクセストークンとリフレッシュトークンの
/// 両方をローテーションして返します。検証に失敗したトークンは再利用できません。
///
/// # Errors
/// - 無効または期限切れのリフレッシュトークン（401）。
/// - 内部エラー（500）。
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<impl IntoResponse> {
    let unified = state
        .auth_refresh_success_response(&body.refresh_token)
        .await?;
    Ok(ApiOk(unified))
}
