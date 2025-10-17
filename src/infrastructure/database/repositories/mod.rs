// src/infrastructure/database/repositories/mod.rs
//! Diesel Repository 実装
//!
//! このモジュールは、Domain層の Repository Port を Diesel ORM で実装します。
//! 各リポジトリは対応する Domain Entity の永続化を担当します。
//!
//! # アーキテクチャ
//!
//! - **DieselUserRepository**: User エンティティの CRUD 操作 ✅ (Phase 3 Week 10 完成)
//! - **DieselPostRepository**: Post エンティティの CRUD 操作 ✅ (Phase 3 Week 10 完成)
//! - **DieselCommentRepository**: Comment エンティティの CRUD 操作 (Phase 3 Week 10 予定)
//! - **DieselTagRepository**: Tag エンティティの CRUD 操作
//! - **DieselCategoryRepository**: Category エンティティの CRUD 操作
//!
//! # Feature ゲート
//!
//! すべての Repository 実装は `restructure_domain` feature の下に置かれます。
//! Phase 3 実装時に有効になります。

// Phase 3 Week 10: User Repository 実装完了
#[cfg(feature = "restructure_domain")]
pub mod user_repository;

// Phase 3 Week 10: Post Repository 実装完了
#[cfg(feature = "restructure_domain")]
pub mod post_repository;

// Phase 3 Week 10: Comment Repository 実装完了
#[cfg(feature = "restructure_domain")]
pub mod comment_repository;

#[cfg(feature = "restructure_domain")]
pub use user_repository::DieselUserRepository;

#[cfg(feature = "restructure_domain")]
pub use post_repository::DieselPostRepository;

#[cfg(feature = "restructure_domain")]
pub use comment_repository::DieselCommentRepository;

// Phase 3 Week 10: Comment Repository (予定)
// #[cfg(feature = "restructure_domain")]
// pub mod comment_repository;

#[cfg(test)]
mod tests {
    // Phase 3 統合テスト用のスナップショット（testcontainers 使用）
    // 実装は Phase 3 以降
}
