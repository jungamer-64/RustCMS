use std::time::Duration;

/// Cache-or-compute helper that hides cache feature flag branching.
///
/// If the `cache` feature is enabled it will first attempt to read from cache,
/// otherwise it computes the value and stores it with the provided TTL.
/// When the `cache` feature is disabled, it simply computes and returns the value.
///
/// # Errors
///
/// Returns an error if the `compute` future returns an error. Cache get/set
/// failures are ignored on set (best-effort) but a successful cache hit is
/// required to deserialize into `T`; deserialization failures will propagate
/// as an error from the underlying cache client.
pub async fn cache_or_compute<T, F, Fut>(
    state: crate::AppState,
    key: &str,
    ttl_secs: u64,
    compute: F,
) -> crate::Result<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync + 'static,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = crate::Result<T>>,
{
    #[cfg(feature = "cache")]
    {
        if let Ok(Some(v)) = state.cache.get::<T>(key).await {
            return Ok(v);
        }
        let v = compute().await?;
        // best-effort set
        let _ = state
            .cache
            .set(key.to_string(), &v, Some(Duration::from_secs(ttl_secs)))
            .await;
        Ok(v)
    }
    #[cfg(not(feature = "cache"))]
    {
        compute().await
    }
}
