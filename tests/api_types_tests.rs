/// Tests for API type safety and response structures
use cms_backend::utils::api_types::{ApiResponse, PaginatedResponse, Pagination, PaginationQuery};
use cms_backend::error::AppError;
use validator::ValidationErrors;
use serde_json;

#[cfg(test)]
mod api_types_tests {
    use super::*;

    #[test]
    fn test_api_response_success_serialization() {
        let response = ApiResponse::success("test data");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"test data\""));
    }

    #[test]
    fn test_api_response_success_with_message() {
        let response =
            ApiResponse::success_with_message("test data", "Success message".to_string());
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Success message\""));
        assert!(json.contains("\"data\":\"test data\""));
    }

    #[test]
    fn test_api_response_error_serialization() {
        let response = ApiResponse::<()>::error("Invalid input".to_string());
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Invalid input\""));
    }

    #[test]
    fn test_paginated_response_serialization() {
        let data = vec!["item1", "item2", "item3"];
        let pagination = Pagination {
            page: 1,
            per_page: 10,
            total: 3,
            total_pages: 1,
        };
        let response = PaginatedResponse { data, pagination };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"per_page\":10"));
        assert!(json.contains("\"total\":3"));
        assert!(json.contains("\"total_pages\":1"));
    }

    #[test]
    fn test_pagination_query_defaults() {
        let query: PaginationQuery = serde_json::from_str("{}").unwrap();

        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, 20);
    }

    #[test]
    fn test_pagination_query_validation() {
        // Valid query
        let query: Result<PaginationQuery, _> =
            serde_json::from_str(r#"{"page": 2, "per_page": 20}"#);
        assert!(query.is_ok());

        // Test validation method
        let mut query = PaginationQuery {
            page: 0,
            per_page: 0,
        };
        query.validate();
        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, 20);
    }

    #[test]
    fn test_pagination_query_offset() {
        let query = PaginationQuery {
            page: 3,
            per_page: 10,
        };
        assert_eq!(query.offset(), 20); // (3-1) * 10 = 20
    }

    #[test]
    // Removed obsolete status_code() tests (AppError no longer exposes direct status_code helper)

    #[test]
    fn test_type_safety_with_generic_data() {
        #[derive(serde::Serialize)]
        struct TestData {
            id: u32,
            name: String,
        }

        let test_data = TestData {
            id: 1,
            name: "Test".to_string(),
        };

        let response = ApiResponse::success(test_data);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"name\":\"Test\""));
    }

    #[test]
    fn test_app_error_display_validation() {
        // Build empty ValidationErrors and inject a fake field error through serde roundtrip (simplified)
        let ve = ValidationErrors::new();
        let error = AppError::Validation(ve);
        let msg = format!("{}", error);
        assert!(msg.contains("Validation error"));
    }
}
