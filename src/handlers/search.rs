//! Search Handlers
//!
//! Provides full-text search endpoints with Tantivy

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use utoipa::ToSchema;
use serde_json::json;
use std::time::Duration;

// Using ApiOk newtype for unified success responses
use crate::utils::response_ext::ApiOk;
use crate::{AppState, Result};

#[cfg(feature = "search")]
use crate::search::{
    SearchRequest, SearchResults, SearchFilter, FilterOperator,
};

#[cfg(not(feature = "search"))]
mod _search_shim {
    use serde::Serialize;

    #[derive(Debug)]
    pub struct SearchRequest {
        pub query: String,
        pub filters: Option<Vec<SearchFilter>>,
        pub facets: Option<()>,
        pub limit: Option<usize>,
        pub offset: Option<usize>,
        pub sort_by: Option<String>,
        pub sort_order: Option<SortOrder>,
    }

    #[derive(Debug, Serialize)]
    pub struct SearchResults<T: Serialize> {
        pub results: Vec<T>,
        pub total: usize,
    }

    #[derive(Debug, Default, Serialize)]
    pub struct SearchStats {}

    #[derive(Debug)]
    pub struct SearchFilter {
        pub field: String,
        pub value: String,
        pub operator: FilterOperator,
    }

    #[derive(Debug)]
    pub enum FilterOperator {
        Equals,
    }

    pub type SortOrder = crate::utils::api_types::SortOrder;
}

#[cfg(not(feature = "search"))]
use _search_shim::{FilterOperator, SearchFilter, SearchRequest, SearchResults, SearchStats, SortOrder};

/// Search query parameters
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub doc_type: Option<String>, // "post" or "user"
    pub sort_by: Option<String>,
    pub sort_order: Option<crate::utils::api_types::SortOrder>,
}

/// Search suggestion query
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct SuggestQuery {
    pub prefix: String,
    pub limit: Option<usize>,
}

/// Search endpoint
#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "Search",
    params(SearchQuery),
    responses(
        (status=200, description="Search results (ApiResponse<SearchResults>)", examples((
            "SearchResults" = (
                summary = "検索結果例",
                value = json!({
                    "success": true,
                    "data": {
                        "results": [
                            {"id": "550e8400-e29b-41d4-a716-446655440000", "title": "Hello World", "doc_type": "post"}
                        ],
                        "total": 1
                    },
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=500, description="Server error")
    )
)]
pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse> {
    // Normalize pagination controls
    let (limit, offset) = crate::models::pagination::normalize_limit_offset_usize(query.limit, query.offset);

    // Build search request
    let search_request = SearchRequest {
        query: query.q.clone(),
        filters: query.doc_type.clone().map(|doc_type| {
            vec![SearchFilter {
                field: "doc_type".to_string(),
                value: doc_type,
                operator: FilterOperator::Equals,
            }]
        }),
        facets: None,
        limit: Some(limit),
        offset: Some(offset),
    sort_by: query.sort_by,
    sort_order: query.sort_order,
    };

    // Try cache first
    let cache_key = crate::utils::cache_key::CacheKeyBuilder::new("search")
        .kv("q", &query.q)
        .kv("limit", limit)
        .kv("offset", offset)
        .kv("type", query.doc_type.as_deref().unwrap_or("all"))
        .build();

    let results = crate::utils::cache_helpers::cache_or_compute(
        &state,
        &cache_key,
        crate::utils::cache_ttl::CACHE_TTL_SHORT,
        || async move {
            #[cfg(feature = "search")]
            { return state.search_execute(search_request).await; }
            #[cfg(not(feature = "search"))]
            { Ok(SearchResults { results: vec![], total: 0 }) }
        },
    ).await?;

    Ok(ApiOk(results))
}

/// Search suggestions endpoint
#[utoipa::path(
    get,
    path = "/api/v1/search/suggest",
    tag = "Search",
    params(SuggestQuery),
    responses(
        (status=200, description="Suggestions list (ApiResponse<{ suggestions: string[] }>)", examples((
            "Suggestions" = (
                summary = "サジェスト例",
                value = json!({
                    "success": true,
                    "data": {"suggestions": ["hel", "hello", "hello world"]},
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=500, description="Server error")
    )
)]
pub async fn suggest(
    State(state): State<AppState>,
    Query(query): Query<SuggestQuery>,
) -> Result<impl IntoResponse> {
    let (limit, _offset) = crate::models::pagination::normalize_limit_offset_usize(query.limit, Some(0));

    // Try cache first
    let cache_key = crate::utils::cache_key::CacheKeyBuilder::new("suggest")
        .kv("prefix", &query.prefix)
        .kv("limit", limit)
        .build();
    let suggestions: Vec<String> = crate::utils::cache_helpers::cache_or_compute(
        &state,
        &cache_key,
        crate::utils::cache_ttl::CACHE_TTL_LONG,
        || async move {
            #[cfg(feature = "search")]
            { state.search_suggest(&query.prefix, limit).await }
            #[cfg(not(feature = "search"))]
            { Ok(Vec::new()) }
        },
    ).await?;

    Ok(ApiOk(json!({ "suggestions": suggestions })))
}

/// Search statistics endpoint
#[utoipa::path(
    get,
    path = "/api/v1/search/stats",
    tag = "Search",
    responses(
        (status=200, description="Search stats (ApiResponse<SearchStats>)", examples((
            "Stats" = (
                summary = "統計例",
                value = json!({
                    "success": true,
                    "data": {"index_size": 12345, "documents": 42},
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=500, description="Server error")
    )
)]
pub async fn search_stats(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // Try cache first
    let cache_key = crate::utils::cache_key::CacheKeyBuilder::new("search:stats").build();
    let stats = crate::utils::cache_helpers::cache_or_compute(
        &state,
        &cache_key,
        crate::utils::cache_ttl::CACHE_TTL_DEFAULT,
        || async move {
            #[cfg(feature = "search")]
            { state.search_get_stats().await }
            #[cfg(not(feature = "search"))]
            { Ok(SearchStats::default()) }
        },
    ).await?;

    Ok(ApiOk(stats))
}

/// Reindex all content
#[utoipa::path(
    post,
    path = "/api/v1/search/reindex",
    tag = "Search",
    security(("BearerAuth" = [])),
    responses(
        (status=200, description="Reindex triggered (ApiResponse<{ message: string }>)", examples((
            "Reindex" = (
                summary = "再インデックス開始",
                value = json!({
                    "success": true,
                    "data": {"message": "Reindexing started - this would be implemented as a background task"},
                    "message": null,
                    "error": null,
                    "validation_errors": null
                })
            )
        ))),
        (status=500, description="Server error")
    )
)]
pub async fn reindex(State(_state): State<AppState>) -> Result<impl IntoResponse> {
    // This would be an admin-only endpoint in production
    // For now, return a placeholder response

    // In a real implementation, you would:
    // 1. Get all posts from database
    // 2. Get all users from database
    // 3. Clear search index
    // 4. Re-index all content
    // 5. Clear search-related cache

    Ok(ApiOk(json!({
        "success": true,
        "message": "Reindexing started - this would be implemented as a background task"
    })))
}

/// Search index health check
#[utoipa::path(
    get,
    path = "/api/v1/search/health",
    tag = "Search",
    responses(
    (status=200, description="Search health (ApiResponse<ServiceHealth>)"),
        (status=500, description="Server error")
    )
)]
pub async fn search_health(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // AppState の包括的なヘルスチェックから search 部分のみ返す
    let h = state.health_check().await?;
    Ok(ApiOk(h.search))
}
