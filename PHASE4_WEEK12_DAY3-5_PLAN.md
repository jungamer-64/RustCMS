# Phase 4 Week 12 Day 3-5 計画（ミドルウェア + ルート統合 + テスト）

**計画日**: 2025年10月18日  
**期間**: Week 12 Day 3-5（3日間）  
**目標**: ハンドラ実装完了 → ミドルウェア統合 → テスト実装 → cargo test パス

---

## 📌 Day 3-4: ミドルウェア実装 + ルート統合

### タスク 1: require_auth ミドルウェア実装

**ファイル**: `src/web/middleware_phase4.rs`

**責務**:

- 🔐 Biscuit トークン検証
- 👤 ユーザー ID 抽出
- 🔑 リクエストコンテキストに ユーザー情報を注入

**実装予定**:

```rust
pub async fn require_auth(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Authorization: Bearer <token> から token 抽出
    let token = extract_bearer_token(&headers)?;
    
    // Biscuit トークン検証
    let biscuit = verify_biscuit_token(token)?;
    
    // ユーザー ID 抽出
    let user_id = extract_user_id_from_biscuit(&biscuit)?;
    
    // リクエストエクステンションに追加（後続ハンドラで取得）
    request.extensions_mut().insert(user_id);
    
    Ok(next.run(request).await)
}
```

**テスト予定**:

```rust
#[tokio::test]
async fn test_require_auth_with_valid_token() {
    // ✅ 検証: パス (user_id in request)
}

#[tokio::test]
async fn test_require_auth_with_expired_token() {
    // ✅ 検証: 401 Unauthorized
}

#[tokio::test]
async fn test_require_auth_without_token() {
    // ✅ 検証: 400 Bad Request
}
```

---

### タスク 2: rate_limit ミドルウェア実装

**ファイル**: `src/web/middleware_phase4.rs`

**責務**:

- 📊 リクエストレート追跡（IP ベース）
- 🚫 超過時の 429 Too Many Requests 応答

**実装予定**:

```rust
pub async fn rate_limit(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // IP アドレスから rate limiter を取得
    if !check_rate_limit(addr.ip()).await? {
        return Err(AppError::RateLimitExceeded);
    }
    
    Ok(next.run(request).await)
}
```

**テスト予定**:

```rust
#[tokio::test]
async fn test_rate_limit_within_quota() {
    // ✅ 検証: パス (rate_limit > threshold)
}

#[tokio::test]
async fn test_rate_limit_exceeded() {
    // ✅ 検証: 429 Too Many Requests
}
```

---

### タスク 3: request_logging ミドルウェア実装

**ファイル**: `src/web/middleware_phase4.rs`

**責務**:

- 📝 リクエスト/レスポンスログ出力
- ⏱️ レスポンス時間測定

**実装予定**:

```rust
pub async fn request_logging(
    Request { uri, method, .. }: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    
    tracing::info!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        duration_ms = duration.as_millis(),
    );
    
    response
}
```

**テスト予定**:

```rust
#[tokio::test]
async fn test_request_logging() {
    // ✅ 検証: トレースメッセージ出力確認
}
```

---

### タスク 4: routes.rs 完成化（全エンドポイント集約）

**ファイル**: `src/web/routes.rs`

**完成予定の状態**:

```rust
pub fn create_routes() -> Router {
    Router::new()
        // V1 (レガシー)
        .route("/api/v1/health", get(health_check_v1))
        
        // V2 (新規)
        .nest("/api/v2", api_v2_routes())
}

fn api_v2_routes() -> Router {
    Router::new()
        // Public エンドポイント
        .route("/health", get(health_check_v2))
        .route("/users/register", post(register_user))
        .route("/auth/login", post(login))
        
        // Protected エンドポイント (require_auth + rate_limit)
        .route("/users/:id", get(get_user))
            .layer(middleware::from_fn(require_auth))
            .layer(middleware::from_fn(rate_limit))
        .route("/users/:id", put(update_user))
            .layer(middleware::from_fn(require_auth))
        // ... etc
}
```

**テスト予定**:

```rust
#[tokio::test]
async fn test_routes_public_endpoints() {
    // ✅ 検証: /health, /register, /login は認証なしでアクセス可能
}

#[tokio::test]
async fn test_routes_protected_endpoints() {
    // ✅ 検証: /users/:id は require_auth マウント済み
}
```

---

## 📌 Day 5: ユニットテスト実装 + 統合確認

### タスク 5: ハンドラユニットテスト作成

**ファイル**: `src/web/handlers/mod.rs` (tests section)

