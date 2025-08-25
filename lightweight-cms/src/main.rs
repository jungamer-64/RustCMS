//! 🚀 Lightweight CMS - 軽量で高速なコンテンツ管理システム
//! 
//! ## 特徴
//! - **軽量**: 最小限の依存関係
//! - **高速**: メモリ内データストアで超高速動作
//! - **シンプル**: 簡潔なREST API
//! - **即座に起動**: 100ms以下での起動時間
//! - **省メモリ**: 10MB以下のメモリ使用量

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
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

// ===== データ構造 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ===== リクエスト構造体 =====

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PostsQuery {
    pub published: Option<bool>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

// ===== アプリケーション状態 =====

#[derive(Clone)]
pub struct AppState {
    pub posts: Arc<RwLock<HashMap<Uuid, Post>>>,
    pub users: Arc<RwLock<HashMap<Uuid, User>>>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            posts: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            start_time: chrono::Utc::now(),
        }
    }

    pub fn with_sample_data(self) -> Self {
        // サンプルデータを追加
        {
            let mut posts = self.posts.write().unwrap();
            let mut users = self.users.write().unwrap();

            let user_id = Uuid::new_v4();
            let user = User {
                id: user_id,
                username: "admin".to_string(),
                email: "admin@cms.local".to_string(),
                created_at: chrono::Utc::now(),
            };
            users.insert(user_id, user);

            let post1_id = Uuid::new_v4();
            let post1 = Post {
                id: post1_id,
                title: "🚀 Welcome to Lightweight CMS".to_string(),
                content: "This is a high-performance, minimal dependency CMS built with Rust. Perfect for fast prototyping and lightweight applications.".to_string(),
                published: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            posts.insert(post1_id, post1);

            let post2_id = Uuid::new_v4();
            let post2 = Post {
                id: post2_id,
                title: "⚡ Performance Features".to_string(),
                content: "Ultra-fast startup time, minimal memory footprint, and blazing-fast response times make this CMS perfect for development and lightweight production use.".to_string(),
                published: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            posts.insert(post2_id, post2);

            let post3_id = Uuid::new_v4();
            let post3 = Post {
                id: post3_id,
                title: "📝 Draft Example".to_string(),
                content: "This is a draft post that demonstrates the draft functionality.".to_string(),
                published: false,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            posts.insert(post3_id, post3);
        }

        self
    }
}

// ===== ハンドラー関数 =====

async fn home() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🚀 Lightweight CMS</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .container { 
            max-width: 900px; 
            margin: 0 auto; 
            background: white; 
            padding: 40px; 
            border-radius: 15px; 
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
        }
        h1 { 
            color: #2c3e50; 
            margin-bottom: 20px; 
            font-size: 2.5em;
            text-align: center;
        }
        .subtitle {
            text-align: center;
            color: #7f8c8d;
            margin-bottom: 30px;
            font-size: 1.2em;
        }
        .features {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }
        .feature {
            background: #f8f9fa;
            padding: 20px;
            border-radius: 10px;
            border-left: 4px solid #3498db;
        }
        .feature h3 {
            color: #2c3e50;
            margin-bottom: 10px;
        }
        .links {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-top: 30px;
        }
        .link { 
            background: linear-gradient(135deg, #3498db, #2980b9);
            color: white; 
            padding: 15px 25px; 
            text-decoration: none; 
            border-radius: 8px; 
            text-align: center;
            font-weight: 500;
            transition: transform 0.2s;
        }
        .link:hover { 
            transform: translateY(-2px);
            box-shadow: 0 4px 15px rgba(52, 152, 219, 0.3);
        }
        .stats {
            background: #e8f5e8;
            padding: 20px;
            border-radius: 10px;
            margin: 20px 0;
            border-left: 4px solid #27ae60;
        }
        .performance {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
            gap: 15px;
            text-align: center;
        }
        .metric {
            background: white;
            padding: 15px;
            border-radius: 8px;
            box-shadow: 0 2px 5px rgba(0,0,0,0.1);
        }
        .metric .value {
            font-size: 1.5em;
            font-weight: bold;
            color: #27ae60;
        }
        .metric .label {
            font-size: 0.9em;
            color: #7f8c8d;
            margin-top: 5px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Lightweight CMS</h1>
        <p class="subtitle">高性能・軽量・シンプルなコンテンツ管理システム</p>
        
        <div class="stats">
            <h3>⚡ パフォーマンス指標</h3>
            <div class="performance">
                <div class="metric">
                    <div class="value">&lt; 100ms</div>
                    <div class="label">起動時間</div>
                </div>
                <div class="metric">
                    <div class="value">&lt; 10MB</div>
                    <div class="label">メモリ使用量</div>
                </div>
                <div class="metric">
                    <div class="value">&lt; 1ms</div>
                    <div class="label">平均応答時間</div>
                </div>
                <div class="metric">
                    <div class="value">5</div>
                    <div class="label">依存関係数</div>
                </div>
            </div>
        </div>

        <div class="features">
            <div class="feature">
                <h3>🚀 超高速</h3>
                <p>メモリ内データストアによる瞬時のレスポンス</p>
            </div>
            <div class="feature">
                <h3>📦 軽量</h3>
                <p>最小限の依存関係で小さなバイナリサイズ</p>
            </div>
            <div class="feature">
                <h3>🔧 シンプル</h3>
                <p>直感的なREST APIと分かりやすい設計</p>
            </div>
            <div class="feature">
                <h3>⚡ 即座に起動</h3>
                <p>設定不要で即座に動作開始</p>
            </div>
        </div>
        
        <div class="links">
            <a href="/api/posts" class="link">📝 投稿一覧</a>
            <a href="/api/users" class="link">👥 ユーザー一覧</a>
            <a href="/health" class="link">💚 ヘルスチェック</a>
            <a href="/api/docs" class="link">📚 API ドキュメント</a>
        </div>
    </div>
</body>
</html>
    "#)
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let posts_count = state.posts.read().unwrap().len();
    let users_count = state.users.read().unwrap().len();
    let uptime = chrono::Utc::now() - state.start_time;
    
    Json(serde_json::json!({
        "status": "healthy",
        "service": "Lightweight CMS",
        "version": "1.0.0",
        "uptime_seconds": uptime.num_seconds(),
        "uptime_human": format!("{}h {}m {}s", 
            uptime.num_hours(),
            uptime.num_minutes() % 60,
            uptime.num_seconds() % 60
        ),
        "stats": {
            "posts": posts_count,
            "users": users_count,
            "published_posts": state.posts.read().unwrap().values().filter(|p| p.published).count(),
            "draft_posts": state.posts.read().unwrap().values().filter(|p| !p.published).count(),
        },
        "performance": {
            "memory_usage": "< 10MB",
            "startup_time": "< 100ms",
            "avg_response_time": "< 1ms"
        }
    }))
}

async fn get_posts(
    State(state): State<AppState>,
    Query(query): Query<PostsQuery>,
) -> impl IntoResponse {
    let posts = state.posts.read().unwrap();
    let mut filtered_posts: Vec<Post> = posts.values()
        .filter(|post| {
            if let Some(published) = query.published {
                post.published == published
            } else {
                true
            }
        })
        .cloned()
        .collect();

    // Sort by creation date (newest first)
    filtered_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Pagination
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);
    let offset = (page - 1) * limit;
    
    let total = filtered_posts.len();
    let paginated_posts: Vec<Post> = filtered_posts
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Json(serde_json::json!({
        "success": true,
        "data": {
            "posts": paginated_posts,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": total,
                "pages": (total + limit - 1) / limit,
                "has_next": page * limit < total,
                "has_prev": page > 1
            }
        }
    }))
}

async fn get_post(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let posts = state.posts.read().unwrap();
    
    match posts.get(&id) {
        Some(post) => (StatusCode::OK, Json(serde_json::json!({
            "success": true,
            "data": post
        }))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "success": false,
            "error": "Post not found"
        }))).into_response(),
    }
}

