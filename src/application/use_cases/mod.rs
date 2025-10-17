// Legacy use case files
pub mod create_user;
pub mod get_user_by_id;
pub mod update_user;

pub use create_user::CreateUserUseCase;
pub use get_user_by_id::GetUserByIdUseCase;
pub use update_user::UpdateUserUseCase;

// Phase 2+ integration: Consolidated command/query modules
// These re-exports facilitate incremental migration
pub mod category;
pub mod comment;
pub mod post;
pub mod tag;
pub mod user;

// Re-export primary DTOs and commands for convenience
pub use category::{CategoryDto, CreateCategoryRequest};
pub use comment::{CommentDto, CreateCommentRequest};
pub use post::{CreatePostRequest, PostDto, UpdatePostRequest};
pub use tag::{CreateTagRequest, TagDto};
pub use user::{CreateUserRequest, UserDto};
