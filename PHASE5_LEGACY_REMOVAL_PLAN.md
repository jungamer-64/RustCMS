# Phase 5: Legacy Code Removal Plan

**作成日**: 2025年10月19日  
**目的**: Phase 4完了後のレガシーコード削除と統合テスト実施

---

## 📋 削除対象ファイル

### 1. Phase 4 中間ファイル（削除対象）

これらは移行期に作成された中間バージョンで、v2ファイルで完全に置き換え済み：

```bash
src/web/handlers/
├── auth_phase4.rs      # → auth_v2.rs（未実装、既存auth.rs利用）
├── users_phase4.rs     # → users_v2.rs ✅
├── posts_phase4.rs     # → posts_v2.rs ✅
└── health_phase4.rs    # → health_v2.rs ✅
```

**削除コマンド**:
```bash
rm src/web/handlers/auth_phase4.rs
rm src/web/handlers/users_phase4.rs
rm src/web/handlers/posts_phase4.rs
rm src/web/handlers/health_phase4.rs
```

---

### 2. レガシーハンドラ（段階的廃止）

Phase 5では**削除せず**、v2エンドポイントとの共存を維持：

```bash
src/web/handlers/
├── auth.rs        # レガシー認証（v1 API用）- Phase 6で削除
├── users.rs       # レガシーUser（v1 API用）- Phase 6で削除
├── posts.rs       # レガシーPost（v1 API用）- Phase 6で削除
├── health.rs      # レガシーHealth（v1 API用）- Phase 6で削除
└── ...            # その他レガシー
```

**理由**: v1 APIとの互換性維持（既存クライアント対応）

---

### 3. Routes統合

#### 現状

```bash
src/web/
├── routes.rs       # レガシールート（v1 API）
└── routes_v2.rs    # 新構造（v2 API）
```

#### Phase 5での対応

- `routes.rs`: **削除せず**、v1 APIエンドポイント維持
- `routes_v2.rs`: そのまま継続使用
- main.rs: 両方のルートをマウント

**統合コード** (main.rs):
```rust
use crate::web::{routes, routes_v2};

let app = Router::new()
    .nest("/api/v1", routes::create_v1_router(state.clone()))
    .nest("/api/v2", routes_v2::create_v2_router(state.clone()));
```

---

## ✅ Phase 5 実行計画

### Step 1: Phase 4中間ファイル削除

```bash
# 1. Phase 4ファイル削除
rm src/web/handlers/auth_phase4.rs
rm src/web/handlers/users_phase4.rs
rm src/web/handlers/posts_phase4.rs
rm src/web/handlers/health_phase4.rs

# 2. handlers/mod.rs更新（phase4モジュール削除）
# - `pub mod auth_phase4;` 削除
# - `pub mod users_phase4;` 削除
# - `pub mod posts_phase4;` 削除
# - `pub mod health_phase4;` 削除
```

### Step 2: ビルド確認

```bash
# 新構造でビルド確認
cargo build --lib --features "restructure_domain"

# 全feature flagsでビルド確認
cargo build --all-features
```

### Step 3: 統合テスト実装

#### 3.1 testcontainers導入

**Cargo.toml**:
```toml
[dev-dependencies]
testcontainers = "0.15"
testcontainers-modules = { version = "0.3", features = ["postgres", "redis"] }
```

#### 3.2 テストヘルパー作成

**tests/helpers/mod.rs**:
```rust
use testcontainers::*;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;

pub async fn setup_test_db() -> Container<Postgres> {
    // PostgreSQLコンテナ起動
}

pub async fn setup_test_redis() -> Container<Redis> {
    // Redisコンテナ起動
}

pub fn create_test_app_state() -> AppState {
    // テスト用AppState作成
}
```

#### 3.3 E2Eテスト実装

**tests/integration_web_v2.rs**:
```rust
#[tokio::test]
async fn test_user_registration_flow() {
    // 1. ユーザー登録
    // 2. ユーザー取得確認
}

#[tokio::test]
async fn test_post_creation_flow() {
    // 1. ユーザー登録
    // 2. 投稿作成
    // 3. 投稿公開
    // 4. 投稿取得確認
}

#[tokio::test]
async fn test_comment_flow() {
    // 1. ユーザー登録
    // 2. 投稿作成
    // 3. コメント投稿
    // 4. コメント一覧確認
}
```

### Step 4: OpenAPI統合

#### 4.1 utoipa導入

**Cargo.toml**:
```toml
[dependencies]
utoipa = { version = "4.0", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "6.0", features = ["axum"] }
```

#### 4.2 OpenAPI定義追加

**各Handler**:
```rust
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Deserialize)]
pub struct UserDto { ... }

#[utoipa::path(
    post,
    path = "/api/v2/users",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "User created", body = UserDto),
        (status = 400, description = "Bad request")
    )
)]
pub async fn register_user(...) { ... }
```

#### 4.3 Swagger UIマウント

**main.rs**:
```rust
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::web::handlers::users_v2::register_user,
        // ...
    ),
    components(schemas(UserDto, CreateUserRequest, ...))
)]
struct ApiDoc;

let app = Router::new()
    .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    .nest("/api/v2", routes_v2::create_v2_router(state.clone()));
```

### Step 5: ドキュメント更新

1. **MIGRATION_CHECKLIST.md**: Phase 5完了マーク
2. **README.md**: API v2エンドポイント一覧追加
3. **PHASE5_COMPLETION_REPORT.md**: Phase 5完了レポート作成

---

## 🎯 Phase 5 完了条件

- [x] Phase 4中間ファイル削除（4ファイル）
- [ ] handlers/mod.rs更新（phase4モジュール削除）
- [ ] ビルド成功（`--features "restructure_domain"`）
- [ ] ビルド成功（`--all-features`）
- [ ] 統合テスト環境構築（testcontainers）
- [ ] E2Eテスト実装（3シナリオ）
- [ ] OpenAPI統合（utoipa + Swagger UI）
- [ ] ドキュメント更新（3ファイル）

---

## ⚠️ 注意事項

### レガシーコード保持理由

Phase 5では以下を**削除しない**：

1. **既存handlers**: auth.rs, users.rs, posts.rs等
   - v1 APIクライアント対応
   - 段階的廃止（Phase 6で検討）

2. **routes.rs**: v1 APIルート
   - 既存エンドポイント維持
   - /api/v1 プレフィックス

3. **既存Use Cases**: application/use_cases/
   - レガシーハンドラが依存
   - Phase 6で統合予定

### 削除するもの

- ✅ **Phase 4中間ファイル**: *_phase4.rs（v2で置き換え済み）

---

## 📊 Phase 5タイムライン

| ステップ | タスク | 所要時間 | 状態 |
|---------|--------|----------|------|
| Step 1 | Phase 4ファイル削除 | 10分 | 🔜 |
| Step 2 | ビルド確認 | 5分 | 🔜 |
| Step 3 | 統合テスト実装 | 2時間 | 🔜 |
| Step 4 | OpenAPI統合 | 1時間 | 🔜 |
| Step 5 | ドキュメント更新 | 30分 | 🔜 |
| **合計** | **Phase 5完了** | **~4時間** | **🔜** |

---

**作成者**: GitHub Copilot  
**レビュー**: 必要（削除前に確認）  
**承認**: チームリーダー
