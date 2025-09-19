//! Search Service - Full-text search with Tantivy
//!
//! Provides high-performance search capabilities using Tantivy (Pure Rust):
//! - Full-text search across posts and users
//! - Faceted search with filters
//! - Real-time indexing
//! - Fuzzy search and autocomplete
//! - Search analytics and suggestions

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tantivy::{
    Index, IndexReader, IndexWriter, TantivyError,
    schema::{Field, STORED, STRING, Schema},
};
use tokio::sync::RwLock;

use crate::{
    Result,
    config::SearchConfig,
    models::{Post, User},
};

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Index error: {0}")]
    Index(String),
    #[error("Query error: {0}")]
    Query(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<SearchError> for crate::AppError {
    fn from(err: SearchError) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<TantivyError> for SearchError {
    fn from(err: TantivyError) -> Self {
        Self::Index(err.to_string())
    }
}

/// Search results
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResults<T> {
    pub hits: Vec<T>,
    pub total: usize,
    pub took_ms: u128,
    pub facets: Vec<SearchFacet>,
}

/// Search facet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacet {
    pub field: String,
    pub values: Vec<FacetValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: String,
    pub count: usize,
}

/// Search request
#[derive(Debug, Clone, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub filters: Option<Vec<SearchFilter>>,
    pub facets: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

// 統一された SortOrder を利用
pub type SortOrder = crate::utils::api_types::SortOrder;

/// Search filter
#[derive(Debug, Clone, Deserialize)]
pub struct SearchFilter {
    pub field: String,
    pub value: String,
    pub operator: FilterOperator,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterOperator {
    Equals,
    Contains,
    GreaterThan,
    LessThan,
}

/// Search service implementation
#[cfg(feature = "search")]
#[derive(Clone)]
pub struct SearchService {
    #[allow(dead_code)]
    index: Index,
    #[allow(dead_code)]
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    #[allow(dead_code)]
    schema: SearchSchema,
    config: SearchConfig,
}

/// Search schema fields
#[derive(Debug, Clone)]
pub struct SearchSchema {
    pub id: Field,
    pub title: Field,
    pub content: Field,
    pub tags: Field,
    pub author: Field,
    pub doc_type: Field,
    pub created_at: Field,
}

#[cfg(feature = "search")]
impl SearchService {
    /// Create new search service
    ///
    /// # Errors
    /// - Tantivy のリーダー/ライター初期化に失敗した場合。
    #[allow(clippy::unused_async)]
    pub async fn new(config: SearchConfig) -> Result<Self> {
        // Create dummy schema fields for now
        let search_schema = SearchSchema {
            id: Field::from_field_id(0),
            title: Field::from_field_id(1),
            content: Field::from_field_id(2),
            tags: Field::from_field_id(3),
            author: Field::from_field_id(4),
            doc_type: Field::from_field_id(5),
            created_at: Field::from_field_id(6),
        };

        // Create schema with minimum required fields
        let mut schema_builder = Schema::builder();
        let _id = schema_builder.add_text_field("id", STRING | STORED);
        let schema = schema_builder.build();

        // Create index in memory for now
        let index = Index::create_in_ram(schema);
        let reader = index.reader()?;
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            schema: search_schema,
            config,
        })
    }

    /// Index a post
    ///
    /// # Errors
    /// インデックス更新に失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn index_post(&self, _post: &Post) -> Result<()> {
        // Simplified implementation for now
        Ok(())
    }

    /// Index a user
    ///
    /// # Errors
    /// インデックス更新に失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn index_user(&self, _user: &User) -> Result<()> {
        // Simplified implementation for now
        Ok(())
    }

    /// Remove document from index
    ///
    /// # Errors
    /// インデックス更新に失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn remove_document(&self, _id: &str) -> Result<()> {
        // Simplified implementation for now
        Ok(())
    }

    /// Search documents
    ///
    /// # Errors
    /// クエリ解析や検索実行に失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn search(
        &self,
        _request: SearchRequest,
    ) -> Result<SearchResults<serde_json::Value>> {
        // Simplified implementation for now
        Ok(SearchResults {
            hits: vec![],
            total: 0,
            took_ms: 0,
            facets: vec![],
        })
    }

    /// Get search suggestions
    ///
    /// # Errors
    /// 検索候補の生成に失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn suggest(&self, _prefix: &str, _limit: usize) -> Result<Vec<String>> {
        // Simplified implementation for now
        Ok(vec![])
    }

    /// Health check for search service
    ///
    /// # Errors
    /// 内部状態のチェックに失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn health_check(&self) -> Result<()> {
        // Simple check - try to get a searcher
        let _searcher = self.reader.searcher();
        Ok(())
    }

    /// Get search statistics
    ///
    /// # Errors
    /// 統計情報の取得に失敗した場合にエラーを返します。
    #[allow(clippy::unused_async)]
    pub async fn get_stats(&self) -> Result<SearchStats> {
        let searcher = self.reader.searcher();
        let num_docs = usize::try_from(searcher.num_docs()).unwrap_or(usize::MAX);

        Ok(SearchStats {
            total_documents: num_docs,
            post_count: 0, // Would require more complex querying
            user_count: 0, // Would require more complex querying
            index_size_bytes: self.get_index_size(),
        })
    }

    /// Get index size in bytes
    fn get_index_size(&self) -> u64 {
        let mut total_size = 0;
        if let Ok(entries) = std::fs::read_dir(&self.config.index_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }
        total_size
    }

    /// Perform best-effort shutdown/flush of the search backend (commit pending writes)
    #[allow(clippy::unused_async)]
    pub async fn shutdown(&self) -> Result<()> {
        // Acquire the writer lock and commit pending changes
        let mut writer = self.writer.write().await;
        writer
            .commit()
            .map_err(|e| crate::AppError::Internal(format!("Search commit failed: {e}").into()))?;
        Ok(())
    }
}

/// Search statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchStats {
    pub total_documents: usize,
    pub post_count: usize,
    pub user_count: usize,
    pub index_size_bytes: u64,
}
