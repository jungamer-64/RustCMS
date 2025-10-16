# Phase 5: レガシーコード段階的削除 & API v1→v2 マイグレーション計画

## 🎯 目標

- ✅ Phase 4.9 完成: Presentation層 (handlers/router/responses)
- 🎯 Phase 5: レガシーコード (`src/handlers/`, `src/repositories/`) を段階的に削除
- 🎯 `/api/v1` から `/api/v2` へのカナリアリリース実施

## 📋 レガシーコード分析

### src/handlers/ (レガシー版)

| ファイル | 行数 | 対象エンドポイント | 新規対応 (src/presentation/http/handlers.rs) |
|---------|------|----------|-----------|
| `mod.rs` | ~50 | ルーティング | ✅ router.rs で対応 |
| `users.rs` | ~200 | User CRUD | ✅ register_user, get_user, update_user, delete_user |
| `posts.rs` | ~250 | Post CRUD | ✅ create_post, get_post, update_post |
| `auth.rs` | ~150 | 認証フロー | ⏳ Phase 5+1 で実装 |
| `api_keys.rs` | ~180 | API Key管理 | ⏳ Phase 5+1 で実装 |
| `search.rs` | ~200 | 全文検索 | ⏳ Phase 5+1 で実装 |
| `admin.rs` | ~150 | 管理者機能 | ⏳ Phase 5+1 で実装 |
| `health.rs` | ~50 | ヘルスチェック | ⏳ Phase 5+1 で実装 |
| `metrics.rs` | ~100 | メトリクス | ⏳ Phase 5+1 で実装 |

**合計: 約1,330行**

### src/repositories/ (レガシー版)

| ファイル | 行数 | 責務 |
|---------|------|-----|
| `mod.rs` | ~30 | エクスポート |
| `user_repository.rs` | ~300 | User DB実装 |
| `post.rs` | ~250 | Post DB実装 |

**合計: 約580行**

## 🔄 マイグレーション戦略

### Phase 5-1: API バージョニング準備 (1週間)

#### タスク

1. **Feature flag による並行稼働準備**

```toml
# Cargo.toml 既存設定
[features]
default = ["database", "cache", "search", "auth"]
restructure_domain = []
restructure_application = []
restructure_presentation = []

# 新規フラグ（Phase 5）
legacy_api_v1 = []  # 旧ハンドラー使用
api_v2 = []         # 新ハンドラー使用
```

2. **/api/v1 ルーティング分離**

```rust
// src/routes/mod.rs
pub fn api_v1_routes() -> Router {
    Router::new()
        .route("/users", post(users::register_user))
        .route("/users/:id", get(users::get_user))
        // ... (旧ハンドラー)
}

#[cfg(feature = "api_v2")]
pub fn api_v2_routes() -> Router {
    // new api_v2_router() from src/presentation/http/router.rs
    api_v2_router()
}
```

3. **環境変数による動的ルーティング**

```rust
// src/main.rs
let app = if use_legacy_api_v1 {
    Router::new()
        .nest("/api/v1", api_v1_routes())
} else {
    Router::new()
        .nest("/api/v2", api_v2_routes())
};
```

#### ファイル修正予定

- [ ] `src/routes/mod.rs` - v1/v2 ルーティング分離
- [ ] `src/main.rs` - 環境変数制御追加
- [ ] `Cargo.toml` - feature フラグ追加

### Phase 5-2: E2E テスト準備 (1週間)

#### タスク

1. **API v1 に対する E2E テスト作成**

```bash
tests/
├── e2e_api_v1/
│   ├── users_test.rs
│   ├── posts_test.rs
│   ├── auth_test.rs
│   └── mod.rs
└── e2e_api_v2/
    ├── users_test.rs
    ├── posts_test.rs
    └── mod.rs
```

2. **スナップショットテスト統合**

```bash
cargo insta test --test e2e_api_v1 --features "legacy_api_v1"
cargo insta test --test e2e_api_v2 --features "api_v2"
```

#### ファイル作成予定

- [ ] `tests/e2e_api_v1/mod.rs` - v1 テストヘルパー
- [ ] `tests/e2e_api_v2/mod.rs` - v2 テストヘルパー
- [ ] `tests/e2e_api_v1/users_test.rs` - v1 User E2E
- [ ] `tests/e2e_api_v2/users_test.rs` - v2 User E2E

### Phase 5-3: Staging デプロイ (1週間)

#### Deployment Plan

```mermaid
Staging 環境
├─ API v1 (旧): /api/v1
│  └─ Feature: legacy_api_v1
└─ API v2 (新): /api/v2
   └─ Feature: api_v2
```

#### 検証項目

- [ ] v1 E2E テスト: 全通過
- [ ] v2 E2E テスト: 全通過
- [ ] レスポンス時間: v1 と v2 で 5% 以内
- [ ] エラーハンドリング: 両方で一貫性

