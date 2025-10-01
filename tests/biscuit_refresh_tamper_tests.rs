//! Tampering tests for refresh biscuit tokens

use base64::Engine; // for decode/encode methods on STANDARD

mod common;

fn naive_tamper(b64: &str) -> String {
    // Base64 decode then perform a harmless ascii replacement to change token_type string; this invalidates signature
    let raw = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap_or_default();
    let mut text = String::from_utf8_lossy(&raw).to_string();
    if text.contains("refresh") {
        text = text.replace("refresh", "access");
    }
    // re-encode => biscuit verification should fail due to signature mismatch
    base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
}

#[tokio::test]
async fn tampered_refresh_token_rejected() {
    common::setup();
    let Some(db) = common::build_db().await else {
        eprintln!("SKIP tampered_refresh_token_rejected: DATABASE_URL not set");
        return;
    };
    let auth = common::build_auth(&db, 60, 300).await;
    let user = common::dummy_user();
    let issued = auth.create_auth_response(user, false).await.expect("issue");
    let tampered = naive_tamper(&issued.tokens.refresh_token);
    assert!(
        auth.refresh_access_token(&tampered).await.is_err(),
        "tampered token should be invalid"
    );
}

#[tokio::test]
async fn random_garbage_token_rejected() {
    common::setup();
    let Some(db) = common::build_db().await else {
        eprintln!("SKIP random_garbage_token_rejected: DATABASE_URL not set");
        return;
    };
    let auth = common::build_auth(&db, 60, 300).await;
    // 生成されていない乱数 base64 文字列
    let garbage = base64::engine::general_purpose::STANDARD.encode("totally-invalid-token");
    assert!(
        auth.refresh_access_token(&garbage).await.is_err(),
        "garbage token should be invalid"
    );
}
