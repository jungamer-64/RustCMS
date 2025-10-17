// Legacy use case files (Phase 5で削除予定 - 一時的にコメントアウト)
// pub mod create_user;
// pub mod get_user_by_id;
// pub mod update_user;

// pub use create_user::CreateUserUseCase as LegacyCreateUserUseCase;
// pub use get_user_by_id::GetUserByIdUseCase as LegacyGetUserByIdUseCase;
// pub use update_user::UpdateUserUseCase as LegacyUpdateUserUseCase;

// Phase 2+ integration: Consolidated command/query modules (Legacy)
// These re-exports facilitate incremental migration
pub mod category;
// pub mod comment_legacy; // Phase 5で削除予定（ファイルが存在しないためコメントアウト）
// pub mod post_legacy; // Phase 5で削除予定（DTOのみ、Use Case実装なし）
pub mod tag;

// Phase 3: New Use Cases structure (user/, post/, comment/ directories)
pub mod comment; // Phase 3 Week 8-9: New Comment Use Cases
pub mod post;
pub mod user;

// Phase 3 Week 11: Unit of Work 使用例
#[cfg(feature = "restructure_domain")]
pub mod examples_unit_of_work;

// Re-export primary DTOs and commands for convenience
pub use category::{CategoryDto, CreateCategoryRequest};
// pub use comment_legacy::{CommentDto as LegacyCommentDto, CreateCommentRequest as LegacyCreateCommentRequest};
// pub use post_legacy::{CreatePostRequest as LegacyCreatePostRequest, PostDto as LegacyPostDto, UpdatePostRequest as LegacyUpdatePostRequest};
pub use tag::{CreateTagRequest, TagDto};
