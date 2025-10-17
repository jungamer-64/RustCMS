// Legacy use case files
pub mod create_user;
pub mod get_user_by_id;
pub mod update_user;

pub use create_user::CreateUserUseCase;
pub use get_user_by_id::GetUserByIdUseCase;
pub use update_user::UpdateUserUseCase;

// Phase 2+ integration: Consolidated command/query modules
// These re-exports facilitate incremental migration
pub mod user;
pub mod post;
pub mod comment;
pub mod tag;
pub mod category;

// Re-export primary DTOs and commands for convenience
pub use user::{CreateUserRequest, UserDto};
pub use post::{CreatePostRequest, UpdatePostRequest, PostDto};
pub use comment::{CreateCommentRequest, CommentDto};
pub use tag::{CreateTagRequest, TagDto};
pub use category::{CreateCategoryRequest, CategoryDto};
