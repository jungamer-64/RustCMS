//! OpenAPI Documentation - Simplified for compilation
//!
//! Temporary simplified OpenAPI configuration to resolve compilation issues
//! Full API documentation will be restored after fixing dependencies

use utoipa::OpenApi;
use utoipa::Modify;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

use crate::app::{AppMetrics, HealthStatus, ServiceHealth};

/// Add security schemes dynamically to avoid macro incompatibility.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert(Default::default());
        let mut http = Http::new(HttpAuthScheme::Bearer);
        http.bearer_format = Some("Biscuit".to_string());
        components.add_security_scheme("BearerAuth", SecurityScheme::Http(http));

        #[cfg(feature = "legacy-auth-flat")]
        {
            // NOTE(Phase 4 / 3.0.0): This conditional inclusion of LoginResponse will be removed.
            // After removal, only AuthSuccessResponse remains as the canonical auth schema.
            use utoipa::ToSchema;
            if !components.schemas.contains_key("LoginResponse") {
                components.schemas.insert(
                    "LoginResponse".to_string(),
                    <crate::handlers::auth::LoginResponse as ToSchema>::schema(),
                );
            }
        }
    }
}

// (Schemas temporarily minimized during refactor; extend later as needed)


#[derive(OpenApi)]
#[openapi(
    info(
        title = "Enterprise CMS API",
        version = "2.0.0",
    description = "Production-ready Content Management System API\n\nAuth Response Unification: The unified auth schema is AuthSuccessResponse (nested tokens object). When feature 'auth-flat-fields' is enabled, deprecated flattened token fields (access_token / refresh_token / biscuit_token / expires_in / session_id / token) are still emitted for backward compatibility; these will be removed in 3.0.0. Disable the feature to preview the post-removal shape now. If feature 'legacy-auth-flat' is enabled, the historical LoginResponse schema is included for reference only and will also be removed in 3.0.0."
    ),
    paths(
        // Generated from handler #[utoipa::path] attributes; listing here enables explicit inclusion
        // Health
        crate::handlers::health::health_check,
        crate::handlers::health::liveness,
        crate::handlers::health::readiness,
    // Metrics
    crate::handlers::metrics::metrics,
        // Auth
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::refresh_token,
        // Posts
        crate::handlers::posts::create_post,
        crate::handlers::posts::get_post,
        crate::handlers::posts::get_posts,
        crate::handlers::posts::update_post,
        crate::handlers::posts::delete_post,
    crate::handlers::posts::get_posts_by_tag,
    crate::handlers::posts::publish_post,
        // Users
        crate::handlers::users::get_users,
        crate::handlers::users::get_user,
        crate::handlers::users::update_user,
        crate::handlers::users::delete_user,
    crate::handlers::users::get_user_posts,
    crate::handlers::users::change_user_role,
        // Search
        crate::handlers::search::search,
        crate::handlers::search::suggest,
        crate::handlers::search::search_stats,
        crate::handlers::search::reindex,
        crate::handlers::search::search_health
    ,
    // API Keys
    crate::handlers::api_keys::create_api_key,
    crate::handlers::api_keys::list_api_keys,
    crate::handlers::api_keys::revoke_api_key
    ),
    servers(
        (url = "http://localhost:3000", description = "Development server")
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Metrics", description = "Metrics exposition")
    ),
    components(
        schemas(
            AppMetrics,
            HealthStatus,
            ServiceHealth,
            // Auth
            crate::handlers::auth::RegisterRequest,
            crate::auth::LoginRequest,
            crate::handlers::auth::RefreshRequest,
            crate::utils::auth_response::AuthSuccessResponse,
            // Posts
            crate::handlers::posts::PostQuery,
            crate::handlers::posts::PostDto,
            crate::models::pagination::Paginated<crate::handlers::posts::PostDto>,
            // Users
            crate::handlers::users::UserQuery,
            crate::models::pagination::Paginated<crate::utils::common_types::UserInfo>,
            // Search queries (responses are dynamic JSON; queries documented)
            crate::handlers::search::SearchQuery,
            crate::handlers::search::SuggestQuery,
            // Generic related
            crate::utils::api_types::Pagination,
            crate::utils::api_types::ValidationError,
            crate::utils::api_types::ApiResponse<serde_json::Value>,
            crate::utils::api_types::ApiResponseExample
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
