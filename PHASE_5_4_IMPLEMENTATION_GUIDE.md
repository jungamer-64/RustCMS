# Phase 5-4 実装ガイド: Deprecation ヘッダー設計

**対象**: API v1 エンドポイント (約50個)
**RFC**: RFC 8594 - HTTP Sunset Header
**有効期間**: 2025-01-17 ～ 2025-03-17 (2ヶ月)

---

## 📋 Deprecation ヘッダー設計

### 1. 実装対象ヘッダー

| ヘッダー | 値 | 必須度 | 用途 |
|---------|-----|--------|------|
| `Deprecation` | `true` | 🔴 必須 | API が非推奨であることを表示 |
| `Sunset` | RFC 2616 日付 | 🔴 必須 | 削除予定日 (2025-03-17) |
| `Link` | rel="successor-version" | 🟡 推奨 | 新 API バージョンへのリンク |
| `Warning` | 299 - "Deprecation" | 🟡 推奨 | RFC 7231 互換性警告 |

### 2. ヘッダー実装例

```
HTTP/1.1 200 OK
Deprecation: true
Sunset: Sun, 17 Mar 2025 00:00:00 GMT
Link: </api/v2/users/123>; rel="successor-version"
Warning: 299 - "Deprecation: This endpoint will be removed on 2025-03-17"
Content-Type: application/json

{ "id": "123", "username": "john", ... }
```

### 3. Rust 実装パターン

```rust
// src/middleware/deprecation_headers.rs
use axum::{
    http::{HeaderMap, HeaderValue},
    response::Response,
    middleware::Next,
};

pub async fn add_deprecation_headers(
    mut response: Response,
    original_uri: &str,
) -> Response {
    // v1 エンドポイントのみに追加
    if original_uri.contains("/api/v1/") {
        let headers = response.headers_mut();

        headers.insert(
            "Deprecation",
            HeaderValue::from_static("true"),
        );

        headers.insert(
            "Sunset",
            HeaderValue::from_static("Sun, 17 Mar 2025 00:00:00 GMT"),
        );

        // /api/v1/users/123 → /api/v2/users/123 にマップ
        let v2_path = original_uri.replace("/api/v1/", "/api/v2/");
        let link_header = format!("<{}>; rel=\"successor-version\"", v2_path);
        headers.insert(
            "Link",
            HeaderValue::from_str(&link_header).unwrap(),
        );
    }

    response
}
```

---

## 🎯 実装チェックリスト

### Phase 5-4a: Deprecation ヘッダー追加 (Week 1)

#### タスク 1: ミドルウェア実装 (2日)

- [ ] `src/middleware/deprecation_headers.rs` 作成
- [ ] ヘッダー追加ロジック実装
- [ ] URI パース & v2 パス変換

#### タスク 2: ルーティング統合 (1日)

- [ ] `src/main.rs` に middleware 追加
- [ ] v1 ルーティングに適用
- [ ] v2 ルーティングには適用しない (feature flag)

#### タスク 3: テスト作成 (2日)

- [ ] `tests/deprecation_headers_test.rs` 作成 (50+ テスト)
- [ ] 各エンドポイント検証
- [ ] ヘッダー値の正確性確認

#### タスク 4: ドキュメント更新 (1日)

- [ ] `docs/API.md` 非推奨マーク追加
- [ ] `docs/MIGRATION_GUIDE.md` 作成
- [ ] クライアント向け移行ガイド

---

## 📊 対象エンドポイント (50個)

### Users (8)

```
GET    /api/v1/users
POST   /api/v1/users
GET    /api/v1/users/:id
PUT    /api/v1/users/:id
DELETE /api/v1/users/:id
PATCH  /api/v1/users/:id/email
PATCH  /api/v1/users/:id/password
POST   /api/v1/users/search
```

### Posts (10)

```
GET    /api/v1/posts
POST   /api/v1/posts
GET    /api/v1/posts/:id
PUT    /api/v1/posts/:id
DELETE /api/v1/posts/:id
PATCH  /api/v1/posts/:id/publish
PATCH  /api/v1/posts/:id/draft
GET    /api/v1/posts/:id/comments
POST   /api/v1/posts/:id/comments
GET    /api/v1/posts/author/:author_id
```

### Comments (8)

```
GET    /api/v1/comments
POST   /api/v1/comments
GET    /api/v1/comments/:id
PUT    /api/v1/comments/:id
DELETE /api/v1/comments/:id
PATCH  /api/v1/comments/:id/approve
GET    /api/v1/comments/post/:post_id
GET    /api/v1/comments/author/:author_id
```

### Tags (6)

```
GET    /api/v1/tags
POST   /api/v1/tags
GET    /api/v1/tags/:id
PUT    /api/v1/tags/:id
DELETE /api/v1/tags/:id
GET    /api/v1/tags/:id/posts
```

