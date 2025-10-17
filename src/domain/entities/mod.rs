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

pub mod user;
pub mod post;
pub mod comment;
pub mod tag;
pub mod category;

// Re-export primary types for convenience
pub use user::{Email, User, UserId, Username};
pub use post::{Post, PostId};
pub use comment::{Comment, CommentId};
pub use tag::{Tag, TagId, TagName};
pub use category::{Category, CategoryId, CategorySlug};

