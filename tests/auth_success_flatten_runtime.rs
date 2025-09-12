//! Runtime serialization guard for `AuthSuccessResponse` flattened token fields.
//! Ensures that when feature `auth-flat-fields` is enabled the deprecated fields still serialize,
//! and when disabled they are absent, preventing accidental resurrection or premature removal.
use chrono::Utc;
use cms_backend::models::UserRole;
use cms_backend::utils::auth_response::{AuthSuccessResponse, AuthTokens};
use cms_backend::utils::common_types::UserInfo;

fn sample_user() -> UserInfo {
    UserInfo {
        id: "11111111-2222-3333-4444-555555555555".into(),
        username: "tester".into(),
        email: "tester@example.com".into(),
        first_name: None,
        last_name: None,
        role: UserRole::Subscriber,
        is_active: true,
        email_verified: true,
        last_login: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[cfg(feature = "auth-flat-fields")]
#[test]
fn auth_success_serialization_contains_flat_fields() {
    let tokens = AuthTokens {
        access_token: "A".into(),
        refresh_token: "R".into(),
        biscuit_token: "B".into(),
        expires_in: 1234,
        session_id: "S".into(),
    };
    let user = sample_user();
    let resp = AuthSuccessResponse::from_parts(&tokens, user);
    let v = serde_json::to_value(&resp).expect("serialize resp");
    let obj = v.as_object().unwrap();
    for k in [
        "access_token",
        "refresh_token",
        "biscuit_token",
        "expires_in",
        "session_id",
        "token",
    ] {
        assert!(
            obj.contains_key(k),
            "expected key `{k}` present with feature enabled"
        );
    }
}

#[cfg(not(feature = "auth-flat-fields"))]
#[test]
fn auth_success_serialization_excludes_flat_fields() {
    let tokens = AuthTokens {
        access_token: "A".into(),
        refresh_token: "R".into(),
        biscuit_token: "B".into(),
        expires_in: 1234,
        session_id: "S".into(),
    };
    let user = sample_user();
    let resp = AuthSuccessResponse::from_parts(&tokens, user);
    let v = serde_json::to_value(&resp).expect("serialize resp");
    let obj = v.as_object().unwrap();
    for k in [
        "access_token",
        "refresh_token",
        "biscuit_token",
        "expires_in",
        "session_id",
        "token",
    ] {
        assert!(
            !obj.contains_key(k),
            "key `{k}` should be absent with feature disabled"
        );
    }
}
