//! Web Layer (Presentation Layer / 監査推奨)
//!
//! HTTP API の実装を担うレイヤーです。
//! 監査では `web/` という命名を推奨していますが、
//! 実質的には `presentation/` layer と同じ役割です。
//!
//! ## 構成
//! - `handlers/`: HTTP ハンドラ（薄い層、ユースケース呼び出しのみ）
//! - `middleware/`: ミドルウェア（認証、レート制限、ロギング等）
//! - `routes`: ルート定義（後で実装予定）
//!
//! ## 設計原則
//! - ハンドラは薄く保つ（ビジネスロジックはApplicationレイヤーへ）
//! - DTOへの変換はハンドラ内で実施
//! - エラーは `AppError` を通じて統一的にレスポンスに変換

pub mod handlers;
pub mod middleware;

// 将来の拡張ポイント
// pub mod routes;
// pub mod extractors;
// pub mod responses;

// Re-exports for convenience
// Note: Specific re-exports to avoid ambiguous glob conflicts (auth module in both)
pub use handlers::{admin, api_keys, health, metrics, posts, search, users};
pub use middleware::{
    api_key, common, compression, csrf, deprecation, logging, permission, rate_limiting,
    request_id, security,
};

/// Web layer prelude
pub mod prelude {
    // Specific re-exports to avoid ambiguous glob conflicts
    pub use super::handlers::{admin, api_keys, health, metrics, posts, search, users};
    pub use super::middleware::{
        api_key, common, compression, csrf, deprecation, logging, permission, rate_limiting,
        request_id, security,
    };
}
