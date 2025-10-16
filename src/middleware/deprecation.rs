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
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// v1 API 削除予定日 (RFC 2616 形式)
const SUNSET_DATE: &str = "Sun, 17 Mar 2025 00:00:00 GMT";



/// Axum middleware function (Phase 5-4)
/// 
/// v1 エンドポイントに RFC 8594 準拠の Deprecation ヘッダーを追加します。
/// routes/mod.rs から `middleware::from_fn(add_deprecation_headers)` で使用。
/// 
/// # 追加されるヘッダー
/// - `Deprecation: true`
/// - `Sunset: Sun, 17 Mar 2025 00:00:00 GMT`
/// - `Link: </api/v2/...>; rel="successor-version"`
/// - `Warning: 299 - "Deprecation: ..."`
pub async fn add_deprecation_headers(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;

    if path.contains("/api/v1/") {
        let headers = response.headers_mut();

        // 1. Deprecation: true (RFC 8594)
        headers.insert("Deprecation", HeaderValue::from_static("true"));

        // 2. Sunset: RFC 2616 日付形式
        headers.insert("Sunset", HeaderValue::from_static(SUNSET_DATE));

        // 3. Link: successor-version
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

    response
}

/// Deprecated: 旧関数名（後方互換性のため残す）
#[deprecated(since = "3.0.0", note = "Use `add_deprecation_headers` instead")]
pub async fn deprecation_middleware(req: Request<Body>, next: Next) -> impl IntoResponse {
    add_deprecation_headers(req, next).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_sunset_date_format() {
        // RFC 2616 形式の検証
        assert_eq!(SUNSET_DATE, "Sun, 17 Mar 2025 00:00:00 GMT");
    }

    #[test]
    fn test_v1_path_to_v2_conversion() {
        let paths = vec![
            ("/api/v1/users/123", "/api/v2/users/123"),
            ("/api/v1/posts", "/api/v2/posts"),
            (
                "/api/v1/comments/456/approve",
                "/api/v2/comments/456/approve",
            ),
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
        assert_eq!(
            link_header,
            "</api/v2/users/123>; rel=\"successor-version\""
        );
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
