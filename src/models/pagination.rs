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

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    
    #[serde(default = "default_limit")]
    pub limit: usize,
}

impl PaginationQuery {
    pub fn validate(&mut self) {
        if self.page == 0 {
            self.page = 1;
        }
        if self.limit == 0 || self.limit > 100 {
            self.limit = 10;
        }
    }

    pub fn offset(&self) -> usize {
        (self.page - 1) * self.limit
    }
}

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    10
}
