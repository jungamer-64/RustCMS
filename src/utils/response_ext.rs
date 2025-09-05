use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use crate::utils::api_types::ApiResponse;

/// (Deprecated) 自動で `ApiResponse` に包むためのトレイト。
///
/// 置き換え: `ApiOk(value)` もしくは `Json(ApiResponse::success(value))` を直接返してください。
/// このトレイトは後方互換のため暫定残置されますが今後削除予定です。
#[deprecated(note = "Use ApiOk<T> newtype (e.g. return ApiOk(data)) or return Json(ApiResponse<T>) directly.")]
pub trait IntoApiOk: Sized {
    type Resp: IntoResponse;
    fn into_api_ok(self) -> Self::Resp;
}

#[allow(deprecated)]
impl<T> IntoApiOk for T where T: Serialize {
    type Resp = Json<ApiResponse<T>>;
    fn into_api_ok(self) -> Self::Resp { Json(ApiResponse::success(self)) }
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
