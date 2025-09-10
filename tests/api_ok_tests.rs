use axum::http::StatusCode;
use axum::response::IntoResponse;
use cms_backend::utils::api_types::ApiResponse;
use cms_backend::utils::response_ext::ApiOk;
use serde_json::Value;

#[test]
fn api_ok_wraps_value() {
    let resp = ApiOk(serde_json::json!({"foo":"bar"})).into_response();
    // Body is boxed; collect bytes
    let body_bytes = futures::executor::block_on(async {
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
    });
    let v: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["data"]["foo"], "bar");
    assert!(v["error"].is_null());
}

#[test]
fn api_ok_created_status_tuple() {
    let tuple = (StatusCode::CREATED, ApiOk(serde_json::json!({"id": 123})));
    let resp = tuple.into_response();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body_bytes = futures::executor::block_on(async {
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
    });
    let v: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["data"]["id"], 123);
}

#[test]
fn api_response_manual_construction_example() {
    // Ensure manual construction stays backward compatible
    let manual: ApiResponse<Value> = ApiResponse::success(serde_json::json!({"k":"v"}));
    let json = serde_json::to_value(&manual).unwrap();
    assert_eq!(json["data"]["k"], "v");
}
