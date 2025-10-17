/// Pagination - Common pagination types for CQRS queries
///
/// Provides reusable pagination parameters and result wrappers.
use serde::{Deserialize, Serialize};

/// Pagination parameters for queries
///
/// Used across all list queries to provide consistent pagination behavior.
///
/// # Examples
///
/// ```
/// use cms_backend::application::queries::PaginationParams;
///
/// let params = PaginationParams::new(20, 0); // 20 items, page 1
/// assert_eq!(params.limit(), 20);
/// assert_eq!(params.offset(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationParams {
    limit: i64,
    offset: i64,
}

impl PaginationParams {
    /// Create new pagination parameters
    ///
    /// # Arguments
    /// * `limit` - Maximum number of items to return (clamped to 1-100)
    /// * `offset` - Number of items to skip
    pub fn new(limit: i64, offset: i64) -> Self {
        Self {
            limit: limit.clamp(1, 100), // Enforce reasonable limits
            offset: offset.max(0),       // No negative offsets
        }
    }

    /// Create pagination for first page
    pub fn first_page(limit: i64) -> Self {
        Self::new(limit, 0)
    }

    /// Create pagination for specific page number (1-indexed)
    pub fn page(page_number: i64, page_size: i64) -> Self {
        let offset = (page_number - 1).max(0) * page_size;
        Self::new(page_size, offset)
    }

    /// Get limit
    pub fn limit(&self) -> i64 {
        self.limit
    }

    /// Get offset
    pub fn offset(&self) -> i64 {
        self.offset
    }

    /// Calculate page number (1-indexed)
    pub fn page_number(&self) -> i64 {
        if self.limit == 0 {
            return 1;
        }
        (self.offset / self.limit) + 1
    }

    /// Get next page parameters
    pub fn next_page(&self) -> Self {
        Self::new(self.limit, self.offset + self.limit)
    }

    /// Get previous page parameters (if exists)
    pub fn prev_page(&self) -> Option<Self> {
        if self.offset == 0 {
            return None;
        }
        let new_offset = (self.offset - self.limit).max(0);
        Some(Self::new(self.limit, new_offset))
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self::new(20, 0) // Default: 20 items, first page
    }
}

/// Paginated result wrapper
///
/// Contains items and pagination metadata.
///
/// # Type Parameters
/// * `T` - Type of items in the result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationResult<T> {
    /// Items in current page
    pub items: Vec<T>,
    /// Total number of items (across all pages)
    pub total: i64,
    /// Current limit
    pub limit: i64,
    /// Current offset
    pub offset: i64,
}

impl<T> PaginationResult<T> {
    /// Create new pagination result
    pub fn new(items: Vec<T>, total: i64, params: PaginationParams) -> Self {
        Self {
            items,
            total,
            limit: params.limit(),
            offset: params.offset(),
        }
    }

    /// Check if there is a next page
    pub fn has_next_page(&self) -> bool {
        self.offset + self.limit < self.total
    }

    /// Check if there is a previous page
    pub fn has_prev_page(&self) -> bool {
        self.offset > 0
    }

    /// Get total number of pages
    pub fn total_pages(&self) -> i64 {
        if self.limit == 0 {
            return 1;
        }
        (self.total + self.limit - 1) / self.limit
    }

    /// Get current page number (1-indexed)
    pub fn current_page(&self) -> i64 {
        if self.limit == 0 {
            return 1;
        }
        (self.offset / self.limit) + 1
    }

    /// Map items to a different type
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
        // Limit too high
        let params = PaginationParams::new(200, 0);
        assert_eq!(params.limit(), 100);

        // Limit too low
        let params = PaginationParams::new(0, 0);
        assert_eq!(params.limit(), 1);

        // Negative offset
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
        let params = PaginationParams::page(3, 20); // Page 3, 20 items per page
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 40); // Skip first 2 pages (40 items)
        assert_eq!(params.page_number(), 3);
    }

    #[test]
    fn test_pagination_params_next_prev() {
        let params = PaginationParams::new(20, 20); // Page 2

        // Next page
        let next = params.next_page();
        assert_eq!(next.offset(), 40);
        assert_eq!(next.page_number(), 3);

        // Previous page
        let prev = params.prev_page().unwrap();
        assert_eq!(prev.offset(), 0);
        assert_eq!(prev.page_number(), 1);

        // No previous for first page
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
        let params = PaginationParams::new(10, 20); // Page 3
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
}
