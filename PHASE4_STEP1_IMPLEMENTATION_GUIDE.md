# Phase 4 Step 1: Web 層基本構造実装ガイド

**ステータス**: 🚀 実装開始  
**実装期間**: Week 12-13  
**対象ファイル**:
- `src/web/mod.rs` - ✅ 既存を拡張
- `src/web/routes.rs` - ✅ 作成済み
- `src/web/handlers/users_phase4.rs` - ✅ 作成済み
- `src/web/handlers/posts_phase4.rs` - ✅ 作成済み
- `src/web/handlers/auth_phase4.rs` - ✅ 作成済み
- `src/web/handlers/health_phase4.rs` - ✅ 作成済み
- `src/web/middleware_phase4.rs` - ✅ 作成済み

---

## 📋 実装チェックリスト

### A. ディレクトリ構造確認

```bash
src/web/
├── mod.rs                    # Web層ルート（ライブラリ化）
├── routes.rs                 # ✅ ルート定義集約
├── middleware_phase4.rs      # ✅ ミドルウェア（認証/レート制限/ロギング）
├── handlers/
│   ├── mod.rs                # ハンドラルート
│   ├── users_phase4.rs       # ✅ ユーザーハンドラ
│   ├── posts_phase4.rs       # ✅ 投稿ハンドラ
│   ├── auth_phase4.rs        # ✅ 認証ハンドラ
│   └── health_phase4.rs      # ✅ ヘルスチェック
```

**状態**: 主要ファイル作成完了 ✅

### B. 統合ステップ

**ステップ 1**: `src/web/mod.rs` を更新（routes.rs をエクスポート）

```rust
// src/web/mod.rs 追加

pub mod routes;

// レガシーエクスポートと並行
pub use routes::create_router;
```

**ステップ 2**: `src/web/handlers/mod.rs` を更新（Phase 4 ハンドラをモジュール化）

```rust
// src/web/handlers/mod.rs 追加

#[cfg(feature = "restructure_domain")]
pub mod users_phase4;

#[cfg(feature = "restructure_domain")]
pub mod posts_phase4;

#[cfg(feature = "restructure_domain")]
pub mod auth_phase4;

#[cfg(feature = "restructure_domain")]
pub mod health_phase4;
```

**ステップ 3**: `src/main.rs` または `src/lib.rs` で routes を統合

```rust
// 例: main.rs で app 初期化

use cms_backend::web::create_router;

// ...

let router = create_router(state).await;
```

### C. ビルド・テスト実行

```bash
# Step 1: Feature gate で Phase 4 コードをビルド
cargo build --lib --features "restructure_domain" 2>&1 | head -50

# Step 2: ハンドラユニットテスト実行
cargo test --lib --features "restructure_domain" web::handlers

# Step 3: ルート定義テスト
cargo test --lib --features "restructure_domain" web::routes

# Step 4: ミドルウェアテスト
cargo test --lib --features "restructure_domain" web::middleware
```

---

## 🎯 実装詳細

### 1. ハンドラ設計パターン

すべてのハンドラは以下パターンに従います：

```rust
// パターン
pub async fn handler_name(
    State(state): State<Arc<AppState>>,    // AppState（Use Cases / Repositories 含む）
    Path(id): Path<Uuid>,                  // URL パラメータ
    Query(filter): Query<SomeFilter>,      // Query パラメータ
    Json(request): Json<SomeRequest>,      // JSON ボディ
) -> Result<Json<SomeDto>, AppError> {    // Response + Error
    // 1. DTO をデシリアライズ（自動）
    // 2. Use Case を構築
    // 3. Use Case.execute() を呼び出し
    // 4. Domain Entity から DTO に変換
    // 5. HTTP レスポンスを返却
    
    let use_case = SomeUseCase::new(/*dependencies*/);
    let result = use_case.execute(request).await?;
    let dto = SomeDto::from(result);
    
    Ok(Json(dto))
}
```

### 2. HTTP ステータスコード戦略

