// src/application/dto/common.rs
//! 共通 DTO 定義
//!
//! ページネーション、エラーレスポンス等の共通型を定義します。

use serde::{Deserialize, Serialize};

/// ページネーションリクエスト
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationRequest {
    /// ページ番号（0-based）
    #[serde(default)]
    pub page: u32,
    
    /// 1ページあたりのアイテム数
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

impl Default for PaginationRequest {
    fn default() -> Self {
        Self {
            page: 0,
            per_page: default_per_page(),
        }
    }
}

fn default_per_page() -> u32 {
    20
}

/// ページネーションレスポンス
#[derive(Debug, Clone, Serialize)]
pub struct PaginationResponse<T> {
    /// アイテムリスト
    pub items: Vec<T>,
    
    /// 総アイテム数
    pub total: u64,
    
    /// 現在のページ番号
    pub page: u32,
    
    /// 1ページあたりのアイテム数
    pub per_page: u32,
    
    /// 総ページ数
    pub total_pages: u32,
}

impl<T> PaginationResponse<T> {
    /// 新しいページネーションレスポンスを作成
    pub fn new(items: Vec<T>, total: u64, page: u32, per_page: u32) -> Self {
        let total_pages = if per_page > 0 {
            ((total as f64) / (per_page as f64)).ceil() as u32
        } else {
            0
        };
        
        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}

/// エラーレスポンス
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    /// エラーコード
    pub code: String,
    
    /// エラーメッセージ
    pub message: String,
    
    /// 詳細情報（オプション）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// 新しいエラーレスポンスを作成
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }
    
    /// 詳細情報を追加
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_request_default() {
        let request = PaginationRequest::default();
        assert_eq!(request.page, 0);
        assert_eq!(request.per_page, 20);
    }

    #[test]
    fn test_pagination_response_total_pages() {
        let response = PaginationResponse::new(
            vec!["item1".to_string(), "item2".to_string()],
            45,
            0,
            20,
        );
        
        assert_eq!(response.total, 45);
        assert_eq!(response.total_pages, 3); // 45 / 20 = 2.25 -> 3
    }

    #[test]
    fn test_error_response() {
        let error = ErrorResponse::new("NOT_FOUND", "User not found");
        assert_eq!(error.code, "NOT_FOUND");
        assert_eq!(error.message, "User not found");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_error_response_with_details() {
        let error = ErrorResponse::new("VALIDATION_ERROR", "Invalid input")
            .with_details(serde_json::json!({
                "field": "email",
                "reason": "invalid format"
            }));
        
        assert_eq!(error.code, "VALIDATION_ERROR");
        assert!(error.details.is_some());
    }
}
