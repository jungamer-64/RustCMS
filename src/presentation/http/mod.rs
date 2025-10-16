//! HTTP Handlers（Phase 4 Step 1）
//!
//! 薄いハンドラー実装: HTTP リクエスト/レスポンス変換のみ
//! ビジネスロジックはアプリケーション層に委譲
//!
//! # 設計パターン
//! - Request → DTO deserialize
//! - Application Layer (Command/Query) execute
//! - DTO → Response serialize
//!
//! 参考: RESTRUCTURE_EXAMPLES.md, Handler の実装例

pub mod adapters;
pub mod handlers;
pub mod middleware;
pub mod responses;
pub mod router;

// Re-export handlers for easy access
#[cfg(feature = "restructure_application")]
pub use handlers::*;
