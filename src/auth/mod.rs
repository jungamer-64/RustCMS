//! Authentication Service - Biscuit (統一版)
//!
//! リファクタリング概要 (2025-09):
//! - Key 管理: `PrivateKey` / `PublicKey` から毎回 `KeyPair` を再生成していた非効率を解消し、起動時に確定した `KeyPair` を保持
//! - `remember_me` TTL: 旧仕様 ("2倍 or refresh 以下") の曖昧さを廃止し、通常 = `config.access_token_ttl_secs` / remember_me = 24h 固定
//! - セッションストレージ: `HashMap` 直使用を排除し `SessionStore` trait + `InMemorySessionStore` 抽象化 (将来 Redis/Postgres 差替え容易化)
//! - Refresh Token 並行リクエスト: ポリシーを明示 (旧トークン即失効)。version ミスマッチは `InvalidToken`
//! - 有効期限検証: Biscuit の `exp` fact を parse 時に必ず検証 (期限切れは `TokenExpired`)
//! - テスト容易性: セッション操作は trait 経由。全削除は `#[cfg(test)]` のみ公開。
//!
//! 既存の機能説明は下記オリジナルコメントを継承。

//! 目的: 既存の JWT / Biscuit 併用実装を廃止し、Biscuit トークンのみで
//! アクセス/リフレッシュ (スライディングセッション) を提供する。
//!
//! 提供機能:
//! - Biscuit 署名トークン (access / refresh の2種類)
//! - `WebAuthn` (未改変・今後拡張用プレースホルダ)
//! - Argon2 パスワード検証
//! - RBAC (role -> permissions マッピング)
//!
//! トークン仕様 (更新後):
//! - `access biscuit`: 有効期限 `config.access_token_ttl_secs` (`remember_me=false`) / 24h 固定 (`remember_me`=true)
//! - `refresh biscuit`: 有効期限 30d (設定値) / 使用時に `refresh_version` +1 し旧 refresh トークン即失効
//! - Biscuit 内 facts:
//! ```text
//! user("<uuid>", "<username>", "<role>");
//! token_type("access"|"refresh");
//! exp(<unix_ts>);          // 失効時刻 (秒)
//! session("<session_id>", <version>);
//! ```
//! - refresh 使用時: version インクリメント -> 旧トークンは version ミスマッチで無効化 (並行リクエスト対策)
//! - セッション状態: `SessionStore` 抽象 (現状 `InMemory`)。

pub mod error;
pub mod key_management;
pub mod session;
mod biscuit;
mod service;

pub use error::AuthError;
pub use session::{InMemorySessionStore, SessionData, SessionStore};
pub use service::{AuthContext, AuthResponse, AuthService, LoginRequest};

use crate::utils::common_types::SessionId;

#[inline]
fn mask_session_id(sid: &SessionId) -> String {
    let s = sid.as_ref();
    if s.len() <= 6 {
        return "***".to_string();
    }
    format!("{}…{}", &s[..3], &s[s.len() - 3..])
}
