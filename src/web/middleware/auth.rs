use crate::app::AppState;
use crate::error::AppError;
use axum::{extract::Request, http::header::AUTHORIZATION, middleware::Next, response::Response};
use tracing::{error, warn};

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

/// # Errors
///
/// 認証情報が欠落・不正・検証失敗の場合は `AppError::Authentication` を返します。
/// アプリケーション状態が取得できないなど内部要因の場合は `AppError::Internal` を返します。
pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(parse_authorization_header)
        .ok_or_else(|| {
            warn!("Authorization header is missing or invalid.");
            AppError::Authentication("Missing or malformed authorization header.".to_string())
        })?;

    let state = req.extensions().get::<AppState>().cloned().ok_or_else(|| {
        error!("AppState not found in request extensions. This is a configuration issue.");
        AppError::Internal("Internal server error.".to_string())
    })?;

    match state.auth_verify_biscuit(token).await {
        Ok(ctx) => {
            req.extensions_mut().insert(ctx);
            Ok(next.run(req).await)
        }
        Err(e) => {
            // セキュリティベストプラクティス: エラーメッセージを統一してタイミング攻撃を防ぐ
            // 内部エラー情報はログに記録し、クライアントには一般的なメッセージのみ返す
            warn!("Token validation failed: {:?}", e);
            Err(AppError::Authentication(
                "Invalid or expired token.".to_string(),
            ))
        }
    }
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
