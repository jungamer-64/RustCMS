// src/application/mod.rs
//! アプリケーション層 (Application Layer) - 監査済み構造
//!
//! Commands + Queries + DTOs を統合した CQRS パターンを採用します。
//!
//! ## 構造（監査推奨）
//! - **user.rs**: User CQRS統合（Commands + Queries + DTOs）
//! - **post.rs**: Post CQRS統合（Commands + Queries + DTOs）
//! - **comment.rs**: Comment CQRS統合（Commands + Queries + DTOs）
//! - **category.rs**: Category CQRS統合（Commands + Queries + DTOs）
//! - **dto/**: 共通DTOモジュール（pagination等）
//! - **ports/**: インターフェース定義（Repository, Service等）
//!
//! ## 設計原則
//! - Entity + Value Objects 統合パターン（domain層）
//! - Commands + Queries + DTOs 統合パターン（application層）
//! - 500行未満は単一ファイル推奨
//! - Repository Port への依存性注入

// ============================================================================
// Phase 3 完成版: CQRS統合構造（監査済み）
// ============================================================================

pub mod common;

/// DTOs - Data Transfer Objects（共通モジュール）
pub mod dto;
pub use dto::*;

/// Ports - インターフェース定義（Repository, Service等）
pub mod ports;
pub use ports::{cache, events, repositories, CacheService, EventPublisher};

// ============================================================================
// CQRS統合モジュール（Commands + Queries + DTOs）
// ============================================================================

/// User CQRS統合（Commands + Queries + DTOs）
pub mod user;

/// Post CQRS統合（Commands + Queries + DTOs）
pub mod post;

/// Comment CQRS統合（Commands + Queries + DTOs）
pub mod comment;

/// Category CQRS統合（Commands + Queries + DTOs）
pub mod category;

/// Queries（CQRSクエリ層）
pub mod queries;

pub mod use_cases;
pub use use_cases::*;

pub mod services {
    // Re-exports for service-like modules (eg: limiter, auth glue) can go here.
}
