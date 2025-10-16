// src/infrastructure/database/repositories/mod.rs
//! Diesel Repository 実装
//!
//! このモジュールは、Domain層の Repository Port を Diesel ORM で実装します。
//! 各リポジトリは対応する Domain Entity の永続化を担当します。
//!
//! # アーキテクチャ
//!
//! - **DieselUserRepository**: User エンティティの CRUD 操作
//! - **DieselPostRepository**: Post エンティティの CRUD 操作
//! - **DieselCommentRepository**: Comment エンティティの CRUD 操作
//! - **DieselTagRepository**: Tag エンティティの CRUD 操作
//! - **DieselCategoryRepository**: Category エンティティの CRUD 操作
//!
//! # Feature ゲート
//!
//! すべての Repository 実装は `restructure_application` feature の下に置かれます。
//! Phase 3 実装時のみ有効になります。

#[cfg(feature = "restructure_application")]
pub mod diesel_user_repository;

#[cfg(feature = "restructure_application")]
pub mod diesel_post_repository;

#[cfg(feature = "restructure_application")]
pub mod diesel_comment_repository;

#[cfg(feature = "restructure_application")]
pub mod diesel_tag_repository;

#[cfg(feature = "restructure_application")]
pub mod diesel_category_repository;

#[cfg(feature = "restructure_application")]
pub use diesel_user_repository::DieselUserRepository;

#[cfg(feature = "restructure_application")]
pub use diesel_post_repository::DieselPostRepository;

#[cfg(feature = "restructure_application")]
pub use diesel_comment_repository::DieselCommentRepository;

#[cfg(feature = "restructure_application")]
pub use diesel_tag_repository::DieselTagRepository;

#[cfg(feature = "restructure_application")]
pub use diesel_category_repository::DieselCategoryRepository;

#[cfg(test)]
mod tests {
    // Phase 3 統合テスト用のスナップショット（testcontainers 使用）
    // 実装は Phase 3 以降
}
