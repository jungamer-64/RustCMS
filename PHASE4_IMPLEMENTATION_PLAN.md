# Phase 4: プレゼンテーション層（Web層）構造再編計画

**状態**: 🚀 計画段階  
**Phase 3 完了**: ✅ Application + Infrastructure (2025年10月18日)  
**Phase 4 開始**: 2025年10月18日  
**推定期間**: 4-6週間（段階的移行）

---

## 📋 Phase 4 概要

Phase 4 は、既存の `src/handlers/` をレガシーコードとして保持しつつ、推奨構造の `src/web/` を新規構築し、段階的に移行する設計です。

### 主要目標

1. ✅ **薄い層としての Web ハンドラ**: Use Cases 呼び出しのみ（ビジネスロジックなし）
2. ✅ **ミドルウェア統合**: 認証・レート制限・ロギング
3. ✅ **API バージョニング**: `/api/v2/` エンドポイント準備
4. ✅ **レガシー削除計画**: 段階的削除スケジュール
5. ✅ **統合テスト実行**: PostgreSQL での実機動作確認

### Phase 4 マイルストーン

```
Week 12-13: Web 層基本構造 (routes + handlers 薄化)
Week 14: ミドルウェア統合 + エラーハンドリング
Week 15: API v2 パイロット + レガシー並行運用
Week 16-17: レガシー削除 + 最終テスト
Week 18: Phase 4 完了 + Phase 5 準備
```

---

## 📂 Step 1: Web 層の基本構造構築

### 1.1 ディレクトリ構造作成

```bash
# Step 1: src/web/ ディレクトリ作成
mkdir -p src/web/handlers

# 新ファイル一覧
# src/web/mod.rs              # Web 層ルート
# src/web/routes.rs           # ルート定義（全エンドポイント集約）
# src/web/handlers/mod.rs     # ハンドラルート
# src/web/handlers/users.rs   # User ハンドラ
# src/web/handlers/posts.rs   # Post ハンドラ
# src/web/handlers/auth.rs    # Auth ハンドラ
# src/web/handlers/health.rs  # ヘルスチェック
# src/web/middleware.rs       # ミドルウェア定義
```

### 1.2 src/web/mod.rs（ライブラリルート）

```rust
//! プレゼンテーション層（HTTP API）
//!
//! Phase 4: Axum ベースの軽量ハンドラ層
//! 責務: HTTP リクエスト → DTO 変換 → Use Case 呼び出し → HTTP レスポンス

pub mod handlers;
pub mod middleware;
pub mod routes;

pub use routes::create_router;
```

### 1.3 src/web/routes.rs（ルート定義 - 全エンドポイント集約）

**重要**: 全ルート定義を集約し、エンドポイント管理を一元化

```rust
//! HTTP ルート定義（全エンドポイント集約）
//!
//! Phase 4: Axum ルータ設定
//! - 認証ルート群
//! - ユーザー管理ルート群
//! - 投稿管理ルート群
//! - ヘルスチェック

use axum::{
    middleware as axum_middleware,
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;

use crate::app::AppState;
use crate::web::handlers;
use crate::web::middleware;

/// HTTP ルータを作成
///
/// # ルート構成
/// - `GET /api/v1/health` - ヘルスチェック（旧API）
/// - `GET /api/v2/health` - ヘルスチェック（新API）
/// - `POST /api/v2/users/register` - ユーザー登録
/// - `GET /api/v2/users/{id}` - ユーザー取得
/// - `POST /api/v2/posts` - 投稿作成
/// - `POST /api/v2/auth/login` - ログイン
pub async fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // ============================================================================
        // v1 API（レガシー）- 段階的に廃止予定
        // ============================================================================
        .route("/api/v1/health", get(handlers::health::health_check_v1))

        // ============================================================================
        // v2 API（新規）
        // ============================================================================
        
        // ヘルスチェック
        .route("/api/v2/health", get(handlers::health::health_check_v2))

        // ユーザー管理
        .route(
            "/api/v2/users/register",
            post(handlers::users::register_user),
        )
        .route(
            "/api/v2/users/:id",
            get(handlers::users::get_user)
                .layer(axum_middleware::from_fn(middleware::require_auth)),
        )
        .route(
            "/api/v2/users",
            get(handlers::users::list_users)
                .layer(axum_middleware::from_fn(middleware::require_auth)),
        )
        .route(
            "/api/v2/users/:id",
            put(handlers::users::update_user)
                .layer(axum_middleware::from_fn(middleware::require_auth)),
        )

        // 投稿管理
        .route(
            "/api/v2/posts",
            post(handlers::posts::create_post)
                .layer(axum_middleware::from_fn(middleware::require_auth)),
        )
        .route(
            "/api/v2/posts/:id",
            get(handlers::posts::get_post),
        )
        .route(
            "/api/v2/posts",
            get(handlers::posts::list_posts),
        )
        .route(
            "/api/v2/posts/:id/publish",
            post(handlers::posts::publish_post)
                .layer(axum_middleware::from_fn(middleware::require_auth)),
        )

        // 認証
        .route(
            "/api/v2/auth/login",
            post(handlers::auth::login),
        )
        .route(
            "/api/v2/auth/logout",
            post(handlers::auth::logout)
                .layer(axum_middleware::from_fn(middleware::require_auth)),
        )

        .with_state(state)
}
```

