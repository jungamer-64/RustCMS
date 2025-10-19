# Phase 4 Week 12 Day 3-5 詳細実装計画（新構造対応）

**作成日**: 2025年10月18日  
**適用範囲**: Phase 4 Week 12 Day 3-5  
**ステータス**: 🔜 準備中  
**監査ベース**: ⭐⭐⭐⭐⭐ (4.8/5.0)

---

## 🎯 実装目標

### Week 12 Day 3-5 で達成すべき成果

| 項目 | 目標 | 方法 | テスト数 |
|------|------|------|---------|
| **ミドルウェア実装** | 3個完成 | Tower middleware | 6+ |
| **ルート統合** | 全エンドポイント集約 | routes.rs | 4+ |
| **ユニットテスト** | 12+個実装 | tokio::test | 12+ |
| **コンパイル確認** | 0 警告 | cargo check | - |
| **テスト実行** | 全てパス | cargo test | ✅ |

---

## 📌 Phase 4 新構造の確認事項

### 🔴 Critical - 必ず確認

1. **common/ ディレクトリ**（shared ではなく）
   - ✅ 既存: `src/common/` 存在
   - 📝 TODO: types.rs, telemetry.rs の中身確認

2. **Entity + Value Objects 統合**
   - ✅ 既存: `src/domain/user.rs` で実装済み
   - 📝 TODO: 他ドメインモデルも統合パターン確認

3. **Repository実装の統合**
   - ✅ 既存: `src/infrastructure/database/repositories.rs`
   - 📝 TODO: Diesel用モデルとの連携確認

4. **イベントシステム**
   - 🔄 現在: `src/events.rs` + `src/listeners.rs` で運用中
   - 📝 Phase 4 計画: `infrastructure/events/` に移行（Day 3-5 後の Week 13）

### 🟢 Info - 参考事項

1. **CQRS 統合**
   - ✅ 既存: `src/application/user.rs`, `post.rs` で Commands + Queries + DTOs 統合済み
   - 📝 TODO: ハンドラから Use Cases 呼び出しの確認

---

## 🛠️ Day 3 実装詳細：ミドルウェア実装

### タスク 3.1: require_auth ミドルウェア実装

**ファイル**: `src/web/middleware.rs`

**実装予定コード**:

```rust
// src/web/middleware.rs - require_auth 実装

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use crate::common::error::AppError;
use crate::infrastructure::auth::BiscuitToken;

/// Biscuit トークン検証ミドルウェア
/// 
/// # 責務
/// - Authorization ヘッダから Bearer token 抽出
/// - Biscuit トークン検証
/// - ユーザー ID をリクエストエクステンションに注入
/// - 検証失敗時: 401 Unauthorized, 400 Bad Request
pub async fn require_auth(
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // TODO: Day 3 で実装完了
    // 1. extract_bearer_token(&headers) 
    // 2. verify_biscuit_token(token)
    // 3. extract_user_id_from_biscuit(&biscuit)
    // 4. request.extensions_mut().insert(user_id)
    // 5. next.run(request).await
    
    Ok(next.run(request).await)
}
```

**テスト予定**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_require_auth_with_valid_token() {
        // ✅ 検証: Bearer token 付き → user_id in request
    }

    #[tokio::test]
    async fn test_require_auth_without_token() {
        // ✅ 検証: token なし → 400 Bad Request
    }

    #[tokio::test]
    async fn test_require_auth_with_invalid_token() {
        // ✅ 検証: 無効な token → 401 Unauthorized
    }
}
```

**実装ステップ** (30分):

1. ファイル作成: `src/web/middleware.rs`
2. require_auth 関数スケルトン作成
3. Biscuit 検証ロジック記載（TODO コメント付き）
4. テストスケルトン作成
5. cargo check 確認

---

### タスク 3.2: rate_limit ミドルウェア実装

**ファイル**: `src/web/middleware.rs`

**実装予定コード**:

```rust
// src/web/middleware.rs - rate_limit 実装

