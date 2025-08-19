use axum::{
    body::Body,
    extract::Request,
    http::{header::CONTENT_ENCODING, StatusCode},
    middleware::Next,
    response::Response,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

pub async fn compression_middleware(
    req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    let accepts_gzip = req.headers()
        .get("accept-encoding")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.contains("gzip"))
        .unwrap_or(false);

    let response = next.run(req).await;

    if !accepts_gzip {
        return Ok(response);
    }

    // Only compress text-based content types
    let content_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !should_compress(content_type) {
        return Ok(response);
    }

    // For now, return the response as-is
    // Full compression implementation would require body streaming
    Ok(response)
}

fn should_compress(content_type: &str) -> bool {
    content_type.starts_with("text/") ||
    content_type.starts_with("application/json") ||
    content_type.starts_with("application/javascript") ||
    content_type.starts_with("application/xml")
}
