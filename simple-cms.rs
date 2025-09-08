// 完全に独立したシンプルCMS - 単一ファイルでの実装
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

// データ構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub content: String,
    pub slug: String,
    pub published: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub content: String,
    pub slug: String,
    pub published: bool,
    pub created_at: String,
    pub updated_at: String,
}

// リクエスト構造体
#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub slug: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub slug: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePageRequest {
    pub title: String,
    pub content: String,
    pub slug: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub published: Option<bool>,
}

// Use shared API types
use crate::utils::api_types::{ApiResponse, Pagination};
use crate::utils::url_encoding::generate_safe_slug;

// インメモリストレージ
#[derive(Debug, Clone)]
pub struct InMemoryStore {
    pub posts: Arc<RwLock<HashMap<String, Post>>>,
    pub users: Arc<RwLock<HashMap<String, User>>>,
    pub pages: Arc<RwLock<HashMap<String, Page>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        let store = Self {
            posts: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            pages: Arc::new(RwLock::new(HashMap::new())),
        };

        // サンプルデータを追加
        store.seed_data();
        store
    }

    fn seed_data(&self) {
        // サンプル投稿
        let sample_post = Post {
            id: Uuid::new_v4().to_string(),
            title: "Welcome to Simple CMS".to_string(),
            content: "This is a sample post in our Simple CMS system.".to_string(),
            slug: "welcome-to-simple-cms".to_string(),
            published: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        // サンプルユーザー
        let sample_user = User {
            id: Uuid::new_v4().to_string(),
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            role: "admin".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // サンプルページ
        let sample_page = Page {
            id: Uuid::new_v4().to_string(),
            title: "About Us".to_string(),
            content: "This is our about page.".to_string(),
            slug: "about".to_string(),
            published: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        self.posts.write().unwrap().insert(sample_post.id.clone(), sample_post);
        self.users.write().unwrap().insert(sample_user.id.clone(), sample_user);
        self.pages.write().unwrap().insert(sample_page.id.clone(), sample_page);
    }
}

// ヘルパー関数
fn generate_slug(title: &str) -> String { generate_safe_slug(title) }

#[inline]
fn now_iso() -> String { chrono::Utc::now().to_rfc3339() }

#[inline]
fn normalize_page_limit(params: &QueryParams, default_limit: usize, max_limit: usize) -> (usize, usize) {
    let page = params.page.unwrap_or(1).max(1);
    let mut limit = params.limit.unwrap_or(default_limit);
    if limit == 0 { limit = default_limit; }
    if limit > max_limit { limit = max_limit; }
    (page, limit)
}

#[inline]
fn apply_published_filter<T, F>(list: &mut Vec<T>, published_opt: Option<bool>, get_published: F)
where
    F: Fn(&T) -> bool,
{
    if let Some(published) = published_opt {
        list.retain(|item| get_published(item) == published);
    }
}
fn sort_by_created_desc<T, F>(list: &mut Vec<T>, get_created_at: F)
where
    F: Fn(&T) -> &String,
{
    list.sort_by(|a, b| get_created_at(b).cmp(get_created_at(a)));
}

fn paginate<T: Clone>(items: &[T], page: usize, per_page: usize) -> (Vec<T>, Pagination) {
    let total = items.len();
    let total_pages = (total as f64 / per_page as f64).ceil() as usize;
    let start = (page - 1) * per_page;
    let end = std::cmp::min(start + per_page, total);

    let paginated_items = if start < total {
        items[start..end].to_vec()
    } else {
        vec![]
    };

    let pagination = Pagination {
        page: page as u32,
        per_page: per_page as u32,
        total: total as u64,
        total_pages: total_pages as u32,
    };

    (paginated_items, pagination)
}

#[inline]
fn respond_paginated<T: Serialize>(data: Vec<T>, pagination: Pagination) -> impl IntoResponse {
    Json(ApiResponse::success(crate::utils::api_types::PaginatedResponse { data, pagination }))
}

// APIハンドラー

// ヘルスチェック
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "service": "Simple CMS",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

// 投稿関連のハンドラー
async fn get_posts(
    State(store): State<InMemoryStore>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let posts = store.posts.read().unwrap();
    let mut post_list: Vec<Post> = posts.values().cloned().collect();
    apply_published_filter(&mut post_list, params.published, |p| p.published);
    sort_by_created_desc(&mut post_list, |p| &p.created_at);
    let (page, limit) = normalize_page_limit(&params, 20, 100);
    let (paginated_posts, pagination) = paginate(&post_list, page, limit);

    respond_paginated(paginated_posts, pagination)
}

async fn get_post(
    State(store): State<InMemoryStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let posts = store.posts.read().unwrap();
    
    if let Some(post) = posts.get(&id) {
    Json(ApiResponse::success(post.clone()))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Post not found".to_string())),
        )
    }
}

async fn create_post(
    State(store): State<InMemoryStore>,
    Json(payload): Json<CreatePostRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let slug = payload.slug.unwrap_or_else(|| generate_slug(&payload.title));
    let now = now_iso();

    let post = Post {
        id: id.clone(),
        title: payload.title,
        content: payload.content,
        slug,
        published: payload.published.unwrap_or(false),
        created_at: now.clone(),
        updated_at: now,
    };

    store.posts.write().unwrap().insert(id.clone(), post.clone());

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(post)),
    )
}

