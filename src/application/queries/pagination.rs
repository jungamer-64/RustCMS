//! Pagination - Common pagination types for CQRS queries

use serde::{Deserialize, Serialize};

// ============================================================================
// PaginationParams
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationParams {
    limit: i64,
    offset: i64,
}

impl PaginationParams {
    /// 新しいページネーションパラメータを作成
    /// 
    /// limit は 1-100 にクランプされます
    pub fn new(limit: i64, offset: i64) -> Self {
        Self {
            limit: limit.clamp(1, 100),
            offset: offset.max(0),
        }
    }

    pub const fn first_page(limit: i64) -> Self {
        Self { limit, offset: 0 }
    }

    /// ページ番号（1-indexed）からページネーションを作成
    pub fn page(page_number: i64, page_size: i64) -> Self {
        let offset = (page_number - 1).max(0) * page_size;
        Self::new(page_size, offset)
    }

    pub const fn limit(self) -> i64 {
        self.limit
    }

    pub const fn offset(self) -> i64 {
        self.offset
    }

    /// 現在のページ番号を計算（1-indexed）
    pub const fn page_number(self) -> i64 {
        if self.limit == 0 {
            return 1;
        }
        (self.offset / self.limit) + 1
    }

    pub const fn next_page(self) -> Self {
        Self {
            limit: self.limit,
            offset: self.offset + self.limit,
        }
    }

    pub const fn prev_page(self) -> Option<Self> {
        if self.offset == 0 {
            return None;
        }
        let diff = self.offset - self.limit;
        let new_offset = if diff > 0 { diff } else { 0 };
        Some(Self {
            limit: self.limit,
            offset: new_offset,
        })
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self::new(20, 0)
    }
}

// ============================================================================
// PaginationResult
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl<T> PaginationResult<T> {
    pub const fn new(items: Vec<T>, total: i64, params: PaginationParams) -> Self {
        Self {
            items,
            total,
            limit: params.limit(),
            offset: params.offset(),
        }
    }

    pub const fn has_next_page(&self) -> bool {
        self.offset + self.limit < self.total
    }

    pub const fn has_prev_page(&self) -> bool {
        self.offset > 0
    }

    pub const fn total_pages(&self) -> i64 {
        if self.limit == 0 {
            return 1;
        }
        (self.total + self.limit - 1) / self.limit
    }

    pub const fn current_page(&self) -> i64 {
        if self.limit == 0 {
            return 1;
        }
        (self.offset / self.limit) + 1
    }

    /// アイテムを別の型にマップ
    pub fn map<U, F>(self, f: F) -> PaginationResult<U>
    where
        F: FnMut(T) -> U,
    {
        PaginationResult {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            limit: self.limit,
            offset: self.offset,
        }
    }

    /// 空の結果を作成
    pub const fn empty(params: PaginationParams) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            limit: params.limit(),
            offset: params.offset(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_new() {
        let params = PaginationParams::new(20, 10);
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 10);
    }

    #[test]
    fn test_pagination_params_clamping() {
        // Limit が大きすぎる
        let params = PaginationParams::new(200, 0);
        assert_eq!(params.limit(), 100);

        // Limit が小さすぎる
        let params = PaginationParams::new(0, 0);
        assert_eq!(params.limit(), 1);

        // Offset が負
        let params = PaginationParams::new(20, -10);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_first_page() {
        let params = PaginationParams::first_page(25);
        assert_eq!(params.limit(), 25);
        assert_eq!(params.offset(), 0);
        assert_eq!(params.page_number(), 1);
    }

    #[test]
    fn test_pagination_params_page() {
        let params = PaginationParams::page(3, 20);
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 40);
        assert_eq!(params.page_number(), 3);
    }

    #[test]
    fn test_pagination_params_navigation() {
        let params = PaginationParams::new(20, 20);

        // 次のページ
        let next = params.next_page();
        assert_eq!(next.offset(), 40);
        assert_eq!(next.page_number(), 3);

        // 前のページ
        let prev = params.prev_page().unwrap();
        assert_eq!(prev.offset(), 0);
        assert_eq!(prev.page_number(), 1);

        // 最初のページには前ページなし
        let first = PaginationParams::first_page(20);
        assert!(first.prev_page().is_none());
    }

    #[test]
    fn test_pagination_result() {
        let items = vec![1, 2, 3, 4, 5];
        let params = PaginationParams::new(5, 0);
        let result = PaginationResult::new(items, 25, params);

        assert_eq!(result.items.len(), 5);
        assert_eq!(result.total, 25);
        assert_eq!(result.current_page(), 1);
        assert_eq!(result.total_pages(), 5);
        assert!(result.has_next_page());
        assert!(!result.has_prev_page());
    }

    #[test]
    fn test_pagination_result_last_page() {
        let items = vec![21, 22, 23];
        let params = PaginationParams::new(10, 20);
        let result = PaginationResult::new(items, 23, params);

        assert_eq!(result.current_page(), 3);
        assert_eq!(result.total_pages(), 3);
        assert!(!result.has_next_page());
        assert!(result.has_prev_page());
    }

    #[test]
    fn test_pagination_result_map() {
        let items = vec![1, 2, 3];
        let params = PaginationParams::new(3, 0);
        let result = PaginationResult::new(items, 10, params);

        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.total, 10);
    }

    #[test]
    fn test_pagination_result_empty() {
        let params = PaginationParams::default();
        let result: PaginationResult<String> = PaginationResult::empty(params);

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
        assert!(!result.has_next_page());
        assert!(!result.has_prev_page());
    }

    #[test]
    fn test_pagination_default() {
        let params = PaginationParams::default();
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 0);
    }
}