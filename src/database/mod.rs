pub mod pool;
pub mod schema;

pub use pool::{DatabasePool, Pool, PooledConnection};

use crate::{
    config::DatabaseConfig,
    models::{
        CreatePostRequest, CreateUserRequest, Post, UpdatePostRequest, UpdateUserRequest, User,
    },
    Result,
};
#[cfg(all(feature = "database", not(test)))]
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use uuid::Uuid;

// Diesel 2.x の embed_migrations: Cargo.toml からの相対パスでディレクトリ配下の
// up.sql / down.sql を持つバージョンディレクトリを埋め込む。
// 以前: 存在しない feature(with-migrations) と fallback(".") でビルド失敗を誘発していたため撤去。
// 単純に migrations ディレクトリを埋め込む。テストでは speed / isolation のためスキップ。
#[cfg(all(feature = "database", not(test)))]
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(Debug, Clone)]
pub struct Database {
    pool: DatabasePool,
}

impl Database {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = DatabasePool::new(&config.url, config.max_connections)?;

        #[cfg(all(feature = "database", not(test)))]
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
    pub async fn list_users(
        &self,
        _role: Option<&str>,
        _active_only: Option<bool>,
    ) -> Result<Vec<User>> {
        // Placeholder: return empty list
        Ok(vec![])
    }

    /// Get user by username helper used by admin CLI (stub)
    pub async fn get_user_by_username(&self, username_str: &str) -> Result<User> {
        let mut conn = self.get_connection()?;
        let user = User::find_by_username(&mut conn, username_str)
            .map_err(|_| crate::AppError::NotFound("User not found".to_string()))?;
        Ok(user)
    }

    pub async fn get_user_by_id(&self, _id: Uuid) -> Result<User> {
        let mut conn = self.get_connection()?;
        let user = User::find_by_id(&mut conn, _id)
            .map_err(|_| crate::AppError::NotFound("User not found".to_string()))?;
        Ok(user)
    }

    /// Find a user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<User> {
        let mut conn = self.get_connection()?;
        let user =
            crate::models::User::find_by_email(&mut conn, email)
                .map_err(|e| crate::AppError::Internal(format!("DB error finding user by email: {}", e)))?;
        Ok(user)
    }

    /// Update user's last login timestamp
    pub async fn update_last_login(&self, id: Uuid) -> Result<()> {
        let mut conn = self.get_connection()?;
        crate::models::User::update_last_login(&mut conn, id)
            .map_err(|e| crate::AppError::Internal(format!("DB error updating last_login: {}", e)))?;
        Ok(())
    }

    pub async fn get_users(
        &self,
        _page: u32,
        _limit: u32,
        _role: Option<String>,
        _active: Option<bool>,
        _sort: Option<String>,
    ) -> Result<Vec<User>> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn update_user(&self, id: Uuid, request: UpdateUserRequest) -> Result<User> {
        let mut conn = self.get_connection()?;
        let updated = User::update(&mut conn, id, &request)
            .map_err(|e| crate::AppError::Internal(format!("Failed to update user: {}", e)))?;
        Ok(updated)
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

    pub async fn get_posts(
        &self,
        _page: u32,
        _limit: u32,
        _status: Option<String>,
        _author: Option<Uuid>,
        _tag: Option<String>,
        _sort: Option<String>,
    ) -> Result<Vec<Post>> {
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

    pub async fn count_posts_by_author(&self, author: Uuid) -> Result<usize> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;
        let mut conn = self.get_connection()?;
        let total: i64 = posts_dsl::posts
            .filter(posts_dsl::author_id.eq(author))
            .count()
            .get_result(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to count posts by author: {}", e)))?;
        Ok(total as usize)
    }

    /// Delete a user by ID
    pub async fn delete_user(&self, id: Uuid) -> Result<()> {
        let mut conn = self.get_connection()?;
        let affected = User::delete(&mut conn, id)
            .map_err(|e| crate::AppError::Internal(format!("Failed to delete user: {}", e)))?;
        if affected == 0 { return Err(crate::AppError::NotFound("User not found".to_string())); }
        Ok(())
    }

    /// Reset user password helper used by admin CLI
    pub async fn reset_user_password(&self, id: Uuid, new_password: &str) -> Result<()> {
        use diesel::prelude::*;
        use crate::database::schema::users::dsl as users_dsl;
        let mut conn = self.get_connection()?;
        let hash = crate::utils::password::hash_password(new_password)?;
        let affected = diesel::update(users_dsl::users.find(id))
            .set((users_dsl::password_hash.eq(Some(hash)), users_dsl::updated_at.eq(chrono::Utc::now())))
            .execute(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to reset password: {}", e)))?;
        if affected == 0 { return Err(crate::AppError::NotFound("User not found".to_string())); }
        Ok(())
    }

    #[cfg(all(feature = "database", not(test)))]
    fn run_migrations(pool: &DatabasePool) -> Result<()> {
        let mut conn = pool.get()?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| crate::AppError::Internal(format!("Migration error: {}", e)))?;

        Ok(())
    }
}
