//! ドメイン層 (Domain Layer)
//!
//! ビジネスロジックの中核を担うレイヤーです。
//! - Entity: ビジネスオブジェクトと不変条件
//! - Value Objects: 検証済みの値型
//! - Domain Services: 複数エンティティにまたがるロジック
//! - Domain Events: ドメインイベント定義

// ============================================================================
// Phase 1-2: 新しいドメイン層構造（監査済み）
// ============================================================================

#[cfg(feature = "restructure_domain")]
pub mod services;

// Phase 1-2: 新しいドメインモデル
#[cfg(feature = "restructure_domain")]
pub mod user; // Entity + Value Objects 統合

// Phase 2 で実装予定
#[cfg(feature = "restructure_domain")]
pub mod post; // Entity + Value Objects 統合

#[cfg(feature = "restructure_domain")]
pub mod comment; // Entity + Value Objects 統合

#[cfg(feature = "restructure_domain")]
pub mod tag; // Entity + Value Objects 統合

#[cfg(feature = "restructure_domain")]
pub mod category; // Entity + Value Objects 統合

// Phase 2 拡張: Domain Events
#[cfg(feature = "restructure_domain")]
pub mod events; // Domain Events

// ============================================================================
// レガシー構造（既存コードとの並行稼働）
// ============================================================================

#[cfg(feature = "database")]
pub mod models {
    pub use crate::models::*;
}

#[cfg(not(feature = "database"))]
pub mod models {
    // Database feature is disabled: provide an empty placeholder so callers
    // can still refer to `crate::domain::models` without causing build errors.
}

pub mod value_objects;

// Re-export common domain types

/// Domain prelude: common types that application code may import during
/// the incremental migration to the domain layer.
pub mod prelude {
    pub use super::value_objects::*;
}
