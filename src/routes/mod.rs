//! API Routes
//!
//! Defines all HTTP routes and their corresponding handlers

use axum::{
    routing::{get, post, delete},
    Router,
};

use crate::{
    AppState,
    handlers,
};

/// Create the main application router
pub fn create_router() -> Router<AppState> {
    let mut router = Router::new()
        // Root API info
        .route("/api/v1", get(handlers::api_info))
        // Health check routes
        .nest("/api/v1/health", health_routes())
        // Build info endpoint
        .route("/api/v1/info", get(handlers::api_info))
        // 404 handler
        .fallback(handlers::not_found);

    // Add conditional routes based on features
    #[cfg(feature = "auth")]
    {
        router = router.nest("/api/v1/auth", auth_routes());
    }

    #[cfg(feature = "database")]
    {
        router = router.nest("/api/v1/posts", post_routes());
        router = router.nest("/api/v1/users", user_routes());
    // Admin-only management endpoints (simple token auth in handlers)
    router = router.nest("/api/v1/admin", admin_routes());
    }

    #[cfg(feature = "search")]
    {
        router = router.nest("/api/v1/search", search_routes());
    }

    router
}

/// Authentication routes
#[cfg(feature = "auth")]
fn auth_routes() -> Router<AppState> {
    use crate::handlers::auth;
    Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/profile", get(auth::profile))
        .route("/refresh", post(auth::refresh_token))
}

/// Post routes
#[cfg(feature = "database")]
fn post_routes() -> Router<AppState> {
    use crate::handlers::posts;
    Router::new()
        // CRUD operations
        .route("/", get(posts::get_posts).post(posts::create_post))
        .route("/:id", get(posts::get_post)
                     .put(posts::update_post)
                     .delete(posts::delete_post))
}

/// User routes
#[cfg(feature = "database")]
fn user_routes() -> Router<AppState> {
    use crate::handlers::users;
    Router::new()
        // User CRUD
        .route("/", get(users::get_users))
        .route("/:id", get(users::get_user)
                     .put(users::update_user)
                     .delete(users::delete_user))
}

#[cfg(feature = "database")]
fn admin_routes() -> Router<AppState> {
    use crate::handlers::admin;
    Router::new()
        .route("/posts", get(admin::list_posts).post(admin::create_post))
        .route("/posts/:id", delete(admin::delete_post))
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
