//! API Key 認証ミドルウェア (Phase 5.2 - 新AppState対応版)
//!
//! このモジュールはリクエストヘッダ `X-API-Key` による API キー認証を行います。
//!
//! Phase 5.2 での変更点:
//! - 新しい `Arc<AppState>` 構造に対応
//! - State<Arc<AppState>> からの抽出に変更
//! - 簡略化された実装 (APIキーリポジトリが実装されるまでの暫定版)
//!
//! TODO Phase 5.3: APIキーリポジトリの完全実装後に機能を復元
//! - データベースからのAPIキー検証
//! - レート制限の統合
//! - Biscuit トークン生成との統合

use axum::http::StatusCode;
use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::infrastructure::app_state::AppState;

/// 抽出結果を request extensions に格納する型。
///
/// 下流ハンドラは `req.extensions().get::<ApiKeyAuthInfo>()` で参照し、
/// キーの所有者や権限を元にアクセス制御を行えます。
#[derive(Clone, Debug)]
pub struct ApiKeyAuthInfo {
    /// 検証済み API キーの識別子
    pub api_key_id: uuid::Uuid,
    /// キー所有者のユーザー ID
    pub user_id: uuid::Uuid,
    /// キーに付与された権限のリスト(例: "read:posts")
    pub permissions: Vec<String>,
}

/// API キーを受け取る HTTP リクエストヘッダ名
pub const HEADER_NAME: &str = "X-API-Key";

/// API キーを用いた認証レイヤ (Phase 5.2 簡略版)。
///
/// # 現在の実装状態
/// この実装は暫定版です。完全なAPIキーリポジトリが実装されるまで、
/// 基本的な構造のみを提供します。
///
/// # Errors
/// 次の条件でエラーを返します:
/// - 認証ヘッダが欠落/不正な場合
/// - AppState が取得できない場合
///
/// # Returns
/// 成功時は検証済み情報を extensions に格納し、次のミドルウェア/ハンドラへフォワードします。
///
/// # TODO Phase 5.3+
/// - APIキーリポジトリの実装
/// - データベースからの検証
/// - レート制限の統合
/// - Argon2 ハッシュ検証
/// - 有効期限チェック
pub async fn api_key_auth_layer(
    State(_state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    // 1. ヘッダ取得
    let Some(header_val) = req.headers().get(HEADER_NAME) else {
        return Err((StatusCode::UNAUTHORIZED, "API key missing"));
    };

    let Ok(raw_key) = header_val.to_str() else {
        return Err((StatusCode::BAD_REQUEST, "Invalid header"));
    };

    // 2. 基本的な形式チェック
    if raw_key.len() < 10 || !raw_key.starts_with("ak_") {
        return Err((StatusCode::UNAUTHORIZED, "Malformed API key"));
    }

    // 3. TODO Phase 5.3: データベースからAPIキーを検証
    // 現時点では暫定的にダミー情報を格納
    // let api_key_repo = state.api_key_repository()?;
    // let api_key = api_key_repo.find_by_lookup_hash(&lookup_hash).await?;

    debug!("API key authentication requested (暫定実装)");
    warn!("API key authentication is not fully implemented yet. Phase 5.3 で完全実装予定。");

    // 暫定: 開発中はヘッダがあれば通過させる (本番では削除すること)
    // TODO Phase 5.3: 実際の検証ロジックを実装
    let dummy_info = ApiKeyAuthInfo {
        api_key_id: uuid::Uuid::new_v4(),
        user_id: uuid::Uuid::new_v4(),
        permissions: vec!["read:all".to_string()],
    };

    req.extensions_mut().insert(dummy_info);
    Ok(next.run(req).await)
}