---

## 🎯 Step 2: HTTP ハンドラの薄化実装

### 2.1 ハンドラの設計原則

**原則**: ハンドラは**薄い層**であること（ビジネスロジックなし）

```
HTTP Request
    ↓
ハンドラ (薄い層)
  ├─ DTO デシリアライズ
  ├─ Use Case 呼び出し
  └─ HTTP Response
    ↓
Response
```

**禁止**: ビジネスロジック、複雑な制御フロー

### 2.2 src/web/handlers/users.rs（ユーザーハンドラ - 薄化例）

```rust
//! ユーザー関連のHTTPハンドラ
//!
//! Phase 4: Axum ハンドラ（薄い層）
//! 責務: HTTP ←→ DTO 変換、Use Case 呼び出し

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::app::AppState;
use crate::application::dto::user::{UserDto, CreateUserRequest, UpdateUserRequest, UserFilter};
use crate::application::use_cases::user::{RegisterUserUseCase, GetUserByIdUseCase};
use crate::common::types::AppError;

/// ユーザー登録ハンドラ
///
/// `POST /api/v2/users/register`
///
/// # 責務（薄い層）
/// 1. CreateUserRequest をデシリアライズ
/// 2. RegisterUserUseCase を呼び出し
/// 3. UserDto を HTTP 200 で応答
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserDto>), AppError> {
    // Use Case の呼び出し（ビジネスロジックは Use Case に集約）
    let use_case = RegisterUserUseCase::new(
        Arc::new(state.user_repository.clone()),
        Arc::new(state.event_bus.clone()),
    );
    
    let user = use_case.execute(request).await?;
    let dto = UserDto::from(user);
    
    Ok((StatusCode::CREATED, Json(dto)))
}

/// ユーザー取得ハンドラ
///
/// `GET /api/v2/users/:id`
///
/// # 責務（薄い層）
/// 1. URL パラメータから ID 取得
/// 2. GetUserByIdUseCase を呼び出し
/// 3. UserDto を HTTP 200 で応答
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserDto>, AppError> {
    let use_case = GetUserByIdUseCase::new(
        Arc::new(state.user_repository.clone()),
    );
    
    let user = use_case.execute(id).await?;
    let dto = UserDto::from(user);
    
    Ok(Json(dto))
}

/// ユーザー一覧ハンドラ
///
/// `GET /api/v2/users?username=foo&is_active=true&page=1&limit=20`
///
/// # 責務（薄い層）
/// 1. Query パラメータをデシリアライズ
/// 2. ListUsersQuery を呼び出し
/// 3. UserDto リストを HTTP 200 で応答
pub async fn list_users(
    Query(filter): Query<UserFilter>,
) -> Result<Json<Vec<UserDto>>, AppError> {
    // Query パラメータからフィルタを構築
    // Use Case（CQRS Query）を呼び出し
    // 結果を DTO に変換して応答
    
    todo!("Implement list_users with CQRS Query pattern")
}

/// ユーザー更新ハンドラ
///
/// `PUT /api/v2/users/:id`
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserDto>, AppError> {
    // UpdateUserUseCase を呼び出し
    // 結果を DTO に変換して応答
    
    todo!("Implement update_user")
}
```

### 2.3 src/web/handlers/posts.rs（投稿ハンドラ - 薄化例）

