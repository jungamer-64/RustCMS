// src/infrastructure/database/mod.rs
//! インフラストラクチャレイヤー - データベース層
//!
//! このモジュールは、Diesel ORM を使用したデータベースアクセス実装を提供します。
//!
//! # 構成
//!
//! - **models**: Diesel スキーマモデル ↔ Domain Entity マッピング
//! - **repositories**: Repository Port の Diesel 実装

#[cfg(feature = "restructure_application")]
pub mod models;

#[cfg(feature = "restructure_application")]
pub mod repositories;

#[cfg(feature = "restructure_application")]
pub mod unit_of_work;

#[cfg(feature = "restructure_application")]
pub use models::{DbCategory, DbComment, DbPost, DbTag, DbUser};

#[cfg(feature = "restructure_application")]
pub use repositories::DieselUserRepository;

#[cfg(feature = "restructure_application")]
pub use unit_of_work::UnitOfWork;

#[cfg(test)]
mod tests {
    // Phase 3 統合テスト用
}
