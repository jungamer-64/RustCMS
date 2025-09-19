//! データベース層
//!
//! Diesel + r2d2 による接続プールと、アプリ用の CRUD ヘルパ関数群を提供します。
//! 主な責務：
//! - コネクションプールの確立（`pool` モジュール）
//! - マイグレーションの実行（feature `database` 有効時）
//! - ユーザー / 投稿エンティティの検索・作成・更新・削除
//! - ページング・フィルタ・ソートの適用（Macro によるボイラープレート削減）
//!
//! 設計メモ：
//! - `with_conn` ヘルパでコネクション取得とエラーマッピングを一元化
//! - `NotFound` と Internal を `AppError` に正規化して上位層に伝播
//! - 並行性は r2d2 の最大接続数で制御（ブロッキング注意）
//! - 検索インデックス連携は上位層（`AppState`）が呼び出し側フックで実施
//!
//! NOTE: This module is large (~600+ LOC). Codacy flagged file complexity.
//! Automated refactors are risky; prefer incremental refactors:
//!  - extract smaller submodules (connections, migrations, queries)
//!  - add unit tests for extracted pieces
//!  - consider repository/service patterns to reduce module size
//!
//! See: docs/REFACTORING_GUIDE.md for a suggested plan (create if needed)

pub mod pool;
pub mod schema;

pub use pool::{DatabasePool, Pool, PooledConnection};

use crate::{
    Result,
    config::DatabaseConfig,
    models::{
        CreatePostRequest, CreateUserRequest, Post, UpdatePostRequest, UpdateUserRequest, User,
    },
};
#[cfg(all(feature = "database", not(test)))]
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use secrecy::ExposeSecret;
use uuid::Uuid;
use crate::repositories::UserRepository;

// Macros to DRY optional filter application for Diesel boxed queries
macro_rules! apply_eq_filter {
    ($query:ident, $opt:expr, $col:path) => {
        if let Some(val) = $opt.as_ref() {
            $query = $query.filter($col.eq(val));
        }
    };
}

#[allow(unused_macros)]
macro_rules! apply_tag_contains {
    ($query:ident, $opt:expr, $col:path) => {
        if let Some(val) = $opt.as_ref() {
            #[allow(unused_imports)]
            use diesel::PgArrayExpressionMethods;
            $query = $query.filter($col.contains(vec![val.clone()]));
        }
    };
}

// Grouped filter application macros to remove repeated triplets
macro_rules! apply_user_filters {
    ($query:ident, $role:expr, $active:expr) => {{
        apply_eq_filter!($query, $role, crate::database::schema::users::dsl::role);
        apply_eq_filter!(
            $query,
            $active,
            crate::database::schema::users::dsl::is_active
        );
    }};
}

macro_rules! apply_post_filters {
    ($query:ident, $status:expr, $author:expr, $tag:expr) => {{
        apply_eq_filter!($query, $status, crate::database::schema::posts::dsl::status);
        apply_eq_filter!(
            $query,
            $author,
            crate::database::schema::posts::dsl::author_id
        );
        apply_tag_contains!($query, $tag, crate::database::schema::posts::dsl::tags);
    }};
}

// Macro to DRY ordering logic based on (column_name, desc) with a default
// Usage:
//   apply_order_match!(query, sort_col, desc, default_order,
//       "created_at" => (users_dsl::created_at.asc(), users_dsl::created_at.desc()),
//       "updated_at" => (users_dsl::updated_at.asc(), users_dsl::updated_at.desc()),
//   );
macro_rules! apply_order_match {
    ($query:ident, $sort_col:expr, $desc:expr, $default:expr, $( $name:literal => ($asc:expr, $desc_e:expr) ),+ $(,)?) => {{
        $query = match ($sort_col.as_str(), $desc) {
            $( ($name, true) => $query.order($desc_e),
               ($name, false) => $query.order($asc), )+
            _ => $query.order($default),
        };
    }};
}

