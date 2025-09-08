use axum::{extract::{State, Path}, Json, Extension};
use serde::{Serialize, Deserialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AppState, Result};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateApiKeyPayload {
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreatedApiKeyResponse {
    pub api_key: crate::models::ApiKeyResponse,
    /// 生の API Key (一度だけ表示)。クライアントは安全に保存すること。
    pub raw_key: String,
}

#[utoipa::path(post, path="/api/v1/api-keys", security(("BearerAuth" = [])), request_body=CreateApiKeyPayload, responses((status=201, description="Created", body=CreatedApiKeyResponse, examples((
    "Created" = (
        summary = "APIキー作成",
        value = json!({
            "api_key": {"id": "550e8400-e29b-41d4-a716-446655440000", "name": "integration", "permissions": ["read:posts"], "revoked": false},
            "raw_key": "raw_api_key_sample_once_only"
        })
    )
)))))]
pub async fn create_api_key(State(state): State<AppState>, Extension(auth): Extension<crate::auth::AuthContext>, Json(payload): Json<CreateApiKeyPayload>) -> Result<(axum::http::StatusCode, Json<CreatedApiKeyResponse>)> {
    let (api_key, raw) = state.db_create_api_key(payload.name, auth.user_id, payload.permissions).await?;
    tracing::info!(api_key_id=%api_key.id, user_id=%auth.user_id, masked_raw=%crate::models::api_key::ApiKey::mask_raw(&raw), "api key created");
    Ok((axum::http::StatusCode::CREATED, Json(CreatedApiKeyResponse { api_key, raw_key: raw })))
}

#[utoipa::path(get, path="/api/v1/api-keys", security(("BearerAuth" = [])), responses((status=200, body=[crate::models::ApiKeyResponse], examples((
    "List" = (
        summary = "APIキー一覧",
        value = json!([{ "id": "550e8400-e29b-41d4-a716-446655440000", "name": "integration", "permissions": ["read:posts"], "revoked": false }])
    )
)))))]
pub async fn list_api_keys(State(state): State<AppState>, Extension(auth): Extension<crate::auth::AuthContext>) -> Result<Json<Vec<crate::models::ApiKeyResponse>>> {
    let keys = state.db_list_api_keys(auth.user_id, false).await?;
    Ok(Json(keys))
}

#[utoipa::path(delete, path="/api/v1/api-keys/{id}", security(("BearerAuth" = [])), params(("id"=Uuid, Path)), responses((status=200, examples((
    "Revoked" = (
        summary = "APIキー失効",
        value = json!({"status": "revoked"})
    )
)))))]
pub async fn revoke_api_key(State(state): State<AppState>, Extension(auth): Extension<crate::auth::AuthContext>, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>> {
    state.db_revoke_api_key_owned(id, auth.user_id).await?;
    Ok(Json(serde_json::json!({"status":"revoked"})))
}