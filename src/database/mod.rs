pub mod pool;
pub mod schema;

pub use pool::{DatabasePool, Pool, PooledConnection};

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use uuid::Uuid;
use crate::{
    config::DatabaseConfig,
    models::{User, Post, CreateUserRequest, UpdateUserRequest, CreatePostRequest, UpdatePostRequest},
    Result,
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(Debug, Clone)]
pub struct Database {
    pool: DatabasePool,
}

impl Database {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = DatabasePool::new(&config.url, config.max_connections)?;
        
        if config.enable_migrations {
            Self::run_migrations(&pool)?;
        }

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub fn get_connection(&self) -> Result<PooledConnection> {
        self.pool.get()
    }

    pub async fn health_check(&self) -> Result<serde_json::Value> {
        self.pool.health_check().await?;
        Ok(serde_json::json!({
            "status": "healthy",
            "pool_size": 10, // self.pool.size(),
        }))
    }

    // User CRUD operations
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        

        // Build user with hashed password (this returns crate::AppError on failure)
        let user = User::new_with_password(
            request.username,
            request.email,
            &request.password,
            request.first_name,
            request.last_name,
            request.role,
        )?;

        let mut conn = self.get_connection()?;

        // Insert and return the created user
        let created: User = User::create(&mut conn, &user)?;
        Ok(created)
    }

    /// List users helper used by admin CLI (stub)
    pub async fn list_users(&self, _role: Option<&str>, _active_only: Option<bool>) -> Result<Vec<User>> {
        // Placeholder: return empty list
        Ok(vec![])
    }

    /// Get user by username helper used by admin CLI (stub)
    pub async fn get_user_by_username(&self, _username: &str) -> Result<User> {
        // Placeholder: not implemented, return NotFound
        Err(crate::AppError::NotFound("User not found".to_string()))
    }

    pub async fn get_user_by_id(&self, _id: Uuid) -> Result<User> {
        // Placeholder implementation
        Err(crate::AppError::NotFound("User not found".to_string()))
    }

    pub async fn get_users(&self, _page: u32, _limit: u32, _role: Option<String>, _active: Option<bool>, _sort: Option<String>) -> Result<Vec<User>> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn update_user(&self, _id: Uuid, _request: UpdateUserRequest) -> Result<User> {
        // Placeholder implementation
        Err(crate::AppError::Internal("Not implemented".to_string()))
    }

    pub async fn count_users(&self) -> Result<usize> {
        // Placeholder implementation
        Ok(0)
    }

    // Post CRUD operations
    pub async fn create_post(&self, request: CreatePostRequest) -> Result<Post> {
        use diesel::prelude::*;

        let mut conn = self.get_connection()?;

        // Try to choose an author: prefer a user with role = 'admin', otherwise use the first user
        use crate::database::schema::users::dsl as users_dsl;

        let author = users_dsl::users
            .filter(users_dsl::role.eq("admin"))
            .first::<crate::models::user::User>(&mut conn)
            .or_else(|_| users_dsl::users.first::<crate::models::user::User>(&mut conn))
            .map_err(|e| crate::AppError::Internal(format!("Failed to find author user: {}", e)))?;

        let author_id = author.id;

        // Convert request into NewPost (generates id inside)
        let new_post = request.into_new_post(author_id);

        // Insert and return the created post
        use crate::database::schema::posts::dsl as posts_dsl;

        let inserted: Post = diesel::insert_into(posts_dsl::posts)
            .values(&new_post)
            .get_result(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to create post: {}", e)))?;

        Ok(inserted)
    }

    pub async fn get_post_by_id(&self, _id: Uuid) -> Result<Post> {
        // Placeholder implementation
        Err(crate::AppError::NotFound("Post not found".to_string()))
    }

    pub async fn get_posts(&self, _page: u32, _limit: u32, _status: Option<String>, _author: Option<Uuid>, _tag: Option<String>, _sort: Option<String>) -> Result<Vec<Post>> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn update_post(&self, _id: Uuid, _request: UpdatePostRequest) -> Result<Post> {
        // Placeholder implementation
        Err(crate::AppError::Internal("Not implemented".to_string()))
    }

    pub async fn delete_post(&self, _id: Uuid) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }

    pub async fn count_posts(&self, _tag: Option<&str>) -> Result<usize> {
        // Placeholder implementation
        Ok(0)
    }

    pub async fn count_posts_by_author(&self, _author_id: Uuid) -> Result<usize> {
        // Placeholder implementation
        Ok(0)
    }

    /// Delete a user by ID
    pub async fn delete_user(&self, _id: Uuid) -> Result<()> {
        // Placeholder - would delete user from database
        Ok(())
    }

    /// Reset user password helper used by admin CLI (stub)
    pub async fn reset_user_password(&self, _id: Uuid, _new_password: &str) -> Result<()> {
        // Placeholder: noop
        Ok(())
    }

    fn run_migrations(pool: &DatabasePool) -> Result<()> {
        
        
        let mut conn = pool.get()?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| crate::AppError::Internal(format!("Migration error: {}", e)))?;
        
        Ok(())
    }
}
