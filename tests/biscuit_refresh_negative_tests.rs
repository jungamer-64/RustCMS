//! Negative tests for refresh token: expiry & version mismatch

mod common;

#[tokio::test]
async fn refresh_fails_after_expiry() {
    common::setup();
    let Some(db) = common::build_db().await else {
        eprintln!("SKIP refresh_fails_after_expiry: DATABASE_URL not set");
        return;
    };
    // Very short refresh TTL
    let auth = common::build_auth(&db, 1, 2).await; // access 1s, refresh 2s
    let user = common::dummy_user();
    let issued = auth.create_auth_response(user, false).await.expect("issue");
    // Wait until refresh expiry passes
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn refresh_version_mismatch_reuse_old() {
    common::setup();
    let Some(db) = common::build_db().await else {
        eprintln!("SKIP refresh_version_mismatch_reuse_old: DATABASE_URL not set");
        return;
    };
    let auth = common::build_auth(&db, 30, 60).await;
    let user = common::dummy_user();
    let issued = auth
        .create_auth_response(user.clone(), false)
        .await
        .expect("issue");
    // First rotation
    let (rot_tokens, _) = auth
        .refresh_access_token(&issued.tokens.refresh_token)
        .await
        .expect("rotate");
    // Reuse original refresh => should fail (version bump)
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err()
    );
    // Rotated token still works once
    let _ = auth
        .refresh_access_token(&rot_tokens.refresh_token)
        .await
        .expect("second ok");
}
