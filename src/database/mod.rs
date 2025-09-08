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

// Small helpers to reduce repeated error mapping patterns across DB helpers.
fn map_diesel_result<T>(res: std::result::Result<T, diesel::result::Error>, not_found_msg: &str, ctx: &str) -> Result<T> {
    match res {
        Ok(v) => Ok(v),
        Err(e) => match e {
            diesel::result::Error::NotFound => Err(crate::AppError::NotFound(not_found_msg.to_string())),
            other => Err(crate::AppError::Internal(format!("{}: {}", ctx, other))),
        },
    }
}

fn ensure_affected_nonzero(affected: usize, not_found_msg: &str) -> Result<()> {
    if affected == 0 {
        Err(crate::AppError::NotFound(not_found_msg.to_string()))
    } else {
        Ok(())
    }
}

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
        use diesel::prelude::*;
        use crate::database::schema::users::dsl as users_dsl;

        let mut conn = self.get_connection()?;

        let page = if _page == 0 { 1 } else { _page } as i64;
        let limit = match _limit { 0 => 10, n if n > 100 => 100, n => n } as i64;
        let offset = (page - 1) * limit;

        let mut query = users_dsl::users.into_boxed();

        if let Some(role) = _role.as_ref() {
            query = query.filter(users_dsl::role.eq(role));
        }
        if let Some(active) = _active.as_ref() {
            query = query.filter(users_dsl::is_active.eq(active));
        }

        let (sort_col, desc) = {
            let allowed = ["created_at", "updated_at", "username"];
            let (c, d) = crate::utils::sort::parse_sort(_sort, "created_at", true, &allowed);
            (c, d)
        };

        query = match (sort_col.as_str(), desc) {
            ("created_at", true) => query.order(users_dsl::created_at.desc()),
            ("created_at", false) => query.order(users_dsl::created_at.asc()),
            ("updated_at", true) => query.order(users_dsl::updated_at.desc()),
            ("updated_at", false) => query.order(users_dsl::updated_at.asc()),
            ("username", true) => query.order(users_dsl::username.desc()),
            ("username", false) => query.order(users_dsl::username.asc()),
            _ => query.order(users_dsl::created_at.desc()),
        };

        let results = query
            .offset(offset)
            .limit(limit)
            .load::<User>(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to list users: {}", e)))?;

        Ok(results)
    }

    pub async fn update_user(&self, id: Uuid, request: UpdateUserRequest) -> Result<User> {
        let mut conn = self.get_connection()?;
        let updated = User::update(&mut conn, id, &request)
            .map_err(|e| crate::AppError::Internal(format!("Failed to update user: {}", e)))?;
        Ok(updated)
    }

    pub async fn count_users(&self) -> Result<usize> {
        use diesel::prelude::*;
        use crate::database::schema::users::dsl as users_dsl;
        let mut conn = self.get_connection()?;
        let total: i64 = users_dsl::users
            .count()
            .get_result(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to count users: {}", e)))?;
        Ok(total as usize)
    }

    /// Count users with optional filters (for accurate pagination totals)
    pub async fn count_users_filtered(&self, _role: Option<String>, _active: Option<bool>) -> Result<usize> {
        use diesel::prelude::*;
        use crate::database::schema::users::dsl as users_dsl;
        let mut conn = self.get_connection()?;

        let mut query = users_dsl::users.into_boxed();
        if let Some(role) = _role.as_ref() {
            query = query.filter(users_dsl::role.eq(role));
        }
        if let Some(active) = _active.as_ref() {
            query = query.filter(users_dsl::is_active.eq(active));
        }

        let total: i64 = query
            .count()
            .get_result(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to count users (filtered): {}", e)))?;
        Ok(total as usize)
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
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;

        let mut conn = self.get_connection()?;
        let post = map_diesel_result(
            posts_dsl::posts.find(_id).first::<Post>(&mut conn),
            "Post not found",
            "Failed to fetch post",
        )?;

        Ok(post)
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
        use diesel::prelude::*;
    // no raw SQL needed
        use crate::database::schema::posts::dsl as posts_dsl;

        let mut conn = self.get_connection()?;

        // Pagination guards
        let page = if _page == 0 { 1 } else { _page } as i64;
        let limit = match _limit {
            0 => 10,
            n if n > 100 => 100,
            n => n,
        } as i64;
        let offset = (page - 1) * limit;

        let mut query = posts_dsl::posts.into_boxed();

        if let Some(status) = _status.as_ref() {
            query = query.filter(posts_dsl::status.eq(status));
        }
        if let Some(author) = _author.as_ref() {
            query = query.filter(posts_dsl::author_id.eq(author));
        }
        if let Some(tag) = _tag.as_ref() {
            // tags @> ARRAY[tag]
            // Prefer Diesel array contains if available; fallback to SQL fragment
            #[allow(unused_imports)]
            use diesel::PgArrayExpressionMethods;
            query = query.filter(posts_dsl::tags.contains(vec![tag.clone()]));
        }

        // Sort parsing via common helper; supports created_at, updated_at, published_at, title and optional '-' prefix
        let (sort_col, desc) = {
            let allowed = ["created_at", "updated_at", "published_at", "title"];
            let (c, d) = crate::utils::sort::parse_sort(_sort, "created_at", true, &allowed);
            (c, d)
        };

        query = match (sort_col.as_str(), desc) {
            ("created_at", true) => query.order(posts_dsl::created_at.desc()),
            ("created_at", false) => query.order(posts_dsl::created_at.asc()),
            ("updated_at", true) => query.order(posts_dsl::updated_at.desc()),
            ("updated_at", false) => query.order(posts_dsl::updated_at.asc()),
            // Emulate NULLS LAST/FIRST using a composite order
            ("published_at", true) => query.order((posts_dsl::published_at.is_null().asc(), posts_dsl::published_at.desc())),
            ("published_at", false) => query.order((posts_dsl::published_at.is_null().desc(), posts_dsl::published_at.asc())),
            ("title", true) => query.order(posts_dsl::title.desc()),
            ("title", false) => query.order(posts_dsl::title.asc()),
            _ => query.order(posts_dsl::created_at.desc()),
        };

        let results = query
            .offset(offset)
            .limit(limit)
            .load::<Post>(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to list posts: {}", e)))?;

        Ok(results)
    }

    pub async fn update_post(&self, _id: Uuid, _request: UpdatePostRequest) -> Result<Post> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;

        let mut conn = self.get_connection()?;

        // Load existing to compute derived fields and keep unchanged values
        let existing = map_diesel_result(
            posts_dsl::posts.find(_id).first::<Post>(&mut conn),
            "Post not found",
            "Failed to fetch post",
        )?;

        // Compute new values
        let new_title = _request.title.as_ref().cloned().unwrap_or_else(|| existing.title.clone());
        let new_slug = _request.slug.as_ref().cloned().unwrap_or_else(|| existing.slug.clone());
        let new_content = _request.content.as_ref().cloned().unwrap_or_else(|| existing.content.clone());
        let new_excerpt = if _request.excerpt.is_some() { _request.excerpt.clone() } else { existing.excerpt.clone() };
        let new_tags = _request.tags.clone().unwrap_or_else(|| existing.tags.clone());
        let new_categories = match &_request.category {
            Some(cat) => vec![cat.trim().to_lowercase()],
            None => existing.categories.clone(),
        };
        let new_meta_title = if _request.meta_title.is_some() { _request.meta_title.clone() } else { existing.meta_title.clone() };
        let new_meta_description = if _request.meta_description.is_some() { _request.meta_description.clone() } else { existing.meta_description.clone() };

        // status / published_at handling
        let mut new_status = if let Some(st) = &_request.status { st.to_string() } else { existing.status.clone() };
        let mut new_published_at = if _request.published_at.is_some() { _request.published_at } else { existing.published_at };
        if let Some(published) = _request.published {
            if published {
                new_status = "published".to_string();
                if new_published_at.is_none() { new_published_at = Some(chrono::Utc::now()); }
            } else {
                new_status = "draft".to_string();
            }
        }

        let now = chrono::Utc::now();

        let updated = diesel::update(posts_dsl::posts.find(_id))
            .set((
                posts_dsl::title.eq(new_title),
                posts_dsl::slug.eq(new_slug),
                posts_dsl::content.eq(new_content),
                posts_dsl::excerpt.eq(new_excerpt),
                posts_dsl::tags.eq(new_tags),
                posts_dsl::categories.eq(new_categories),
                posts_dsl::meta_title.eq(new_meta_title),
                posts_dsl::meta_description.eq(new_meta_description),
                posts_dsl::status.eq(new_status),
                posts_dsl::published_at.eq(new_published_at),
                posts_dsl::updated_at.eq(now),
            ))
            .get_result::<Post>(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to update post: {}", e)))?;

        Ok(updated)
    }

    pub async fn delete_post(&self, _id: Uuid) -> Result<()> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;
        let mut conn = self.get_connection()?;
        let affected = diesel::delete(posts_dsl::posts.find(_id))
            .execute(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to delete post: {}", e)))?;
        ensure_affected_nonzero(affected as usize, "Post not found")?;
        Ok(())
    }

    pub async fn count_posts(&self, _tag: Option<&str>) -> Result<usize> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;
        let mut conn = self.get_connection()?;

        let mut query = posts_dsl::posts.into_boxed();
        if let Some(tag) = _tag {
            #[allow(unused_imports)]
            use diesel::PgArrayExpressionMethods;
            query = query.filter(posts_dsl::tags.contains(vec![tag.to_string()]));
        }
        let total: i64 = query
            .count()
            .get_result(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to count posts: {}", e)))?;
        Ok(total as usize)
    }

    /// Count posts with optional filters to match listing totals
    pub async fn count_posts_filtered(
        &self,
        _status: Option<String>,
        _author: Option<Uuid>,
        _tag: Option<String>,
    ) -> Result<usize> {
        use diesel::prelude::*;
        use crate::database::schema::posts::dsl as posts_dsl;
        let mut conn = self.get_connection()?;

        let mut query = posts_dsl::posts.into_boxed();
        if let Some(status) = _status.as_ref() {
            query = query.filter(posts_dsl::status.eq(status));
        }
        if let Some(author) = _author.as_ref() {
            query = query.filter(posts_dsl::author_id.eq(author));
        }
        if let Some(tag) = _tag.as_ref() {
            #[allow(unused_imports)]
            use diesel::PgArrayExpressionMethods;
            query = query.filter(posts_dsl::tags.contains(vec![tag.clone()]));
        }

        let total: i64 = query
            .count()
            .get_result(&mut conn)
            .map_err(|e| crate::AppError::Internal(format!("Failed to count posts (filtered): {}", e)))?;
        Ok(total as usize)
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
        ensure_affected_nonzero(affected as usize, "User not found")?;
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
        ensure_affected_nonzero(affected as usize, "User not found")?;
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
