//! Deprecation Headers 統合テスト（Phase 5-4）
//!
//! RFC 8594 準拠の Deprecation ヘッダーが全 v1 エンドポイントに付与されることを検証。
//!
//! テスト実行:
//! ```bash
//! cargo test --test deprecation_headers_integration --features "database auth"
//! ```

#[cfg(all(feature = "database", feature = "auth"))]
mod integration_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`

    /// テスト用の最小限の Router を構築（deprecation middleware 含む）
    fn create_test_router() -> Router {
        use axum::{Router, middleware, routing::get};
        use cms_backend::middleware::deprecation::add_deprecation_headers;

        // v1 routes with deprecation middleware
        let v1_routes = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route("/users", get(|| async { "Users list" }))
            .route("/posts", get(|| async { "Posts list" }))
            .layer(middleware::from_fn(add_deprecation_headers));

        // v2 routes without deprecation (comparison)
        let v2_routes = Router::new()
            .route("/health", get(|| async { "OK v2" }))
            .route("/users", get(|| async { "Users list v2" }));

        Router::new()
            .nest("/api/v1", v1_routes)
            .nest("/api/v2", v2_routes)
    }

    #[tokio::test]
    async fn test_v1_health_has_deprecation_header() {
        // Given: v1 health endpoint
        let app = create_test_router();

        // When: GET /api/v1/health
        let request = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: Deprecation ヘッダーが存在
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("deprecation"));
        assert_eq!(response.headers()["deprecation"], "true");
    }

    #[tokio::test]
    async fn test_v1_users_has_sunset_header() {
        // Given: v1 users endpoint
        let app = create_test_router();

        // When: GET /api/v1/users
        let request = Request::builder()
            .uri("/api/v1/users")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: Sunset ヘッダーが存在（2025-03-17）
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("sunset"));
        let sunset = response.headers()["sunset"].to_str().unwrap();
        assert!(sunset.contains("2025"));
        assert!(sunset.contains("Mar"));
    }

    #[tokio::test]
    async fn test_v1_posts_has_link_header() {
        // Given: v1 posts endpoint
        let app = create_test_router();

        // When: GET /api/v1/posts
        let request = Request::builder()
            .uri("/api/v1/posts")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: Link ヘッダーが v2 を指す
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("link"));
        let link = response.headers()["link"].to_str().unwrap();
        assert!(link.contains("/api/v2/posts"));
        assert!(link.contains("successor-version"));
    }

    #[tokio::test]
    async fn test_v1_health_has_warning_header() {
        // Given: v1 health endpoint
        let app = create_test_router();

        // When: GET /api/v1/health
        let request = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: Warning ヘッダーが存在
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("warning"));
        let warning = response.headers()["warning"].to_str().unwrap();
        assert!(warning.contains("299"));
        assert!(warning.contains("Deprecation"));
        assert!(warning.contains("2025-03-17"));
    }

    #[tokio::test]
    async fn test_v1_all_four_headers_present() {
        // Given: v1 users endpoint
        let app = create_test_router();

        // When: GET /api/v1/users
        let request = Request::builder()
            .uri("/api/v1/users")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: 全4つの RFC 8594 ヘッダーが存在
        assert_eq!(response.status(), StatusCode::OK);

        // 1. Deprecation: true
        assert!(response.headers().contains_key("deprecation"));
        assert_eq!(response.headers()["deprecation"], "true");

        // 2. Sunset: date
        assert!(response.headers().contains_key("sunset"));

        // 3. Link: successor-version
        assert!(response.headers().contains_key("link"));

        // 4. Warning: 299
        assert!(response.headers().contains_key("warning"));
    }

    #[tokio::test]
    async fn test_v2_health_no_deprecation_header() {
        // Given: v2 health endpoint
        let app = create_test_router();

        // When: GET /api/v2/health
        let request = Request::builder()
            .uri("/api/v2/health")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: Deprecation ヘッダーが**存在しない**
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key("deprecation"));
        assert!(!response.headers().contains_key("sunset"));
    }

    #[tokio::test]
    async fn test_v2_users_no_deprecation_header() {
        // Given: v2 users endpoint
        let app = create_test_router();

        // When: GET /api/v2/users
        let request = Request::builder()
            .uri("/api/v2/users")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();

        // Then: v2 は deprecation ヘッダーなし
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key("deprecation"));
    }
}

// ============================================================================
// Summary
// ============================================================================
//
// テスト数: 7個
// - v1 endpoints: Deprecation, Sunset, Link, Warning ヘッダー検証 (5 tests)
// - v2 endpoints: Deprecation ヘッダーが**ない**ことを検証 (2 tests)
//
// カバレッジ:
// - ✅ RFC 8594 準拠（4ヘッダー）
// - ✅ v1 vs v2 の差異
// - ✅ パス変換（/api/v1/posts → /api/v2/posts）
//
// Next steps:
// - 全 v1 エンドポイント（auth, admin, api-keys, search）に拡張
// - E2E テスト（testcontainers）
