//! Unified CMS binary combining capabilities of cms-lightweight, cms-simple, and simple-cms example.
//!
//! Goals:
//! - Single entrypoint for in‑memory demo / smoke testing.
//! - Health, posts, pages, users, stats endpoints (from simple-cms & cms-simple).
//! - Uses shared ApiResponse / Pagination types from core crate.
//! - Keeps implementation self‑contained & feature-gated under `dev-tools`.
//!
//! This is NOT the full production server (see main server binaries). It is
//! intended for quick local experimentation without DB / Redis.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::{Arc, RwLock}};
use tower_http::cors::{CorsLayer, Any};
use uuid::Uuid;

use cms_backend::utils::api_types::{ApiResponse, Pagination, PaginatedResponse, PaginationQuery};
use cms_backend::utils::url_encoding::generate_safe_slug;

// ---------- Data Models ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post { id: String, title: String, content: String, slug: String, published: bool, created_at: String, updated_at: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Page { id: String, title: String, content: String, slug: String, published: bool, created_at: String, updated_at: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User { id: String, username: String, email: String, role: String, created_at: String }

#[derive(Debug, Deserialize)]
struct CreatePostRequest { title: String, content: String, slug: Option<String>, published: Option<bool> }
#[derive(Debug, Deserialize)]
struct UpdatePostRequest { title: Option<String>, content: Option<String>, slug: Option<String>, published: Option<bool> }
#[derive(Debug, Deserialize)]
struct CreatePageRequest { title: String, content: String, slug: Option<String>, published: Option<bool> }
#[derive(Debug, Deserialize)]
struct CreateUserRequest { username: String, email: String, role: Option<String> }

// ---------- Store ----------
#[derive(Clone)]
struct Store { posts: Arc<RwLock<HashMap<String, Post>>>, pages: Arc<RwLock<HashMap<String, Page>>>, users: Arc<RwLock<HashMap<String, User>>> }
impl Store { fn new() -> Self { let s = Self { posts: Arc::new(RwLock::new(HashMap::new())), pages: Arc::new(RwLock::new(HashMap::new())), users: Arc::new(RwLock::new(HashMap::new())) }; s.seed(); s }
    fn seed(&self) { let now = now(); self.posts.write().unwrap().insert("welcome".into(), Post { id: Uuid::new_v4().to_string(), title: "Welcome".into(), content: "Unified CMS sample post".into(), slug: "welcome".into(), published: true, created_at: now.clone(), updated_at: now.clone()});
        self.pages.write().unwrap().insert("about".into(), Page { id: Uuid::new_v4().to_string(), title: "About".into(), content: "About page".into(), slug: "about".into(), published: true, created_at: now.clone(), updated_at: now.clone()});
        self.users.write().unwrap().insert("admin".into(), User { id: Uuid::new_v4().to_string(), username: "admin".into(), email: "admin@example.com".into(), role: "admin".into(), created_at: now }); }
}

// ---------- Helpers ----------
fn now() -> String { chrono::Utc::now().to_rfc3339() }
fn paginate_vec<T: Clone>(mut v: Vec<T>, page: u32, per_page: u32) -> (Vec<T>, Pagination) { let total = v.len();
    // stable order by insertion (could sort by created desc if we tracked it strongly)
    let start = (page.saturating_sub(1) * per_page) as usize; let end = std::cmp::min(start + per_page as usize, total);
    let data = if start < total { v.drain(start..end).collect() } else { vec![] };
    let total_pages = if per_page == 0 { 0 } else { ((total as f64) / (per_page as f64)).ceil() as u32 };
    (data, Pagination { page, per_page, total: total as u64, total_pages }) }

// ---------- Handlers ----------
async fn health(State(store): State<Store>) -> Json<ApiResponse<serde_json::Value>> { Json(ApiResponse::success(serde_json::json!({
    "status":"healthy","service":"cms-unified","version": env!("CARGO_PKG_VERSION"),
    "counts": {"posts": store.posts.read().unwrap().len(), "pages": store.pages.read().unwrap().len(), "users": store.users.read().unwrap().len()}
})))}

async fn list_posts(State(store): State<Store>, Query(mut q): Query<PaginationQuery>) -> Json<ApiResponse<PaginatedResponse<Post>>> { q.validate(); let page = q.page; let per = q.per_page; let posts: Vec<Post> = store.posts.read().unwrap().values().cloned().collect(); let (data, pag) = paginate_vec(posts, page, per); Json(ApiResponse::success(PaginatedResponse { data, pagination: pag })) }

async fn get_post(State(store): State<Store>, Path(id): Path<String>) -> impl IntoResponse { let posts = store.posts.read().unwrap(); if let Some(p) = posts.get(&id) { Json(ApiResponse::success(p.clone())).into_response() } else { (StatusCode::NOT_FOUND, Json(ApiResponse::error("Post not found".into()))).into_response() } }

