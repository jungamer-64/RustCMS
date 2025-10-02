//! API Key 認証ミドルウェア
//!
//! このモジュールはリクエストヘッダ `X-API-Key` による API キー認証を行います。
//! 主な処理の流れは次の通りです:
//!
//! 1) ヘッダ `X-API-Key` を取得・検証（接頭辞 `ak_`、最低長）
//! 2) アダプタベースの失敗回数レートリミットを確認（ブロックなら 429）
//! 3) 生キーから決定的な `lookup_hash` を導出し、DB で検索
//!    - 新旧互換のため、必要ならレガシー経路で lookup をバックフィル
//! 4) Argon2 ハッシュ照合で生キーを検証、期限切れをチェック
//! 5) 成功時は `last_used` の更新を試行し、失敗カウンタをクリア
//! 6) 検証済み情報（キーID/ユーザーID/権限）を request extensions に格納
//!
//! feature フラグ:
//! - `auth`: 認証と失敗レートリミットのアダプタを有効化
//! - `monitoring`: メトリクス（成功/失敗カウンタや追跡キー数ゲージ）を発行
//! - `database`: レガシー lookup バックフィルのための DB アクセスを有効化
//!
//! セキュリティ補足:
//! - 生キーはヘッダで搬送されるため HTTPS 前提です。
//! - DB 保持は Argon2 ハッシュで行われ、照合時のみ生キーを利用します。
//! - ブルートフォース抑止のため、キーごとの失敗回数に基づくレートリミットを適用します。
//! - 検証済みの権限は `ApiKeyAuthInfo.permissions` に格納されます。下流ハンドラは用途に応じてチェックしてください。

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
    Lazy::new(ApiKeyFailureLimiterAdapter::from_env);

/// 抽出結果を request extensions に格納する型。
///
/// 下流ハンドラは `req.extensions().get::<ApiKeyAuthInfo>()` で参照し、
/// キーの所有者や権限を元にアクセス制御を行えます。
/// 
/// **注意**: この型は後方互換性のために残されていますが、内部的には
/// `AuthContext` に変換されて使用されます。新しいコードでは
/// `AuthContext` を直接使用することを推奨します。
#[derive(Clone, Debug)]
pub struct ApiKeyAuthInfo {
    /// 検証済み API キーの識別子
    pub api_key_id: uuid::Uuid,
    /// キー所有者のユーザー ID
    pub user_id: uuid::Uuid,
    /// キーに付与された権限のリスト（例: "read:posts"）
    pub permissions: Vec<String>,
}

/// API キーを受け取る HTTP リクエストヘッダ名
pub const HEADER_NAME: &str = "X-API-Key";

// NOTE: Actual rate limit configuration & state is handled by `rate_limit_backend::GLOBAL_RATE_LIMITER`.
// This module focuses only on auth flow + metric emission.

#[cfg(feature = "auth")]
/// API キーを用いた認証レイヤ。
///
/// # Errors
/// 次の条件でエラーを返します:
/// - 認証ヘッダが欠落/不正な場合
/// - API キーが不正・期限切れの場合
/// - レートリミットによりブロックされた場合
/// - 内部状態が不足している場合（AppState 不在など）
///
/// # Returns
/// 成功時は検証済み情報を extensions に格納し、次のミドルウェア/ハンドラへフォワードします。
#[allow(clippy::cognitive_complexity)]
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
    let Some(header_val) = req.headers().get(HEADER_NAME) else {
        record_fail("missing_header");
        return Err((StatusCode::UNAUTHORIZED, "API key missing"));
    };
    let Ok(raw) = header_val.to_str() else {
        record_fail("invalid_header_encoding");
        return Err((StatusCode::BAD_REQUEST, "Invalid header"));
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
                .set(usize_to_f64(API_KEY_FAILURE_LIMITER.tracked_len()));
        }
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "API key attempts rate limited",
        ));
    }
    let api_key = match state.db_get_api_key_by_lookup_hash(&lookup_hash).await {
        Ok(k) => k,
        Err(_) => {
            if let Some(k) = legacy_fallback_fetch(&state, raw).await {
                k
            } else {
                record_fail("not_found");
                return Err((StatusCode::UNAUTHORIZED, "Invalid API key"));
            }
        }
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
    
    let permissions = api_key.get_permissions();
    
    // API Key 認証を経由した場合も、Biscuit ベースの AuthContext を生成
    let auth_context = match state
        .auth_create_biscuit_from_api_key(api_key.user_id, permissions.clone())
        .await
    {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!(?e, "Failed to create auth context from API key");
            record_fail("context_creation_failed");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Auth context creation failed"));
        }
    };
    
    // 後方互換性のため ApiKeyAuthInfo も格納
    let info = ApiKeyAuthInfo {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        permissions,
    };
    
    #[cfg(feature = "monitoring")]
    gauge!("api_key_rate_limit_tracked_keys")
        .set(usize_to_f64(API_KEY_FAILURE_LIMITER.tracked_len()));
    #[cfg(feature = "monitoring")]
    counter!("api_key_auth_success_total").increment(1);
    debug!(api_key_id=%info.api_key_id, user_id=%info.user_id, perms=?info.permissions, "API key authenticated and converted to Biscuit context");
    
    // 新しい統一認証コンテキストを格納
    req.extensions_mut().insert(auth_context);
    // 後方互換性のために ApiKeyAuthInfo も格納
    req.extensions_mut().insert(info);

    Ok(next.run(req).await)
}

#[inline]
#[allow(clippy::cast_precision_loss)]
#[cfg(feature = "monitoring")]
/// `usize` をメトリクス用に `f64` へ変換します（ゲージ値設定のため）。
const fn usize_to_f64(n: usize) -> f64 {
    n as f64
}

#[cfg(all(feature = "database", feature = "auth"))]
/// 既存データに `lookup_hash` が未設定なレガシーキーのためのフォールバック検索。
///
/// 生キーから直接検出し、成功時には集中化された `AppState` のラッパでバックフィルします。
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
/// database/auth いずれかが無効なビルドでは、フォールバックは常に無効。
async fn legacy_fallback_fetch(
    _state: &crate::AppState,
    _raw: &str,
) -> Option<crate::models::ApiKey> {
    None
}
