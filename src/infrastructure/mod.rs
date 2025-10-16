//! インフラストラクチャ層 (Infrastructure Layer)
//!
//! 技術的実装の詳細を担うレイヤーです。
//! - Database: DB実装（Diesel）
//! - Cache: キャッシュ実装（Redis）
//! - Search: 検索実装（Tantivy）
//! - Auth: 認証実装（Biscuit, WebAuthn）
//! - Events: イベントバス実装

// ============================================================================
// Phase 3: 新しいインフラストラクチャ層構造（監査済み）
// ============================================================================

// Phase 3 で実装予定
// #[cfg(all(feature = "restructure_application", feature = "database"))]
// pub mod database;  // DB実装統合（models.rs, repositories.rs）
// #[cfg(all(feature = "restructure_application", feature = "cache"))]
// pub mod cache;     // Cache実装統合
// #[cfg(all(feature = "restructure_application", feature = "search"))]
// pub mod search;    // Search実装統合
// #[cfg(all(feature = "restructure_application", feature = "auth"))]
// pub mod auth;      // Auth実装統合
// #[cfg(feature = "restructure_application")]
// pub mod events;    // Event Bus実装

// ============================================================================
// レガシー構造（既存コードとの並行稼働）
// ============================================================================

// Re-export surface for infrastructure implementations (database, cache, search, repositories)
// This file exposes concrete implementations while keeping original paths intact.

// Database adapter re-exports are feature gated because `crate::database` is
// only compiled when the `database` feature is enabled.
#[cfg(feature = "database")]
pub mod database {
    pub use crate::database::*;
}

// Cache adapter re-exports are feature gated because `crate::cache` is
// only compiled when the `cache` feature is enabled.
#[cfg(feature = "cache")]
pub mod cache {
    pub use crate::cache::*;
}

// Search adapter
#[cfg(feature = "search")]
pub mod search {
    pub use crate::search::*;
}

// Auth-related infrastructure
#[cfg(feature = "auth")]
pub mod auth {
    pub use crate::auth::*;
}

// Repositories are defined unconditionally but may themselves be feature-gated
// internally. Re-export them so callers can refer to `crate::infrastructure::repositories`.
pub mod repositories;

// Re-export the gated modules at the top level where appropriate.
#[cfg(feature = "database")]
pub use database::*;

#[cfg(feature = "cache")]
pub use cache::*;

#[cfg(feature = "search")]
pub use search::*;

#[cfg(feature = "auth")]
pub use auth::*;

pub use repositories::*;
