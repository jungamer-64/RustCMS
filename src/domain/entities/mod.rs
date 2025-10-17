//! Domain Entities
//!
//! Entity + Value Objects 統合パターン
//!
//! このモジュールには以下が含まれます：
//! - User Entity + Value Objects (UserId, Email, Username)
//! - Post Entity + Value Objects
//! - Comment Entity + Value Objects
//! - Tag Entity + Value Objects
//! - Category Entity + Value Objects

pub mod category;
pub mod comment;
pub mod post;
pub mod tag;
pub mod user;

// Re-export primary types for convenience
pub use category::{Category, CategoryId, CategorySlug};
pub use comment::{Comment, CommentId};
pub use post::{Post, PostId};
pub use tag::{Tag, TagId, TagName};
pub use user::{Email, User, UserId, Username};
