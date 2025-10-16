# API v1 → v2 クライアント移行ガイド

**対象**: すべての RustCMS API クライアント (JavaScript, Python, Go, Rust など)
**非推奨開始**: 2025-01-17
**削除予定日**: 2025-03-17
**移行期間**: 2ヶ月

---

## 🚀 概要

### なぜ移行が必要か

- **API v1 は 2025-03-17 に削除されます**
- **v2 には新機能と改善が含まれています**
- **データベースパフォーマンス 25-66% 向上**
- **よりモダンな REST API 設計**

### 移行の難易度

- **JavaScript/TypeScript**: ⭐ 簡単 (URL 置換のみ)
- **Python**: ⭐ 簡単 (URL 置換のみ)
- **Go**: ⭐ 簡単 (URL 置換のみ)
- **Rust**: ⭐ 簡単 (URL 置換のみ)

---

## 📋 Phase 別移行計画

### Phase 1: 準備 (1 週間)

**タスク**:

- [ ] 現在の v1 API 使用状況を確認
- [ ] 変更点を理解
- [ ] テストスイートを準備

**確認コマンド**:

```bash
# 1. ログから v1 API 呼び出しを検出
grep -r "/api/v1/" src/

# 2. v1 エンドポイント使用数をカウント
grep -r "/api/v1/" src/ | wc -l
```

### Phase 2: 実装 (1 週間)

**タスク**:

- [ ] URL をすべて v2 に更新
- [ ] エラーハンドリングを修正
- [ ] ページネーションロジックを修正

### Phase 3: テスト (1 週間)

**タスク**:

- [ ] ユニットテスト実行
- [ ] インテグレーションテスト実行
- [ ] ステージング環境でテスト

### Phase 4: デプロイ (1 週間)

**タスク**:

- [ ] 開発環境にデプロイ
- [ ] ステージング環境にデプロイ
- [ ] 本番環境にデプロイ

---

## 🔄 主要な変更点

### 1. ベース URL の変更

| 変更前 (v1) | 変更後 (v2) |
|-----------|----------|
| `/api/v1` | `/api/v2` |

**最も簡単な対応**: グローバル置換で完了

```javascript
// Before
const API_BASE = 'https://api.example.com/api/v1';

// After
const API_BASE = 'https://api.example.com/api/v2';
```

### 2. エラーレスポンス形式の変更

**v1 エラーレスポンス**:

```json
HTTP/1.1 400 Bad Request
{
  "error": "Invalid email format"
}
```

**v2 エラーレスポンス**:

```json
HTTP/1.1 400 Bad Request
{
  "errors": [
    {
      "field": "email",
      "message": "Invalid email format",
      "code": "INVALID_FORMAT"
    }
  ],
  "request_id": "req-12345"
}
```

**更新コード例**:

```javascript
// Before (v1)
try {
  const response = await fetch(`${API_BASE}/users`, { method: 'POST', body });
  if (!response.ok) {
    const data = await response.json();
    console.error('Error:', data.error); // ✅ v1: string
  }
} catch (e) {
  console.error(e);
}

// After (v2)
try {
  const response = await fetch(`${API_BASE}/users`, { method: 'POST', body });
  if (!response.ok) {
    const data = await response.json();
    console.error('Errors:', data.errors); // ✅ v2: array
    data.errors.forEach(err => {
      console.log(`${err.field}: ${err.message} (${err.code})`);
    });
  }
} catch (e) {
  console.error(e);
}
```

### 3. ページネーションの変更

**v1 ページネーション** (ページベース):

```bash
GET /api/v1/posts?page=2&limit=10
# 2ページ目、1ページ 10 件
```

**v2 ページネーション** (オフセットベース):

```bash
GET /api/v2/posts?offset=10&limit=10
# 10 件スキップ後、10 件取得
```

**変換ロジック**:

```python
# Before (v1)
def get_page_offset_v1(page: int, limit: int) -> int:
    return (page - 1) * limit

# After (v2)
def get_page_offset_v2(page: int, limit: int) -> int:
    return (page - 1) * limit  # Same calculation!
    # Just change the parameter name in the URL

# Example migration
# v1: page=2&limit=10 → offset=(2-1)*10=10
# v2: offset=10&limit=10 → Same result
```