use axum::extract::ConnectInfo;
use std::net::SocketAddr;

/// レート制限ミドルウェア（IP ベース）
/// 
/// # 責務
/// - クライアント IP からリクエストレートを追跡
/// - 超過時: 429 Too Many Requests
pub async fn rate_limit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // TODO: Day 3 で実装完了
    // 1. check_rate_limit(addr.ip())
    // 2. 超過時: Err(AppError::RateLimitExceeded)
    // 3. 正常: next.run(request).await
    
    Ok(next.run(request).await)
}
```

**テスト予定**:

```rust
#[tokio::test]
async fn test_rate_limit_within_quota() {
    // ✅ 検証: レート内 → パス
}

#[tokio::test]
async fn test_rate_limit_exceeded() {
    // ✅ 検証: レート超過 → 429 Too Many Requests
}
```

**実装ステップ** (30分):

1. rate_limit 関数スケルトン作成
2. IP アドレス追跡ロジック記載（TODO コメント付き）
3. レート超過ロジック記載
4. テストスケルトン作成
5. cargo check 確認

---

### タスク 3.3: request_logging ミドルウェア実装

**ファイル**: `src/web/middleware.rs`

**実装予定コード**:

```rust
// src/web/middleware.rs - request_logging 実装

use std::time::Instant;
use tracing::info;

/// リクエストロギングミドルウェア
/// 
/// # 責務
/// - リクエストメタデータ（メソッド、URI、ステータス）のログ出力
/// - レスポンス時間（ミリ秒）の測定・記録
pub async fn request_logging(
    request: Request,
    next: Next,
) -> Response {
    // TODO: Day 3 で実装完了
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();
    
    let response = next.run(request).await;
    
    let duration_ms = start.elapsed().as_millis();
    let status = response.status();
    
    info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = duration_ms,
        "HTTP request completed"
    );
    
    response
}
```

**テスト予定**:

```rust
#[tokio::test]
async fn test_request_logging_output() {
    // ✅ 検証: tracing::info! でログ出力されている
}
```

**実装ステップ** (20分):

1. request_logging 関数スケルトン作成
2. トレーシングロジック実装
3. テストスケルトン作成
4. cargo check 確認

---

### タスク 3.4: routes.rs 完成化（全エンドポイント集約）

**ファイル**: `src/web/routes.rs`

**現在の状態確認**:

```bash
# src/web/routes.rs の確認
wc -l src/web/routes.rs
# 現在: ~70行（基本構造完成）
```

**完成予定の構造**:

```rust
// src/web/routes.rs - 完成版

use axum::{
    routing::{get, post, put, delete},
    middleware,
    Router,
};
use crate::web::{
    handlers::{users, posts, auth, health},
    middleware_phase4::{require_auth, rate_limit, request_logging},
};

pub fn create_routes() -> Router {
    Router::new()
        // ============================================================
        // V1 (レガシー) - 段階的削除予定
        // ============================================================
        .route("/api/v1/health", get(health::health_check_v1))
        
        // ============================================================
        // V2 (新規) - Phase 4 で完成
        // ============================================================
        .nest("/api/v2", api_v2_routes())
        
        // グローバルミドルウェア: リクエストロギング
        .layer(middleware::from_fn(request_logging))
}

fn api_v2_routes() -> Router {
    Router::new()
        // ============================================================
        // Public エンドポイント（認証不要）
        // ============================================================
        .route("/health", get(health::health_check_v2))
        .route("/users/register", post(users::register_user))
        .route("/auth/login", post(auth::login))
        
        // ============================================================
        // Protected エンドポイント（認証必須）
        // ============================================================
        .route("/users/:id", 
            get(users::get_user)
                .put(users::update_user)
        )
        .route("/users", 
            get(users::list_users)
        )
        
        .route("/posts", 
            post(posts::create_post)
                .get(posts::list_posts)
        )
        .route("/posts/:id", 
            get(posts::get_post)
        )
        .route("/posts/:id/publish", 
            post(posts::publish_post)
        )
        
        .route("/auth/logout", 
            post(auth::logout)
        )
        
        // Protect: 以下のルートに require_auth + rate_limit を適用
        .layer(middleware::from_fn(require_auth))
        .layer(middleware::from_fn(rate_limit))
}
```

**テスト予定**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_public_endpoints_accessible() {
        // ✅ /health, /register, /login は認証なしでアクセス可能
    }

    #[tokio::test]
    async fn test_protected_endpoints_require_auth() {
        // ✅ /users/:id は require_auth マウント済み
    }

    #[tokio::test]
    async fn test_404_handling() {
        // ✅ 存在しないパス → 404 Not Found
    }
}
```