### Phase 5-4: Canary Release (2週間)

#### Traffic Split

```
Week 1:
└─ API v2 traffic: 10% (canary)
   API v1 traffic: 90% (stable)

Week 2:
└─ API v2 traffic: 50% (ramp-up)
   API v1 traffic: 50% (stable)

Week 3:
└─ API v2 traffic: 90% (production)
   API v1 traffic: 10% (deprecated, monitoring)

Week 4:
└─ API v2 traffic: 100% (stable)
   API v1 traffic: 0% (sunset)
```

#### 監視項目

- [ ] エラー率 (< 1%)
- [ ] レスポンス時間 (< 100ms p99)
- [ ] ユーザーセッション稼働率 (> 99%)
- [ ] CPU/メモリ使用率

#### ロールバック条件

- エラー率 > 5%
- レスポンス時間 p99 > 500ms
- クリティカルバグ 3件以上未解決
- ユーザー報告: 10件以上のクラッシュ

### Phase 5-5: API v1 Deprecation (1週間)

#### 対応

1. **API v1 エンドポイントに Deprecation ヘッダー追加**

```rust
// src/handlers/mod.rs
pub async fn users_handler_v1(...) -> impl IntoResponse {
    let response = /* レスポンス生成 */;
    (
        [("Deprecation", "true"), ("Sunset", "Wed, 01 Jan 2026 00:00:00 GMT")],
        response,
    )
}
```

2. **クライアント側通知**
   - ドキュメント更新: v1 → v2 への移行ガイド作成
   - メール通知: API ユーザーへ v1 廃止予告
   - Log: v1 使用状況の監視継続

#### ファイル修正予定

- [ ] `docs/API.md` - v2 エンドポイント一覧
- [ ] `docs/MIGRATION_V1_TO_V2.md` - マイグレーションガイド

### Phase 5-6: レガシーコード削除 (1週間)

#### タスク

1. **src/handlers/ 完全削除**

```bash
rm -rf src/handlers/
```

2. **src/repositories/ 完全削除**

```bash
rm -rf src/repositories/
```

3. **src/routes/mod.rs から v1 ルーティング削除**

```rust
// Before
pub fn api_routes() -> Router {
    Router::new()
        .nest("/api/v1", api_v1_routes())
        .nest("/api/v2", api_v2_routes())
}

// After
pub fn api_routes() -> Router {
    Router::new()
        .nest("/api/v2", api_v2_routes())
}
```

4. **Feature flag クリーンアップ**

```toml
# Cargo.toml
[features]
# default から legacy_api_v1 を削除
default = ["database", "cache", "search", "auth", "api_v2"]  # ← api_v2 必須化

# legacy_api_v1 は削除
```

#### ファイル削除予定

- [ ] `src/handlers/` ディレクトリ全体
- [ ] `src/repositories/` ディレクトリ全体

## 📊 Phase 5 タイムライン

| Phase | 週 | タスク | 完了予定 |
|-------|----|----|--------|
| **5-1** | W1 | API バージョニング準備 | 2025-01-24 |
| **5-2** | W2 | E2E テスト準備 | 2025-01-31 |
| **5-3** | W3 | Staging デプロイ | 2025-02-07 |
| **5-4** | W4-5 | Canary Release | 2025-02-21 |
| **5-5** | W6 | API v1 Deprecation | 2025-02-28 |
| **5-6** | W7 | レガシーコード削除 | 2025-03-07 |

## 🛠️ ロールバック計画

### Critical Issues (即座にロールバック)

- [ ] クライアント数 > 50% が API v2 で致命的エラー報告
- [ ] レスポンス時間 p99 > 1000ms
- [ ] セキュリティ脆弱性 (CVSS > 7.0)

### ロールバック実行

```bash
# 環境変数で v1 に戻す
export USE_LEGACY_API_V1=true
export API_V2_ENABLED=false

# Traffic を即座に v1 に戻す
# (Load Balancer 設定変更)
```

## ✅ チェックリスト

### Phase 5 開始前

- [ ] Phase 4.9 が完全に完成 ✅ (2025-01-17)
- [ ] Domain 188テスト全通過 ✅
- [ ] handlers.rs 8個のハンドラー実装化 ✅
- [ ] router.rs 14個のルート定義完成 ✅
- [ ] 統合テスト 12個全通過 ✅

### Phase 5-1 完了時

- [ ] API v1/v2 ルーティング分離完成
- [ ] 環境変数制御実装完成
- [ ] Feature flag 動作確認 ✅

### Phase 5 全体完了時

- [ ] API v2 Production 稼働
- [ ] レガシーコード完全削除
- [ ] テストカバレッジ ≥ 85%
- [ ] 本番環境で安定稼働 2週間以上

---

**作成日**: 2025年01月17日
**ステータス**: Phase 5 準備中
**次アクション**: Phase 5-1 から開始予定

