//! Search Handlers
//! 
//! Provides full-text search endpoints with Tantivy

use axum::{
    response::{IntoResponse, Json},
    extract::{State, Query},
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::{
    AppState, Result,
    search::{SearchRequest, SearchResults},
};

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
        filters: query.doc_type.clone().map(|doc_type| vec![
            crate::search::SearchFilter {
                field: "doc_type".to_string(),
                value: doc_type,
                operator: crate::search::FilterOperator::Equals,
            }
        ]),
        facets: None,
        limit: query.limit,
        offset: query.offset,
        sort_by: query.sort_by,
        sort_order: query.sort_order.and_then(|order| {
            match order.to_lowercase().as_str() {
                "asc" => Some(crate::search::SortOrder::Asc),
                "desc" => Some(crate::search::SortOrder::Desc),
                _ => None,
            }
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

    if let Ok(Some(cached)) = state.cache.get::<SearchResults<serde_json::Value>>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Perform search
    let results = state.search.search(search_request).await?;
    
    // Cache results for 2 minutes
    if let Err(e) = state.cache.set(cache_key, &results, Some(Duration::from_secs(120))).await {
        eprintln!("Failed to cache search results: {}", e);
    }

    Ok(Json(results))
}

/// Search suggestions endpoint
pub async fn suggest(
    State(state): State<AppState>,
    Query(query): Query<SuggestQuery>,
) -> Result<impl IntoResponse> {
    let limit = query.limit.unwrap_or(10);
    
    // Try cache first
    let cache_key = format!("suggest:{}:{}", query.prefix, limit);
    if let Ok(Some(cached)) = state.cache.get::<Vec<String>>(&cache_key).await {
        return Ok(Json(json!({
            "suggestions": cached
        })));
    }

    // Get suggestions
    let suggestions = state.search.suggest(&query.prefix, limit).await?;
    
    // Cache for 10 minutes
    if let Err(e) = state.cache.set(cache_key, &suggestions, Some(Duration::from_secs(600))).await {
        eprintln!("Failed to cache suggestions: {}", e);
    }

    Ok(Json(json!({
        "suggestions": suggestions
    })))
}

/// Search statistics endpoint
pub async fn search_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse> {
    // Try cache first
    let cache_key = "search:stats";
    if let Ok(Some(cached)) = state.cache.get::<crate::search::SearchStats>(&cache_key).await {
        return Ok(Json(cached));
    }

    // Get fresh stats
    let stats = state.search.get_stats().await?;
    
    // Cache for 5 minutes
    if let Err(e) = state.cache.set(cache_key.to_string(), &stats, Some(Duration::from_secs(300))).await {
        eprintln!("Failed to cache search stats: {}", e);
    }

    Ok(Json(stats))
}

/// Reindex all content
pub async fn reindex(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse> {
    // This would be an admin-only endpoint in production
    // For now, return a placeholder response
    
    // In a real implementation, you would:
    // 1. Get all posts from database
    // 2. Get all users from database  
    // 3. Clear search index
    // 4. Re-index all content
    // 5. Clear search-related cache

    Ok(Json(json!({
        "success": true,
        "message": "Reindexing started - this would be implemented as a background task"
    })))
}

/// Search index health check
pub async fn search_health(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse> {
    Ok(Json(json!({
        "status": "unhealthy",
        "error": "Search health check not implemented"
    })))
}
