//! Common Layer (監査推奨: `shared` → `common`)
//!
//! レイヤー横断で使用される共通機能です。
//! - types: 共通型定義（Result型、エラー型階層、DTOs、Pagination等）
//! - helpers: 純粋関数ユーティリティ（date, hash, text, URL等）
//! - security: セキュリティ関連ヘルパー（password, validation）
//! - validation: バリデーション関数
//! - telemetry: 監視・ロギング（Phase 2+で実装予定）
//!
//! ## 注意
//! このモジュールは `shared` から `common` に改名されました（Rustの慣習）。

// Phase 1 で実装完了: エラー型階層
pub mod error_types;

// Phase 2 で実装完了: Utility type modules (copied from src/shared/)
// NOTE: types/ subdirectory has its own mod.rs for nested organization
// types/ は API types, DTOs, Pagination 等のユーティリティ型を含む

// types サブディレクトリは独立したモジュールとして宣言
pub mod type_utils;

// Helpers, security, validation
pub mod helpers;
pub mod security;
pub mod validation;

// Convenience re-exports from error_types (PRIMARY error types)
pub use error_types::{
    AppError, ApplicationError, ApplicationResult, DomainError, DomainResult, InfrastructureError,
    InfrastructureResult, Result,
};

// types モジュール用のエイリアス (backward compatibility)
pub mod types {
    pub use super::error_types::*;
}

// Prelude for ease of use
pub mod prelude {
    pub use super::error_types::*;
    pub use super::helpers::*;
    pub use super::security::*;
    pub use super::validation::*;
}

// Phase 2+ で実装予定
// pub mod telemetry;
