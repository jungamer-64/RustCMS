use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use r2d2::Pool;
use uuid::Uuid;
use chrono::Utc;

use crate::models::*;
use crate::schema::{users, posts};
use crate::errors::AppError;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn create_connection_pool(database_url: &str) -> Result<DbPool, AppError> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .map_err(|e| AppError::Internal(format!("Failed to create connection pool: {}", e)))
}

#[derive(Clone)]
pub struct DatabaseService {
    pub pool: DbPool,
}

impl DatabaseService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn get_connection(&self) -> Result<DbConnection, AppError> {
        self.pool
            .get()
            .map_err(|e| AppError::Internal(format!("Failed to get database connection: {}", e)))
    }

    // User operations
    pub async fn create_user(&self, new_user: NewUser) -> Result<User, AppError> {
        let mut conn = self.get_connection()?;
        
        let user = diesel::insert_into(users::table)
            .values(&new_user)
            .returning(User::as_returning())
            .get_result(&mut conn)
            .map_err(|e| AppError::Internal(format!("Failed to create user: {}", e)))?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, AppError> {
        let mut conn = self.get_connection()?;
        
        users::table
            .filter(users::id.eq(user_id))
            .first(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("User not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<User, AppError> {
        let mut conn = self.get_connection()?;
        
        users::table
            .filter(users::username.eq(username))
            .first(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("User not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, AppError> {
        let mut conn = self.get_connection()?;
        
        users::table
            .filter(users::email.eq(email))
            .first(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("User not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn update_user(&self, user_id: Uuid, update_user: UpdateUser) -> Result<User, AppError> {
        let mut conn = self.get_connection()?;
        
        let updated_user = diesel::update(users::table.filter(users::id.eq(user_id)))
            .set(&update_user)
            .returning(User::as_returning())
            .get_result(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("User not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Failed to update user: {}", e)))?;

        Ok(updated_user)
    }

    pub async fn delete_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let mut conn = self.get_connection()?;
        
        diesel::delete(users::table.filter(users::id.eq(user_id)))
            .execute(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("User not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Failed to delete user: {}", e)))?;

        Ok(())
    }

    // Post operations
    pub async fn create_post(&self, new_post: NewPost) -> Result<Post, AppError> {
        let mut conn = self.get_connection()?;
        
        let post = diesel::insert_into(posts::table)
            .values(&new_post)
            .returning(Post::as_returning())
            .get_result(&mut conn)
            .map_err(|e| AppError::Internal(format!("Failed to create post: {}", e)))?;

        Ok(post)
    }

    pub async fn get_post_by_id(&self, post_id: Uuid) -> Result<Post, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .filter(posts::id.eq(post_id))
            .first(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("Post not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn get_post_by_slug(&self, slug: &str) -> Result<Post, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .filter(posts::slug.eq(slug))
            .first(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("Post not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn get_posts(&self, limit: i64, offset: i64) -> Result<Vec<Post>, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .order(posts::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn get_posts_by_author(&self, author_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Post>, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .filter(posts::author_id.eq(author_id))
            .order(posts::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn get_published_posts(&self, limit: i64, offset: i64) -> Result<Vec<Post>, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .filter(posts::status.eq("published"))
            .order(posts::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn update_post(&self, post_id: Uuid, update_post: UpdatePost) -> Result<Post, AppError> {
        let mut conn = self.get_connection()?;
        
        let mut update_data = update_post;
        update_data.updated_at = Some(Utc::now());
        
        let updated_post = diesel::update(posts::table.filter(posts::id.eq(post_id)))
            .set(&update_data)
            .returning(Post::as_returning())
            .get_result(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("Post not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Failed to update post: {}", e)))?;

        Ok(updated_post)
    }

    pub async fn delete_post(&self, post_id: Uuid) -> Result<(), AppError> {
        let mut conn = self.get_connection()?;
        
        diesel::delete(posts::table.filter(posts::id.eq(post_id)))
            .execute(&mut conn)
            .map_err(|diesel::NotFound| AppError::NotFound("Post not found".to_string()))
            .map_err(|e| AppError::Internal(format!("Failed to delete post: {}", e)))?;

        Ok(())
    }

    pub async fn count_posts(&self) -> Result<i64, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .count()
            .get_result(&mut conn)
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }

    pub async fn count_posts_by_status(&self, status: &str) -> Result<i64, AppError> {
        let mut conn = self.get_connection()?;
        
        posts::table
            .filter(posts::status.eq(status))
            .count()
            .get_result(&mut conn)
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }
}
