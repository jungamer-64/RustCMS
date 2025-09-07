use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

/// Simple admin token header guard middleware.
/// Validates `x-admin-token` using utils::auth_utils::check_admin_token.
pub async fn admin_auth_layer(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = req.headers();
    if let Some(val) = headers.get("x-admin-token") {
        if !crate::utils::auth_utils::check_admin_token(val.to_str().unwrap_or("")) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(req).await)
}
