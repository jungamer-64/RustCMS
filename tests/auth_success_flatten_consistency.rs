//! Ensures deprecated flattened fields in AuthSuccessResponse remain consistent with tokens.* until Phase 4 removal.
use cms_backend::utils::auth_response::{AuthTokens, AuthSuccessResponse};
use cms_backend::models::UserRole;
use cms_backend::utils::common_types::UserInfo;
use chrono::Utc;

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

#[test]
fn auth_success_flatten_fields_match_tokens() {
    let tokens = AuthTokens {
        access_token: "ACCESS".into(),
        refresh_token: "REFRESH".into(),
        biscuit_token: "BISCUIT".into(),
        expires_in: 3600,
        session_id: "sess_abc".into(),
    };
    let user = sample_user();
    let resp = AuthSuccessResponse::from_parts(&tokens, user);

    // Use deprecated fields (allow) and assert equality.
    #[allow(deprecated)]
    {
        assert_eq!(resp.access_token, resp.tokens.access_token);
        assert_eq!(resp.refresh_token, resp.tokens.refresh_token);
        assert_eq!(resp.biscuit_token, resp.tokens.biscuit_token);
        assert_eq!(resp.expires_in, resp.tokens.expires_in);
        assert_eq!(resp.session_id, resp.tokens.session_id);
        assert_eq!(resp.token, resp.tokens.access_token);
    }
}
