//! Shared Core Library
//!
//! このクレートはRustCMSの全レイヤーで共有される基礎的な型、エラー、ヘルパー関数を提供します。
//! 他のクレートへの依存を最小限に抑え、循環依存を避けるように設計されています。
//!
//! # モジュール構成
//!
//! - `error`: 共通エラー型とResult型
//! - `helpers`: 純粋関数ユーティリティ (date, hash, text, url等)
//! - `security`: セキュリティ関連ヘルパー (password, validation)
//! - `types`: 共通型定義 (API types, DTOs, Pagination等)
//! - `validation`: 入力検証ユーティリティ
//!
//! # Features
//!
//! - `password`: パスワードハッシュ機能 (argon2, bcrypt)
//! - `encoding`: Base64エンコーディング
//! - `web`: Web API関連の型 (axum, http)

/// Common error types and Result aliases
pub mod error;

/// Helper utilities
pub mod helpers;

/// Security utilities
pub mod security;

/// Common type definitions
pub mod types;

/// Validation utilities
pub mod validation;

pub use types::common_types::SessionId;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::error::*;
    pub use crate::helpers::*;
    pub use crate::security::*;
    pub use crate::types::*;
    pub use crate::validation::*;
}
