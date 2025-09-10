use crate::app::AppState;
use axum::{
    extract::Request,
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

/// Authorization ヘッダの簡易パーサ
/// 許容スキーム: "Bearer" / "Biscuit"（どちらも同等に扱う）
pub fn parse_authorization_header(value: &str) -> Option<&str> {
    let v = value.trim();
    if let Some(rest) = v.strip_prefix("Bearer ") {
        return Some(rest.trim());
    }
    if let Some(rest) = v.strip_prefix("Biscuit ") {
        return Some(rest.trim());
    }
    None
}

pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Parse supported schemes and validate token -> inject AuthContext
    let token = parse_authorization_header(auth_header).ok_or(StatusCode::UNAUTHORIZED)?;

    let state = req
        .extensions()
        .get::<AppState>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    match state.auth_verify_biscuit(token).await {
        Ok(ctx) => {
            // Handlers can extract `Extension<crate::auth::AuthContext>`
            req.extensions_mut().insert(ctx);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
