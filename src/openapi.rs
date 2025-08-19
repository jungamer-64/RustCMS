//! OpenAPI Documentation - Simplified for compilation
//! 
//! Temporary simplified OpenAPI configuration to resolve compilation issues
//! Full API documentation will be restored after fixing dependencies

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Enterprise CMS API",
        version = "2.0.0",
        description = "Production-ready Content Management System API"
    ),
    servers(
        (url = "http://localhost:3000", description = "Development server")
    ),
)]
pub struct ApiDoc;