**実装ステップ** (30分):

1. routes.rs 打ち直し（全エンドポイント集約）
2. ハンドラ → routes パスの確認
3. middleware マウント検証
4. テストスケルトン作成
5. cargo check 確認

---

## 🧪 Day 4 実装詳細：テスト実装

### タスク 4.1: ミドルウェアテスト実装

**ファイル**: `src/web/middleware.rs` (tests section)

**テスト項目** (6個):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_require_auth_valid_token() { /* ... */ }

    #[tokio::test]
    async fn test_require_auth_without_token() { /* ... */ }

    #[tokio::test]
    async fn test_rate_limit_within_quota() { /* ... */ }

    #[tokio::test]
    async fn test_rate_limit_exceeded() { /* ... */ }

    #[tokio::test]
    async fn test_request_logging_duration() { /* ... */ }

    #[tokio::test]
    async fn test_middleware_chain_order() { /* ... */ }
}
```

**実装ステップ** (40分):

1. Mock AppState 作成
2. require_auth テスト実装（3個）
3. rate_limit テスト実装（1個）
4. request_logging テスト実装（1個）
5. integration テスト実装（1個）
6. cargo test 確認

---

### タスク 4.2: ルート定義テスト実装

**ファイル**: `tests/web_routes_phase4.rs` (新規作成)

**テスト項目** (4個):

```rust
#[tokio::test]
async fn test_public_routes_accessible() {
    // /api/v2/health - OK
    // /api/v2/users/register - OK
    // /api/v2/auth/login - OK
}

#[tokio::test]
async fn test_protected_routes_require_auth() {
    // /api/v2/users/:id (without token) → 401
}

#[tokio::test]
async fn test_404_not_found() {
    // /api/v2/invalid → 404
}

#[tokio::test]
async fn test_method_not_allowed() {
    // POST /api/v2/health → 405
}
```

**実装ステップ** (20分):

1. テストファイル作成: `tests/web_routes_phase4.rs`
2. public routes テスト実装
3. protected routes テスト実装
4. error case テスト実装
5. cargo test 確認

---

## 🏁 Day 5 実装詳細：ハンドラテスト + 統合確認

### タスク 5.1: ハンドラユニットテスト実装

**ファイル**: `src/web/handlers/mod.rs` (tests section)

**テスト項目** (12個):

| ハンドラ | テスト1 | テスト2 | 合計 |
|---------|---------|---------|------|
| register_user | 成功 | 重複エラー | 2個 |
| get_user | 成功 | 404 エラー | 2個 |
| update_user | 成功 | 権限エラー | 2個 |
| create_post | 成功 | 状態エラー | 2個 |
| publish_post | 成功 | 権限不足 | 2個 |
| login | 成功 | 認証失敗 | 2個 |

**実装ステップ** (90分):

1. Mock Use Cases 作成（mockall）
2. register_user テスト実装（2個）
3. get_user テスト実装（2個）
4. update_user テスト実装（2個）
5. create_post テスト実装（2個）
6. publish_post テスト実装（2個）
7. login テスト実装（2個）
8. cargo test 確認

---

### タスク 5.2: 統合確認

**コマンド**:

```bash
# 1. コンパイル確認
cargo check --lib --features "restructure_domain"

