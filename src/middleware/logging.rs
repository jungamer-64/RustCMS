use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{info, warn};

#[inline]
fn level_for_status(status: axum::http::StatusCode) -> tracing::Level {
    if status.is_server_error() {
        tracing::Level::ERROR
    } else if status.is_client_error() {
        tracing::Level::WARN
    } else {
        tracing::Level::INFO
    }
}

#[inline]
fn log_with_level(
    level: tracing::Level,
    method: &axum::http::Method,
    uri: &axum::http::Uri,
    status: axum::http::StatusCode,
    duration: std::time::Duration,
) {
    match level {
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
}

pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status();
    let duration = start.elapsed();
    let log_level = level_for_status(status);
    log_with_level(log_level, &method, &uri, status, duration);

    response
}