| ハンドラ操作 | ステータスコード | 例 |
|------------|-----------------|-----|
| **作成成功** | 201 Created | POST /users → 201 |
| **取得成功** | 200 OK | GET /users/{id} → 200 |
| **更新成功** | 200 OK | PUT /users/{id} → 200 |
| **削除成功** | 204 No Content | DELETE /users/{id} → 204 |
| **入力エラー** | 400 Bad Request | JSON デシリアライズ失敗 |
| **認可エラー** | 401 Unauthorized | token 無効 |
| **権限エラー** | 403 Forbidden | 他人のリソース編集 |
| **見つからない** | 404 Not Found | 存在しないユーザー |
| **重複エラー** | 409 Conflict | ユーザー名重複登録 |
| **サーバーエラー** | 500 Internal Server Error | Use Case エラー |

### 3. ハンドラ実装例（User Register）

```rust
/// ユーザー登録
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserDto>), AppError> {
    // Step 1: Use Case 初期化
    let use_case = RegisterUserUseCase::new(
        Arc::new(state.user_repository.clone()),
        Arc::new(state.event_bus.clone()),
    );
    
    // Step 2: ビジネスロジック実行（Use Case）
    let user = use_case.execute(request).await?;
    
    // Step 3: DTO 変換
    let dto = UserDto::from(user);
    
    // Step 4: HTTP レスポンス（201 Created）
    Ok((StatusCode::CREATED, Json(dto)))
}
```

### 4. エラーハンドリング

ハンドラから `AppError` をスロー → Axum の `impl IntoResponse` で自動変換

```rust
// Use Case から エラー返却
use_case.execute(request).await?  // AppError が自動的に HTTP Response に変換

// エラーマッピング例（src/error.rs で定義）
AppError::ValidationError => 400 Bad Request
AppError::NotFound => 404 Not Found
AppError::Conflict => 409 Conflict
AppError::Unauthorized => 401 Unauthorized
AppError::InternalServerError => 500 Internal Server Error
```

---

## ✅ テスト戦略

### 単体テスト（ハンドラ）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_register_user_success() {
        // Arrange: Mock AppState / Use Case
        let request = CreateUserRequest {
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            password: "pass123".to_string(),
        };
        
        // Act: ハンドラ呼び出し
        // let (status, response) = register_user(...).await?;
        
        // Assert: 201 Created + UserDto
        // assert_eq!(status, StatusCode::CREATED);
        // assert_eq!(response.username, "alice");
    }
}
```

### 統合テスト（E2E）

```bash
# curl で実機確認
curl -X POST http://localhost:8080/api/v2/users/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "secure123"
  }'

# 期待するレスポンス（201 Created）
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "email": "alice@example.com",
  "created_at": "2025-10-18T10:30:00Z",
  "is_active": true
}
```

---

## 🔧 実装の進め方

### Week 12

**Day 1-2: ハンドラ実装**
- [ ] `users_phase4.rs`: `register_user` + `get_user` 実装（UseCase 呼び出しまで）
- [ ] `posts_phase4.rs`: `create_post` + `list_posts` 実装
- [ ] `auth_phase4.rs`: `login` 実装（Biscuit トークン生成）
- [ ] `health_phase4.rs`: ✅ 完了

**Day 3-4: ルート・ミドルウェア統合**
- [ ] `routes.rs` 完成化（すべてのエンドポイント）
- [ ] `middleware_phase4.rs` 実装（require_auth, rate_limit, request_logging）
- [ ] ハンドラにミドルウェアをマウント

**Day 5: テスト・ビルド確認**
- [ ] ハンドラユニットテスト作成
- [ ] `cargo test --lib --features "restructure_domain"` 実行
- [ ] ビルド成功確認

### Week 13

**Day 1-2: 統合テスト実装**
- [ ] PostgreSQL コンテナ起動
- [ ] マイグレーション実行
- [ ] E2E テスト (curl / Postman)
- [ ] レスポンス検証

**Day 3-4: ドキュメント・ API 仕様**
- [ ] OpenAPI/Swagger 生成
- [ ] Migration Guide 作成（v1 → v2）
- [ ] API ドキュメント完成

**Day 5: Phase 4 Step 2 への移行**
- [ ] ミドルウェア機能強化（レート制限実装）
- [ ] エラーハンドリング詳細化
- [ ] レガシーコード削除計画確認

---

## 📊 進捗追跡

```markdown
### Phase 4 Step 1 進捗

