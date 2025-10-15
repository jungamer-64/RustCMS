//! `OpenAPI` の security スキーム適用を検証するテスト
use axum::{body::to_bytes, response::IntoResponse};
use cms_backend::handlers; // lib ターゲット (crate 名は Cargo.toml の package 名に依存)
use hyper::StatusCode;

async fn fetch_openapi_json() -> serde_json::Value {
    let resp = handlers::openapi_json()
        .await
        .expect("OpenAPI generation should succeed")
        .into_response();
    // Body を bytes へ取り出し
    let (parts, body) = resp.into_parts();
    // body は axum 0.8 の Body (hyper::Body 互換)
    // axum::body::to_bytes は (body, limit) 署名。無制限扱いで None。
    let bytes = to_bytes(body, usize::MAX).await.expect("body bytes");
    assert_eq!(parts.status, StatusCode::OK);
    serde_json::from_slice(&bytes).expect("valid json")
}

#[tokio::test]
async fn protected_endpoint_has_biscuit_security() {
    let v = fetch_openapi_json().await;
    let paths = v
        .get("paths")
        .and_then(|p| p.as_object())
        .expect("paths missing");
    let posts = paths
        .get("/api/v1/posts")
        .and_then(|p| p.get("get"))
        .and_then(|m| m.as_object())
        .expect("/api/v1/posts get missing");
    let sec = posts
        .get("security")
        .and_then(|s| s.as_array())
        .expect("security array missing");
    assert_eq!(
        sec.len(),
        1,
        "expected exactly one security requirement (BiscuitAuth)"
    );
    let obj = sec[0].as_object().expect("entry object");
    assert!(obj.contains_key("BiscuitAuth"), "BiscuitAuth key missing");
}

#[tokio::test]
async fn public_endpoint_has_no_security() {
    let v = fetch_openapi_json().await;
    let paths = v
        .get("paths")
        .and_then(|p| p.as_object())
        .expect("paths missing");
    // When `auth` feature is enabled the login endpoint should exist and be public.
    // When `auth` is disabled the login path will not be present in the spec; accept that too.
    if cfg!(feature = "auth") {
        let login = paths
            .get("/api/v1/auth/login")
            .and_then(|p| p.get("post"))
            .and_then(|m| m.as_object())
            .expect("/api/v1/auth/login post missing");
        assert!(
            login.get("security").is_none(),
            "public login endpoint should have no security block"
        );
    } else {
        assert!(
            paths.get("/api/v1/auth/login").is_none(),
            "auth disabled -> login should be absent"
        );
    }
}

#[tokio::test]
async fn biscuit_auth_endpoint_has_biscuit_security() {
    let v = fetch_openapi_json().await;
    let paths = v
        .get("paths")
        .and_then(|p| p.as_object())
        .expect("paths missing");
    let reindex = paths
        .get("/api/v1/search/reindex")
        .and_then(|p| p.get("post"))
        .and_then(|m| m.as_object())
        .expect("/api/v1/search/reindex post missing");
    let sec = reindex
        .get("security")
        .and_then(|s| s.as_array())
        .expect("security missing");
    assert_eq!(
        sec.len(),
        1,
        "expected exactly one security requirement (BiscuitAuth)"
    );
    let obj = sec[0].as_object().expect("entry object");
    assert!(obj.contains_key("BiscuitAuth"), "BiscuitAuth key missing");
}

#[tokio::test]
async fn api_key_permissions_extension_present() {
    let v = fetch_openapi_json().await;
    let components = v
        .get("components")
        .and_then(|c| c.as_object())
        .expect("components missing");
    let perms = components
        .get("x-apiKey-permissions")
        .and_then(|p| p.as_array())
        .expect("x-apiKey-permissions missing");
    assert!(!perms.is_empty(), "permissions array should not be empty");
    assert!(
        perms.iter().any(|p| p.as_str() == Some("posts:read")),
        "expected posts:read in permissions"
    );
}

#[tokio::test]
async fn api_key_header_security_scheme_present() {
    let v = fetch_openapi_json().await;
    let components = v
        .get("components")
        .and_then(|c| c.as_object())
        .expect("components missing");
    let security_schemes = components
        .get("securitySchemes")
        .and_then(|s| s.as_object())
        .expect("securitySchemes missing");
    assert!(
        security_schemes.contains_key("ApiKeyHeader"),
        "ApiKeyHeader scheme missing"
    );
    let scheme = security_schemes
        .get("ApiKeyHeader")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(scheme.get("type").and_then(|v| v.as_str()), Some("apiKey"));
    assert_eq!(
        scheme.get("name").and_then(|v| v.as_str()),
        Some("X-API-Key")
    );
}

#[tokio::test]
async fn api_key_management_endpoints_have_biscuit_security() {
    let v = fetch_openapi_json().await;
    let paths = v
        .get("paths")
        .and_then(|p| p.as_object())
        .expect("paths missing");
    for (path, method) in [
        ("/api/v1/api-keys", "post"),
        ("/api/v1/api-keys", "get"),
        ("/api/v1/api-keys/{id}", "delete"),
    ] {
        if let Some(pitem) = paths.get(path) {
            if let Some(op) = pitem.get(method).and_then(|m| m.as_object()) {
                let sec = op
                    .get("security")
                    .and_then(|s| s.as_array())
                    .expect("security block missing");
                assert_eq!(
                    sec.len(),
                    1,
                    "{method} {path} should have exactly one security requirement (BiscuitAuth)"
                );
                let obj = sec[0].as_object().expect("entry object");
                assert!(
                    obj.contains_key("BiscuitAuth"),
                    "{method} {path} should have BiscuitAuth",
                );
            } else {
                panic!("operation {method} {path} missing");
            }
        } else {
            panic!("path {path} missing");
        }
    }
}