# 2. Clippy チェック
cargo clippy --lib --features "restructure_domain" -- -D warnings

# 3. 全ハンドラテスト実行
cargo test --lib web::handlers:: --features "restructure_domain" -v

# 4. 全ルートテスト実行
cargo test --lib web::routes:: --features "restructure_domain" -v

# 5. 全ミドルウェアテスト実行
cargo test --lib web::middleware:: --features "restructure_domain" -v

# 6. 統合テスト実行
cargo test --test web_routes_phase4 --features "restructure_domain" -v

# 7. 全体テスト（ハンドラ + ルート + ミドルウェア）
cargo test --lib web:: --features "restructure_domain" -q
```

**期待される出力**:

```bash
test result: ok. 22+ passed; 0 failed

# テスト分解:
#   - ハンドラテスト: 12個
#   - ルートテスト: 4個
#   - ミドルウェアテスト: 6個
```

**実装ステップ** (45分):

1. cargo check 実行
2. cargo clippy 実行
3. 全テスト実行
4. エラーがあれば修正
5. ドキュメント更新（Week 12 最終報告書作成）

---

## 📊 進捗トラッキング表

| Day | タスク | 所要時間 | ステータス |
|-----|--------|---------|-----------|
| **3** | require_auth 実装 | 30分 | 🔜 |
| | rate_limit 実装 | 30分 | 🔜 |
| | request_logging 実装 | 20分 | 🔜 |
| | routes.rs 完成化 | 30分 | 🔜 |
| | cargo check + clippy | 15分 | 🔜 |
| **Day 3 合計** | | **2時間5分** | 🔜 |
| | | | |
| **4** | ミドルウェアテスト | 40分 | 🔜 |
| | ルートテスト | 20分 | 🔜 |
| | 全テスト実行確認 | 15分 | 🔜 |
| | ドキュメント見直し | 15分 | 🔜 |
| **Day 4 合計** | | **1時間30分** | 🔜 |
| | | | |
| **5** | ハンドラテスト実装 | 90分 | 🔜 |
| | 統合確認 | 45分 | 🔜 |
| | Week 12 最終報告書 | 30分 | 🔜 |
| **Day 5 合計** | | **2時間45分** | 🔜 |
| | | | |
| **Week 12 全体** | **Day 3-5 合計** | **6時間20分** | 🔜 |

---

## ✅ Success Criteria（Week 12 終了時）

### 必須条件

- [ ] ミドルウェア 3個 完全実装
- [ ] routes.rs 全エンドポイント集約完成
- [ ] ハンドラテスト 12+個 実装
- [ ] cargo test --lib web:: 全てパス
- [ ] ビルド警告 0

### Quality Gate

- [ ] 0 コンパイル警告
- [ ] 0 clippy 警告
- [ ] テストカバレッジ ≥ 80%
- [ ] ドキュメント完成度 ≥ 95%
- [ ] エラーケーステスト ≥ 90%

---

## 🎯 新構造への適応ポイント

### 📌 common/ ディレクトリ使用

```rust
// ❌ 古い import
use crate::shared::types::AppError;

// ✅ 新しい import
use crate::common::types::AppError;
```

### 📌 ハンドラから Use Cases 呼び出し

```rust
// ✅ 新パターン（薄い層）
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserDto>), AppError> {
    // 1. Use Case の初期化（state から取得）
    let use_case = RegisterUserUseCase::new(state.user_repository.clone());
    
    // 2. Use Case 実行
    let user = use_case.execute(request).await?;
    
    // 3. DTO 変換
    let dto = UserDto::from(user);
    
    // 4. HTTP レスポンス
    Ok((StatusCode::CREATED, Json(dto)))
}
```

### 📌 ミドルウェア統合パターン

```rust
// ✅ 新パターン（Tower middleware）
.layer(middleware::from_fn(require_auth))
.layer(middleware::from_fn(rate_limit))
.layer(middleware::from_fn(request_logging))
```

---

**次のステップ**: Day 3 から実装開始（ミドルウェア実装）