#### A. ハンドラ実装
- [x] users_phase4.rs (4 endpoints)
- [x] posts_phase4.rs (4 endpoints)
- [x] auth_phase4.rs (2 endpoints)
- [x] health_phase4.rs (2 endpoints)
- [ ] ハンドラ実装詳細化（Use Cases 連携）

#### B. ルート定義
- [x] routes.rs 作成
- [ ] routes.rs 完成化（すべてのエンドポイント集約）
- [ ] ルートマッピングテスト

#### C. ミドルウェア
- [x] middleware_phase4.rs 作成（スケルトン）
- [ ] require_auth 実装（Biscuit 検証）
- [ ] rate_limit 実装（Redis or ローカル）
- [ ] request_logging 実装

#### D. テスト
- [ ] ハンドラユニットテスト
- [ ] ルート統合テスト
- [ ] E2E テスト（curl）

#### E. ドキュメント
- [ ] API 仕様（OpenAPI）
- [ ] Migration Guide（v1 → v2）
- [ ] 開発ガイド

**現在の進捗**: 📊 0/13 (準備完了、実装開始待機)
```

---

## 🚨 よくある落とし穴

### ❌ 禁止事項

1. **ハンドラにビジネスロジックを書かない**
   ```rust
   // ❌ 禁止
   pub async fn register_user(...) -> Result<...> {
       if email.contains("@") {  // ← ビジネスロジック！
           // ...
       }
   }
   
   // ✅ 推奨: Use Case に委譲
   pub async fn register_user(...) -> Result<...> {
       let use_case = RegisterUserUseCase::new(...);
       use_case.execute(request).await  // ← Use Case がやる
   }
   ```

2. **リポジトリを直接呼び出さない**
   ```rust
   // ❌ 禁止
   let user = state.user_repository.get_by_id(id).await?;
   
   // ✅ 推奨: Use Case を通す
   let use_case = GetUserByIdUseCase::new(...);
   let user = use_case.execute(id).await?;
   ```

3. **ハンドラでデータベーストランザクションを管理しない**
   ```rust
   // ❌ 禁止
   let tx = state.db.begin_transaction().await?;
   // ...
   tx.commit().await?;
   
   // ✅ 推奨: Use Case 内で Unit of Work パターン
   let use_case = SomeUseCase::new(...);
   use_case.execute(...).await?  // ← Unit of Work 内部
   ```

4. **複数のハンドラから同じロジックをコピペしない**
   ```rust
   // ❌ 禁止: register_user と update_user で email 検証コピー
   
   // ✅ 推奨: Email検証ロジックを Value Object に集約
   use crate::domain::user::Email;
   let email = Email::new("user@example.com")?;  // ← 一度きり
   ```

### ⚠️ 注意事項

1. **DTOの変換忘れ**
   ```rust
   // ❌ 禁止: Domain Entity をそのまま応答
   let user: User = use_case.execute(...).await?;
   Ok(Json(user))  // ← Domain Entity を HTTP で公開！
   
   // ✅ 推奨: DTO 経由
   let user: User = use_case.execute(...).await?;
   let dto = UserDto::from(user);
   Ok(Json(dto))
   ```

2. **ステータスコードの誤り**
   ```rust
   // ❌ 禁止: 全て 200 OK で応答
   pub async fn register_user(...) -> Result<Json<UserDto>> {
       // 201 Created じゃなくて 200 OK？
   }
   
   // ✅ 推奨: RFC 7231 準拠
   pub async fn register_user(...) -> Result<(StatusCode, Json<UserDto>)> {
       Ok((StatusCode::CREATED, Json(dto)))
   }
   ```

---

## 📚 参考資料

- **Axum ドキュメント**: https://docs.rs/axum/latest/axum/
- **Tower ミドルウェア**: https://docs.rs/tower/latest/tower/
- **HTTP ステータスコード（RFC 7231）**: https://tools.ietf.org/html/rfc7231
- **REST API 設計ガイド**: https://restfulapi.net/

---

**作成日**: 2025年10月18日  
**最終更新**: 2025年10月18日  
**推奨開始**: 準備完了、コンパイル確認後の実装開始
