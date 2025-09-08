use crate::utils::common_types::UserInfo;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub biscuit_token: String,
    pub expires_in: i64,
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct AuthSuccess<T> {
    pub success: bool,
    pub tokens: AuthTokens,
    pub user: Option<UserInfo>,
    pub extra: Option<T>,
}

impl<T> AuthSuccess<T> {
    pub fn new(tokens: AuthTokens, user: Option<UserInfo>, extra: Option<T>) -> Self {
        Self { success: true, tokens, user, extra }
    }
}

impl From<crate::auth::AuthResponse> for AuthTokens {
    fn from(a: crate::auth::AuthResponse) -> Self {
        AuthTokens {
            access_token: a.access_token,
            refresh_token: a.refresh_token,
            biscuit_token: a.biscuit_token,
            expires_in: a.expires_in,
            session_id: a.session_id,
        }
    }
}

impl From<crate::auth::RefreshResponse> for AuthTokens {
    fn from(r: crate::auth::RefreshResponse) -> Self {
        AuthTokens {
            access_token: r.access_token,
            refresh_token: r.refresh_token,
            // RefreshResponse には biscuit_token が含まれないため空文字列で埋める (将来的に付与する場合はここで拡張)
            biscuit_token: String::new(),
            expires_in: r.expires_in,
            session_id: r.session_id,
        }
    }
}
