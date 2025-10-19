//! Phase 6-E: Legacy pagination utils (disabled with restructure_domain)
#![cfg(not(feature = "restructure_domain"))]

use crate::models::pagination::Paginated;

/// Generic helper to fetch items and total count, returning `Paginated<T>`.
/// Keeps handlers small and consistent.
///
/// # Errors
///
/// Propagates errors from `fetch_items` and `count_total` closures when either
/// of those asynchronous operations fail.
pub async fn fetch_paginated<T, FI, FC, FutI, FutC>(
    page: u32,
    limit: u32,
    fetch_items: FI,
    count_total: FC,
) -> crate::Result<Paginated<T>>
where
    FI: FnOnce() -> FutI,
    FutI: std::future::Future<Output = crate::Result<Vec<T>>>,
    FC: FnOnce() -> FutC,
    FutC: std::future::Future<Output = crate::Result<usize>>,
{
    let items = fetch_items().await?;
    let total = count_total().await?;
    Ok(Paginated::new(items, total, page, limit))
}

/// Cached variant that combines cache lookup + `fetch_paginated` in one place.
/// This reduces duplication across handlers that follow the same pattern.
///
/// # Errors
///
/// Returns any error produced by the provided `fetch_items`/`count_total`
/// closures or by the underlying cache layer when computing or deserializing
/// the cached `Paginated<T>` value.
pub async fn fetch_paginated_cached<T, FI, FC, FutI, FutC>(
    state: crate::AppState,
    cache_key: String,
    ttl_seconds: u64,
    page: u32,
    limit: u32,
    fetch_items: FI,
    count_total: FC,
) -> crate::Result<Paginated<T>>
where
    // Send bounds ensure the closure can be moved into the async cache closure safely
    FI: FnOnce() -> FutI + Send + 'static,
    FutI: std::future::Future<Output = crate::Result<Vec<T>>> + Send + 'static,
    FC: FnOnce() -> FutC + Send + 'static,
    FutC: std::future::Future<Output = crate::Result<usize>> + Send + 'static,
    // Cached value Paginated<T> must be (de)serializable and Send across tasks
    T: Send + Sync + 'static + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    crate::utils::cache_helpers::cache_or_compute(
        state,
        &cache_key,
        ttl_seconds,
        move || async move { fetch_paginated(page, limit, fetch_items, count_total).await },
    )
    .await
}

/// Convenience wrapper for handlers that follow the common pattern:
/// - build a cache key outside
/// - wrap filters into an Arc to clone into closures
/// - call `fetch_paginated_cached` with the provided closures that use those filters
///
/// # Errors
///
/// Same as [`fetch_paginated_cached`]: errors from the item/count builders or
/// the cache layer are propagated.
#[allow(clippy::too_many_arguments)]
pub async fn fetch_paginated_cached_with_filters<T, FI, FC, FutI, FutC, Filt>(
    state: crate::AppState,
    cache_key: String,
    ttl_seconds: u64,
    page: u32,
    limit: u32,
    filters: std::sync::Arc<Filt>,
    build_items: impl Fn(std::sync::Arc<Filt>) -> FI,
    build_count: impl Fn(std::sync::Arc<Filt>) -> FC,
) -> crate::Result<Paginated<T>>
where
    FI: FnOnce() -> FutI + Send + 'static,
    FutI: std::future::Future<Output = crate::Result<Vec<T>>> + Send + 'static,
    FC: FnOnce() -> FutC + Send + 'static,
    FutC: std::future::Future<Output = crate::Result<usize>> + Send + 'static,
    T: Send + Sync + 'static + serde::Serialize + for<'de> serde::Deserialize<'de>,
    Filt: Send + Sync + 'static,
{
    // Build closures by cloning the Arc to capture filter state in each closure
    let fitems = build_items(filters.clone());
    let fcount = build_count(filters.clone());

    fetch_paginated_cached(state, cache_key, ttl_seconds, page, limit, fitems, fcount).await
}

/// Variant where the item fetch returns raw models `M` and a separate mapper converts them to `T`.
/// This removes repetitive `iter().map(...).collect()` boilerplate in handlers.
///
/// # Errors
///
/// Propagates errors from `fetch_models` and `count_total`, as well as any
/// cache compute/serialize/deserialize failures.
#[allow(clippy::too_many_arguments)]
pub async fn fetch_paginated_cached_mapped<T, M, FI, FC, FutI, FutC, Map>(
    state: crate::AppState,
    cache_key: String,
    ttl_seconds: u64,
    page: u32,
    limit: u32,
    fetch_models: FI,
    count_total: FC,
    map: Map,
) -> crate::Result<Paginated<T>>
where
    FI: FnOnce() -> FutI + Send + 'static,
    FutI: std::future::Future<Output = crate::Result<Vec<M>>> + Send + 'static,
    FC: FnOnce() -> FutC + Send + 'static,
    FutC: std::future::Future<Output = crate::Result<usize>> + Send + 'static,
    Map: Fn(&M) -> T + Send + Sync + 'static,
    T: Send + Sync + 'static + serde::Serialize + for<'de> serde::Deserialize<'de>,
    M: Send + Sync + 'static,
{
    crate::utils::cache_helpers::cache_or_compute(
        state,
        &cache_key,
        ttl_seconds,
        move || async move {
            let models = fetch_models().await?;
            let total = count_total().await?;
            let items: Vec<T> = models.iter().map(&map).collect();
            Ok(Paginated::new(items, total, page, limit))
        },
    )
    .await
}
