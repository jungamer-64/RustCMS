use axum::{body::to_bytes, response::IntoResponse};
use cms_backend::handlers;

#[tokio::test]
async fn api_info_alias_returns_same_body() {
    let res_root = handlers::api_info_v1().await.into_response();
    assert_eq!(res_root.status(), axum::http::StatusCode::OK);
    let body_root = to_bytes(res_root.into_body(), usize::MAX).await.unwrap();

    let res_alias = handlers::api_info_info().await.into_response();
    assert_eq!(res_alias.status(), axum::http::StatusCode::OK);
    let body_alias = to_bytes(res_alias.into_body(), usize::MAX).await.unwrap();

    assert_eq!(
        body_root, body_alias,
        "alias endpoint must return identical JSON"
    );
}
