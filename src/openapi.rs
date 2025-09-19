//! `OpenAPI` ドキュメント生成（簡易版）
//!
//! Temporary simplified `OpenAPI` configuration to resolve compilation issues.
//! Adds a Bearer (Biscuit) security scheme dynamically to avoid macro incompatibilities.

#![allow(clippy::needless_for_each)]

use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

use crate::app::{AppMetrics, HealthStatus, ServiceHealth};

/// Add security schemes dynamically to avoid macro incompatibility.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert(utoipa::openapi::Components::default());
        let mut http = Http::new(HttpAuthScheme::Bearer);
        http.bearer_format = Some("Biscuit".to_string());
        components.add_security_scheme("BearerAuth", SecurityScheme::Http(http));
    }
}

// Single ApiDoc definition: both legacy and non-legacy features use the same content.
#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "CMS API",
        version = "2.0.0",
        description = "Simplified API docs for compilation"
    ),
    paths(
        crate::handlers::health::health_check,
        // Auth
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::logout,
        crate::handlers::auth::profile,
        crate::handlers::auth::refresh_token,
        // Posts
        crate::handlers::posts::create_post,
        crate::handlers::posts::get_post,
        crate::handlers::posts::get_posts,
        crate::handlers::posts::update_post,
        crate::handlers::posts::delete_post,
        crate::handlers::posts::get_posts_by_tag,
        crate::handlers::posts::publish_post,
        // Search
        crate::handlers::search::search,
        crate::handlers::search::suggest,
        crate::handlers::search::search_stats,
        crate::handlers::search::reindex,
        crate::handlers::search::search_health,
        // API Keys
        crate::handlers::api_keys::create_api_key,
        crate::handlers::api_keys::list_api_keys,
        crate::handlers::api_keys::revoke_api_key
    ),
    components(
        schemas(
            AppMetrics,
            HealthStatus,
            ServiceHealth,
            crate::utils::auth_response::AuthSuccessResponse,
            crate::utils::auth_response::AuthTokens,
            crate::utils::common_types::UserInfo
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