### Categories (6)

```
GET    /api/v1/categories
POST   /api/v1/categories
GET    /api/v1/categories/:id
PUT    /api/v1/categories/:id
DELETE /api/v1/categories/:id
GET    /api/v1/categories/:id/posts
```

### Search (4)

```
GET    /api/v1/search
GET    /api/v1/search/posts
GET    /api/v1/search/comments
GET    /api/v1/search/tags
```

### Analytics (4)

```
GET    /api/v1/analytics/summary
GET    /api/v1/analytics/posts
GET    /api/v1/analytics/users
GET    /api/v1/analytics/engagement
```

### Auth (2)

```
POST   /api/v1/auth/login
POST   /api/v1/auth/logout
```

### Admin (2)

```
GET    /api/v1/admin/users
POST   /api/v1/admin/users/:id/suspend
```

---

## 🔧 テスト実装テンプレート

```rust
// tests/deprecation_headers_test.rs
use reqwest::Client;

#[tokio::test]
async fn test_v1_user_get_has_deprecation_header() {
    let client = Client::new();
    let response = client
        .get("http://localhost:3000/api/v1/users/123")
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.headers().get("Deprecation"),
        Some(&"true".parse().unwrap()),
        "GET /api/v1/users/:id should have Deprecation header"
    );

    assert!(
        response.headers().contains_key("Sunset"),
        "GET /api/v1/users/:id should have Sunset header"
    );

    let link = response.headers().get("Link").unwrap().to_str().unwrap();
    assert!(
        link.contains("/api/v2/users/123"),
        "Link header should point to v2 endpoint"
    );
}

#[tokio::test]
async fn test_v1_post_create_has_deprecation_header() {
    let client = Client::new();
    let response = client
        .post("http://localhost:3000/api/v1/posts")
        .json(&serde_json::json!({
            "title": "Test Post",
            "content": "Test content"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.headers().get("Deprecation"),
        Some(&"true".parse().unwrap())
    );
}

#[tokio::test]
async fn test_v2_endpoints_no_deprecation_header() {
    let client = Client::new();
    let response = client
        .get("http://localhost:3000/api/v2/users/123")
        .send()
        .await
        .unwrap();

    assert!(
        !response.headers().contains_key("Deprecation"),
        "v2 endpoints should NOT have Deprecation header"
    );
}
```

---

## 📈 クライアント移行ガイド (ドキュメント案)

### API v1 → v2 移行ガイド

#### 概要

API v1 は **2025-03-17 に削除予定** です。今すぐ v2 への移行を開始してください。

#### 主要な変更点

| 機能 | v1 | v2 | 移行手順 |
|------|----|----|--------|
| ベースURL | `/api/v1` | `/api/v2` | URL を置換 |
| エラーレスポンス | `{ error: string }` | `{ errors: [...] }` | エラー配列対応 |
| ページネーション | `?page=1&limit=10` | `?offset=0&limit=10` | オフセット方式に変更 |
| 認証 | `Authorization: Bearer` | `Authorization: Bearer` | 同じ (変更なし) |

#### ステップバイステップ

1. **URL 置換**

   ```bash
   # Before
   curl https://api.example.com/api/v1/users/123

   # After
   curl https://api.example.com/api/v2/users/123
   ```

2. **エラーハンドリング更新**

   ```javascript
   // Before
   if (response.error) {
     console.error(response.error);
   }

   // After
   if (response.errors && response.errors.length > 0) {
     response.errors.forEach(err => console.error(err));
   }
   ```

3. **ページネーション更新**

   ```bash
   # Before
   curl 'https://api.example.com/api/v1/posts?page=2&limit=10'

   # After
   curl 'https://api.example.com/api/v2/posts?offset=10&limit=10'
   ```

---

## ⏰ タイムラインと成功指標

### Week 1: ヘッダー追加 & 通知

- [ ] Deprecation ヘッダー実装
- [ ] 全 v1 エンドポイント対応 (50/50)
- [ ] テスト作成 (50+ tests)
- [ ] ドキュメント公開

### Week 2-3: クライアント移行

- [ ] v1 アクセス < 50% (7日後)
- [ ] v1 アクセス < 20% (14日後)
- [ ] v1 アクセス < 10% (21日後)

### 成功指標

| 指標 | 目標 | 達成条件 |
|------|------|--------|
| **ヘッダー適用率** | 100% | 全 v1 EP が持つ |
| **テスト合格率** | 100% | 50+ テスト全パス |
| **v1 → v2 移行率** | > 90% | ログ分析 |
| **ドキュメント完成度** | 100% | ガイド + API doc |

---

**作成日**: 2025-01-17
**ステータス**: 実装準備中
**次フェーズ**: Phase 5-4 (2025-01-24 開始)
