use crate::utils::common_types::UserInfo;
#[cfg(feature = "auth-flat-fields")]
use crate::utils::deprecation::warn_once;
#[cfg(all(feature = "auth-flat-fields", feature = "monitoring"))]
use metrics::counter;
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

#[cfg_attr(
    feature = "auth-flat-fields",
    doc = "統一認証レスポンス (login/register 用)\n\n`tokens` オブジェクトに加え、後方互換のため従来フラットなフィールド (access_token / refresh_token / biscuit_token / expires_in / session_id / token) も保持する。\n\nNOTE: フラットフィールドは feature `auth-flat-fields` 有効時のみ含まれ 3.0.0 で削除予定。"
)]
#[cfg_attr(
    not(feature = "auth-flat-fields"),
    doc = "統一認証レスポンス (login/register 用)\n\n`tokens` オブジェクトのみを公開 (フラットなトークン互換フィールドは feature `auth-flat-fields` を無効化した構成では除外済み)。 3.0.0 で完全移行予定。"
)]
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
    fn from(a: crate::auth::AuthResponse) -> Self {
        AuthSuccessResponse::from_parts(&a.tokens, a.user)
    }
}

impl AuthSuccessResponse {
    /// Internal constructor used when flattened fields are already absent (auth-flat-fields disabled)
    #[cfg(not(feature = "auth-flat-fields"))]
    fn new_unified(tokens: &AuthTokens, user: UserInfo) -> Self {
        Self {
            success: true,
            tokens: tokens.clone(),
            user,
        }
    }
    pub fn from_parts(tokens: &AuthTokens, user: UserInfo) -> Self {
        #[allow(deprecated)]
        {
            #[cfg(feature = "auth-flat-fields")]
            warn_once(
                "auth_flat_fields",
                "AuthSuccessResponse flattened token fields are deprecated and will be removed in 3.0.0. Disable feature 'auth-flat-fields' to preview removal.",
            );
            #[cfg(feature = "auth-flat-fields")]
            {
                // Metrics: count each construction that still emits deprecated flattened fields
                #[cfg(feature = "monitoring")]
                counter!("auth_flat_fields_legacy_usage_total").increment(1);
                return Self {
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
            }
            #[cfg(not(feature = "auth-flat-fields"))]
            {
                return Self::new_unified(tokens, user);
            }
        }
    }
}