**テスト対象**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_user_success() {
        // Given: 有効な登録リクエスト
        // When: register_user を呼び出し
        // Then: 201 Created + UserDto を返す
    }

    #[tokio::test]
    async fn test_register_user_duplicate_email() {
        // Given: 既存ユーザーと同じメール
        // When: register_user を呼び出し
        // Then: 409 Conflict を返す
    }

    #[tokio::test]
    async fn test_get_user_success() {
        // Given: 有効なユーザー ID + 認証済み
        // When: get_user を呼び出し
        // Then: 200 OK + UserDto を返す
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        // Given: 存在しないユーザー ID
        // When: get_user を呼び出し
        // Then: 404 Not Found を返す
    }

    // ... (6+ テストケース)
}
```

**テスト数目標**: 12+ テスト（各ハンドラ 1-2 テスト）

---

### タスク 6: ルート定義テスト

**ファイル**: `tests/web_routes_phase4.rs` (新規作成)

**テスト対象**:

```rust
#[tokio::test]
async fn test_public_routes_accessible() {
    // ✅ GET /api/v2/health
    // ✅ POST /api/v2/users/register
    // ✅ POST /api/v2/auth/login
}

#[tokio::test]
async fn test_protected_routes_require_auth() {
    // ✅ GET /api/v2/users/:id (without token) → 401
    // ✅ PUT /api/v2/users/:id (without token) → 401
    // ✅ POST /api/v2/posts (without token) → 401
}
```

---

### タスク 7: ビルド & テスト確認

**コマンド**:

```bash
# 1. コンパイル確認
cargo check --lib --features "restructure_domain"

# 2. Clippy チェック
cargo clippy --lib --features "restructure_domain" -- -D warnings

# 3. テスト実行
cargo test --lib web:: --features "restructure_domain"

# 4. ドキュメント確認
cargo doc --lib --features "restructure_domain" --no-deps --open
```

**期待される出力**:

```bash
test web::handlers::tests::test_register_user_success ... ok
test web::handlers::tests::test_get_user_not_found ... ok
test web::routes::tests::test_public_routes_accessible ... ok
test web::middleware::tests::test_require_auth_with_valid_token ... ok

test result: ok. 12+ passed;
```

---

## 📊 進捗トラッキング

### Day 3

- [ ] require_auth ミドルウェア実装（30分）
- [ ] rate_limit ミドルウェア実装（30分）
- [ ] request_logging ミドルウェア実装（20分）
- [ ] routes.rs ネスティング完成化（30分）
- [ ] Clippy チェック（15分）

**Day 3 合計**: 2時間 5分

### Day 4

- [ ] require_auth テスト作成（20分）
- [ ] rate_limit テスト作成（20分）
- [ ] routes テスト作成（30分）
- [ ] test result 確認（15分）
- [ ] ドキュメント見直し（15分）

**Day 4 合計**: 1時間 40分

### Day 5

- [ ] ハンドラユニットテスト作成（1時間）
- [ ] 統合テスト実行確認（30分）
- [ ] cargo test --lib web:: 全てパス確認（15分）
- [ ] 完了報告書作成（30分）

**Day 5 合計**: 2時間 15分

**週間合計**: 6時間

---

## 🎯 Success Criteria（Week 12 Day 5 終了時）

### ✅ コード品質

- [ ] cargo check: 0 warnings
- [ ] cargo clippy: 0 warnings
- [ ] cargo test --lib web::: すべてパス
- [ ] テストカバレッジ: ≥ 80%

### ✅ ドキュメント

- [ ] 各ハンドラドキュメント: 完全
- [ ] ミドルウェアドキュメント: 完全
- [ ] ルート定義ドキュメント: 完全

### ✅ 機能完成

- [ ] require_auth: 完全実装 + テスト
- [ ] rate_limit: 基本実装 + テスト
- [ ] request_logging: 完全実装 + テスト
- [ ] routes.rs: 全エンドポイント集約完了

### ✅ 統合確認

- [ ] Phase 4 レガシー並行動作確認
- [ ] エラーハンドリング: 一貫性確認
- [ ] イベント発行: 確認

---

## 🚀 Next Phase (Week 13)

### Week 13 計画

- [ ] 統合テスト実装（PostgreSQL 連携）
- [ ] OpenAPI ドキュメント生成
- [ ] API v2 パイロット検証
- [ ] Biscuit token 検証詳細化

---

**所要時間**: 6時間（3日間）  
**難度**: 中（ミドルウェア理解が必須）  
**リスク**: 低（レガシーコード並行）
