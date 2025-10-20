//! Authentication Service - Biscuit (統一版)
//!
//! リファクタリング概要 (2025-09):
//! - Key 管理: `PrivateKey` / `PublicKey` から毎回 `KeyPair` を再生成していた非効率を解消し、起動時に確定した `KeyPair` を保持
//! - `remember_me` TTL: 旧仕様 ("2倍 or refresh 以下") の曖昧さを廃止し、通常 = `config.access_token_ttl_secs` / `remember_me` = 24h 固定
//! - セッションストレージ: `HashMap` 直使用を排除し `SessionStore` trait + `InMemorySessionStore` 抽象化 (将来 Redis/Postgres 差替え容易化)
//! - Refresh Token 並行リクエスト: ポリシーを明示 (旧トークン即失効)。version ミスマッチは `InvalidToken`
//! - テスト容易性: セッション操作は trait 経由。全削除は `#[cfg(test)]` のみ公開。
//!
//! 既存の機能説明は下記オリジナルコメントを継承。

//! 目的: 既存の混在実装を廃止し、現行は JWT ベースの認証/認可に統一する。
//!
//! 提供機能:
//! - JWT アクセス/リフレッシュトークン対
//! - `WebAuthn` (未改変・今後拡張用プレースホルダ)
//! - Argon2 パスワード検証
//! - RBAC (role -> permissions マッピング)
//!
//! トークン仕様 (更新後):
//! - `access token`: 有効期限 `config.access_token_ttl_secs` (`remember_me=false`) / 24h 固定 (`remember_me`=true)
//! - `refresh token`: 有効期限 30d (設定値) / 使用時に `session_version` +1 し旧 refresh トークン即失効
//! - Refresh 使用時: version インクリメント -> 旧トークンは version ミスマッチで無効化 (並行リクエスト対策)
//! - セッション状態: `SessionStore` 抽象 (現状 `InMemory`)。

pub mod error;
pub mod jwt; // Phase 5.3: JWT 認証サービス（EdDSA版にリファクタ）
pub mod password_service; // パスワード検証サービス（新規追加）
mod service;
pub mod session;
pub mod unified_context; // Phase 5.3: JWT + Biscuit 統合コンテキスト
pub mod unified_key_management; // 統合Ed25519鍵管理（JWT + Biscuit共通）

pub use error::AuthError;
pub use jwt::{JwtClaims, JwtConfig, JwtService, JwtTokenPair, TokenType};
pub use password_service::PasswordService;
pub use service::{AuthContext, AuthService, LoginRequest};
pub use session::{InMemorySessionStore, SessionData, SessionStore};
pub use unified_context::UnifiedAuthContext;
pub use unified_key_management::{KeyLoadConfig, UnifiedKeyPair};

#[cfg(feature = "restructure_domain")]
use domain::user::UserRole;

#[cfg(not(feature = "restructure_domain"))]
use crate::models::UserRole;

#[inline]
pub fn require_admin_permission(ctx: &AuthContext) -> crate::Result<()> {
    if matches!(ctx.role, UserRole::Admin) || ctx.permissions.iter().any(|p| p == "admin") {
        Ok(())
    } else {
        Err(AuthError::InsufficientPermissions {
            required: "admin".to_string(),
        }
        .into())
    }
}
