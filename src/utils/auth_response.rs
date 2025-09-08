use crate::utils::common_types::UserInfo;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
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
            biscuit_token: r.biscuit_token.unwrap_or_default(),
            expires_in: r.expires_in,
            session_id: r.session_id,
        }
    }
}

/// 統一認証レスポンス (login/register 用)
///
/// tokens オブジェクトに加え、後方互換のため従来フラットなフィールド (access_token / refresh_token / biscuit_token / expires_in / session_id / token) も保持する。
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSuccessResponse {
    pub success: bool,
    pub tokens: AuthTokens,
    pub user: UserInfo,
    // --- Backward compatible flattened fields ---
    pub access_token: String,
    pub refresh_token: String,
    pub biscuit_token: String,
    pub expires_in: i64,
    pub session_id: String,
    /// 旧クライアント互換 (token == access_token)
    pub token: String,
}

impl From<crate::auth::AuthResponse> for AuthSuccessResponse {
    fn from(a: crate::auth::AuthResponse) -> Self {
    let access_token = a.access_token.clone();
    let refresh_token = a.refresh_token.clone();
    let biscuit_token = a.biscuit_token.clone();
    let expires_in = a.expires_in;
    let session_id = a.session_id.clone();
    let user = a.user;
    let tokens = AuthTokens { access_token: access_token.clone(), refresh_token: refresh_token.clone(), biscuit_token: biscuit_token.clone(), expires_in, session_id: session_id.clone() };
    AuthSuccessResponse { success: true, tokens, user, access_token: access_token.clone(), refresh_token, biscuit_token, expires_in, session_id, token: access_token }
    }
}