---

## 💻 言語別移行ガイド

### JavaScript / TypeScript

#### 1. axios を使用している場合

```javascript
// client.js

// Before (v1)
const client = axios.create({
  baseURL: 'https://api.example.com/api/v1',
});

// After (v2)
const client = axios.create({
  baseURL: 'https://api.example.com/api/v2',
});

// エラーハンドリング更新
client.interceptors.response.use(
  response => response,
  error => {
    if (error.response?.status === 400) {
      // v1: error.response.data.error
      // v2: error.response.data.errors[0]
      const errors = error.response.data.errors || [];
      throw new Error(errors.map(e => e.message).join(', '));
    }
    throw error;
  }
);
```

#### 2. fetch を使用している場合

```javascript
// api.js

class APIClient {
  constructor() {
    this.baseUrl = 'https://api.example.com/api/v2'; // Changed from v1
  }

  async request(endpoint, options = {}) {
    const response = await fetch(`${this.baseUrl}${endpoint}`, options);

    if (!response.ok) {
      const data = await response.json();

      // v1: throw new Error(data.error)
      // v2: throw new Error with multiple errors
      const errors = data.errors || [];
      const message = errors.map(e => `${e.field}: ${e.message}`).join('; ');
      throw new Error(message || 'Unknown error');
    }

    return response.json();
  }

  async getPosts(offset = 0, limit = 10) {
    // Changed from: ?page=1&limit=10
    // Changed to:   ?offset=0&limit=10
    return this.request(`/posts?offset=${offset}&limit=${limit}`);
  }
}

export default new APIClient();
```

### Python

#### 1. requests を使用している場合

```python
import requests

# client.py

class APIClient:
    def __init__(self):
        # Before: self.base_url = 'https://api.example.com/api/v1'
        # After:
        self.base_url = 'https://api.example.com/api/v2'
        self.session = requests.Session()

    def request(self, method, endpoint, **kwargs):
        url = f"{self.base_url}{endpoint}"
        response = self.session.request(method, url, **kwargs)

        if not response.ok:
            data = response.json()

            # v1: errors = [data['error']]
            # v2: errors = [f"{e['field']}: {e['message']}" for e in data['errors']]
            errors = data.get('errors', [])
            message = '; '.join([f"{e['field']}: {e['message']}" for e in errors])
            raise Exception(message or 'Unknown error')

        return response.json()

    def get_posts(self, offset=0, limit=10):
        # Changed from: ?page=1&limit=10
        # Changed to:   ?offset=0&limit=10
        return self.request('GET', f'/posts?offset={offset}&limit={limit}')

# Usage
client = APIClient()
posts = client.get_posts()
```

### Go

#### 1. net/http を使用している場合

```go
package api

import (
    "encoding/json"
    "fmt"
    "net/http"
)

// Before: const apiBase = "https://api.example.com/api/v1"
// After:
const apiBase = "https://api.example.com/api/v2"

type ErrorResponse struct {
    Errors []ErrorDetail `json:"errors"`
}

type ErrorDetail struct {
    Field   string `json:"field"`
    Message string `json:"message"`
    Code    string `json:"code"`
}

func GetPosts(offset, limit int) ([]Post, error) {
    // Changed from: /posts?page=1&limit=10
    // Changed to:   /posts?offset=0&limit=10
    url := fmt.Sprintf("%s/posts?offset=%d&limit=%d", apiBase, offset, limit)

    resp, err := http.Get(url)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    if resp.StatusCode >= 400 {
        var errResp ErrorResponse
        json.NewDecoder(resp.Body).Decode(&errResp)

        var message string
        for _, err := range errResp.Errors {
            message += fmt.Sprintf("%s: %s; ", err.Field, err.Message)
        }
        return nil, fmt.Errorf(message)
    }

    var posts []Post
    json.NewDecoder(resp.Body).Decode(&posts)
    return posts, nil
}
```

### Rust

#### 1. reqwest を使用している場合

