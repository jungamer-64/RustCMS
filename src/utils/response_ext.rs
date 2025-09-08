use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use crate::utils::api_types::ApiResponse;
use serde_json::json;

/// Convenience helper returning `ApiOk({"message": msg})` to standardize
/// simple status/message style responses across delete/revoke endpoints.
pub fn ok_message(msg: &str) -> ApiOk<serde_json::Value> { ApiOk(json!({"message": msg})) }

/// Generic helper for delete style endpoints that only need to run an async
/// operation and then return a standard message payload.
pub async fn delete_with<F>(op: F, message: &str) -> crate::Result<ApiOk<serde_json::Value>>
where
    F: std::future::Future<Output = crate::Result<()>>,
{
    op.await?;
    Ok(ok_message(message))
}

/// Newtype wrapper enabling `ApiOk(data)` return style.
pub struct ApiOk<T: Serialize>(pub T);

impl<T: Serialize> From<T> for ApiOk<T> { fn from(value: T) -> Self { Self(value) } }

impl<T: Serialize> IntoResponse for ApiOk<T> {
    fn into_response(self) -> axum::response::Response {
    let body: Json<ApiResponse<T>> = Json(ApiResponse::success(self.0));
        body.into_response()
    }
}
