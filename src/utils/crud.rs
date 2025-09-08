use crate::{AppState, Result};
use axum::http::StatusCode;
use serde::Serialize;
use crate::utils::response_ext::ApiOk;

/// Generic create helper returning (201, ApiOk(dto)) with optional post-create hook.
pub async fn create_entity<Req, Model, Dto, F, Fut, Map, Hook, HookFut>(
    state: AppState,
    req: Req,
    db_create: F,
    map: Map,
    hook: Option<Hook>,
) -> Result<(StatusCode, ApiOk<Dto>)>
where
    F: FnOnce(AppState, Req) -> Fut,
    Fut: std::future::Future<Output = Result<Model>>,
    Map: FnOnce(&Model) -> Dto,
    Dto: Serialize,
    Hook: FnOnce(&Model, AppState) -> HookFut,
    HookFut: std::future::Future<Output = ()>,
{
    let model = db_create(state.clone(), req).await?;
    if let Some(h) = hook { h(&model, state.clone()).await; }
    Ok((StatusCode::CREATED, ApiOk(map(&model))))
}

/// Generic update helper producing ApiOk(dto)
pub async fn update_entity<Req, Model, Dto, F, Fut, Map, IdxFut>(
    state: AppState,
    id: uuid::Uuid,
    req: Req,
    db_update: F,
    map: Map,
    search_index: Option<IdxFut>,
) -> Result<ApiOk<Dto>>
where
    F: FnOnce(AppState, uuid::Uuid, Req) -> Fut,
    Fut: std::future::Future<Output = Result<Model>>,
    Map: FnOnce(&Model) -> Dto,
    Dto: Serialize,
    IdxFut: std::future::Future<Output = ()>,
{
    let model = db_update(state.clone(), id, req).await?;
    if let Some(f) = search_index { f.await; }
    Ok(ApiOk(map(&model)))
}

/// Generic cached get helper: compute dto & wrap.
pub async fn get_cached_entity<Dto, Fut, Loader>(
    state: AppState,
    cache_key: String,
    ttl: u64,
    loader: Loader,
) -> Result<ApiOk<Dto>>
where
    Dto: Serialize + Send + Sync + 'static + for<'de> serde::Deserialize<'de>,
    Loader: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Dto>> + Send + 'static,
{
    let dto = crate::utils::cache_helpers::cache_or_compute(
        state,
        &cache_key,
        ttl,
        move || async move { loader().await },
    ).await?;
    Ok(ApiOk(dto))
}
