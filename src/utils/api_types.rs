use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 標準的なAPIレスポンス構造
///
/// ジェネリック型 `T` を含むラッパー。OpenAPI 上では良く使う組合せを aliases で公開する。
/// 例: `ApiResponsePostResponse` は `ApiResponse<PostResponse>` のエイリアス。
#[derive(Debug, Serialize, ToSchema)]
// NOTE: utoipa に独自 aliases 属性は無いため削除。必要ならば個別の型ラッパーを定義する。
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            error: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            error: None,
        }
    }

    pub fn error(error: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: None,
            error: Some(error),
        }
    }
}

/// ページネーション情報
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// バリデーションエラーレスポンス
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationErrorResponse {
    pub success: bool,
    pub error: String,
    pub validation_errors: Vec<ValidationError>,
}
