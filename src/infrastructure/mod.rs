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

// Phase 3 実装中: restructure_domain フラグで新 database 層を有効化
#[cfg(all(feature = "restructure_domain", feature = "database"))]
pub mod database;

// Phase 3-4 で実装完了
pub mod events; // Event Bus実装 (src/events.rs, src/listeners.rs から移行)

// Phase 3 で実装予定
// #[cfg(all(feature = "restructure_application", feature = "cache"))]
// pub mod cache;     // Cache実装統合
// #[cfg(all(feature = "restructure_application", feature = "search"))]
// pub mod search;    // Search実装統合
// #[cfg(all(feature = "restructure_application", feature = "auth"))]
// pub mod auth;      // Auth実装統合

// ============================================================================
// レガシー構造（既存コードとの並行稼働）
// ============================================================================

// Re-export surface for infrastructure implementations (database, cache, search, repositories)
// This file exposes concrete implementations while keeping original paths intact.

// Database adapter re-exports are feature gated because `crate::database` is
// only compiled when the `database` feature is enabled.
// Note: Phase 3 new database impl (restructure_domain) takes precedence over legacy
#[cfg(all(not(feature = "restructure_domain"), feature = "database"))]
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

// Phase 3: DieselRepositories は database から直接インポート（ambiguous re-export を回避）
#[cfg(all(feature = "restructure_domain", feature = "database"))]
pub use database::{DieselUserRepository, DieselPostRepository, DieselCommentRepository, DieselUnitOfWork};

pub use repositories::*;