// Macro to apply the usual user sorting rules (keeps Diesel types hidden inside macro)
macro_rules! apply_user_sort {
    ($query:ident, $sort:expr) => {{
        let allowed = ["created_at", "updated_at", "username"];
        let (col, desc) = crate::utils::sort::parse_sort($sort.clone(), "created_at", true, &allowed);
        apply_order_match!(
            $query,
            col,
            desc,
            crate::database::schema::users::dsl::created_at.desc(),
            "created_at" => (crate::database::schema::users::dsl::created_at.asc(), crate::database::schema::users::dsl::created_at.desc()),
            "updated_at" => (crate::database::schema::users::dsl::updated_at.asc(), crate::database::schema::users::dsl::updated_at.desc()),
            "username" => (crate::database::schema::users::dsl::username.asc(), crate::database::schema::users::dsl::username.desc()),
        );
    }};
}

// Macro to apply the usual post sorting rules (includes special-case for published_at)
macro_rules! apply_post_sort {
    ($query:ident, $sort:expr) => {{
        let allowed = ["created_at", "updated_at", "published_at", "title"];
        let (sort_col, desc) = crate::utils::sort::parse_sort($sort, "created_at", true, &allowed);
        if sort_col == "published_at" {
            // Use fully-qualified dsl path here to avoid requiring local imports at call site
            $query = if desc {
                $query.order((crate::database::schema::posts::dsl::published_at.is_null().asc(), crate::database::schema::posts::dsl::published_at.desc()))
            } else {
                $query.order((crate::database::schema::posts::dsl::published_at.is_null().desc(), crate::database::schema::posts::dsl::published_at.asc()))
            };
        } else {
            apply_order_match!(
                $query,
                sort_col,
                desc,
                crate::database::schema::posts::dsl::created_at.desc(),
                "created_at" => (crate::database::schema::posts::dsl::created_at.asc(), crate::database::schema::posts::dsl::created_at.desc()),
                "updated_at" => (crate::database::schema::posts::dsl::updated_at.asc(), crate::database::schema::posts::dsl::updated_at.desc()),
                "title" => (crate::database::schema::posts::dsl::title.asc(), crate::database::schema::posts::dsl::title.desc()),
            );
        }
    }};
}

// Small helpers to reduce repeated error mapping patterns across DB helpers.
fn map_diesel_result<T>(
    res: std::result::Result<T, diesel::result::Error>,
    not_found_msg: &str,
    ctx: &str,
) -> Result<T> {
    match res {
        Ok(v) => Ok(v),
        Err(e) => match e {
            diesel::result::Error::NotFound => {
                Err(crate::AppError::NotFound(not_found_msg.into()))
            }
            other => Err(crate::AppError::Internal(format!("{ctx}: {other}"))),
        },
    }
}

fn ensure_affected_nonzero(affected: usize, not_found_msg: &str) -> Result<()> {
    if affected == 0 {
    Err(crate::AppError::NotFound(not_found_msg.into()))
    } else {
        Ok(())
    }
}

// Helper to map arbitrary DB errors into AppError::Internal with a context message.
fn map_internal_err<T, E: std::fmt::Display>(
    res: std::result::Result<T, E>,
    ctx: &str,
) -> Result<T> {
    res.map_err(|e| crate::AppError::Internal(format!("{ctx}: {e}")))
}

