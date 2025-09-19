//! Session missing scenario: refresh should fail when session record is removed

mod common;

#[tokio::test]
async fn refresh_fails_when_session_missing() {
    common::setup();
    let Some(db) = common::build_db().await else {
        eprintln!("SKIP refresh_fails_when_session_missing: DATABASE_URL not set");
        return;
    };
    let auth = common::build_auth(&db, 60, 300).await;
    let user = common::dummy_user();
    let issued = auth.create_auth_response(user, false).await.expect("issue");
    // Simulate eviction by constructing a fresh AuthService (new in-memory store)
    let auth = common::build_auth(&db, 60, 300).await;
    assert!(
        auth.refresh_access_token(&issued.tokens.refresh_token)
            .await
            .is_err(),
        "refresh should fail after session map cleared"
    );
}
