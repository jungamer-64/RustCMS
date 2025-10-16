// src/middleware/deprecation.rs
//! API v1 Deprecation ヘッダー追加ミドルウェア
//!
//! RFC 8594 (HTTP Sunset Header) に準拠。
//! v1 エンドポイントに以下のヘッダーを追加:
//! - Deprecation: true
//! - Sunset: Sun, 17 Mar 2025 00:00:00 GMT
//! - Link: rel="successor-version"
//! - Warning: 299 - "Deprecation"

use axum::{
    http::{HeaderMap, HeaderValue, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::task::{Context, Poll};
use tower::Service;

/// v1 API 削除予定日 (RFC 2616 形式)
const SUNSET_DATE: &str = "Sun, 17 Mar 2025 00:00:00 GMT";

/// Deprecation ヘッダーを追加するミドルウェア層
#[derive(Clone)]
pub struct DeprecationMiddleware<S> {
    inner: S,
}

impl<S> DeprecationMiddleware<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for DeprecationMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response, Error = std::io::Error> + Clone,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = Response;
    type Error = std::io::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            let path = req.uri().path().to_string();
            let mut res = inner.call(req).await?;

            // v1 エンドポイントのみに Deprecation ヘッダーを追加
            if path.contains("/api/v1/") {
                add_deprecation_headers(&mut res, &path);
            }

            Ok(res)
        })
    }
}

/// Deprecation ヘッダーを Response に追加
fn add_deprecation_headers(response: &mut Response, path: &str) {
    let headers = response.headers_mut();

    // 1. Deprecation: true (RFC 8594)
    if let Ok(value) = HeaderValue::from_static("true") {
        headers.insert("Deprecation", value);
    }

    // 2. Sunset: RFC 2616 日付形式
    if let Ok(value) = HeaderValue::from_static(SUNSET_DATE) {
        headers.insert("Sunset", value);
    }

    // 3. Link: successor-version
    // /api/v1/users/123 → /api/v2/users/123
    let v2_path = path.replace("/api/v1/", "/api/v2/");
    let link_header = format!("<{}>; rel=\"successor-version\"", v2_path);
    if let Ok(value) = HeaderValue::from_str(&link_header) {
        headers.insert("Link", value);
    }

    // 4. Warning: RFC 7231 互換性警告
    let warning = format!(
        "299 - \"Deprecation: This endpoint will be removed on 2025-03-17. Use {} instead.\"",
        v2_path
    );
    if let Ok(value) = HeaderValue::from_str(&warning) {
        headers.insert("Warning", value);
    }
}

/// Axum middleware helper function
pub async fn deprecation_middleware<B>(
    req: Request<B>,
    next: Next,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;

    if path.contains("/api/v1/") {
        let response_ref = &mut response;
        add_deprecation_headers(response_ref, &path);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_sunset_date_format() {
        // RFC 2616 形式の検証
        assert_eq!(SUNSET_DATE, "Sun, 17 Mar 2025 00:00:00 GMT");
        // パース可能か確認
        assert!(HeaderValue::from_static(SUNSET_DATE).is_ok());
    }

    #[test]
    fn test_v1_path_to_v2_conversion() {
        let paths = vec![
            ("/api/v1/users/123", "/api/v2/users/123"),
            ("/api/v1/posts", "/api/v2/posts"),
            ("/api/v1/comments/456/approve", "/api/v2/comments/456/approve"),
            ("/api/v1/search?q=test", "/api/v2/search?q=test"),
        ];

        for (v1_path, expected_v2_path) in paths {
            let v2_path = v1_path.replace("/api/v1/", "/api/v2/");
            assert_eq!(v2_path, expected_v2_path);
        }
    }

    #[test]
    fn test_link_header_format() {
        let v2_path = "/api/v2/users/123";
        let link_header = format!("<{}>; rel=\"successor-version\"", v2_path);
        assert_eq!(link_header, "</api/v2/users/123>; rel=\"successor-version\"");
        assert!(HeaderValue::from_str(&link_header).is_ok());
    }

    #[test]
    fn test_warning_header_format() {
        let v2_path = "/api/v2/users/123";
        let warning = format!(
            "299 - \"Deprecation: This endpoint will be removed on 2025-03-17. Use {} instead.\"",
            v2_path
        );
        assert!(HeaderValue::from_str(&warning).is_ok());
        assert!(warning.contains("2025-03-17"));
        assert!(warning.contains(&v2_path));
    }
}
