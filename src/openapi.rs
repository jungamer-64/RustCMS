//! `OpenAPI` Documentation - Simplified for compilation
//!
//! Temporary simplified `OpenAPI` configuration to resolve compilation issues
//! Full API documentation will be restored after fixing dependencies

#![allow(clippy::needless_for_each)]

use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

use crate::app::{AppMetrics, HealthStatus, ServiceHealth};

/// Add security schemes dynamically to avoid macro incompatibility.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
    let components = openapi.components.get_or_insert(utoipa::openapi::Components::default());
        let mut http = Http::new(HttpAuthScheme::Bearer);
        http.bearer_format = Some("Biscuit".to_string());
        components.add_security_scheme("BearerAuth", SecurityScheme::Http(http));
    }
}

// Minimal, well-formed OpenApi declarations: one for legacy feature, one for non-legacy.

#[cfg(feature = "legacy-auth-flat")]
#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Enterprise CMS API",
        version = "2.0.0",
        description = "Simplified API docs for compilation"
    ),
    paths(
        crate::handlers::health::health_check
    ),
    components(
        schemas(
            AppMetrics,
            HealthStatus,
            ServiceHealth
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

#[cfg(not(feature = "legacy-auth-flat"))]
#[allow(clippy::needless_for_each)]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Enterprise CMS API",
        version = "2.0.0",
        description = "Simplified API docs for compilation"
    ),
    paths(
        crate::handlers::health::health_check
    ),
    components(
        schemas(
            AppMetrics,
            HealthStatus,
            ServiceHealth
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