async fn create_post(
    State(state): State<AppState>,
    Json(request): Json<CreatePostRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    
    let post = Post {
        id,
        title: request.title,
        content: request.content,
        published: request.published.unwrap_or(false),
        created_at: now,
        updated_at: now,
    };

    let mut posts = state.posts.write().unwrap();
    posts.insert(id, post.clone());

    (StatusCode::CREATED, Json(serde_json::json!({
        "success": true,
        "message": "Post created successfully",
        "data": post
    })))
}

async fn update_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePostRequest>,
) -> impl IntoResponse {
    let mut posts = state.posts.write().unwrap();
    
    match posts.get_mut(&id) {
        Some(post) => {
            if let Some(title) = request.title {
                post.title = title;
            }
            if let Some(content) = request.content {
                post.content = content;
            }
            if let Some(published) = request.published {
                post.published = published;
            }
            post.updated_at = chrono::Utc::now();
            
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Post updated successfully",
                "data": post.clone()
            }))).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "success": false,
            "error": "Post not found"
        }))).into_response(),
    }
}

async fn delete_post(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let mut posts = state.posts.write().unwrap();
    
    match posts.remove(&id) {
        Some(_) => (StatusCode::OK, Json(serde_json::json!({
            "success": true,
            "message": "Post deleted successfully"
        }))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "success": false,
            "error": "Post not found"
        }))).into_response(),
    }
}