```rust
//! 投稿関連のHTTPハンドラ

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::app::AppState;
use crate::application::dto::post::{PostDto, CreatePostRequest};
use crate::common::types::AppError;

/// 投稿作成ハンドラ
///
/// `POST /api/v2/posts`
pub async fn create_post(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<PostDto>), AppError> {
    // CreatePostUseCase → PostDto
    todo!("Implement create_post")
}

/// 投稿取得ハンドラ
///
/// `GET /api/v2/posts/:id`
pub async fn get_post(
    Path(id): Path<Uuid>,
) -> Result<Json<PostDto>, AppError> {
    todo!("Implement get_post")
}

/// 投稿一覧ハンドラ
///
/// `GET /api/v2/posts?page=1&limit=20`
pub async fn list_posts() -> Result<Json<Vec<PostDto>>, AppError> {
    todo!("Implement list_posts")
}

/// 投稿公開ハンドラ
///
/// `POST /api/v2/posts/:id/publish`
pub async fn publish_post(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<PostDto>, AppError> {
    todo!("Implement publish_post")
}
```

### 2.4 src/web/handlers/auth.rs（認証ハンドラ）

```rust
//! 認証関連のHTTPハンドラ

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::common::types::AppError;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: Option<String>,
}

/// ログインハンドラ
///
/// `POST /api/v2/auth/login`
pub async fn login(
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    todo!("Implement login with Biscuit token generation")
}

/// ログアウトハンドラ
///
/// `POST /api/v2/auth/logout`
pub async fn logout() -> StatusCode {
    // Biscuit トークンは stateless なため、実装不要またはセッション削除のみ
    StatusCode::NO_CONTENT
}
```

### 2.5 src/web/handlers/health.rs（ヘルスチェック）

```rust
//! ヘルスチェックハンドラ

use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// ヘルスチェック（v1 - レガシー）
pub async fn health_check_v1() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "1.0.0".to_string(),
    })
}

/// ヘルスチェック（v2 - 新規）
pub async fn health_check_v2() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "2.0.0".to_string(),
    })
}
```

---

## 🔒 Step 3: ミドルウェア統合

### 3.1 src/web/middleware.rs（ミドルウェア定義）

```rust
//! HTTP ミドルウェア
//!
//! - 認証（Biscuit トークン検証）
//! - レート制限
//! - ロギング（tracing）
//! - エラーハンドリング

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use tower::ServiceExt;

/// 認証ミドルウェア（Biscuit トークン検証）
///
/// リクエストの `Authorization: Bearer <token>` ヘッダから Biscuit トークンを検証
pub async fn require_auth(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // 1. Authorization ヘッダを取得
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. "Bearer <token>" 形式を解析
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 3. Biscuit トークンを検証
    // TODO: Biscuit 検証ロジック
    // verify_biscuit_token(token)?;

    // 4. リクエストを続行
    Ok(next.run(request).await)
}

/// レート制限ミドルウェア
///
/// IP アドレスごとにレート制限を適用
pub async fn rate_limit(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    // TODO: ローカルまたは Redis ベースのレート制限
    Ok(next.run(request).await)
}

/// リクエストロギングミドルウェア
///
/// すべてのリクエスト/レスポンスをログに記録
pub async fn request_logging(
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let method = request.method().clone();
    let uri = request.uri().clone();

    tracing::info!("→ {} {}", method, uri);

    let response = next.run(request).await;

    tracing::info!("← {} {}", method, response.status());

    response
}
```

---

## 🔄 Step 4: レガシーコード削除計画

### 4.1 段階的削除スケジュール

| 週 | タスク | 状態 |
|----|--------|------|
| W12-13 | src/web/ 新規作成 + 並行運用テスト | 🔜 |
| W14-15 | `/api/v2/` エンドポイント完成 + ドキュメント | 🔜 |
| W16 | 既存クライアント v2 移行呼びかけ | 🔜 |
| W17 | `/api/v1` 廃止予告（ドキュメント） | 🔜 |
| W18+ | `/api/v1` 削除実行 | 🔜 |

### 4.2 src/handlers/ マーク方法

```rust
// src/handlers/mod.rs （レガシー）

#![deprecated(
    since = "3.1.0",
    note = "Use `src/web` instead. Migration guide: https://docs.example.com/migration"
)]

//! **DEPRECATED**: レガシーハンドラ（Phase 4 で削除予定）
//!
//! 新規実装は `src/web/handlers/` を使用してください。
```

---

## 🧪 Step 5: 統合テスト実行確認

