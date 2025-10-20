// src/application/queries/mod.rs
/// CQRS Queries - Read-only query objects for data retrieval
///
/// This module implements the Query side of CQRS (Command Query Responsibility Segregation).
/// Queries are optimized for reading data and do not modify state.
///
/// **Design Principles**:
/// - Queries are read-only (no state mutations)
/// - Independent from Commands (separate models)
/// - Can bypass domain layer for performance (direct DB access)
/// - Return DTOs, not domain entities (when appropriate)
///
/// **Architecture**:
/// ```text
/// Query Request → Query Handler → Repository → Database
///                                            ↓
///                                         DTO Response
/// ```

// Re-export query modules
pub mod pagination;

pub mod user_queries;

pub mod post_queries;

// Re-export common types
pub use pagination::{PaginationParams, PaginationResult};

pub use user_queries::{ListUsersQuery, UserFilter, UserSortField};

pub use post_queries::{ListPostsQuery, PostFilter, PostSortField, SearchPostsQuery};
