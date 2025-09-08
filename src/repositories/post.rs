use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::de::DeserializeOwned;
use crate::{
    error::{AppError, Result},
    cache::manager::{CacheManager, cache_keys},
    monitoring::metrics::PerformanceMonitor,
    models::post::{Post, CreatePostRequest, UpdatePostRequest, PostFilter, PostResponse},
};

#[derive(Clone)]
pub struct PostRepository {
    db: PgPool,
    cache: CacheManager,
    metrics: PerformanceMonitor,
}

impl PostRepository {
    pub fn new(db: PgPool, cache: CacheManager, metrics: PerformanceMonitor) -> Self {
        Self { db, cache, metrics }
    }

    // Generic helper to attempt reading a typed value from cache and record metrics.
    async fn try_get_cache<T>(&self, cache_key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned + Clone + Send + Sync + 'static,
    {
        if let Some(cached) = self.cache.get::<T>(cache_key).await? {
            self.metrics.record_cache_operation("get", true, std::time::Duration::from_millis(1));
            return Ok(Some(cached));
        }
        self.metrics.record_cache_operation("get", false, std::time::Duration::from_millis(1));
        Ok(None)
    }

    pub async fn create(&self, req: CreatePostRequest, author_id: Uuid) -> Result<Post> {
        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "insert".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let id = Uuid::new_v4();
        let now = Utc::now();
        let slug = req.slug.unwrap_or_else(|| generate_slug(&req.title));

        let post = sqlx::query_as!(
            Post,
            r#"
            INSERT INTO posts (id, title, content, slug, author_id, published, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            id,
            req.title,
            req.content,
            slug,
            author_id,
            req.published.unwrap_or(false),
            now,
            now
        )
        .fetch_one(&self.db)
        .await?;

        timer.stop();

        // Invalidate cache
        self.cache.delete_pattern("posts:list:*").await?;
        
        Ok(post)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Post>> {
        let cache_key = cache_keys::post(&id.to_string());
        
        // Try cache first
        if let Some(cached_post) = self.try_get_cache::<Post>(&cache_key).await? {
            return Ok(Some(cached_post));
        }

        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "select".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let post = sqlx::query_as!(
            Post,
            "SELECT * FROM posts WHERE id = $1",
            id
        )
        .fetch_optional(&self.db)
        .await?;

        timer.stop();

        // Cache the result if found
        if let Some(ref post) = post {
            self.cache.set(&cache_key, post, Some(300)).await?; // 5 minutes cache
        }

        Ok(post)
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<Option<Post>> {
        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "select".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let post = sqlx::query_as!(
            Post,
            "SELECT * FROM posts WHERE slug = $1",
            slug
        )
        .fetch_optional(&self.db)
        .await?;

        timer.stop();

        Ok(post)
    }

    pub async fn list(&self, filter: PostFilter) -> Result<PostResponse> {
        let cache_key = cache_keys::posts_list(filter.page, filter.limit, filter.published);
        
        // Try cache first
        if let Some(cached_response) = self.try_get_cache::<PostResponse>(&cache_key).await? {
            return Ok(cached_response);
        }

        let offset = (filter.page - 1) * filter.limit;
        
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, title, content, slug, author_id, published, created_at, updated_at FROM posts"
        );
        
        let mut count_builder = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM posts");
        
        if let Some(published) = filter.published {
            query_builder.push(" WHERE published = ");
            query_builder.push_bind(published);
            
            count_builder.push(" WHERE published = ");
            count_builder.push_bind(published);
        }

        if let Some(author_id) = filter.author_id {
            let clause = if filter.published.is_some() { " AND " } else { " WHERE " };
            query_builder.push(clause);
            query_builder.push("author_id = ");
            query_builder.push_bind(author_id);
            
            count_builder.push(clause);
            count_builder.push("author_id = ");
            count_builder.push_bind(author_id);
        }

        query_builder.push(" ORDER BY created_at DESC LIMIT ");
        query_builder.push_bind(filter.limit as i64);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset as i64);

        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "select".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let posts_query = query_builder.build_query_as::<Post>();
        let count_query = count_builder.build_query_scalar::<i64>();

        let (posts, total_count) = tokio::try_join!(
            posts_query.fetch_all(&self.db),
            count_query.fetch_one(&self.db)
        )?;

        timer.stop();

        let response = PostResponse {
            posts,
            pagination: crate::models::pagination::PaginationInfo {
                page: filter.page,
                limit: filter.limit,
                total: total_count as usize,
                total_pages: crate::models::pagination::calc_total_pages(total_count as usize, filter.limit as u32) as usize,
            },
        };

        // Cache the result
        self.cache.set(&cache_key, &response, Some(60)).await?; // 1 minute cache

        Ok(response)
    }

    pub async fn update(&self, id: Uuid, req: UpdatePostRequest) -> Result<Post> {
        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "update".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let now = Utc::now();
        
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE posts SET updated_at = ");
        query_builder.push_bind(now);
        
        if let Some(title) = &req.title {
            query_builder.push(", title = ");
            query_builder.push_bind(title);
        }
        
        if let Some(content) = &req.content {
            query_builder.push(", content = ");
            query_builder.push_bind(content);
        }
        
        if let Some(slug) = &req.slug {
            query_builder.push(", slug = ");
            query_builder.push_bind(slug);
        }
        
        if let Some(published) = req.published {
            query_builder.push(", published = ");
            query_builder.push_bind(published);
        }
        
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);
        query_builder.push(" RETURNING *");

        let query = query_builder.build_query_as::<Post>();
        let post = query.fetch_one(&self.db).await?;

        timer.stop();

        // Invalidate cache
        let cache_key = cache_keys::post(&id.to_string());
        self.cache.delete(&cache_key).await?;
        self.cache.delete_pattern("posts:list:*").await?;

        Ok(post)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "delete".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let result = sqlx::query!("DELETE FROM posts WHERE id = $1", id)
            .execute(&self.db)
            .await?;

        timer.stop();

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Post not found".to_string()));
        }

        // Invalidate cache
        let cache_key = cache_keys::post(&id.to_string());
        self.cache.delete(&cache_key).await?;
        self.cache.delete_pattern("posts:list:*").await?;

        Ok(())
    }

