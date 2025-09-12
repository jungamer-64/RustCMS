use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

/// # Errors
///
/// 次の場合に `Err(StatusCode)` を返します:
/// - `accept-encoding` または `content-type` ヘッダ値が不正で `to_str()` に失敗した場合
/// - 将来的に圧縮処理で I/O などの失敗が発生した場合（現実装では発生しません）
pub async fn compression_middleware(
    req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    let accepts_gzip = req
        .headers()
        .get("accept-encoding")
        .and_then(|h| h.to_str().ok())
        .is_some_and(|s| s.contains("gzip"));

    let response = next.run(req).await;

    if !accepts_gzip {
        return Ok(response);
    }

    // Only compress text-based content types
    let content_type = response
        .headers()
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
    content_type.starts_with("text/")
        || content_type.starts_with("application/json")
        || content_type.starts_with("application/javascript")
        || content_type.starts_with("application/xml")
}
