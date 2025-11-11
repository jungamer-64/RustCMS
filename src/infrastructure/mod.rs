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

// Phase 5: AppState 新実装（DDD準拠、レガシーapp.rs削除に伴う）
pub mod app_state;

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
// Phase 9: crate::database removed in Phase 7
// Database infrastructure is now in infrastructure::database
// pub mod database {
//     pub use crate::database::*;
// }

// Phase 9: crate::cache removed in Phase 7
// Cache implementation needs to be moved to infrastructure::cache
// #[cfg(feature = "cache")]
// pub mod cache {
//     pub use crate::cache::*;
// }

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

// Phase 9: Legacy re-exports removed (database/cache/search modules deleted in Phase 7)
// Re-export the gated modules at the top level where appropriate.
// #[cfg(feature = "database")]
// pub use database::*;

// #[cfg(feature = "cache")]
// pub use cache::*;

// #[cfg(feature = "search")]
// pub use search::*;

#[cfg(feature = "auth")]
pub use auth::*;

// Phase 9: New Repository implementations (audited structure)
#[cfg(all(feature = "restructure_domain", feature = "database"))]
pub use database::{DieselCommentRepository, DieselPostRepository, DieselUserRepository};

#[cfg(all(feature = "restructure_domain", feature = "database"))]
pub use database::DieselUnitOfWork;