    pub async fn search(&self, query: &str, filter: PostFilter) -> Result<PostResponse> {
        let timer = crate::monitoring::metrics::start_timer(
            "database_query_duration_seconds",
            vec![
                ("operation".to_string(), "search".to_string()),
                ("table".to_string(), "posts".to_string()),
            ],
        );

        let offset = (filter.page - 1) * filter.limit;
        
        let search_query = format!("%{}%", query.to_lowercase());
        
        let posts = sqlx::query_as!(
            Post,
            r#"
            SELECT * FROM posts 
            WHERE (LOWER(title) LIKE $1 OR LOWER(content) LIKE $1)
            AND ($2::bool IS NULL OR published = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            search_query,
            filter.published,
            filter.limit as i64,
            offset as i64
        )
        .fetch_all(&self.db)
        .await?;

        let total_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM posts 
            WHERE (LOWER(title) LIKE $1 OR LOWER(content) LIKE $1)
            AND ($2::bool IS NULL OR published = $2)
            "#,
            search_query,
            filter.published
        )
        .fetch_one(&self.db)
        .await?
        .unwrap_or(0);

        timer.stop();

        Ok(PostResponse {
            posts,
            pagination: crate::models::pagination::PaginationInfo {
                page: filter.page,
                limit: filter.limit,
                total: total_count as usize,
                total_pages: ((total_count as f64) / (filter.limit as f64)).ceil() as usize,
            },
        })
    }
}

fn generate_slug(title: &str) -> String { crate::utils::url_encoding::generate_safe_slug(title) }
