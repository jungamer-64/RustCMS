//! アプリケーション層 (Application Layer)
//!
//! ユースケースの実装とポート定義を担うレイヤーです。
//! - Use Cases: ビジネスユースケースの実装
//! - Ports: インターフェース定義（Repository, Service等）
//! - DTOs: Data Transfer Objects
//! - Commands/Queries: CQRS パターン

// ============================================================================
// Phase 3: 新しいアプリケーション層構造（監査済み）
// ============================================================================

pub mod dto;
pub mod ports;

// Phase 3 Step 7-8: CQRS統合（Commands + Queries + DTOs）
#[cfg(feature = "restructure_application")]
pub mod user;

#[cfg(feature = "restructure_application")]
pub mod post;

#[cfg(feature = "restructure_application")]
pub mod comment;

#[cfg(feature = "restructure_application")]
pub mod tag;

#[cfg(feature = "restructure_application")]
pub mod category;

// ============================================================================
// レガシー構造（既存コードとの並行稼働）
// ============================================================================

pub use ports::*;

// Re-export surface for application-layer services, handlers and use-cases
// This file intentionally re-exports existing handlers and services so callers
// can start referring to `crate::application::...` during the restructure.

pub mod handlers {
    pub use crate::handlers::*;
}

pub mod use_cases;
pub use use_cases::*;

pub mod services {
    // Re-exports for service-like modules (eg: limiter, auth glue) can go here.
}

pub use handlers::*;

/// Application prelude: commonly used handler/service types for migrating
/// call sites to `crate::application`.
pub mod prelude {
    pub use super::handlers::*;
}
