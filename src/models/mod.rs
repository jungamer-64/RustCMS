#[cfg(feature = "database")]
pub mod api_key;
pub mod pagination;
#[cfg(feature = "database")]
pub mod post;
#[cfg(feature = "database")]
pub mod user;

#[cfg(feature = "database")]
pub use api_key::*;
#[cfg(feature = "database")]
pub use post::*;
#[cfg(feature = "database")]
pub use user::*;

// Minimal placeholder implementations for when the `database` feature is
// disabled. These types provide just enough shape for the rest of the codebase
// to compile (APIs remain stubbed / NotImplemented at runtime).
#[cfg(not(feature = "database"))]
pub mod api_key {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
    pub struct ApiKey {
        pub id: Uuid,
        pub user_id: Uuid,
        pub name: String,
        pub permissions: Vec<String>,
        pub revoked: bool,
        pub api_key_lookup_hash: String,
    }

    impl ApiKey {
        pub const ALLOWED_PERMISSIONS: &'static [&'static str] = &[
            "posts:read",
            "posts:write",
            "users:read",
            "users:write",
            "search:reindex",
        ];

        pub fn mask_raw(_raw: &str) -> String {
            "<masked>".to_string()
        }

        pub fn to_response(&self) -> ApiKeyResponse {
            ApiKeyResponse {
                id: self.id,
                name: self.name.clone(),
                permissions: self.permissions.clone(),
                revoked: self.revoked,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
    pub struct ApiKeyResponse {
        pub id: Uuid,
        pub name: String,
        pub permissions: Vec<String>,
        pub revoked: bool,
    }
}

#[cfg(not(feature = "database"))]
pub use api_key::*;

#[cfg(not(feature = "database"))]
pub mod post {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub enum PostStatus {
        Draft,
        Published,
        Archived,
    }

    impl PostStatus {
        pub fn parse_str(s: &str) -> Result<Self, crate::AppError> {
            match s {
                "draft" => Ok(Self::Draft),
                "published" => Ok(Self::Published),
                "archived" => Ok(Self::Archived),
                _ => Err(crate::AppError::BadRequest(format!(
                    "Invalid post status: {s}"
                ))),
            }
        }
    }

    impl Default for PostStatus {
        fn default() -> Self {
            Self::Draft
        }
    }

    impl std::fmt::Display for PostStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Draft => write!(f, "draft"),
                Self::Published => write!(f, "published"),
                Self::Archived => write!(f, "archived"),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct Post {
        pub id: Uuid,
        pub title: String,
        pub slug: String,
        pub content: String,
        pub excerpt: Option<String>,
        pub author_id: Uuid,
        pub status: String,
        pub featured_image_id: Option<Uuid>,
        pub tags: Vec<String>,
        pub categories: Vec<String>,
        pub meta_title: Option<String>,
        pub meta_description: Option<String>,
        pub published_at: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
    pub struct CreatePostRequest {
        pub title: String,
        pub content: String,
        pub excerpt: Option<String>,
        pub slug: Option<String>,
        pub published: Option<bool>,
        pub tags: Option<Vec<String>>,
        pub category: Option<String>,
        pub featured_image: Option<String>,
        pub meta_title: Option<String>,
        pub meta_description: Option<String>,
        pub published_at: Option<DateTime<Utc>>,
        pub status: Option<PostStatus>,
    }

    // `Default` is derived for `CreatePostRequest` above; no manual impl needed.

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
    pub struct UpdatePostRequest {
        pub title: Option<String>,
        pub content: Option<String>,
        pub excerpt: Option<String>,
        pub slug: Option<String>,
        pub published: Option<bool>,
        pub tags: Option<Vec<String>>,
        pub category: Option<String>,
        pub featured_image: Option<String>,
        pub meta_title: Option<String>,
        pub meta_description: Option<String>,
        pub published_at: Option<DateTime<Utc>>,
        pub status: Option<PostStatus>,
    }

    impl UpdatePostRequest {
        pub fn empty() -> Self {
            Self {
                title: None,
                content: None,
                excerpt: None,
                slug: None,
                published: None,
                tags: None,
                category: None,
                featured_image: None,
                meta_title: None,
                meta_description: None,
                published_at: None,
                status: None,
            }
        }

        pub fn publish_now(mut self) -> Self {
            self.published_at = Some(Utc::now());
            self.status = Some(PostStatus::Published);
            self
        }
    }
}

#[cfg(not(feature = "database"))]
pub use post::*;

#[cfg(not(feature = "database"))]
pub mod user {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct User {
        pub id: Uuid,
        pub username: String,
        pub email: String,
        pub first_name: Option<String>,
        pub last_name: Option<String>,
        pub role: String,
        pub is_active: bool,
        pub password_hash: Option<String>,
        pub email_verified: bool,
        pub last_login: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct CreateUserRequest {
        pub username: String,
        pub email: String,
        pub password: String,
        pub first_name: Option<String>,
        pub last_name: Option<String>,
        pub role: crate::models::UserRole,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
    pub struct UpdateUserRequest {
        pub username: Option<String>,
        pub email: Option<String>,
        pub first_name: Option<String>,
        pub last_name: Option<String>,
        pub role: Option<crate::models::UserRole>,
        pub is_active: Option<bool>,
    }

    impl UpdateUserRequest {
        pub fn empty() -> Self {
            Self {
                username: None,
                email: None,
                first_name: None,
                last_name: None,
                role: None,
                is_active: None,
            }
        }

        pub fn deactivate() -> Self {
            Self {
                is_active: Some(false),
                ..Self::empty()
            }
        }

        pub fn with_role(role: crate::models::UserRole) -> Self {
            Self {
                role: Some(role),
                ..Self::empty()
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
    pub enum UserRole {
        SuperAdmin,
        Admin,
        Editor,
        Author,
        Contributor,
        Subscriber,
    }

    impl UserRole {
        pub fn as_str(&self) -> &'static str {
            match self {
                UserRole::SuperAdmin => "super_admin",
                UserRole::Admin => "admin",
                UserRole::Editor => "editor",
                UserRole::Author => "author",
                UserRole::Contributor => "contributor",
                UserRole::Subscriber => "subscriber",
            }
        }

        /// Parse a role string into a `UserRole`.
        ///
        /// Returns an `AppError::BadRequest` when the input is not a known role.
        pub fn parse_str(s: &str) -> Result<Self, crate::AppError> {
            match s {
                "super_admin" => Ok(UserRole::SuperAdmin),
                "admin" => Ok(UserRole::Admin),
                "editor" => Ok(UserRole::Editor),
                "author" => Ok(UserRole::Author),
                "contributor" => Ok(UserRole::Contributor),
                "subscriber" => Ok(UserRole::Subscriber),
                _ => Err(crate::AppError::BadRequest(format!(
                    "Invalid user role: {s}"
                ))),
            }
        }
    }

    impl Default for UserRole {
        fn default() -> Self {
            UserRole::Subscriber
        }
    }
}

#[cfg(not(feature = "database"))]
pub use user::*;
