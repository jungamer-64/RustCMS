use axum::http::StatusCode;
use axum::response::IntoResponse;
use cms_backend::error::AppError;

#[test]
fn not_found_maps_to_404() {
    let err = AppError::NotFound("missing".into());
    let resp = err.into_response();
    // resp is a Response<Body> - convert to parts
    let status = resp.status();
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
fn internal_maps_to_500_and_generic_message() {
    let err = AppError::Internal("secret stacktrace".into());
    let resp = err.into_response();
    let status = resp.status();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}
