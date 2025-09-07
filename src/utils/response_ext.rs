use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use crate::utils::api_types::ApiResponse;

/// Newtype wrapper enabling `ApiOk(data)` return style.
pub struct ApiOk<T: Serialize>(pub T);

impl<T: Serialize> From<T> for ApiOk<T> { fn from(value: T) -> Self { Self(value) } }

impl<T: Serialize> IntoResponse for ApiOk<T> {
    fn into_response(self) -> axum::response::Response {
    let body: Json<ApiResponse<T>> = Json(ApiResponse::success(self.0));
        body.into_response()
    }
}
