//! 認証ミドルウェア (Phase 5.2 - 新AppState対応版)

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
use tracing::{debug, warn};

/// Authorization ヘッダの簡易パーサ
/// Biscuit 認証トークンをサポート (Bearer または Biscuit スキーム)
#[must_use]
pub fn parse_authorization_header(value: &str) -> Option<&str> {
    let trimmed_val = value.trim_start();
    trimmed_val
        .strip_prefix("Biscuit ")
        .or_else(|| trimmed_val.strip_prefix("Bearer "))
        .map(str::trim)
}

/// 認証ミドルウェア (Phase 5.2 版)
///
/// # Errors
///
/// 認証情報が欠落・不正・検証失敗の場合は `AppError::Authentication` を返します。
/// アプリケーション状態が取得できないなど内部要因の場合は `AppError::Internal` を返します。
///
/// # TODO Phase 5.3+
/// - Biscuit トークン検証の完全実装
/// - JWT トークンのサポート
/// - トークンリフレッシュ機能
pub async fn auth_middleware(
    State(_state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(parse_authorization_header)
        .ok_or_else(|| {
            warn!("Authorization header is missing or invalid.");
            AppError::Authentication("Missing or malformed authorization header.".to_string())
        })?;

    // TODO Phase 5.3: Biscuit/JWT トークン検証の実装
    // match state.auth_verify_biscuit(token).await {
    //     Ok(ctx) => {
    //         req.extensions_mut().insert(ctx);
    //         Ok(next.run(req).await)
    //     }
    //     Err(e) => {
    //         warn!("Token validation failed: {:?}", e);
    //         Err(AppError::Authentication("Invalid or expired token.".to_string()))
    //     }
    // }
    
    warn!("Auth middleware is using temporary implementation. Phase 5.3 で完全実装予定。");
    debug!("Token received: {} bytes", token.len());
    
    // 暫定: 開発中はトークンがあれば通過させる
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authorization_header() {
        // 正常なケース
        assert_eq!(
            parse_authorization_header("Biscuit mytoken"),
            Some("mytoken")
        );
        assert_eq!(
            parse_authorization_header("Bearer mytoken"),
            Some("mytoken")
        );
        assert_eq!(
            parse_authorization_header("  Bearer   mytoken  "),
            Some("mytoken")
        );

        // エッジケース
        assert_eq!(parse_authorization_header("Basic someauth"), None);
        assert_eq!(parse_authorization_header("Bearer "), Some(""));
        assert_eq!(parse_authorization_header(""), None);
        assert_eq!(parse_authorization_header("   "), None);

        // 大文字小文字の区別
        assert_eq!(parse_authorization_header("bearer mytoken"), None);
        assert_eq!(parse_authorization_header("BEARER mytoken"), None);

        // 複数のスペース
        assert_eq!(
            parse_authorization_header("Bearer    mytoken"),
            Some("mytoken")
        );
        assert_eq!(
            parse_authorization_header("Biscuit    mytoken"),
            Some("mytoken")
        );
    }
}
