use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    pub total: u64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

pub fn success_response<T>(data: T) -> ApiResponse<T> {
    ApiResponse {
        success: true,
        data: Some(data),
        error: None,
        message: None,
    }
}

pub fn success_response_with_message<T>(data: T, message: String) -> ApiResponse<T> {
    ApiResponse {
        success: true,
        data: Some(data),
        error: None,
        message: Some(message),
    }
}

pub fn error_response(error: String) -> ApiResponse<()> {
    ApiResponse {
        success: false,
        data: None,
        error: Some(error),
        message: None,
    }
}

pub fn paginated_response<T>(
    data: Vec<T>,
    total: u64,
    page: u32,
    limit: u32,
) -> ApiResponse<PaginatedResponse<T>> {
    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
    
    ApiResponse {
        success: true,
        data: Some(PaginatedResponse {
            data,
            pagination: PaginationInfo {
                total,
                page,
                limit,
                total_pages,
            },
        }),
        error: None,
        message: None,
    }
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: Some(message),
        }
    }

    pub fn error(error: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            message: None,
        }
    }

    pub fn error_with_data<D>(error: String, data: Option<D>) -> ApiResponse<D> {
        ApiResponse {
            success: false,
            data,
            error: Some(error),
            message: None,
        }
    }
}
