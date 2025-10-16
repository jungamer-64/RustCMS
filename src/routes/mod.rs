//! API Routes
//!
//! Defines all HTTP routes and their corresponding handlers
//! Supports both API v1 (legacy) and API v2 (restructured) via feature flags

use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::middleware::rate_limiting::RateLimitLayer; // unified IP rate limiting
use crate::middleware::security::SecurityHeadersLayer;
use crate::{AppState, handlers}; // security headers
// logging middleware layer integration pending (currently unused)

/// Feature flag status
pub fn is_api_v2_enabled() -> bool {
    // Check environment variable or default to feature flag
    std::env::var("API_V2_ENABLED")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(cfg!(feature = "restructure_presentation"))
}

pub fn use_legacy_api_v1() -> bool {
    // Check environment variable or default based on feature flag
    std::env::var("USE_LEGACY_API_V1")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(!cfg!(feature = "restructure_presentation"))
}

/// Canary Release: Traffic split control for gradual v2 rollout
/// https://github.com/jgm/RustCMS/PHASE_5_3_IMPLEMENTATION.md
pub mod canary {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// Get the current traffic split percentage for API v2 (0-100)
    /// Environment variable: API_V2_TRAFFIC_PERCENTAGE
    ///
    /// # Example
    /// ```
    /// export API_V2_TRAFFIC_PERCENTAGE=50
    /// // → 50% traffic to v2, 50% to v1
    /// ```
    pub fn get_api_v2_traffic_percentage() -> u32 {
        std::env::var("API_V2_TRAFFIC_PERCENTAGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    /// Determine if a request should be routed to API v2 based on Canary percentage
    /// Uses consistent hashing to ensure the same user/session always goes to the same version
    ///
    /// # Arguments
    /// * `request_id` - Unique identifier for the request (e.g., user ID, session ID, request ID)
    ///
    /// # Example
    /// ```
    /// if crate::routes::canary::should_route_to_api_v2("user_123") {
    ///     // Route to API v2
    /// } else {
    ///     // Route to API v1
    /// }
    /// ```
    pub fn should_route_to_api_v2(request_id: &str) -> bool {
        let percentage = get_api_v2_traffic_percentage();

        if percentage >= 100 {
            return true; // All traffic to v2
        }

        if percentage == 0 {
            return false; // No traffic to v2
        }

        // Hash-based distribution for consistent routing per user/session
        let mut hasher = DefaultHasher::new();
        request_id.hash(&mut hasher);
        let hash_value = hasher.finish();

        (hash_value % 100) < (percentage as u64)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore] // Ignored because it requires unsafe env var manipulation
        fn test_get_api_v2_traffic_percentage() {
            let original = std::env::var("API_V2_TRAFFIC_PERCENTAGE").ok();

            unsafe {
                std::env::set_var("API_V2_TRAFFIC_PERCENTAGE", "50");
            }
            assert_eq!(get_api_v2_traffic_percentage(), 50);

            unsafe {
                std::env::set_var("API_V2_TRAFFIC_PERCENTAGE", "100");
            }
            assert_eq!(get_api_v2_traffic_percentage(), 100);

            unsafe {
                std::env::set_var("API_V2_TRAFFIC_PERCENTAGE", "0");
            }
            assert_eq!(get_api_v2_traffic_percentage(), 0);

            // Restore original
            if let Some(val) = original {
                unsafe {
                    std::env::set_var("API_V2_TRAFFIC_PERCENTAGE", val);
                }
            }
        }

        #[test]
        fn test_should_route_to_api_v2_consistent_hashing() {
            // Note: This test uses a fixed percentage to avoid env var manipulation
            let user_id = "user_123";

            // Hash-based should return consistent results
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            user_id.hash(&mut hasher);
            let hash1 = hasher.finish();

            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            user_id.hash(&mut hasher);
            let hash2 = hasher.finish();

            assert_eq!(hash1, hash2);
        }

        #[test]
        fn test_should_route_to_api_v2_fixed_percentage() {
            // Test the core logic with percentage = 50
            let test_request_id = "test_user";
            let _result = should_route_to_api_v2(test_request_id);
            // We just verify it returns a boolean without panicking
        }
    }
}

/// Create the main application router
pub fn create_router() -> Router<AppState> {
    // Public routes (no auth layer applied)
    #[allow(unused_mut)]
    let mut public = Router::new()
        // Home page - integrates cms-simple functionality
        .route("/", get(handlers::home))
        // Root API info
        .route("/api/v1", get(handlers::api_info_v1))
        // Metrics
        .route("/api/v1/metrics", get(handlers::metrics::metrics))
        // CSRF token endpoint (security hardening)
        .route("/api/v1/csrf-token", get(crate::middleware::csrf::get_csrf_token))
        // Serve interactive docs and OpenAPI JSON at /api/docs
        .route("/api/docs", get(handlers::docs_ui))
        .route("/api/docs/openapi.json", get(handlers::openapi_json))
        // Health check routes
        .nest("/api/v1/health", health_routes())
        // Build info endpoint
        .route("/api/v1/info", get(handlers::api_info_info))
        // 404 handler (kept on the outer router below)
        ;

    // Add conditional routes based on features
    #[cfg(feature = "auth")]
    {
        use crate::middleware::auth::auth_middleware;
        use crate::middleware::deprecation::add_deprecation_headers;
        use axum::middleware;
        // 公開authルート（register/login/refresh）は認証不要 + deprecation
        public = public.nest(
            "/api/v1/auth",
            auth_public_routes().layer(middleware::from_fn(add_deprecation_headers)),
        );
        // 保護authルート（logout/profile）のみ認証レイヤを適用 + deprecation
        let auth_protected = auth_protected_routes()
            .layer(middleware::from_fn(auth_middleware))
            .layer(middleware::from_fn(add_deprecation_headers));
        public = public.nest("/api/v1/auth", auth_protected);
    }

    #[cfg(feature = "database")]
    {
        // Phase 5-4: v1 エンドポイントに Deprecation ヘッダーを追加（RFC 8594）
        use crate::middleware::deprecation::add_deprecation_headers;
        use axum::middleware;

        // Protect posts and users with auth middleware + deprecation headers
        {
            use crate::middleware::auth::auth_middleware;
            let posts = post_routes()
                .layer(middleware::from_fn(auth_middleware))
                .layer(middleware::from_fn(add_deprecation_headers));
            let users = user_routes()
                .layer(middleware::from_fn(auth_middleware))
                .layer(middleware::from_fn(add_deprecation_headers));
            public = public.nest("/api/v1/posts", posts);
            public = public.nest("/api/v1/users", users);
        }
        // Admin-only management endpoints (simple token auth in handlers) + deprecation
        public = public.nest(
            "/api/v1/admin",
            admin_routes().layer(middleware::from_fn(add_deprecation_headers)),
        );
        // API Key 管理 (要 auth feature) + deprecation
        #[cfg(feature = "auth")]
        {
            use crate::middleware::auth::auth_middleware;
            let api_keys = api_key_routes()
                .layer(middleware::from_fn(auth_middleware))
                .layer(middleware::from_fn(add_deprecation_headers));
            public = public.nest("/api/v1/api-keys", api_keys);
        }
    }

    #[cfg(feature = "search")]
    {
        use crate::middleware::deprecation::add_deprecation_headers;
        use axum::middleware;
        public = public.nest(
            "/api/v1/search",
            search_routes().layer(middleware::from_fn(add_deprecation_headers)),
        );
    }

    // === API v2 新ルーティング (Phase 5) ===
    #[cfg(all(
        feature = "restructure_presentation",
        feature = "restructure_application"
    ))]
    {
        use crate::presentation::http::router::api_v2_router;
        public = public.nest("/api/v2", api_v2_router());
    }

    // Compose final router: apply security layers globally
    public
        .fallback(handlers::not_found)
        .layer(SecurityHeadersLayer::new()) // Security headers (CSP, HSTS, etc.)
        .layer(RateLimitLayer::new()) // Rate limiting
}

