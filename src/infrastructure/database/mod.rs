// src/infrastructure/database/mod.rs
//! インフラストラクチャレイヤー - データベース層
//!
//! このモジュールは、Diesel ORM を使用したデータベースアクセス実装を提供します。
//!
//! # 構成
//!
//! - **schema**: Diesel スキーマ定義（既存 database::schema を使用）
//! - **models**: Diesel スキーマモデル ↔ Domain Entity マッピング ✅
//! - **repositories**: Repository Port の Diesel 実装 ✅ (Phase 3 Week 10: User ✅, Post ✅, Comment ✅)
//! - **unit_of_work**: トランザクション管理 ✅ (Phase 3 Week 11)

// レガシー database モジュールの schema を再エクスポート
#[cfg(feature = "restructure_domain")]
pub use crate::database::schema;

#[cfg(feature = "restructure_domain")]
pub mod models;

#[cfg(feature = "restructure_domain")]
pub mod repositories;

#[cfg(feature = "restructure_domain")]
pub mod unit_of_work;

#[cfg(feature = "restructure_domain")]
pub use models::{DbUser, NewDbUser, DbPost, NewDbPost, DbComment, NewDbComment};

#[cfg(feature = "restructure_domain")]
pub use repositories::{DieselUserRepository, DieselPostRepository, DieselCommentRepository};

#[cfg(feature = "restructure_domain")]
pub use unit_of_work::DieselUnitOfWork;

#[cfg(test)]
mod tests {
    // Phase 3 統合テスト用
}
