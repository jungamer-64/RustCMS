use std::time::Duration;

/// Cache-or-compute helper that hides cache feature flag branching.
/// If cache feature is disabled, simply computes and returns the value.
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
        return Ok(v);
    }
    #[cfg(not(feature = "cache"))]
    {
        return compute().await;
    }
}
