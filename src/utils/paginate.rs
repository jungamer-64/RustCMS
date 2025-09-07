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