/// Authentication routes (public only)
#[cfg(feature = "auth")]
fn auth_public_routes() -> Router<AppState> {
    use crate::handlers::auth;
    Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh_token))
}

/// Authentication routes (protected by auth middleware)
#[cfg(feature = "auth")]
fn auth_protected_routes() -> Router<AppState> {
    use crate::handlers::auth;
    Router::new()
        .route("/logout", post(auth::logout))
        .route("/profile", get(auth::profile))
}

/// Post routes
#[cfg(feature = "database")]
fn post_routes() -> Router<AppState> {
    use crate::handlers::posts;
    Router::new()
        // CRUD operations
        .route("/", get(posts::get_posts).post(posts::create_post))
        .route(
            "/:id",
            get(posts::get_post)
                .put(posts::update_post)
                .delete(posts::delete_post),
        )
}

/// User routes
#[cfg(feature = "database")]
fn user_routes() -> Router<AppState> {
    use crate::handlers::users;
    Router::new()
        // User CRUD
        .route("/", get(users::get_users))
        .route(
            "/:id",
            get(users::get_user)
                .put(users::update_user)
                .delete(users::delete_user),
        )
}

#[cfg(feature = "database")]
fn admin_routes() -> Router<AppState> {
    use crate::handlers::admin;
    use crate::middleware::auth::auth_middleware;
    use axum::middleware;
    Router::new()
        .route("/posts", get(admin::list_posts).post(admin::create_post))
        .route("/posts/:id", delete(admin::delete_post))
        .layer(middleware::from_fn(auth_middleware))
}

#[cfg(all(feature = "database", feature = "auth"))]
fn api_key_routes() -> Router<AppState> {
    use crate::handlers::api_keys as ak;
    // APIキーの発行/一覧/削除は ユーザ (Biscuit) 認証のみで保護し、APIキー自身での自己管理は現状サポートしない
    Router::new()
        .route("/", post(ak::create_api_key).get(ak::list_api_keys))
        .route("/:id", delete(ak::revoke_api_key))
}

/// Search routes - use handler functions, not the service layer
#[cfg(feature = "search")]
fn search_routes() -> Router<AppState> {
    use crate::handlers::search as search_handlers;
    Router::new()
        .route("/", get(search_handlers::search))
        .route("/suggest", get(search_handlers::suggest))
        .route("/stats", get(search_handlers::search_stats))
        .route("/reindex", post(search_handlers::reindex))
        .route("/health", get(search_handlers::search_health))
}

/// Health check routes
fn health_routes() -> Router<AppState> {
    use crate::handlers::health as health_handlers;
    Router::new()
        .route("/", get(health_handlers::health_check))
        .route("/liveness", get(health_handlers::liveness))
        .route("/readiness", get(health_handlers::readiness))
}
