//! 認証ハンドラー (Phase 5.4.5)
//!
//! JWT トークンのリフレッシュエンドポイントを提供します。

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::{
    auth::{JwtService, AuthError},
    common::type_utils::common_types::SessionId,
    error::AppError,
    infrastructure::app_state::AppState,
};

/// リフレッシュトークンリクエスト
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// リフレッシュトークン
    pub refresh_token: String,
}

/// リフレッシュトークンレスポンス
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    /// 新しいアクセストークン
    pub access_token: String,
    /// 新しいリフレッシュトークン (オプション)
    pub refresh_token: Option<String>,
    /// トークンの有効期限 (Unix timestamp)
    pub expires_at: i64,
}

/// リフレッシュトークンを使用して新しいアクセストークンを取得
///
/// # エンドポイント
/// POST /api/v2/auth/refresh
///
/// # リクエストボディ
/// ```json
/// {
///   "refresh_token": "eyJ..."
/// }
/// ```
///
/// # レスポンス
/// ```json
/// {
///   "access_token": "eyJ...",
///   "refresh_token": "eyJ...",
///   "expires_at": 1234567890
/// }
/// ```
///
/// # エラー
/// - 400 Bad Request: リフレッシュトークンが無効
/// - 401 Unauthorized: リフレッシュトークンが期限切れ
/// - 500 Internal Server Error: サーバーエラー
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, AppError> {
    info!("Refresh token request received");

    // JWT サービスを取得
    let jwt_service = state.jwt_service()
        .ok_or_else(|| {
            error!("JWT service not initialized in AppState");
            AppError::Internal("JWT service not available".to_string())
        })?;

    // リフレッシュトークンを検証
    let claims = jwt_service
        .verify_refresh_token(&payload.refresh_token)
        .map_err(|e| {
            match e {
                AuthError::TokenExpired => {
                    debug!("Refresh token expired");
                    AppError::Authentication("Refresh token expired".to_string())
                }
                AuthError::InvalidToken => {
                    debug!("Invalid refresh token");
                    AppError::Authentication("Invalid refresh token".to_string())
                }
                _ => {
                    error!("Refresh token verification error: {:?}", e);
                    AppError::Authentication("Token verification failed".to_string())
                }
            }
        })?;

    debug!(
        user_id = %claims.user_id(),
        username = %claims.username,
        "Refresh token verified successfully"
    );

    // 新しいトークンペアを生成
    let token_pair = jwt_service
        .generate_token_pair(
            claims.user_id(),
            claims.username.clone(),
            claims.role.clone(),
            SessionId::from(claims.session_id.clone()),
            false, // remember_me は false (通常のリフレッシュ)
        )
        .map_err(|e| {
            error!("Failed to generate new token pair: {:?}", e);
            AppError::Internal("Failed to generate new tokens".to_string())
        })?;

    info!(
        user_id = %claims.user_id(),
        username = %claims.username,
        "New token pair generated successfully"
    );

    Ok(Json(RefreshTokenResponse {
        access_token: token_pair.access_token,
        refresh_token: Some(token_pair.refresh_token),
        expires_at: token_pair.expires_at.timestamp(),
    }))
}

/// ヘルスチェック用の簡易テスト
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_token_request_deserialization() {
        let json = r#"{"refresh_token": "test_token"}"#;
        let request: RefreshTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.refresh_token, "test_token");
    }

    #[test]
    fn test_refresh_token_response_serialization() {
        let response = RefreshTokenResponse {
            access_token: "new_access".to_string(),
            refresh_token: Some("new_refresh".to_string()),
            expires_at: 1234567890,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("new_access"));
        assert!(json.contains("new_refresh"));
    }
}
