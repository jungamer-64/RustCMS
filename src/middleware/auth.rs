use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::{
    auth::AuthService,
    app::AppState,
    error::Result,
};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Skip authentication for public routes
    let path = req.uri().path();
    if is_public_route(path) {
        return Ok(next.run(req).await);
    }

    // Extract authorization header
    let auth_header = req.headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match state.auth.validate_token(token).await {
        Ok(user) => {
            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED)
    }
}

fn is_public_route(path: &str) -> bool {
    matches!(path,
        "/health" |
        "/metrics" |
        "/api/public/posts" |
        "/api/public/posts/*" |
        "/api/auth/login" |
        "/api/auth/register" |
        "/docs" |
        "/docs/*" |
        "/redoc" |
        "/openapi.json"
    )
}
