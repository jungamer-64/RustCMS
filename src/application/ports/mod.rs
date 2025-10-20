// src/application/ports/mod.rs
//! Application Ports (ポート定義) - 監査推奨構造
//!
//! Port/Adapter パターンのPort定義を集約します。
//! - repositories: データアクセス層のインターフェース（UserRepository, PostRepository等）
//! - cache: キャッシュサービスのインターフェース (CacheService)
//! - search: 検索サービスのインターフェース (SearchService)
//! - events: イベント発行サービスのインターフェース (EventPublisher)
//!
//! Ports define the interfaces that use-cases depend on,
//! allowing for loose coupling and testability.

// Legacy repository definitions (Phase 3-4 で統合予定)
pub mod post_repository;
pub mod user_repository;

pub use post_repository::PostRepository;
pub use user_repository::UserRepository;

// New unified repositories (Phase 1-2 で実装完了)
pub mod repositories;
pub use repositories::*;

// Phase 2: Service ports (監査推奨)
pub mod cache;
pub use cache::{CacheError, CacheService};

// Phase 9: search module removed (legacy code deleted in Phase 7)
// pub mod search;
// pub use search::{
//     AdvancedQuery, FilterOperator, SearchDocument, SearchError, SearchFilter, SearchHit,
//     SearchResults, SearchService,
// };

pub mod events;
pub use events::{DomainEvent, EventError, EventMetadata, EventPublisher};