async fn create_post(State(store): State<Store>, Json(req): Json<CreatePostRequest>) -> impl IntoResponse { let id = Uuid::new_v4().to_string(); let slug = req.slug.unwrap_or_else(|| generate_safe_slug(&req.title)); let now = now(); let post = Post { id: id.clone(), title: req.title, content: req.content, slug, published: req.published.unwrap_or(false), created_at: now.clone(), updated_at: now }; store.posts.write().unwrap().insert(id.clone(), post.clone()); (StatusCode::CREATED, Json(ApiResponse::success(post))) }

async fn update_post(State(store): State<Store>, Path(id): Path<String>, Json(req): Json<UpdatePostRequest>) -> impl IntoResponse { let mut posts = store.posts.write().unwrap(); if let Some(p) = posts.get_mut(&id) { if let Some(t)=req.title{p.title=t;} if let Some(c)=req.content{p.content=c;} if let Some(s)=req.slug{p.slug=s;} if let Some(pubd)=req.published{p.published=pubd;} p.updated_at=now(); Json(ApiResponse::success(p.clone())).into_response() } else { (StatusCode::NOT_FOUND, Json(ApiResponse::error("Post not found".into()))).into_response() } }

async fn delete_post(State(store): State<Store>, Path(id): Path<String>) -> impl IntoResponse { let mut posts = store.posts.write().unwrap(); if posts.remove(&id).is_some() { Json(ApiResponse::success_with_message(serde_json::json!({"id":id}), "Post deleted".into())).into_response() } else { (StatusCode::NOT_FOUND, Json(ApiResponse::error("Post not found".into()))).into_response() } }

async fn list_pages(State(store): State<Store>, Query(mut q): Query<PaginationQuery>) -> Json<ApiResponse<PaginatedResponse<Page>>> { q.validate(); let pages: Vec<Page> = store.pages.read().unwrap().values().cloned().collect(); let (data, pag) = paginate_vec(pages, q.page, q.per_page); Json(ApiResponse::success(PaginatedResponse { data, pagination: pag })) }

async fn create_page(State(store): State<Store>, Json(req): Json<CreatePageRequest>) -> impl IntoResponse { let id = Uuid::new_v4().to_string(); let slug = req.slug.unwrap_or_else(|| generate_safe_slug(&req.title)); let now = now(); let page = Page { id: id.clone(), title: req.title, content: req.content, slug, published: req.published.unwrap_or(false), created_at: now.clone(), updated_at: now }; store.pages.write().unwrap().insert(id.clone(), page.clone()); (StatusCode::CREATED, Json(ApiResponse::success(page))) }

async fn list_users(State(store): State<Store>, Query(mut q): Query<PaginationQuery>) -> Json<ApiResponse<PaginatedResponse<User>>> { q.validate(); let users: Vec<User> = store.users.read().unwrap().values().cloned().collect(); let (data, pag) = paginate_vec(users, q.page, q.per_page); Json(ApiResponse::success(PaginatedResponse { data, pagination: pag })) }

async fn create_user(State(store): State<Store>, Json(req): Json<CreateUserRequest>) -> impl IntoResponse { let id = Uuid::new_v4().to_string(); let now = now(); let user = User { id: id.clone(), username: req.username, email: req.email, role: req.role.unwrap_or_else(|| "user".into()), created_at: now }; store.users.write().unwrap().insert(id.clone(), user.clone()); (StatusCode::CREATED, Json(ApiResponse::success(user))) }

async fn stats(State(store): State<Store>) -> Json<ApiResponse<serde_json::Value>> { let posts = store.posts.read().unwrap(); let pages = store.pages.read().unwrap(); let users = store.users.read().unwrap(); Json(ApiResponse::success(serde_json::json!({
    "totals": {"posts": posts.len(), "pages": pages.len(), "users": users.len()},
    "published_posts": posts.values().filter(|p| p.published).count(),
    "published_pages": pages.values().filter(|p| p.published).count()
})))}

async fn docs() -> Html<String> { Html("<h1>cms-unified</h1><p>Unified lightweight/simple demo CMS API</p>".to_string()) }

// ---------- Main ----------
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { tracing_subscriber::fmt().with_target(false).compact().init(); let store = Store::new();
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app = Router::new()
        .route("/", get(docs))
        .route("/health", get(health))
        .route("/api/docs", get(docs))
        // posts
        .route("/api/posts", get(list_posts).post(create_post))
        .route("/api/posts/:id", get(get_post).put(update_post).delete(delete_post))
        // pages
        .route("/api/pages", get(list_pages).post(create_page))
        // users
        .route("/api/users", get(list_users).post(create_user))
        // stats
        .route("/api/stats", get(stats))
        .layer(cors)
        .with_state(store);

    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3005".into()).parse().unwrap_or(3005);
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🚀 cms-unified running at http://{}", addr);
    println!("📚 Docs: http://{}/api/docs", addr);
    axum::serve(listener, app).await?; Ok(()) }