```rust
use reqwest::Client;

const API_BASE: &str = "https://api.example.com/api/v2"; // Changed from v1

pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_posts(&self, offset: u32, limit: u32) -> Result<Vec<Post>, String> {
        // Changed from: /posts?page=1&limit=10
        // Changed to:   /posts?offset=0&limit=10
        let url = format!("{}​/posts?offset={}&limit={}", API_BASE, offset, limit);

        let response = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let error_data: serde_json::Value = response.json().await
                .map_err(|e| e.to_string())?;

            // v1: error_data["error"].as_str()
            // v2: error_data["errors"][0]["message"]
            let errors = error_data["errors"].as_array().unwrap_or(&vec![]);
            let message = errors.iter()
                .map(|e| {
                    format!("{}: {}",
                        e["field"].as_str().unwrap_or("unknown"),
                        e["message"].as_str().unwrap_or("unknown")
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");

            return Err(message);
        }

        response.json().await.map_err(|e| e.to_string())
    }
}
```

---

## 🧪 テスト例

### テストケース: POST /api/v2/users

#### 成功レスポンス

```bash
curl -X POST https://api.example.com/api/v2/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "john_doe",
    "email": "john@example.com",
    "password": "SecurePass123!"
  }'

# Response (201 Created):
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "john_doe",
  "email": "john@example.com",
  "created_at": "2025-01-17T10:30:00Z"
}
```

#### エラーレスポンス (v2 新形式)

```bash
curl -X POST https://api.example.com/api/v2/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "john_doe",
    "email": "invalid_email",  # Invalid format
    "password": "short"  # Too short
  }'

# Response (400 Bad Request):
{
  "errors": [
    {
      "field": "email",
      "message": "Invalid email format",
      "code": "INVALID_EMAIL_FORMAT"
    },
    {
      "field": "password",
      "message": "Password must be at least 8 characters",
      "code": "PASSWORD_TOO_SHORT"
    }
  ],
  "request_id": "req-12345abcde"
}
```

---

## 📊 移行チェックリスト

### 事前確認

- [ ] 現在の v1 API エンドポイント一覧を作成
- [ ] 使用中のクライアントライブラリを確認
- [ ] バックアップを取得
- [ ] ステージング環境で テスト

### 実装

- [ ] ベース URL を `/api/v1` から `/api/v2` に変更
- [ ] エラーハンドリング を更新 (`error` → `errors` 配列)
- [ ] ページネーション を更新 (`page` → `offset` ベース)
- [ ] すべてのエンドポイントをテスト

### テスト

- [ ] ユニットテスト実行
- [ ] インテグレーションテスト実行
- [ ] ステージング環境で E2E テスト実行
- [ ] ローカルで全テスト実行

### デプロイ

- [ ] 開発環境にデプロイ
- [ ] ステージング環境でテスト
- [ ] 本番環境にデプロイ (段階的に)
- [ ] ログを監視

### 検証

- [ ] 本番環境で 24 時間監視
- [ ] エラーログ確認
- [ ] パフォーマンス指標確認
- [ ] v1 API アクセスを監視

---

## 🆘 問題が発生した場合

### エラー: 404 Not Found

**原因**: エンドポイントが v2 に移動した可能性

**解決策**:

```
# v1: /api/v1/users/:id
# v2: /api/v2/users/:id (同じ)

# 確認: /api/v1 を /api/v2 に置換したか確認
grep -r "/api/v1/" .
```

### エラー: 400 Bad Request (エラー形式が異なる)

**原因**: v2 エラー形式が異なる

**解決策**:

```javascript
// Before (v1)
const error = response.data.error;

// After (v2)
const errors = response.data.errors;
const message = errors.map(e => e.message).join(', ');
```

### エラー: ページネーション が機能しない

**原因**: `page` パラメータが v2 で廃止された

**解決策**:

```
# Before (v1)
/api/v1/posts?page=2&limit=10

# After (v2)
/api/v2/posts?offset=10&limit=10

# 計算: offset = (page - 1) * limit
#       offset = (2 - 1) * 10 = 10
```

---

## 📞 サポート

問題が発生した場合は、以下に連絡してください:

- **メール**: <support@example.com>
- **Slack**: #api-support
- **GitHub Issues**: <https://github.com/example/cms/issues>
- **ドキュメント**: <https://docs.example.com/migration>

---

**作成日**: 2025-01-17
**最終更新**: 2025-01-17
**ステータス**: 公開準備中
