//! Integration-ish auth & metrics flow test.
//!
//! 注意: 実 DB(PostgreSQL) 接続が必要。接続できない場合はテストを早期 return してスキップ扱いにする。
//! データ破壊を避けるため、ランダムユーザー (UUID) を利用し副作用最小化。
//! 実行には `--features database,auth` が必要。search/cache は任意。
#![cfg(all(feature = "database", feature = "auth"))]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt; // provides collect
use serde_json::Value;
use tower::ServiceExt; // for `oneshot`

/// Helper to build JSON POST request
fn post_json(uri: &str, json: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(json.to_string()))
        .expect("request build")
}

#[tokio::test]
async fn auth_register_login_refresh_and_metrics() {
    // Attempt to build full AppState (may fail if DB not available)
    let state = match cms_backend::AppState::from_env().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SKIP auth flow test (state init failed): {e}");
            return; // treat as skipped
        }
    };

    let router = cms_backend::routes::create_router().with_state(state.clone());

    // Randomize credentials to avoid unique constraint collisions
    let uid = uuid::Uuid::new_v4();
    let email = format!("test+{}@example.com", uid);
    let username = format!("user_{}", uid.simple());
    let password = "P@ssw0rd!"; // strength not critical here

    // 1. Register
    let reg_body = serde_json::json!({
        "username": username,
        "email": email,
        "password": password
    });
    let response = router
        .clone()
        .oneshot(post_json("/api/v1/auth/register", &reg_body))
        .await
        .expect("register resp");
    assert_eq!(response.status(), StatusCode::CREATED, "register status");
    let collected = response.into_body().collect().await.expect("collect body");
    let bytes = collected.to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("json parse");
    let refresh_token = json["data"]["refresh_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!refresh_token.is_empty(), "refresh token missing");

    // 2. Login
    let login_body = serde_json::json!({
        "email": email,
        "password": password,
        "remember_me": false
    });
    let response = router
        .clone()
        .oneshot(post_json("/api/v1/auth/login", &login_body))
        .await
        .expect("login resp");
    assert_eq!(response.status(), StatusCode::OK, "login status");
    let collected = response.into_body().collect().await.expect("collect body");
    let bytes = collected.to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("login json");
    assert!(
        json["data"]["access_token"].as_str().is_some(),
        "login access token missing"
    );

    // 3. Refresh
    let refresh_body = serde_json::json!({ "refresh_token": refresh_token });
    let response = router
        .clone()
        .oneshot(post_json("/api/v1/auth/refresh", &refresh_body))
        .await
        .expect("refresh resp");
    assert_eq!(response.status(), StatusCode::OK, "refresh status");
    let collected = response.into_body().collect().await.expect("collect body");
    let bytes = collected.to_bytes();
    let json: Value = serde_json::from_slice(&bytes).expect("refresh json");
    assert!(
        json["data"]["access_token"].as_str().is_some(),
        "refreshed access missing"
    );

    // 4. Metrics (plaintext). Just ensure 200 and contains a known metric label.
    let metrics_req = Request::builder()
        .method("GET")
        .uri("/api/v1/metrics")
        .body(Body::empty())
        .unwrap();
    let response = router
        .clone()
        .oneshot(metrics_req)
        .await
        .expect("metrics resp");
    assert_eq!(response.status(), StatusCode::OK, "metrics status");
    let collected = response
        .into_body()
        .collect()
        .await
        .expect("collect metrics");
    let text = String::from_utf8(collected.to_bytes().to_vec()).expect("utf8 metrics");
    assert!(
        text.contains("cms_total_requests"),
        "metrics output missing expected counter"
    );
}