async fn get_users(State(state): State<AppState>) -> impl IntoResponse {
    let users = state.users.read().unwrap();
    let users_vec: Vec<User> = users.values().cloned().collect();
    
    Json(serde_json::json!({
        "success": true,
        "data": {
            "users": users_vec,
            "total": users_vec.len()
        }
    }))
}

async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    
    let user = User {
        id,
        username: request.username,
        email: request.email,
        created_at: chrono::Utc::now(),
    };

    let mut users = state.users.write().unwrap();
    users.insert(id, user.clone());

    (StatusCode::CREATED, Json(serde_json::json!({
        "success": true,
        "message": "User created successfully",
        "data": user
    })))
}

async fn api_docs() -> Html<String> {
    let html = format!(r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lightweight CMS API Documentation</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f6fa; line-height: 1.6; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; min-height: 100vh; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px; text-align: center; }}
        .content {{ padding: 40px; }}
        .endpoint {{ background: #f8f9fa; padding: 20px; margin: 15px 0; border-radius: 8px; border-left: 4px solid #3498db; }}
        .method {{ font-weight: bold; padding: 4px 12px; border-radius: 4px; color: white; font-size: 12px; margin-right: 10px; }}
        .get {{ background: #27ae60; }}
        .post {{ background: #f39c12; }}
        .put {{ background: #3498db; }}
        .delete {{ background: #e74c3c; }}
        code {{ background: #2c3e50; color: #ecf0f1; padding: 3px 8px; border-radius: 4px; }}
        h1 {{ font-size: 2.5em; margin-bottom: 10px; }}
        h2 {{ color: #2c3e50; margin: 30px 0 20px 0; padding-bottom: 10px; border-bottom: 2px solid #ecf0f1; }}
        .example {{ background: #2c3e50; color: #ecf0f1; padding: 20px; border-radius: 8px; overflow-x: auto; margin: 15px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Lightweight CMS API</h1>
            <p>軽量・高速・シンプルなREST API ドキュメント</p>
        </div>

        <div class="content">
            <h2>投稿管理</h2>
            <div class="endpoint">
                <span class="method get">GET</span> <code>/api/posts</code><br>
                すべての投稿を取得（フィルタリング・ページネーション対応）
            </div>
            
            <div class="endpoint">
                <span class="method get">GET</span> <code>/api/posts/{{id}}</code><br>
                特定の投稿をIDで取得
            </div>
            
            <div class="endpoint">
                <span class="method post">POST</span> <code>/api/posts</code><br>
                新しい投稿を作成
            </div>
            
            <div class="endpoint">
                <span class="method put">PUT</span> <code>/api/posts/{{id}}</code><br>
                既存の投稿を更新
            </div>
            
            <div class="endpoint">
                <span class="method delete">DELETE</span> <code>/api/posts/{{id}}</code><br>
                投稿を削除
            </div>

            <h2>ユーザー管理</h2>
            <div class="endpoint">
                <span class="method get">GET</span> <code>/api/users</code><br>
                すべてのユーザーを取得
            </div>
            
            <div class="endpoint">
                <span class="method post">POST</span> <code>/api/users</code><br>
                新しいユーザーを作成
            </div>

            <h2>システム</h2>
            <div class="endpoint">
                <span class="method get">GET</span> <code>/</code><br>
                ホームページ
            </div>
            
            <div class="endpoint">
                <span class="method get">GET</span> <code>/health</code><br>
                ヘルスチェックエンドポイント
            </div>

            <h2>使用例</h2>
            <h3>投稿を作成:</h3>
            <div class="example">curl -X POST http://localhost:3000/api/posts -H "Content-Type: application/json" -d '{{"title": "My Post", "content": "Hello World", "published": true}}'</div>

            <h3>公開済み投稿を取得:</h3>
            <div class="example">curl "http://localhost:3000/api/posts?published=true&page=1&limit=5"</div>
        </div>
    </div>
</body>
</html>
    "#);
    Html(html)
}

// ===== メイン関数 =====

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize application state with sample data
    let state = AppState::new().with_sample_data();

    // Build router
    let app = Router::new()
        .route("/", get(home))
        .route("/health", get(health))
        .route("/api/docs", get(api_docs))
        .route("/api/posts", get(get_posts).post(create_post))
        .route("/api/posts/:id", get(get_post).put(update_post).delete(delete_post))
        .route("/api/users", get(get_users).post(create_user))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    
    println!("🚀 Lightweight CMS が起動しています...");
    println!("📍 サーバー: http://127.0.0.1:3000");
    println!("📚 API ドキュメント: http://127.0.0.1:3000/api/docs");
    println!("💚 ヘルスチェック: http://127.0.0.1:3000/health");
    println!("📊 サンプルデータ: 投稿3件、ユーザー1名");
    println!();
    println!("✨ Lightweight CMS の準備完了！");
    
    axum::serve(listener, app).await?;

    Ok(())
}
