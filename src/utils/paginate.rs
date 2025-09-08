use crate::models::pagination::Paginated;

/// Generic helper to fetch items and total count, returning Paginated<T>.
/// Keeps handlers small and consistent.
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

/// Cached variant that combines cache lookup + fetch_paginated in one place.
/// This reduces duplication across handlers that follow the same pattern.
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
    T: Send + 'static,
{
    crate::utils::cache_helpers::cache_or_compute(
        state,
        &cache_key,
        ttl_seconds,
        move || async move { fetch_paginated(page, limit, fetch_items, count_total).await },
    )
    .await
}
