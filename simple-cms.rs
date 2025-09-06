// 完全に独立したシンプルCMS - 単一ファイルでの実装
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
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

    // 公開状態でフィルタリング
    if let Some(published) = params.published {
        post_list.retain(|p| p.published == published);
    }

    // 作成日時でソート（新しい順）
    post_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // ページネーション
    let page = params.page.unwrap_or(1).max(1);
    let mut limit = params.limit.unwrap_or(20);
    if limit == 0 { limit = 20; }
    if limit > 100 { limit = 100; }
    let (paginated_posts, pagination) = paginate(&post_list, page, limit);

    Json(ApiResponse::success(crate::utils::api_types::PaginatedResponse { data: paginated_posts, pagination }))
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
    let now = chrono::Utc::now().to_rfc3339();

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
        post.updated_at = chrono::Utc::now().to_rfc3339();

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

    // 作成日時でソート（新しい順）
    user_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // ページネーション
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100);
    let (paginated_users, pagination) = paginate(&user_list, page, limit);

    Json(ApiResponse::success(crate::utils::api_types::PaginatedResponse { data: paginated_users, pagination }))
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
        created_at: chrono::Utc::now().to_rfc3339(),
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

    // 公開状態でフィルタリング
    if let Some(published) = params.published {
        page_list.retain(|p| p.published == published);
    }

    // 作成日時でソート（新しい順）
    page_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // ページネーション
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100);
    let (paginated_pages, pagination) = paginate(&page_list, page, limit);

    Json(ApiResponse::success(crate::utils::api_types::PaginatedResponse { data: paginated_pages, pagination }))
}

async fn create_page(
    State(store): State<InMemoryStore>,
    Json(payload): Json<CreatePageRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let slug = payload.slug.unwrap_or_else(|| generate_slug(&payload.title));
    let now = chrono::Utc::now().to_rfc3339();

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
    );

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
    let html = r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Simple CMS API Documentation</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #2c3e50; border-bottom: 3px solid #3498db; padding-bottom: 10px; }
        h2 { color: #34495e; margin-top: 30px; }
        h3 { color: #7f8c8d; }
        .endpoint { background: #ecf0f1; padding: 15px; margin: 10px 0; border-radius: 5px; border-left: 4px solid #3498db; }
        .method { font-weight: bold; padding: 2px 8px; border-radius: 3px; color: white; font-size: 12px; }
        .get { background: #27ae60; }
        .post { background: #f39c12; }
        .put { background: #9b59b6; }
        .delete { background: #e74c3c; }
        code { background: #2c3e50; color: #ecf0f1; padding: 2px 6px; border-radius: 3px; font-family: 'Courier New', monospace; }
        .example { background: #34495e; color: #ecf0f1; padding: 15px; border-radius: 5px; overflow-x: auto; margin: 10px 0; }
        .status { margin: 20px 0; padding: 15px; background: #d5f4e6; border-radius: 5px; }
        ul { line-height: 1.6; }
        .feature { background: #e8f4fd; padding: 10px; margin: 5px 0; border-radius: 5px; border-left: 3px solid #3498db; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Simple CMS API Documentation</h1>
        
        <div class="status">
            <h3>✅ サービス状況</h3>
            <p>Simple CMS API は正常に動作しています。実用的なCMSとして以下の機能を提供します。</p>
        </div>

        <h2>📋 主要機能</h2>
        <div class="feature">💬 投稿管理 - CRUD操作、公開/下書き状態管理</div>
        <div class="feature">👥 ユーザー管理 - ユーザー作成・一覧表示</div>
        <div class="feature">📄 ページ管理 - 静的ページの作成・管理</div>
        <div class="feature">🔍 検索・フィルタリング - 公開状態による絞り込み</div>
        <div class="feature">📊 ページネーション - 効率的なデータ表示</div>
        <div class="feature">📈 統計情報 - システム全体の概要</div>

        <h2>🔗 API エンドポイント</h2>

        <h3>ヘルスチェック</h3>
        <div class="endpoint">
            <span class="method get">GET</span> <code>/health</code>
            <p>システムの稼働状況を確認</p>
        </div>

        <h3>投稿管理</h3>
        <div class="endpoint">
            <span class="method get">GET</span> <code>/api/posts</code>
            <p>投稿一覧を取得（ページネーション、公開状態フィルタリング対応）</p>
            <p>クエリパラメータ: <code>page</code>, <code>limit</code>, <code>published</code></p>
        </div>
        <div class="endpoint">
            <span class="method get">GET</span> <code>/api/posts/{id}</code>
            <p>特定の投稿を取得</p>
        </div>
        <div class="endpoint">
            <span class="method post">POST</span> <code>/api/posts</code>
            <p>新しい投稿を作成</p>
        </div>
        <div class="endpoint">
            <span class="method put">PUT</span> <code>/api/posts/{id}</code>
            <p>投稿を更新</p>
        </div>
        <div class="endpoint">
            <span class="method delete">DELETE</span> <code>/api/posts/{id}</code>
            <p>投稿を削除</p>
        </div>

        <h3>ユーザー管理</h3>
        <div class="endpoint">
            <span class="method get">GET</span> <code>/api/users</code>
            <p>ユーザー一覧を取得</p>
        </div>
        <div class="endpoint">
            <span class="method post">POST</span> <code>/api/users</code>
            <p>新しいユーザーを作成</p>
        </div>

        <h3>ページ管理</h3>
        <div class="endpoint">
            <span class="method get">GET</span> <code>/api/pages</code>
            <p>ページ一覧を取得</p>
        </div>
        <div class="endpoint">
            <span class="method post">POST</span> <code>/api/pages</code>
            <p>新しいページを作成</p>
        </div>

        <h3>統計情報</h3>
        <div class="endpoint">
            <span class="method get">GET</span> <code>/api/stats</code>
            <p>システム統計情報を取得</p>
        </div>

        <h2>💡 使用例</h2>
        
        <h3>投稿作成</h3>
        <div class="example">
curl -X POST http://localhost:3000/api/posts \
  -H "Content-Type: application/json" \
  -d '{
    "title": "新しい投稿",
    "content": "投稿の内容です",
    "published": true
  }'
        </div>

        <h3>投稿一覧取得（公開済みのみ）</h3>
        <div class="example">
curl "http://localhost:3000/api/posts?published=true&page=1&limit=5"
        </div>

        <h2>🌟 実用性</h2>
        <p>このSimple CMSは以下の点で実用的です：</p>
        <ul>
            <li>✅ 完全な CRUD 操作サポート</li>
            <li>✅ RESTful API 設計</li>
            <li>✅ ページネーション実装</li>
            <li>✅ フィルタリング機能</li>
            <li>✅ エラーハンドリング</li>
            <li>✅ JSON レスポンス統一</li>
            <li>✅ CORS サポート</li>
            <li>✅ 包括的なドキュメント</li>
        </ul>

        <h2>🔧 技術仕様</h2>
        <ul>
            <li>フレームワーク: Axum (高性能 Rust web フレームワーク)</li>
            <li>データストレージ: インメモリ（高速アクセス）</li>
            <li>シリアライゼーション: Serde (JSON)</li>
            <li>非同期処理: Tokio</li>
            <li>CORS対応: tower-http</li>
        </ul>
    </div>
</body>
</html>
"#;
    Html(html)
}

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
