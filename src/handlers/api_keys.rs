use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::utils::response_ext::{ApiOk, delete_with};
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
/// Create an API key for the authenticated user.
///
/// # Errors
/// - 入力検証エラーや DB 書き込みエラーが発生した場合。
/// - 認証情報が無効な場合。
pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::AuthContext>,
    Json(payload): Json<CreateApiKeyPayload>,
) -> Result<(axum::http::StatusCode, ApiOk<CreatedApiKeyResponse>)> {
    let (api_key, raw) = state
        .db_create_api_key(payload.name, auth.user_id, payload.permissions)
        .await?;
    tracing::info!(api_key_id=%api_key.id, user_id=%auth.user_id, masked_raw=%crate::models::api_key::ApiKey::mask_raw(&raw), "api key created");
    Ok((
        axum::http::StatusCode::CREATED,
        ApiOk(CreatedApiKeyResponse {
            api_key,
            raw_key: raw,
        }),
    ))
}

#[utoipa::path(get, path="/api/v1/api-keys", security(("BearerAuth" = [])), responses((status=200, body=[crate::models::ApiKeyResponse], examples((
    "List" = (
        summary = "APIキー一覧",
        value = json!([{ "id": "550e8400-e29b-41d4-a716-446655440000", "name": "integration", "permissions": ["read:posts"], "revoked": false }])
    )
)))))]
/// List API keys owned by the authenticated user.
///
/// # Errors
/// - DB からの読み出しに失敗した場合。
pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::AuthContext>,
) -> Result<ApiOk<Vec<crate::models::ApiKeyResponse>>> {
    let keys = state.db_list_api_keys(auth.user_id, false).await?;
    Ok(ApiOk(keys))
}

#[derive(Serialize, ToSchema)]
pub struct ApiKeyStatus {
    pub status: &'static str,
}

#[utoipa::path(delete, path="/api/v1/api-keys/{id}", security(("BearerAuth" = [])), params(("id"=Uuid, Path)), responses((status=200, description="Revoked", body=crate::utils::api_types::ApiResponse<serde_json::Value>, examples((
    "Revoked" = (
        summary = "APIキー失効",
        value = json!({
            "success": true,
            "data": {"message": "API key revoked"},
            "message": null,
            "error": null,
            "validation_errors": null
        })
    )
)))))]
/// 指定された API キーを失効させます。
///
/// # Errors
/// - 対象のキーが見つからない、または所有者が一致しない場合。
/// - DB 操作に失敗した場合。
pub async fn revoke_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<crate::auth::AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    let fut = async move {
        state
            .db_revoke_api_key_owned(id, auth.user_id)
            .await?;
        Ok::<(), crate::AppError>(())
    };
    delete_with(fut, "API key revoked").await
}
