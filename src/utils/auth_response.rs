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
    #[must_use]
    pub const fn new(tokens: AuthTokens, user: Option<UserInfo>, extra: Option<T>) -> Self {
        Self {
            success: true,
            tokens,
            user,
            extra,
        }
    }
}

impl From<crate::auth::AuthResponse> for AuthTokens {
    fn from(a: crate::auth::AuthResponse) -> Self {
        a.tokens
    }
}

/// 統一認証レスポンス (login/register 用)
///
/// Biscuit トークンベースの認証情報を `tokens` オブジェクトで提供します。
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSuccessResponse {
    pub success: bool,
    pub tokens: AuthTokens,
    pub user: UserInfo,
}

impl From<crate::auth::AuthResponse> for AuthSuccessResponse {
    fn from(a: crate::auth::AuthResponse) -> Self {
        Self::from_parts(&a.tokens, a.user)
    }
}

impl AuthSuccessResponse {
    #[must_use]
    pub fn from_parts(tokens: &AuthTokens, user: UserInfo) -> Self {
        Self {
            success: true,
            tokens: tokens.clone(),
            user,
        }
    }
}
