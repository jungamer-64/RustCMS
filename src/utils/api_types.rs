use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use axum::Json;
use serde_json::{json, Value};
use utoipa::ToSchema;

/// 標準的なAPIレスポンス構造
///
/// ジェネリック型 `T` を含むラッパー。OpenAPI 上では良く使う組合せを aliases で公開する。
/// 例: `ApiResponsePostResponse` は `ApiResponse<PostResponse>` のエイリアス。
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
    "success": true,
    "data": {"example": "value"},
    "message": null,
    "error": null,
    "validation_errors": null
}))]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_errors: Option<Vec<ValidationError>>,    
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
            validation_errors: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            error: None,
            validation_errors: None,
        }
    }

}

impl ApiResponse<()> {
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error),
            validation_errors: None,
        }
    }

    pub fn error_with_validation(error: String, validation_errors: Vec<ValidationError>) -> Self {
        Self {
            success: false,
            data: None,
            message: None,
            error: Some(error),
            validation_errors: Some(validation_errors),
        }
    }
}

/// 便利用成功ヘルパ: `Json(ApiResponse::success(data))` の重複削減。
#[deprecated(note = "Use ApiOk(value) or Json(ApiResponse::success(value)) へ移行。Ok ヘルパは今後削除予定。")]
pub fn ok<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse::success(data))
}

/// メッセージのみの成功レスポンス (data は空 JSON オブジェクト扱い)。
#[deprecated(note = "Use ApiOk(json!({ \"message\": ... })) もしくは Json(ApiResponse::success(...)) へ移行。")]
pub fn ok_message(msg: impl Into<String>) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::success(json!({ "message": msg.into() })))
}

/// エラーヘルパ: `Json(ApiResponse::error(msg))` の重複削減。
#[deprecated(note = "Use Json(ApiResponse::error(...)) へ移行。err ヘルパは今後削除予定。")]
pub fn err(msg: impl Into<String>) -> Json<ApiResponse<()>> {
    let resp: ApiResponse<()> = ApiResponse::error(msg.into());
    Json(resp)
}

/// ページネーション情報
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({"page":1,"per_page":20,"total":42,"total_pages":3}))]
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

/// ページネーション付きレスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

/// ページネーションクエリパラメータ
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    20
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 20,
        }
    }
}

impl PaginationQuery {
    pub fn validate(&mut self) {
        if self.page == 0 {
            self.page = 1;
        }
        if self.per_page == 0 {
            self.per_page = 20;
        }
        if self.per_page > 100 {
            self.per_page = 100;
        }
    }

    pub fn offset(&self) -> u64 {
        ((self.page - 1) * self.per_page) as u64
    }
}

/// ソートパラメータ
#[derive(Debug, Deserialize, ToSchema)]
pub struct SortQuery {
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub order: SortOrder,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// フィルタークエリパラメータ
#[derive(Debug, Deserialize, ToSchema)]
pub struct FilterQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
}

/// リソース作成レスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedResponse {
    pub id: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// リソース更新レスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdatedResponse {
    pub id: String,
    pub message: String,
    pub updated_at: DateTime<Utc>,
}

/// リソース削除レスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct DeletedResponse {
    pub id: String,
    pub message: String,
    pub deleted_at: DateTime<Utc>,
}

/// バリデーションエラー
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({"field":"title","message":"must not be empty"}))]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

// ValidationErrorResponse は統合のため削除 (ApiResponse に統合)

/// ApiResponse の具体例（ジェネリックなし）
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
    "success": true,
    "data": {"message": "Operation completed"},
    "message": null,
    "error": null,
    "validation_errors": null
}))]
pub struct ApiResponseExample {
    pub success: bool,
    pub data: Option<Value>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub validation_errors: Option<Vec<ValidationError>>,
}
