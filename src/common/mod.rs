//! 共有ユーティリティ (Common Utilities)
//!
//! レイヤー横断で使用される共通機能です。
//! - types: 共通型定義（Result型、エラー型階層）
//! - telemetry: 監視・ロギング
//! - utils: 純粋関数ユーティリティ
//!
//! ## 注意
//! このモジュールは `shared` から `common` に改名されました（Rustの慣習）。

// Phase 1 で実装完了
pub mod types;
pub use types::{
    AppError, ApplicationError, ApplicationResult, DomainError, DomainResult, InfrastructureError,
    InfrastructureResult, Result,
};

// Phase 2 で実装予定
// pub mod telemetry;
// pub mod utils;
