//! Search Service Port (インターフェース定義)
//!
//! 全文検索サービスの Port/Adapter パターンによるインターフェース定義です。
//! Infrastructure層がこれらのtraitを実装します。
//!
//! ## 設計原則
//! - Tantivy 検索エンジンの抽象化
//! - Send + Sync制約でスレッド安全性を保証
//! - 非同期メソッド定義 (async_trait)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 検索サービス（Port/Interface）
///
/// Tantivy 等の全文検索エンジンへのアクセスを抽象化します。
/// Infrastructure層で具体的な実装（TantivySearchService等）を提供します。
#[async_trait]
pub trait SearchService: Send + Sync {
    /// ドキュメントをインデックスに追加
    ///
    /// # Errors
    ///
    /// インデックス操作エラーが発生した場合
    async fn index_document(&self, doc: SearchDocument) -> Result<(), SearchError>;

    /// ドキュメントをインデックスから削除
    ///
    /// # Errors
    ///
    /// インデックス操作エラーが発生した場合
    async fn delete_document(&self, doc_id: &str) -> Result<(), SearchError>;

    /// クエリ文字列で検索
    ///
    /// # Errors
    ///
    /// 検索実行エラーが発生した場合
    async fn search(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<SearchResults, SearchError>;

    /// 複雑なクエリで検索（フィルタ・ソート対応）
    ///
    /// # Errors
    ///
    /// 検索実行エラーが発生した場合
    async fn advanced_search(&self, query: AdvancedQuery) -> Result<SearchResults, SearchError>;

    /// インデックスをリビルド
    ///
    /// # Errors
    ///
    /// インデックス操作エラーが発生した場合
    async fn rebuild_index(&self) -> Result<(), SearchError>;
}

/// 検索ドキュメント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64, // Unix timestamp
}

/// 検索結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total: usize,
    pub took_ms: u64,
}

/// 検索ヒット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

/// 高度な検索クエリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedQuery {
    pub query: String,
    pub filters: Vec<SearchFilter>,
    pub sort_by: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

/// 検索フィルタ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub field: String,
    pub value: String,
    pub operator: FilterOperator,
}

/// フィルタ演算子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    Contains,
    GreaterThan,
    LessThan,
}

/// 検索サービスのエラー型
#[derive(Debug, Clone, thiserror::Error)]
pub enum SearchError {
    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Query parse error: {0}")]
    QueryParseError(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Search operation failed: {0}")]
    OperationFailed(String),

    #[error("Unknown search error: {0}")]
    Unknown(String),
}

// Phase 3: Infrastructure層での実装例
//
// ```rust
// pub struct TantivySearchService {
//     index: tantivy::Index,
//     reader: tantivy::IndexReader,
// }
//
// #[async_trait]
// impl SearchService for TantivySearchService {
//     async fn search(&self, query: &str, limit: usize, offset: usize)
//         -> Result<SearchResults, SearchError>
//     {
//         // Tantivy実装
//     }
//     // ... 他のメソッド
// }
// ```