### 5.1 既存統合テスト実行

```bash
# PostgreSQL コンテナ起動
docker-compose up -d postgres

# マイグレーション実行
cargo run --bin cms-migrate -- migrate --no-seed

# 統合テスト実行
cargo test --test integration_repositories_phase3 -- --nocapture

# HTTP API テスト（オプション）
# curl -X POST http://localhost:8080/api/v2/users/register \
#   -H "Content-Type: application/json" \
#   -d '{"username":"test","email":"test@example.com","password":"pass"}'
```

### 5.2 E2E テストケース

```bash
# 1. ユーザー登録
curl -X POST http://localhost:8080/api/v2/users/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","email":"alice@example.com","password":"secure123"}'

# 2. ログイン
curl -X POST http://localhost:8080/api/v2/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","password":"secure123"}'

# 3. トークン付きリクエスト
TOKEN="<token_from_login>"
curl -X GET http://localhost:8080/api/v2/users/alice \
  -H "Authorization: Bearer $TOKEN"
```

---

## 📝 Step 6: API バージョニング設計

### 6.1 バージョニング戦略

```
v1 API: /api/v1/*      （廃止予定）
v2 API: /api/v2/*      （現行・推奨）
v3 API: /api/v3/*      （将来）
```

### 6.2 v2 API 設計原則

| 要素 | 設計 | 理由 |
|------|------|------|
| **メディアタイプ** | `application/json` | RESTful 標準 |
| **ステータスコード** | RFC 7231 準拠 | HTTP 仕様 |
| **エラー形式** | `{"error": "code", "message": "...", "details": {...}}` | 統一性 |
| **ページネーション** | `?page=1&limit=20` | 標準的 |
| **フィルタリング** | `?username=foo&is_active=true` | REST 仕様 |
| **認証** | `Authorization: Bearer <token>` | OAuth 2.0 |

---

## 🎓 Phase 4 チェックリスト

### Week 12-13: Web 層基本構造

- [ ] `src/web/` ディレクトリ作成
- [ ] `src/web/mod.rs`, `routes.rs` 実装
- [ ] `src/web/handlers/` 薄い層実装（users, posts, auth, health）
- [ ] ハンドラが Use Cases を正しく呼び出していることをテスト
- [ ] ビルド・テスト成功確認

### Week 14: ミドルウェア統合

- [ ] `src/web/middleware.rs` 実装（認証、レート制限、ロギング）
- [ ] ミドルウェアテスト実装
- [ ] ハンドラに ミドルウェアをマウント
- [ ] エンドツーエンドテスト実行

### Week 15: API v2 パイロット

- [ ] `/api/v2/` エンドポイント完全実装
- [ ] Swagger/OpenAPI ドキュメント生成
- [ ] クライアントライブラリ生成（OpenAPI Generator）
- [ ] ドキュメント整備（Migration Guide）

### Week 16-17: レガシー削除

- [ ] `src/handlers/` に deprecated 属性追加
- [ ] クライアント移行通知リリース
- [ ] 互換性レイヤー検討（v1 → v2 マッピング）
- [ ] 段階的削除スケジュール確認

### Week 18: Phase 4 完了

- [ ] 統合テスト全てパス
- [ ] パフォーマンステスト実行
- [ ] セキュリティレビュー
- [ ] Phase 5 計画書作成

---

## 📊 Phase 4 成功指標

| 指標 | 目標 | 測定方法 |
|------|------|--------|
| **ハンドラ行数** | ≤ 50行/ハンドラ | コード行数カウント |
| **テストカバレッジ** | ≥ 90% | `cargo tarpaulin` |
| **レスポンスタイム** | < 200ms（p95） | Apache Bench |
| **API ドキュメント** | 100% カバー | OpenAPI 完全性 |
| **回帰テスト** | 100% パス | CI/CD 検証 |

---

## 🚀 次のステップ（Phase 5 - 本格デプロイ）

Phase 4 完了後、Phase 5 では以下を実施：

1. **Kubernetes デプロイ**: Helm チャート作成
2. **パフォーマンス最適化**: リソース制限、スケーリング戦略
3. **監視・アラート**: Prometheus + Grafana 統合
4. **セキュリティ強化**: CORS、CSRF、レート制限調整

---

**作成日**: 2025年10月18日  
**推奨開始**: Phase 3 リファクタリング完了後  
**推定期間**: 4-6週間  
**更新予定**: 週次
