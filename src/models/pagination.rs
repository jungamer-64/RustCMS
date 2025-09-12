use crate::utils::api_types::Pagination as ApiPagination;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    pub page: usize,
    pub limit: usize,
    pub total: usize,
    pub total_pages: usize,
}

impl PaginationInfo {
    pub fn new(page: usize, limit: usize, total: usize) -> Self {
        let total_pages = if total == 0 {
            1
        } else {
            ((total as f64) / (limit as f64)).ceil() as usize
        };

        Self {
            page,
            limit,
            total,
            total_pages,
        }
    }

    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages
    }

    pub fn has_previous_page(&self) -> bool {
        self.page > 1
    }
}

// Conversion to/from shared API Pagination type
impl From<PaginationInfo> for ApiPagination {
    fn from(p: PaginationInfo) -> Self {
        ApiPagination {
            page: p.page as u32,
            per_page: p.limit as u32,
            total: p.total as u64,
            total_pages: p.total_pages as u32,
        }
    }
}

impl From<ApiPagination> for PaginationInfo {
    fn from(p: ApiPagination) -> Self {
        PaginationInfo {
            page: p.page as usize,
            limit: p.per_page as usize,
            total: p.total as usize,
            total_pages: p.total_pages as usize,
        }
    }
}

/// 共通 total_pages 計算 (ハンドラでは u32 を主に使用するため別途 helper)。
pub fn calc_total_pages(total: usize, limit: u32) -> u32 {
    if limit == 0 {
        return 1;
    }
    if total == 0 {
        1
    } else {
        ((total as f64) / (limit as f64)).ceil() as u32
    }
}

// ---- Added unified pagination normalization helpers (for handlers) ----
/// デフォルトページ (u32 ベース API 用)
pub const DEFAULT_PAGE_U32: u32 = 1;
/// デフォルト取得件数 (u32 ベース API 用)
pub const DEFAULT_LIMIT_U32: u32 = 20;

/// ハンドラで散在していた `unwrap_or(1/20)` + 範囲補正ロジックを統合。
/// 上限は暫定 100。0 や過大値は補正される。
pub fn normalize_page_limit(page: Option<u32>, limit: Option<u32>) -> (u32, u32) {
    let page = page.unwrap_or(DEFAULT_PAGE_U32).max(1);
    let mut limit = limit.unwrap_or(DEFAULT_LIMIT_U32);
    if limit == 0 {
        limit = DEFAULT_LIMIT_U32;
    }
    if limit > 100 {
        limit = 100;
    }
    (page, limit)
}

/// usize ベースの正規化（search など usize を使う箇所向け）
pub const DEFAULT_LIMIT_USIZE: usize = DEFAULT_LIMIT_U32 as usize;
pub fn normalize_limit_offset_usize(limit: Option<usize>, offset: Option<usize>) -> (usize, usize) {
    let mut l = limit.unwrap_or(DEFAULT_LIMIT_USIZE);
    if l == 0 {
        l = DEFAULT_LIMIT_USIZE;
    }
    if l > 100 {
        l = 100;
    }
    let off = offset.unwrap_or(0);
    (l, off)
}

// ---------------- Generic Paginated<T> helper (new) ----------------
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

impl<T> Paginated<T> {
    #[must_use]
    pub fn new(items: Vec<T>, total: usize, page: u32, limit: u32) -> Self {
        let total_pages = calc_total_pages(total, limit);
        Self {
            items,
            total,
            page,
            limit,
            total_pages,
        }
    }
    pub fn map<U, F: FnMut(&T) -> U>(&self, mut f: F) -> Paginated<U> {
        Paginated::new(
            self.items.iter().map(&mut f).collect(),
            self.total,
            self.page,
            self.limit,
        )
    }
}

// Bridge to shared API types for responses
impl<T> From<Paginated<T>> for crate::utils::api_types::PaginatedResponse<T> {
    fn from(p: Paginated<T>) -> Self {
        Self {
            data: p.items,
            pagination: crate::utils::api_types::Pagination {
                page: p.page,
                per_page: p.limit,
                total: p.total as u64,
                total_pages: p.total_pages,
            },
        }
    }
}

/// 共通ハンドラ用ビルダ (チェーン可能)
pub struct PaginatedBuilder<T> {
    items: Vec<T>,
    total: usize,
    page: u32,
    limit: u32,
}
impl<T> PaginatedBuilder<T> {
    #[must_use]
    pub fn new(page: u32, limit: u32) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            page,
            limit,
        }
    }
    pub fn items(mut self, items: Vec<T>) -> Self {
        self.items = items;
        self
    }
    pub fn total(mut self, total: usize) -> Self {
        self.total = total;
        self
    }
    #[must_use]
    pub fn build(self) -> Paginated<T> {
        Paginated::new(self.items, self.total, self.page, self.limit)
    }
}