async fn update_post(
    State(store): State<InMemoryStore>,
    Path(id): Path<String>,
    Json(payload): Json<UpdatePostRequest>,
) -> impl IntoResponse {
    let mut posts = store.posts.write().unwrap();
    
    if let Some(post) = posts.get_mut(&id) {
        if let Some(title) = payload.title {
            post.title = title;
        }
        if let Some(content) = payload.content {
            post.content = content;
        }
        if let Some(slug) = payload.slug {
            post.slug = slug;
        }
        if let Some(published) = payload.published {
            post.published = published;
        }
    post.updated_at = now_iso();

    Json(ApiResponse::success(post.clone()))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Post not found".to_string())),
        ).into_response()
    }
}

async fn delete_post(
    State(store): State<InMemoryStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut posts = store.posts.write().unwrap();
    
    if posts.remove(&id).is_some() {
        Json(ApiResponse::success_with_message((), "Post deleted successfully".to_string()))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Post not found".to_string())),
        )
    }
}

// ユーザー関連のハンドラー
async fn get_users(
    State(store): State<InMemoryStore>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let users = store.users.read().unwrap();
    let mut user_list: Vec<User> = users.values().cloned().collect();
    sort_by_created_desc(&mut user_list, |u| &u.created_at);
    let (page, limit) = normalize_page_limit(&params, 10, 100);
    let (paginated_users, pagination) = paginate(&user_list, page, limit);

    respond_paginated(paginated_users, pagination)
}

async fn create_user(
    State(store): State<InMemoryStore>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();

    let user = User {
        id: id.clone(),
        username: payload.username,
        email: payload.email,
        role: payload.role.unwrap_or("user".to_string()),
    created_at: now_iso(),
    };

    store.users.write().unwrap().insert(id.clone(), user.clone());

    (
        StatusCode::CREATED,
        Json(ApiResponse::success_with_message(user, "User created successfully".to_string())),
    )
}

// ページ関連のハンドラー
async fn get_pages(
    State(store): State<InMemoryStore>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let pages = store.pages.read().unwrap();
    let mut page_list: Vec<Page> = pages.values().cloned().collect();
    apply_published_filter(&mut page_list, params.published, |p| p.published);
    sort_by_created_desc(&mut page_list, |p| &p.created_at);
    let (page, limit) = normalize_page_limit(&params, 10, 100);
    let (paginated_pages, pagination) = paginate(&page_list, page, limit);

    respond_paginated(paginated_pages, pagination)
}

async fn create_page(
    State(store): State<InMemoryStore>,
    Json(payload): Json<CreatePageRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let slug = payload.slug.unwrap_or_else(|| generate_slug(&payload.title));
    let now = now_iso();

    let page = Page {
        id: id.clone(),
        title: payload.title,
        content: payload.content,
        slug,
        published: payload.published.unwrap_or(false),
        created_at: now.clone(),
        updated_at: now,
    };

    store.pages.write().unwrap().insert(id.clone(), page.clone());

    (
        StatusCode::CREATED,
        Json(ApiResponse::success_with_message(page, "Page created successfully".to_string())),
    )
}

// 統計ハンドラー
async fn get_stats(State(store): State<InMemoryStore>) -> impl IntoResponse {
    let posts = store.posts.read().unwrap();
    let users = store.users.read().unwrap();
    let pages = store.pages.read().unwrap();

    let published_posts = posts.values().filter(|p| p.published).count();
    let published_pages = pages.values().filter(|p| p.published).count();

    Json(ApiResponse::success(serde_json::json!({
        "total_posts": posts.len(),
        "published_posts": published_posts,
        "draft_posts": posts.len() - published_posts,
        "total_users": users.len(),
        "total_pages": pages.len(),
        "published_pages": published_pages,
        "draft_pages": pages.len() - published_pages,
    })))
}

// ドキュメンテーション
async fn api_docs() -> impl IntoResponse {
    Html(include_str!("templates/simple_cms_docs.html"))
}

#[tokio::main]
async fn main() {
    // 共有ストアを初期化
    let store = InMemoryStore::new();

    // ルーターを構築
    let app = Router::new()
        // ヘルスチェック
        .route("/health", get(health_check))
        
        // ドキュメンテーション
    .route("/", get(api_docs))
    .route("/api/docs", get(api_docs))
        
        // 投稿関連
        .route("/api/posts", get(get_posts).post(create_post))
        .route("/api/posts/:id", get(get_post).put(update_post).delete(delete_post))
        
        // ユーザー関連
        .route("/api/users", get(get_users).post(create_user))
        
        // ページ関連
        .route("/api/pages", get(get_pages).post(create_page))
        
        // 統計
        .route("/api/stats", get(get_stats))
        
        // CORS
        .layer(CorsLayer::permissive())
        
        // 状態共有
        .with_state(store);

    // サーバー起動
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    
    println!("🚀 Simple CMS Server starting...");
    println!("📍 Server running on: http://127.0.0.1:3000");
    println!("📚 API Documentation: http://127.0.0.1:3000/api/docs");
    println!("🔍 Health Check: http://127.0.0.1:3000/health");
    println!("📊 Statistics: http://127.0.0.1:3000/api/stats");
    println!();
    println!("✨ CMS is ready for production use!");
    
    axum::serve(listener, app).await.unwrap();
}
