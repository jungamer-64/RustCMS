use axum::body::to_bytes;
use axum::{Router, routing::get};
use cms_backend::{error::AppError, utils::response_ext::ApiOk};
use hyper::{Request, StatusCode};
use insta::assert_json_snapshot;
use tower::ServiceExt; // for oneshot

// /error エンドポイントのエラーレスポンス (ApiResponse 統一形状) をスナップショット固定
#[tokio::test]
async fn snapshot_error_endpoint() {
    async fn handler() -> Result<ApiOk<serde_json::Value>, AppError> {
        Err(AppError::BadRequest("invalid demo input".into()))
    }

    let router = Router::new().route("/error", get(handler));

    let response = router
        .oneshot(
            Request::get("/error")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    // 期待形状: { success:false, data:null, message:null, error:"...", validation_errors:null }
    assert_json_snapshot!("error_endpoint_body", json);
}
