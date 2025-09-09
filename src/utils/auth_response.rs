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
    fn from(a: crate::auth::AuthResponse) -> Self { a.tokens }
}


/// 統一認証レスポンス (login/register 用)
///
/// tokens オブジェクトに加え、後方互換のため従来フラットなフィールド (access_token / refresh_token / biscuit_token / expires_in / session_id / token) も保持する。
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSuccessResponse {
    pub success: bool,
    pub tokens: AuthTokens,
    pub user: UserInfo,
    #[cfg(feature = "auth-flat-fields")]
    #[deprecated(note = "Use tokens.access_token (will be removed in 3.0.0)")]
    pub access_token: String,
    #[cfg(feature = "auth-flat-fields")]
    #[deprecated(note = "Use tokens.refresh_token (will be removed in 3.0.0)")]
    pub refresh_token: String,
    #[cfg(feature = "auth-flat-fields")]
    #[deprecated(note = "Use tokens.biscuit_token (will be removed in 3.0.0)")]
    pub biscuit_token: String,
    #[cfg(feature = "auth-flat-fields")]
    #[deprecated(note = "Use tokens.expires_in (will be removed in 3.0.0)")]
    pub expires_in: i64,
    #[cfg(feature = "auth-flat-fields")]
    #[deprecated(note = "Use tokens.session_id (will be removed in 3.0.0)")]
    pub session_id: String,
    /// 旧クライアント互換 (token == access_token)
    #[cfg(feature = "auth-flat-fields")]
    #[deprecated(note = "Use tokens.access_token (alias, will be removed in 3.0.0)")]
    pub token: String,
}

impl From<crate::auth::AuthResponse> for AuthSuccessResponse {
    fn from(a: crate::auth::AuthResponse) -> Self { AuthSuccessResponse::from_parts(&a.tokens, a.user) }
}

impl AuthSuccessResponse {
    pub fn from_parts(tokens: &AuthTokens, user: UserInfo) -> Self {
        #[allow(deprecated)]
        Self {
            success: true,
            tokens: tokens.clone(),
            user,
            #[cfg(feature = "auth-flat-fields")]
            access_token: tokens.access_token.clone(),
            #[cfg(feature = "auth-flat-fields")]
            refresh_token: tokens.refresh_token.clone(),
            #[cfg(feature = "auth-flat-fields")]
            biscuit_token: tokens.biscuit_token.clone(),
            #[cfg(feature = "auth-flat-fields")]
            expires_in: tokens.expires_in,
            #[cfg(feature = "auth-flat-fields")]
            session_id: tokens.session_id.clone(),
            #[cfg(feature = "auth-flat-fields")]
            token: tokens.access_token.clone(),
        }
    }
}
