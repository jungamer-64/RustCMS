use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

use crate::schema::{users, posts};

// User Model
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Validate)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub id: Uuid,
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    pub password_hash: String,
    #[validate(length(min = 1))]
    pub role: String,
    pub is_active: Option<bool>,
}

#[derive(AsChangeset, Deserialize, Validate)]
#[diesel(table_name = users)]
pub struct UpdateUser {
    #[validate(length(min = 3, max = 50))]
    pub username: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub role: Option<String>,
    pub is_active: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Post Model
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub slug: String,
    pub status: String,
    pub author_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Validate)]
#[diesel(table_name = posts)]
pub struct NewPost {
    pub id: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 1))]
    pub content: String,
    #[validate(length(min = 1, max = 255))]
    pub slug: String,
    pub status: String,
    pub author_id: Uuid,
}

#[derive(AsChangeset, Deserialize, Validate)]
#[diesel(table_name = posts)]
pub struct UpdatePost {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    #[validate(length(min = 1))]
    pub content: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Request/Response DTOs
#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 3))]
    pub username: String,
    #[validate(length(min = 6))]
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
            is_active: user.is_active,
            created_at: user.created_at,
        }
    }
}

// Search Models for Elasticsearch
#[derive(Serialize, Deserialize, Debug)]
pub struct PostDocument {
    pub id: String,
    pub title: String,
    pub content: String,
    pub slug: String,
    pub status: String,
    pub author_id: String,
    pub author_username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Post> for PostDocument {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.to_string(),
            title: post.title,
            content: post.content,
            slug: post.slug,
            status: post.status,
            author_id: post.author_id.to_string(),
            author_username: String::new(), // Will be filled from join
            created_at: post.created_at,
            updated_at: post.updated_at,
        }
    }
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub author_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub posts: Vec<Post>,
    pub total: u64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}
