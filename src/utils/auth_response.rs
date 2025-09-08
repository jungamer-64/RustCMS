use crate::utils::common_types::UserInfo;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema, Clone)]
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


/// 統一認証レスポンス (login/register 用)
///
/// tokens オブジェクトに加え、後方互換のため従来フラットなフィールド (access_token / refresh_token / biscuit_token / expires_in / session_id / token) も保持する。
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSuccessResponse {
    pub success: bool,
    pub tokens: AuthTokens,
    pub user: UserInfo,
    // --- Backward compatible flattened fields ---
    #[deprecated(note = "Use tokens.access_token")]
    pub access_token: String,
    #[deprecated(note = "Use tokens.refresh_token")]
    pub refresh_token: String,
    #[deprecated(note = "Use tokens.biscuit_token")]
    pub biscuit_token: String,
    #[deprecated(note = "Use tokens.expires_in")]
    pub expires_in: i64,
    #[deprecated(note = "Use tokens.session_id")]
    pub session_id: String,
    /// 旧クライアント互換 (token == access_token)
    #[deprecated(note = "Use tokens.access_token (alias)")]
    pub token: String,
}

impl From<crate::auth::AuthResponse> for AuthSuccessResponse {
    fn from(a: crate::auth::AuthResponse) -> Self {
        let tokens = AuthTokens {
            access_token: a.access_token,
            refresh_token: a.refresh_token,
            biscuit_token: a.biscuit_token,
            expires_in: a.expires_in,
            session_id: a.session_id,
        };
        AuthSuccessResponse::from_parts(&tokens, a.user)
    }
}

impl AuthSuccessResponse {
    pub fn from_parts(tokens: &AuthTokens, user: UserInfo) -> Self {
    // Centralize deprecated flattened field population
    #[allow(deprecated)]
    let resp = Self {
            success: true,
            tokens: tokens.clone(),
            user,
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone(),
            biscuit_token: tokens.biscuit_token.clone(),
            expires_in: tokens.expires_in,
            session_id: tokens.session_id.clone(),
            token: tokens.access_token.clone(),
    };
    resp
    }
}
