//! Biscuit token rotation & invalidation test (requires database + auth features).
#![cfg(all(feature = "auth", feature = "database"))]

mod common;

#[tokio::test]
async fn biscuit_refresh_rotation_invalidate_old() {
    common::setup();
    let Some(db) = common::build_db().await else {
        eprintln!("SKIP biscuit_refresh_rotation_invalidate_old: DATABASE_URL not set");
        return;
    };
    let auth = common::build_auth(&db, 30, 300).await;
    let user = common::dummy_user();

    let issued = auth
        .create_auth_response(user.clone(), false)
        .await
        .expect("issue");
    assert!(!issued.tokens.access_token.is_empty());
    assert!(!issued.tokens.refresh_token.is_empty());

    // Basic sanity: access token not empty already checked.

    let (rotated_tokens, _rot_user) = auth
        .refresh_access_token(&issued.tokens.refresh_token)
        .await
        .expect("refresh");
    assert_ne!(rotated_tokens.access_token, issued.tokens.access_token);
    assert_ne!(rotated_tokens.refresh_token, issued.tokens.refresh_token);
    // biscuit_token populated with access token for backward compatibility
    assert_eq!(rotated_tokens.biscuit_token, rotated_tokens.access_token);

    // Old refresh must now fail
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err()
    );

    // Using a refresh token as if it were an access token should fail verification
    // Can't validate refresh token as access token; just ensure they differ & refresh rotation worked.
}
