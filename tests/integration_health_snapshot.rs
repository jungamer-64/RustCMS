use axum::body::to_bytes;
use axum::{Router, routing::get};
use cms_backend::app::{HealthStatus, ServiceHealth};
use cms_backend::utils::response_ext::ApiOk;
use hyper::{Request, StatusCode};
use insta::assert_json_snapshot;
use serde_json::json;
use tower::ServiceExt; // for oneshot

// 簡易統合: /health エンドポイントの JSON 形状をスナップショット固定
#[tokio::test]
async fn snapshot_health_endpoint() {
    // 実サービス初期化を避けるためダミーの HealthStatus を組み立て
    let dummy = HealthStatus {
        status: "healthy".into(),
        database: ServiceHealth {
            status: "up".into(),
            response_time_ms: 1.0,
            error: None,
            details: json!({}),
        },
        cache: ServiceHealth {
            status: "not_configured".into(),
            response_time_ms: 0.0,
            error: None,
            details: json!({}),
        },
        search: ServiceHealth {
            status: "not_configured".into(),
            response_time_ms: 0.0,
            error: None,
            details: json!({}),
        },
        auth: ServiceHealth {
            status: "not_configured".into(),
            response_time_ms: 0.0,
            error: None,
            details: json!({}),
        },
        timestamp: chrono::Utc::now(),
    };

    let router = Router::new().route(
        "/health",
        get(move || {
            let snapshot = dummy.clone();
            async move { ApiOk(snapshot) }
        }),
    );

    let response = router
        .oneshot(
            Request::get("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    // timestamp は可変なのでマスク
    if let Some(ts) = json.get_mut("data").and_then(|d| d.get_mut("timestamp")) {
        *ts = serde_json::Value::String("<redacted>".into());
    }
    assert_json_snapshot!("health_endpoint_data", json["data"].clone());
}
