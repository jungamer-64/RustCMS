//! HTTP Handlers（Phase 4 Step 1）
//!
//! Phase 10: レガシーhandlers削除により一時無効化
//! Phase 4で新handlers実装時に再有効化予定
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
// Phase 10: handlers削除により一時コメントアウト
// pub mod handlers;
pub mod middleware;
pub mod responses;
pub mod router;

// Re-export handlers for easy access
// Phase 10: handlers削除により一時コメントアウト
// #[cfg(feature = "restructure_application")]
// pub use handlers::*;
