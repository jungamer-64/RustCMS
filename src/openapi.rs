//! OpenAPI Documentation - Simplified for compilation
//!
//! Temporary simplified OpenAPI configuration to resolve compilation issues
//! Full API documentation will be restored after fixing dependencies

use utoipa::OpenApi;

use crate::utils::api_types::{
    ApiResponse, CreatedResponse, DeletedResponse, FilterQuery, PaginatedResponse, Pagination,
    PaginationQuery, SortQuery, UpdatedResponse, ValidationError, ValidationErrorResponse,
};

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
    components(
        schemas(
            Pagination,
            PaginationQuery,
            SortQuery,
            FilterQuery,
            ValidationError,
            ValidationErrorResponse,
            CreatedResponse,
            UpdatedResponse,
            DeletedResponse,
            // register common concrete instances of the generic wrappers so they appear in the OpenAPI components
            PaginatedResponse<CreatedResponse>,
            ApiResponse<CreatedResponse>,
            ApiResponse<ValidationErrorResponse>
        )
    )
)]
pub struct ApiDoc;