// Data holder for post update fields to keep update_post lean and testable
struct PostUpdateData {
    title: String,
    slug: String,
    content: String,
    excerpt: Option<String>,
    tags: Vec<String>,
    categories: Vec<String>,
    meta_title: Option<String>,
    meta_description: Option<String>,
    status: String,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// Compute the final set of fields for a post update, based on the request and existing row
fn compute_post_update_data(existing: &Post, req: &UpdatePostRequest) -> PostUpdateData {
    let title = req.title.clone().unwrap_or_else(|| existing.title.clone());
    let slug = req.slug.clone().unwrap_or_else(|| existing.slug.clone());
    let content = req
        .content
        .clone()
        .unwrap_or_else(|| existing.content.clone());
    let excerpt = merge_opt_option(req.excerpt.as_ref(), existing.excerpt.as_ref());
    let tags = merge_opt(req.tags.as_ref(), &existing.tags);
    let categories = req.category.as_ref().map_or_else(
        || existing.categories.clone(),
        |cat| vec![cat.trim().to_lowercase()],
    );
    let meta_title = merge_opt_option(req.meta_title.as_ref(), existing.meta_title.as_ref());
    let meta_description = merge_opt_option(
        req.meta_description.as_ref(),
        existing.meta_description.as_ref(),
    );

    // status / published_at handling
    let mut status = req
        .status
        .as_ref()
        .map_or_else(|| existing.status.clone(), ToString::to_string);
    let mut published_at = if req.published_at.is_some() {
        req.published_at
    } else {
        existing.published_at
    };
    if let Some(published) = req.published {
        if published {
            status = "published".to_string();
            if published_at.is_none() {
                published_at = Some(chrono::Utc::now());
            }
        } else {
            status = "draft".to_string();
        }
    }

    let updated_at = chrono::Utc::now();

    PostUpdateData {
        title,
        slug,
        content,
        excerpt,
        tags,
        categories,
        meta_title,
        meta_description,
        status,
        published_at,
        updated_at,
    }
}

fn merge_opt<T: Clone>(candidate: Option<&T>, current: &T) -> T {
    candidate.cloned().unwrap_or_else(|| current.clone())
}

fn merge_opt_option<T: Clone>(candidate: Option<&T>, current: Option<&T>) -> Option<T> {
    candidate.cloned().or_else(|| current.cloned())
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
    #[allow(clippy::unused_async)]
    /// Create a new Database instance backed by a connection pool.
    ///
    /// # Errors
    /// Returns an error if the pool cannot be created or if running migrations fails
    /// (when enabled). The error is wrapped in `crate::AppError`.
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = DatabasePool::new(config.url.expose_secret(), config.max_connections)?;

        #[cfg(all(feature = "database", not(test)))]
        if config.enable_migrations {
            Self::run_migrations(&pool)?;
        }

        Ok(Self { pool })
    }

