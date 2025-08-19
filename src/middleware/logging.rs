use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::{info, warn};
use std::time::Instant;

pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status();
    let duration = start.elapsed();

    let log_level = if status.is_server_error() {
        tracing::Level::ERROR
    } else if status.is_client_error() {
        tracing::Level::WARN
    } else {
        tracing::Level::INFO
    };

    match log_level {
        tracing::Level::ERROR => {
            tracing::error!(
                method = %method,
                uri = %uri,
                status = %status,
                duration = ?duration,
                "Request completed with error"
            );
        }
        tracing::Level::WARN => {
            warn!(
                method = %method,
                uri = %uri,
                status = %status,
                duration = ?duration,
                "Request completed with warning"
            );
        }
        _ => {
            info!(
                method = %method,
                uri = %uri,
                status = %status,
                duration = ?duration,
                "Request completed"
            );
        }
    }

    response
}
