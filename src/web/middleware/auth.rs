//! 認証ミドルウェア (Phase 5.3 - JWT + Biscuit 完全実装)
//!
//! # 役割分担
//! - **JWT**: ユーザー認証 (Who are you?)
//! - **Biscuit**: リソース認可 (What can you do?)
//!
//! # 処理フロー
//! 1. Authorization ヘッダーから JWT トークンを抽出
//! 2. JWT トークンを検証してユーザー情報を取得
//! 3. Biscuit トークンを検証して権限情報を抽出
//! 4. 統合認証コンテキストを生成して request extensions に格納

use crate::auth::{AuthError, UnifiedAuthContext};
use crate::error::AppError;
use crate::infrastructure::app_state::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Authorization ヘッダのパース結果
#[derive(Debug)]
enum AuthorizationScheme<'a> {
    /// JWT Bearer トークン (認証用)
    Bearer(&'a str),
    /// Biscuit トークン (認可用 - 将来の拡張用)
    Biscuit(&'a str),
}

/// Authorization ヘッダをパースしてスキームとトークンを抽出
///
/// サポートされるスキーム:
/// - `Bearer <jwt_token>` - JWT 認証 (推奨)
/// - `Biscuit <biscuit_token>` - Biscuit 認可 (将来の拡張用)
fn parse_authorization_header(value: &str) -> Option<AuthorizationScheme<'_>> {
    let trimmed = value.trim_start();

    if let Some(token) = trimmed.strip_prefix("Bearer ") {
        return Some(AuthorizationScheme::Bearer(token.trim()));
    }

    if let Some(token) = trimmed.strip_prefix("Biscuit ") {
        return Some(AuthorizationScheme::Biscuit(token.trim()));
    }

    None
}

/// 認証ミドルウェア (Phase 5.4.4 更新)
///
/// # 処理内容
/// 1. JWT トークンを検証してユーザー認証
/// 2. オプションで Biscuit トークンから権限を抽出
/// 3. セッションの有効性をチェック
/// 4. 統合認証コンテキストを生成
/// 5. request extensions に格納して後続のハンドラで利用可能にする
///
/// # Errors
///
/// - 認証ヘッダが欠落または不正な場合
/// - JWT トークンが無効または期限切れの場合
/// - セッションが無効な場合
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Authorization ヘッダーを取得 (JWT)
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            debug!("Authorization header is missing");
            AppError::Authentication("Missing authorization header".to_string())
        })?;

    // 2. ヘッダーをパース
    let auth_scheme = parse_authorization_header(auth_header).ok_or_else(|| {
        warn!("Invalid authorization scheme");
        AppError::Authentication("Invalid authorization scheme".to_string())
    })?;

    // 3. JWT トークンを検証
    let mut context = match auth_scheme {
        AuthorizationScheme::Bearer(jwt_token) => verify_jwt_token(&state, jwt_token).await?,
        AuthorizationScheme::Biscuit(_biscuit_token) => {
            // Biscuit 単体での認証は未サポート
            warn!("Biscuit-only authentication is not yet implemented");
            return Err(AppError::Authentication(
                "Biscuit-only authentication is not yet supported".to_string(),
            ));
        }
    };

    // 4. オプションで Biscuit トークンから権限を抽出 (Phase 5.4.4)
    if let Some(biscuit_header) = req.headers().get("X-Biscuit-Token") {
        if let Ok(biscuit_token) = biscuit_header.to_str() {
            if let Some(biscuit_scheme) =
                parse_authorization_header(&format!("Biscuit {}", biscuit_token))
            {
                if let AuthorizationScheme::Biscuit(token) = biscuit_scheme {
                    // Biscuit トークンから権限を抽出
                    // TODO: 実際の Biscuit 検証とパーミッション抽出
                    // 現在は仮の実装として、トークン自体を保存
                    context.biscuit_token = Some(token.to_string());
                    debug!("Biscuit token attached to context");
                }
            }
        }
    }

    // 5. セッションの有効性をチェック (Phase 5.5.2)
    if let Some(session_store) = state.session_store() {
        use chrono::Utc;

        let session_id = context.session_id.clone();
        let now = Utc::now();

        // セッションを検証 (JWT に含まれるセッションバージョンを使用)
        if let Err(e) = session_store
            .as_ref()
            .validate_access(session_id, context.session_version, now)
            .await
        {
            warn!("Session validation failed: {:?}", e);
            return Err(AppError::Authentication(
                "Session is invalid or expired".to_string(),
            ));
        }

        debug!("Session validated successfully");
    }

    debug!(
        user_id = %context.user_id,
        username = %context.username,
        role = ?context.role,
        has_biscuit = context.biscuit_token.is_some(),
        "Authentication successful"
    );

    // 6. コンテキストを request extensions に格納
    req.extensions_mut().insert(context);

    // 7. 次のミドルウェア/ハンドラへ
    Ok(next.run(req).await)
}

/// JWT トークンを検証して認証コンテキストを生成 (Phase 5.4.3 更新)
async fn verify_jwt_token(state: &AppState, token: &str) -> Result<UnifiedAuthContext, AppError> {
    // JWT サービスを AppState から取得 (Phase 5.4.3)
    let jwt_service = state.jwt_service().ok_or_else(|| {
        error!("JWT service not initialized in AppState");
        AppError::Internal("JWT service not available".to_string())
    })?;

    // JWT トークンを検証
    let claims = jwt_service
        .verify_access_token(token)
        .map_err(|e| match e {
            AuthError::TokenExpired => {
                debug!("JWT token expired");
                AppError::Authentication("Token expired".to_string())
            }
            AuthError::InvalidTokenFormat | AuthError::InvalidTokenSignature => {
                warn!("Invalid JWT token: {:?}", e);
                AppError::Authentication("Invalid token".to_string())
            }
            _ => {
                error!("JWT verification error: {:?}", e);
                AppError::Authentication("Authentication failed".to_string())
            }
        })?;

    // 統合認証コンテキストを生成
    let context = UnifiedAuthContext::from_jwt(&claims).map_err(|e| {
        error!("Failed to create auth context: {:?}", e);
        AppError::Internal("Failed to create auth context".to_string())
    })?;

    // TODO Phase 5.4.4: Biscuit 権限を追加
    // if let Some(biscuit_token) = extract_biscuit_from_header() {
    //     context = context.with_biscuit_permissions(permissions, Some(biscuit_token));
    // }

    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authorization_header_bearer() {
        let result = parse_authorization_header("Bearer mytoken123");
        assert!(matches!(
            result,
            Some(AuthorizationScheme::Bearer("mytoken123"))
        ));
    }

    #[test]
    fn test_parse_authorization_header_biscuit() {
        let result = parse_authorization_header("Biscuit mytoken456");
        assert!(matches!(
            result,
            Some(AuthorizationScheme::Biscuit("mytoken456"))
        ));
    }

    #[test]
    fn test_parse_authorization_header_with_spaces() {
        let result = parse_authorization_header("Bearer    token_with_spaces  ");
        assert!(matches!(result, Some(AuthorizationScheme::Bearer(t)) if t == "token_with_spaces"));
    }

    #[test]
    fn test_parse_authorization_header_invalid() {
        assert!(parse_authorization_header("Basic sometoken").is_none());
        assert!(parse_authorization_header("InvalidScheme").is_none());
        assert!(parse_authorization_header("").is_none());
    }

    #[test]
    fn test_parse_authorization_header_case_sensitive() {
        // Bearer と bearer は区別される (標準準拠)
        assert!(parse_authorization_header("bearer token").is_none());
        assert!(parse_authorization_header("BEARER token").is_none());
    }
}
