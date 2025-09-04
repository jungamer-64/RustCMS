use cms_backend::utils::api_types::{ok, err};
use cms_backend::error::AppError; // Re-export path likely cms_backend::AppError if different adjust
use axum::response::IntoResponse;
use axum::body::to_bytes;
use http_body_util::BodyExt; // if available; fallback manual collect
use validator::Validate;

#[test]
fn ok_helper_wraps_data() {
    let j = ok(serde_json::json!({"foo":"bar"}));
    let body = serde_json::to_value(&j.0).unwrap();
    assert_eq!(body.get("success").unwrap(), true);
    assert_eq!(body.get("data").unwrap().get("foo").unwrap(), "bar");
}

#[test]
fn err_helper_wraps_error() {
    let j = err("bad");
    let body = serde_json::to_value(&j.0).unwrap();
    assert_eq!(body.get("success").unwrap(), false);
    assert_eq!(body.get("error").unwrap(), "bad");
}

#[derive(Debug, Validate)]
struct DummyInput { #[validate(length(min = 5))] name: String }

#[tokio::test]
async fn validation_error_converted() {
    let input = DummyInput { name: "aa".into() }; // too short
    let err = input.validate().unwrap_err();
    let app_err = AppError::from(err);
    let resp = app_err.into_response();
    // Extract body
    // For axum 0.7: body is http_body::Body; use body::to_bytes with a size limit
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["success"], false);
    assert!(v["error"].as_str().unwrap().contains("Invalid input"));
    assert!(v["validation_errors"].as_array().unwrap().len() >= 1);
}
