#[cfg(feature = "auth")]
use crate::limiter::adapters::ApiKeyFailureLimiterAdapter;
use crate::limiter::{GenericRateLimiter, RateLimitDecision};
use axum::http::StatusCode;
use axum::{body::Body, http::Request, middleware::Next, response::Response};
#[cfg(feature = "monitoring")]
use metrics::{counter, gauge};
use once_cell::sync::Lazy;
use tracing::{debug, warn};

// 統一トレイト対応アダプタ (失敗回数レートリミット) ※AUTH feature 時のみ
#[cfg(feature = "auth")]
static API_KEY_FAILURE_LIMITER: Lazy<ApiKeyFailureLimiterAdapter> =
    Lazy::new(|| ApiKeyFailureLimiterAdapter::from_env());

/// 抽出結果を request extensions に格納するキー
#[derive(Clone, Debug)]
pub struct ApiKeyAuthInfo {
    pub api_key_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub permissions: Vec<String>,
}

pub const HEADER_NAME: &str = "X-API-Key";

// NOTE: Actual rate limit configuration & state is handled by `rate_limit_backend::GLOBAL_RATE_LIMITER`.
// This module focuses only on auth flow + metric emission.

#[allow(dead_code)]
pub async fn api_key_auth_layer(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    #[cfg(feature = "monitoring")]
    let record_fail = |reason: &'static str| {
        counter!("api_key_auth_failure_total", "reason" => reason).increment(1);
    };
    #[cfg(not(feature = "monitoring"))]
    let record_fail = |_reason: &'static str| {};
    #[cfg(feature = "monitoring")]
    {
        // gauges initialised in backend during first use
        counter!("api_key_auth_attempts_total").increment(1);
    }
    // 1. ヘッダ取得
    let header_val = match req.headers().get(HEADER_NAME) {
        Some(v) => v,
        None => {
            record_fail("missing_header");
            return Err((StatusCode::UNAUTHORIZED, "API key missing"));
        }
    };
    let raw = match header_val.to_str() {
        Ok(s) => s,
        Err(_) => {
            record_fail("invalid_header_encoding");
            return Err((StatusCode::BAD_REQUEST, "Invalid header"));
        }
    };

    if raw.len() < 10 || !raw.starts_with("ak_") {
        record_fail("malformed");
        return Err((StatusCode::UNAUTHORIZED, "Malformed API key"));
    }

    // AppState 取得
    let state = match req.extensions().get::<crate::AppState>() {
        Some(s) => s.clone(),
        None => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Missing AppState")),
    };

    // 決定的 lookup hash
    let lookup_hash = crate::models::api_key::ApiKey::lookup_hash(raw);
    // レート制限 (統一アダプタ経由)。
    let decision = API_KEY_FAILURE_LIMITER.check(&lookup_hash);
    if let RateLimitDecision::Blocked { .. } = decision {
        #[cfg(feature = "monitoring")]
        {
            counter!("api_key_auth_failure_total", "reason" => "rate_limited").increment(1);
            gauge!("api_key_rate_limit_tracked_keys")
                .set(API_KEY_FAILURE_LIMITER.tracked_len() as f64);
        }
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "API key attempts rate limited",
        ));
    }
    let api_key = match state.db_get_api_key_by_lookup_hash(&lookup_hash).await {
        Ok(k) => k,
        Err(_) => match legacy_fallback_fetch(&state, raw).await {
            Some(k) => k,
            None => {
                record_fail("not_found");
                return Err((StatusCode::UNAUTHORIZED, "Invalid API key"));
            }
        },
    };

    // 生キーと保存された Argon2 ハッシュを検証 (タイミング計測は wrapper 内で済み)
    if !api_key.verify_key(raw).unwrap_or(false) {
        record_fail("hash_mismatch");
        return Err((StatusCode::UNAUTHORIZED, "Invalid API key"));
    }

    // 期限切れ確認
    if api_key.is_expired(chrono::Utc::now()) {
        record_fail("expired");
        return Err((StatusCode::UNAUTHORIZED, "API key expired"));
    }

    // last_used 更新 (失敗してもリクエストは継続)
    if let Err(e) = state.db_touch_api_key(api_key.id).await {
        warn!(?e, "Failed to update last_used_at");
    }

    // 成功: 該当 lookup_hash の失敗カウントをクリア (早期 +1 を相殺)
    API_KEY_FAILURE_LIMITER.clear(&lookup_hash);
    let info = ApiKeyAuthInfo {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        permissions: api_key.get_permissions(),
    };
    #[cfg(feature = "monitoring")]
    gauge!("api_key_rate_limit_tracked_keys").set(API_KEY_FAILURE_LIMITER.tracked_len() as f64);
    #[cfg(feature = "monitoring")]
    counter!("api_key_auth_success_total").increment(1);
    debug!(api_key_id=%info.api_key_id, user_id=%info.user_id, perms=?info.permissions, "API key authenticated");
    req.extensions_mut().insert(info);

    Ok(next.run(req).await)
}

#[cfg(all(feature = "database", feature = "auth"))]
async fn legacy_fallback_fetch(
    state: &crate::AppState,
    raw: &str,
) -> Option<crate::models::ApiKey> {
    // Delegate to centralized AppState DB wrapper for backfill.
    match state.db_backfill_api_key_lookup_for_raw(raw).await {
        Ok(Some(model)) => Some(model),
        _ => None,
    }
}

#[cfg(not(all(feature = "database", feature = "auth")))]
async fn legacy_fallback_fetch(
    _state: &crate::AppState,
    _raw: &str,
) -> Option<crate::models::ApiKey> {
    None
}
