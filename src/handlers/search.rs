//! Search Handlers
//!
//! Provides full-text search endpoints with Tantivy

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::utils::api_types::ApiResponse;
use crate::{AppState, Result};

#[cfg(feature = "search")]
use crate::search::{
    SearchRequest, SearchResults, SearchFilter, FilterOperator, SortOrder, SearchStats,
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

    #[derive(Debug)]
    pub enum SortOrder {
        Asc,
        Desc,
    }
}

#[cfg(not(feature = "search"))]
use _search_shim::{FilterOperator, SearchFilter, SearchRequest, SearchResults, SearchStats, SortOrder};

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub doc_type: Option<String>, // "post" or "user"
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// Search suggestion query
#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub prefix: String,
    pub limit: Option<usize>,
}

/// Search endpoint
pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse> {
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
        limit: query.limit,
        offset: query.offset,
        sort_by: query.sort_by,
        sort_order: query
            .sort_order
            .and_then(|order| match order.to_lowercase().as_str() {
                "asc" => Some(SortOrder::Asc),
                "desc" => Some(SortOrder::Desc),
                _ => None,
            }),
    };

    // Try cache first
    let cache_key = format!(
        "search:{}:{}:{}:{}",
        query.q,
        query.limit.unwrap_or(20),
        query.offset.unwrap_or(0),
        query.doc_type.as_deref().unwrap_or("all")
    );

    #[cfg(feature = "cache")]
    {
        if let Ok(Some(cached)) = state
            .cache
            .get::<SearchResults<serde_json::Value>>(&cache_key)
            .await
        {
            return Ok(Json(ApiResponse::success(cached)));
        }
    }

    // Perform search (record timing)
    #[cfg(feature = "search")]
    let results = state.search_execute(search_request).await?;
    #[cfg(not(feature = "search"))]
    let results: SearchResults<serde_json::Value> = SearchResults { results: vec![], total: 0 };

    // Cache results for 2 minutes
    #[cfg(feature = "cache")]
    {
        if let Err(e) = state
            .cache
            .set(cache_key, &results, Some(Duration::from_secs(120)))
            .await
        {
            eprintln!("Failed to cache search results: {}", e);
        }
    }

    Ok(Json(ApiResponse::success(results)))
}

/// Search suggestions endpoint
pub async fn suggest(
    State(state): State<AppState>,
    Query(query): Query<SuggestQuery>,
) -> Result<impl IntoResponse> {
    let limit = query.limit.unwrap_or(10);

    // Try cache first
    let cache_key = format!("suggest:{}:{}", query.prefix, limit);
    #[cfg(feature = "cache")]
    {
        if let Ok(Some(cached)) = state.cache.get::<Vec<String>>(&cache_key).await {
            return Ok(Json(ApiResponse::success(json!({ "suggestions": cached }))));
        }
    }

    // Get suggestions (record timing)
    #[cfg(feature = "search")]
    let suggestions = state.search_suggest(&query.prefix, limit).await?;
    #[cfg(not(feature = "search"))]
    let suggestions: Vec<String> = Vec::new();

    // Cache for 10 minutes
    #[cfg(feature = "cache")]
    {
        if let Err(e) = state
            .cache
            .set(cache_key, &suggestions, Some(Duration::from_secs(600)))
            .await
        {
            eprintln!("Failed to cache suggestions: {}", e);
        }
    }

    Ok(Json(ApiResponse::success(
        json!({ "suggestions": suggestions }),
    )))
}

/// Search statistics endpoint
pub async fn search_stats(State(state): State<AppState>) -> Result<impl IntoResponse> {
    // Try cache first
    let cache_key = "search:stats";
    #[cfg(feature = "cache")]
    {
        if let Ok(Some(cached)) = state
            .cache
            .get::<crate::search::SearchStats>(cache_key)
            .await
        {
            return Ok(Json(ApiResponse::success(cached)));
        }
    }

    // Get fresh stats (record timing)
    #[cfg(feature = "search")]
    let stats = state.search_get_stats().await?;
    #[cfg(not(feature = "search"))]
    let stats = SearchStats::default();

    // Cache for 5 minutes
    #[cfg(feature = "cache")]
    {
        if let Err(e) = state
            .cache
            .set(
                cache_key.to_string(),
                &stats,
                Some(Duration::from_secs(300)),
            )
            .await
        {
            eprintln!("Failed to cache search stats: {}", e);
        }
    }

    Ok(Json(ApiResponse::success(stats)))
}

/// Reindex all content
pub async fn reindex(State(_state): State<AppState>) -> Result<impl IntoResponse> {
    // This would be an admin-only endpoint in production
    // For now, return a placeholder response

    // In a real implementation, you would:
    // 1. Get all posts from database
    // 2. Get all users from database
    // 3. Clear search index
    // 4. Re-index all content
    // 5. Clear search-related cache

    Ok(Json(ApiResponse::success(json!({
        "success": true,
        "message": "Reindexing started - this would be implemented as a background task"
    }))))
}

/// Search index health check
pub async fn search_health(State(_state): State<AppState>) -> Result<impl IntoResponse> {
    Ok(Json(ApiResponse::<()>::error(
        "Search health check not implemented".to_string(),
    )))
}