    // Small helper to acquire a pooled connection and run a closure.
    // This removes repeated `let mut conn = self.get_connection()?;` boilerplate.
    fn with_conn<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut PooledConnection) -> Result<T>,
    {
        let mut conn = self.get_connection()?;
        f(&mut conn)
    }

    #[must_use]
    pub const fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    /// プールから接続を取得します。
    ///
    /// # Errors
    ///
    /// 接続の取得に失敗した場合にエラーを返します。
    pub fn get_connection(&self) -> Result<PooledConnection> {
        self.pool.get()
    }

    /// データベースのヘルスチェックを実行します。
    ///
    /// # Errors
    ///
    /// チェック用クエリの実行に失敗した場合にエラーを返します。
    pub async fn health_check(&self) -> Result<serde_json::Value> {
        self.pool.health_check().await?;
        Ok(serde_json::json!({
            "status": "healthy",
            "pool_size": 10, // self.pool.size(),
        }))
    }

    /// Best-effort close for database pool. Currently this is synchronous and
    /// simply drops the inner pool reference; provided for API symmetry and
    /// graceful shutdown hooks.
    #[allow(dead_code)]
    pub async fn close(&self) -> Result<()> {
        // Dropping Arc clones will close pool when last reference is gone. If
        // specific cleanup is needed, implement here.
        Ok(())
    }

    // User CRUD operations
    #[allow(clippy::unused_async)]
    /// ユーザーを作成します。
    ///
    /// # Errors
    ///
    /// 入力検証や保存処理に失敗した場合にエラーを返します。
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User> {
        // Build user with hashed password (this returns crate::AppError on failure)
        let user = User::new_with_password(
            request.username,
            request.email,
            &request.password,
            request.first_name,
            request.last_name,
            &request.role,
        )?;
        self.with_conn(|conn| {
            let created: User = User::create(conn, &user)?;
            Ok(created)
        })
    }

    /// List users helper used by admin CLI (stub)
    #[allow(clippy::unused_async)]
    /// 管理用: ユーザー一覧を返します（スタブ）。
    ///
    /// # Errors
    ///
    /// 内部処理失敗時にエラーを返します。
    pub async fn list_users(
        &self,
        _role: Option<&str>,
        _active_only: Option<bool>,
    ) -> Result<Vec<User>> {
        // Placeholder: return empty list
        Ok(vec![])
    }

    /// Get user by username helper used by admin CLI (stub)
    #[allow(clippy::unused_async)]
    /// ユーザー名でユーザーを取得します。
    ///
    /// # Errors
    ///
    /// 見つからない場合や取得に失敗した場合にエラーを返します。
    pub async fn get_user_by_username(&self, username_str: &str) -> Result<User> {
        // Propagate model-level AppError (preserves NotFound vs other AppError variants)
        self.with_conn(|conn| User::find_by_username(conn, username_str))
    }

    #[allow(clippy::unused_async)]
    /// IDでユーザーを取得します。
    ///
    /// # Errors
    ///
    /// 見つからない場合や取得に失敗した場合にエラーを返します。
    pub async fn get_user_by_id(&self, id: Uuid) -> Result<User> {
        // Propagate model-level AppError so NotFound is preserved
        self.with_conn(|conn| User::find_by_id(conn, id))
    }

    /// Find a user by email
    #[allow(clippy::unused_async)]
    /// メールアドレスでユーザーを取得します。
    ///
    /// # Errors
    ///
    /// 見つからない場合や取得に失敗した場合にエラーを返します。
    pub async fn get_user_by_email(&self, email: &str) -> Result<User> {
        self.with_conn(|conn| {
            let user = Self::run_query(
                || crate::models::User::find_by_email(conn, email),
                "DB error finding user by email",
            )?;
            Ok(user)
        })
    }

    /// Update user's last login timestamp
    /// 最終ログイン時刻を更新します。
    ///
    /// # Errors
    ///
    /// 更新処理に失敗した場合にエラーを返します。
    pub fn update_last_login(&self, id: Uuid) -> Result<()> {
        self.with_conn(|conn| {
            Self::run_query(
                || crate::models::User::update_last_login(conn, id),
                "DB error updating last_login",
            )
        })
    }

    /// ユーザー一覧を取得します（フィルタ/ソート対応）。
    ///
    /// # Errors
    ///
    /// 取得処理に失敗した場合にエラーを返します。
    #[allow(clippy::needless_pass_by_value)]
    pub fn get_users(
        &self,
        page: u32,
        per_page: u32,
        role: Option<String>,
        active: Option<bool>,
        sort: Option<String>,
    ) -> Result<Vec<User>> {
        use crate::database::schema::users::dsl as users_dsl;
        use diesel::prelude::*;

        let (_, limit, offset) = Self::paged_params(page, per_page);

        // Build and execute inside with_conn to centralize connection logic
        self.with_conn(|conn| {
            let mut query = users_dsl::users.into_boxed();
            apply_user_filters!(query, role, active);
            apply_user_sort!(query, sort);

            let results = Self::run_query(
                || query.offset(offset).limit(limit).load::<User>(conn),
                "Failed to list users",
            )?;
            Ok(results)
        })
    }

    /// ユーザー情報を更新します。
    ///
    /// # Errors
    ///
    /// 更新対象が見つからない、または更新に失敗した場合にエラーを返します。
    pub fn update_user(&self, id: Uuid, request: &UpdateUserRequest) -> Result<User> {
        self.with_conn(|conn| {
            // Let the model return AppError (NotFound, etc.) propagate directly.
            User::update(conn, id, request)
        })
    }

        // ... rest of impl Database ...


    // impl UserRepository moved to module scope below
    /// ユーザー数を返します。
    ///
    /// # Errors
    ///
    /// 集計に失敗した場合にエラーを返します。
    pub fn count_users(&self) -> Result<usize> {
        // Reuse the filtered counter to avoid duplicated query logic
        self.count_users_filtered(None, None)
    }

    /// Count users with optional filters (for accurate pagination totals)
    /// 条件付きのユーザー数を返します。
    ///
    /// # Errors
    ///
    /// 集計に失敗した場合にエラーを返します。
    #[allow(clippy::needless_pass_by_value)]
    pub fn count_users_filtered(
        &self,
        role: Option<String>,
        active: Option<bool>,
    ) -> Result<usize> {
        use crate::database::schema::users::dsl as users_dsl;
        use diesel::prelude::*;
        self.with_conn(|conn| {
            let mut query = users_dsl::users.into_boxed();
            apply_user_filters!(query, role, active);
            let total: i64 = Self::count_query(
                || query.count().get_result(conn),
                "Failed to count users (filtered)",
            )?;
            usize::try_from(total)
                .map_err(|_| crate::AppError::Internal("users count overflow".into()))
        })
    }

    // Post CRUD operations
    /// 投稿を作成します。
    ///
    /// # Errors
    ///
    /// 入力検証や保存処理に失敗した場合にエラーを返します。
    pub fn create_post(&self, request: CreatePostRequest) -> Result<Post> {
        use diesel::prelude::*;
        // Try to choose an author: prefer a user with role = 'admin', otherwise use the first user
        use crate::database::schema::posts::dsl as posts_dsl;
        use crate::database::schema::users::dsl as users_dsl;
        self.with_conn(|conn| {
            let author = Self::run_query(
                || {
                    users_dsl::users
                        .filter(users_dsl::role.eq("admin"))
                        .first::<crate::models::user::User>(conn)
                        .or_else(|_| users_dsl::users.first::<crate::models::user::User>(conn))
                },
                "Failed to find author user",
            )?;
            let author_id = author.id;
            let new_post = request.into_new_post(author_id);
            let inserted: Post = Self::run_query(
                || {
                    diesel::insert_into(posts_dsl::posts)
                        .values(&new_post)
                        .get_result(conn)
                },
                "Failed to create post",
            )?;
            Ok(inserted)
        })
    }

    /// 投稿IDで投稿を取得します。
    ///
    /// # Errors
    ///
    /// 見つからない場合や取得に失敗した場合にエラーを返します。
    pub fn get_post_by_id(&self, id: Uuid) -> Result<Post> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        self.with_conn(|conn| {
            let post = Self::get_one_query(
                || posts_dsl::posts.find(id).first::<Post>(conn),
                "Post not found",
                "Failed to fetch post",
            )?;
            Ok(post)
        })
    }

    /// 投稿一覧を取得します（フィルタ/ソート対応）。
    ///
    /// # Errors
    ///
    /// 取得処理に失敗した場合にエラーを返します。
    #[allow(clippy::needless_pass_by_value)]
    pub fn get_posts(
        &self,
        page: u32,
        per_page: u32,
        status: Option<String>,
        author: Option<Uuid>,
        tag: Option<String>,
        sort: Option<String>,
    ) -> Result<Vec<Post>> {
        use diesel::prelude::*;
        // no raw SQL needed
        use crate::database::schema::posts::dsl as posts_dsl;

        // Pagination guards
        let (_, limit, offset) = Self::paged_params(page, per_page);

        self.with_conn(|conn| {
            let mut query = posts_dsl::posts.into_boxed();
            apply_post_filters!(query, status, author, tag);

            apply_post_sort!(query, sort);

            let results = Self::run_query(
                || query.offset(offset).limit(limit).load::<Post>(conn),
                "Failed to list posts",
            )?;
            Ok(results)
        })
    }

    /// 投稿を更新します。
    ///
    /// # Errors
    ///
    /// 更新対象が見つからない、または更新に失敗した場合にエラーを返します。
    pub fn update_post(&self, id: Uuid, request: &UpdatePostRequest) -> Result<Post> {
        // Step 1 load + compute
        let (changes, updated_at) = self.prepare_post_update(id, request)?;
        // Step 2 persist
        let updated = self.persist_post_update(id, &changes)?;
        // Step 3 (optional future: trigger search index update / events) - placeholder uses updated_at to avoid unused warning
        let _ = updated_at; // reserved for future instrumentation
        Ok(updated)
    }

    fn prepare_post_update(
        &self,
        id: Uuid,
        req: &UpdatePostRequest,
    ) -> Result<(PostUpdateData, chrono::NaiveDateTime)> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        // fetch existing inside connection scope
        self.with_conn(|conn| {
            let existing = Self::get_one_query(
                || posts_dsl::posts.find(id).first::<Post>(conn),
                "Post not found",
                "Failed to fetch post",
            )?;
            let mut data = compute_post_update_data(&existing, req);
            data = Self::build_post_changes(&existing, data);
            let ts = chrono::Utc::now().naive_utc();
            Ok((data, ts))
        })
    }

    fn persist_post_update(&self, id: Uuid, changes: &PostUpdateData) -> Result<Post> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        self.with_conn(|conn| {
            let updated = Self::run_query(
                || {
                    diesel::update(posts_dsl::posts.find(id))
                        .set((
                            posts_dsl::title.eq(&changes.title),
                            posts_dsl::slug.eq(&changes.slug),
                            posts_dsl::content.eq(&changes.content),
                            posts_dsl::excerpt.eq(&changes.excerpt),
                            posts_dsl::tags.eq(&changes.tags),
                            posts_dsl::categories.eq(&changes.categories),
                            posts_dsl::meta_title.eq(&changes.meta_title),
                            posts_dsl::meta_description.eq(&changes.meta_description),
                            posts_dsl::status.eq(&changes.status),
                            posts_dsl::published_at.eq(&changes.published_at),
                            posts_dsl::updated_at.eq(&changes.updated_at),
                        ))
                        .get_result::<Post>(conn)
                },
                "Failed to update post",
            )?;
            Ok(updated)
        })
    }

    fn build_post_changes(existing: &Post, mut data: PostUpdateData) -> PostUpdateData {
        // status/publish_at normalization kept here to shrink complexity in caller
        if data.status == "published"
            && existing.published_at.is_none()
            && data.published_at.is_none()
        {
            data.published_at = Some(chrono::Utc::now());
        } else if data.status == "draft" {
            // ensure draft clears published_at
            data.published_at = None;
        }
        data
    }

    // (helper removed; original in-function sorting logic retained to avoid type privacy complications)

    /// 投稿を削除します。
    ///
    /// # Errors
    ///
    /// 該当する投稿が見つからない場合や削除に失敗した場合にエラーを返します。
    pub fn delete_post(&self, id: Uuid) -> Result<()> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        // Use helper to execute and ensure at least one row affected
        self.with_conn(|conn| {
            Self::execute_and_ensure(
                || diesel::delete(posts_dsl::posts.find(id)).execute(conn),
                "Failed to delete post",
                "Post not found",
            )
        })
    }

    /// 投稿数を返します（任意のタグでフィルタ可）。
    ///
    /// # Errors
    ///
    /// 集計に失敗した場合にエラーを返します。
    pub fn count_posts(&self, tag: Option<&str>) -> Result<usize> {
        // Delegate to the filtered counter to avoid duplication
        self.count_posts_filtered(None, None, tag.map(str::to_string))
    }

    /// Count posts with optional filters to match listing totals
    /// 条件付きの投稿数を返します（リストAPIと一致するフィルタ）。
    ///
    /// # Errors
    ///
    /// 集計に失敗した場合にエラーを返します。
    #[allow(clippy::needless_pass_by_value)]
    pub fn count_posts_filtered(
        &self,
        status: Option<String>,
        author: Option<Uuid>,
        tag: Option<String>,
    ) -> Result<usize> {
        use crate::database::schema::posts::dsl as posts_dsl;
        use diesel::prelude::*;
        self.with_conn(|conn| {
            let mut query = posts_dsl::posts.into_boxed();
            apply_post_filters!(query, status, author, tag);
            let total: i64 = Self::count_query(
                || query.count().get_result(conn),
                "Failed to count posts (filtered)",
            )?;
            usize::try_from(total)
                .map_err(|_| crate::AppError::Internal("posts count overflow".into()))
        })
    }

    /// 著者別の投稿数を返します。
    ///
    /// # Errors
    ///
    /// 集計に失敗した場合にエラーを返します。
    pub fn count_posts_by_author(&self, author: Uuid) -> Result<usize> {
        // Delegate to the filtered counter to avoid duplication
        self.count_posts_filtered(None, Some(author), None)
    }

    // Helper to run a diesel execute call and ensure affected > 0, mapping errors consistently.
    fn execute_and_ensure<F, E>(f: F, ctx: &str, not_found_msg: &str) -> Result<()>
    where
        F: FnOnce() -> std::result::Result<usize, E>,
        E: std::fmt::Display,
    {
        let res = map_internal_err(f(), ctx)?;
        ensure_affected_nonzero(res as usize, not_found_msg)?;
        Ok(())
    }

    // Helper to run count/get_result style queries returning i64, mapping errors consistently.
    fn count_query<F, E>(f: F, ctx: &str) -> Result<i64>
    where
        F: FnOnce() -> std::result::Result<i64, E>,
        E: std::fmt::Display,
    {
        let total = map_internal_err(f(), ctx)?;
        Ok(total)
    }

    // Generic helper to run a closure returning Result<T, E> and map errors consistently.
    fn run_query<T, F, E>(f: F, ctx: &str) -> Result<T>
    where
        F: FnOnce() -> std::result::Result<T, E>,
        E: std::fmt::Display,
    {
        let res = map_internal_err(f(), ctx)?;
        Ok(res)
    }

    // Helper to run a diesel first() query that returns Result<T, diesel::result::Error>
    // and map NotFound/other errors consistently.
    fn get_one_query<F, T>(f: F, not_found_msg: &str, ctx: &str) -> Result<T>
    where
        F: FnOnce() -> std::result::Result<T, diesel::result::Error>,
    {
        map_diesel_result(f(), not_found_msg, ctx)
    }

    // Helper to compute page, limit, offset from user-provided values.
    #[allow(clippy::cast_lossless)]
    const fn paged_params(page_in: u32, limit_in: u32) -> (i64, i64, i64) {
        let page_u32 = if page_in == 0 { 1 } else { page_in };
        let page = page_u32 as i64;
        let lim_u32 = match limit_in {
            0 => 10,
            n if n > 100 => 100,
            n => n,
        };
        let limit = lim_u32 as i64;
        let offset = (page - 1) * limit;
        (page, limit, offset)
    }

    /// Delete a user by ID
    /// 指定 ID のユーザーを削除します。
    ///
    /// # Errors
    ///
    /// 対象ユーザーが見つからない、または削除に失敗した場合にエラーを返します。
    pub fn delete_user(&self, id: Uuid) -> Result<()> {
        self.with_conn(|conn| {
            Self::execute_and_ensure(
                || User::delete(conn, id),
                "Failed to delete user",
                "User not found",
            )
        })
    }

    /// Reset user password helper used by admin CLI
    /// 管理者 CLI 用のユーザーパスワードリセット
    ///
    /// # Errors
    ///
    /// パスワードハッシュ化や更新クエリが失敗した場合にエラーを返します。
    pub fn reset_user_password(&self, id: Uuid, new_password: &str) -> Result<()> {
        use crate::database::schema::users::dsl as users_dsl;
        use diesel::prelude::*;
        let hash = crate::utils::password::hash_password(new_password)?;
        self.with_conn(|conn| {
            Self::execute_and_ensure(
                || {
                    diesel::update(users_dsl::users.find(id))
                        .set((
                            users_dsl::password_hash.eq(Some(hash.clone())),
                            users_dsl::updated_at.eq(chrono::Utc::now()),
                        ))
                        .execute(conn)
                },
                "Failed to reset password",
                "User not found",
            )
        })
    }

    #[cfg(all(feature = "database", not(test)))]
    fn run_migrations(pool: &DatabasePool) -> Result<()> {
        let mut conn = pool.get()?;
        // Map migration errors through the common internal error mapper for consistency.
        map_internal_err(conn.run_pending_migrations(MIGRATIONS), "Migration error")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_paged_params_defaults_and_limits() {
        // page 0 -> treated as 1, limit 0 -> default 10
        let (p, l, o) = Database::paged_params(0, 0);
        assert_eq!(p, 1);
        assert_eq!(l, 10);
        assert_eq!(o, 0);

        // large limit clipped to 100
        let (_p2, l2, _o2) = Database::paged_params(2, 1000);
        assert_eq!(l2, 100);
    }

    #[test]
    fn test_merge_opt_and_option() {
        let current = "current".to_string();
        let candidate_some = Some("cand".to_string());
        let candidate_none: Option<String> = None;
        assert_eq!(
            merge_opt(candidate_some.as_ref(), &current),
            "cand".to_string()
        );
        assert_eq!(merge_opt(candidate_none.as_ref(), &current), current);

        let cur_opt: Option<String> = Some("cur".to_string());
        let cand_opt_some = Some("new".to_string());
        let cand_opt_none: Option<String> = None;
        assert_eq!(
            merge_opt_option(cand_opt_some.as_ref(), cur_opt.as_ref()),
            Some("new".to_string())
        );
        assert_eq!(
            merge_opt_option(cand_opt_none.as_ref(), cur_opt.as_ref()),
            Some("cur".to_string())
        );
    }

    #[test]
    fn test_compute_post_update_and_build_changes() {
        let author = Uuid::new_v4();
        let existing = Post {
            id: Uuid::new_v4(),
            title: "Old".to_string(),
            slug: "old".to_string(),
            content: "old content".to_string(),
            excerpt: None,
            author_id: author,
            status: "draft".to_string(),
            featured_image_id: None,
            tags: vec!["rust".to_string()],
            categories: vec!["tech".to_string()],
            meta_title: None,
            meta_description: None,
            published_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let req = crate::models::post::UpdatePostRequest {
            title: Some("New".to_string()),
            content: None,
            excerpt: Some("ex".to_string()),
            slug: None,
            published: Some(true),
            tags: Some(vec!["rust".to_string(), "programming".to_string()]),
            category: Some("Programming".to_string()),
            featured_image: None,
            meta_title: None,
            meta_description: None,
            published_at: None,
            status: None,
        };

        let data = compute_post_update_data(&existing, &req);
        // ensure title was updated and tags merged
        assert_eq!(data.title, "New".to_string());
        assert!(data.tags.contains(&"programming".to_string()));

        // build_post_changes will set published_at when status published and existing none
        let changed = Database::build_post_changes(&existing, data);
        assert_eq!(changed.status, "published".to_string());
        assert!(changed.published_at.is_some());
    }
}
