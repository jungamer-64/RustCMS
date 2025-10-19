// src/infrastructure/database/mod.rs
//! インフラストラクチャレイヤー - データベース層
//!
//! このモジュールは、Diesel ORM を使用したデータベースアクセス実装を提供します。
//!
//! # 責務
//!
//! - **connection**: PostgreSQL接続プール管理（DatabasePool）
//! - **models**: Diesel DBモデル定義（DbUser, DbPost, DbComment, DbCategory, DbTag）
//! - **schema**: 既存レガシースキーマの再エクスポート
//! - **repositories**: Repository Port の Diesel 実装（Phase 3 Week 10完了）
//! - **unit_of_work**: トランザクション管理（Phase 3 Week 11完了）
//!
//! # 設計原則
//!
//! - **依存性の逆転**: Infrastructure層 → Domain層（Ports）
//! - **型安全性**: Diesel の型チェック機構を活用
//! - **スキーマ分離**:既存スキーマと新構造を並行サポート（feature flag）
//!
//! # 構成図
//!
//! ```
//! Connection Pool (connection.rs)
//!         ↓
//!     Models (models.rs)  
//!         ↓  
//! Repositories (repositories/)
//!         ↓
//! Unit of Work (unit_of_work.rs)
//!         ↓
//!    Application Layer
//! ```

// PostgreSQL接続プール
pub mod connection;

// Diesel DBモデル定義
pub mod models;

// 既存スキーマの再エクスポート（レガシー互換）
#[cfg(feature = "database")]
pub use crate::database::schema;

// Phase 3で実装予定のモジュール
#[cfg(feature = "restructure_domain")]
pub mod repositories;

#[cfg(feature = "restructure_domain")]
pub mod unit_of_work;

// モデルのエクスポート
pub use models::{
    DbUser, NewDbUser,
    DbPost, NewDbPost,
    DbComment, NewDbComment,
    DbCategory, NewDbCategory,
    DbTag, NewDbTag,
};

// Phase 3で実装完了したインターフェース
#[cfg(feature = "restructure_domain")]
pub use repositories::{
    DieselUserRepository,
    DieselPostRepository,
    DieselCommentRepository,
};

#[cfg(feature = "restructure_domain")]
pub use unit_of_work::DieselUnitOfWork;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_module_exports() {
        // Models are properly exported
        let _ = std::any::type_name::<DbUser>();
        let _ = std::any::type_name::<DbPost>();
        let _ = std::any::type_name::<DbComment>();
    }
}
