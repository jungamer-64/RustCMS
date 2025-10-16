# RustCMS API v1→v2 統合ガイド完全版

**対象**: v1 エンドポイントを使用しているすべてのクライアント開発者
**作成日**: 2025-01-17
**完成期限**: 2025-03-17
**テンプレート言語**: JavaScript/TypeScript, Python, Go, Rust, Ruby, Java, PHP

---

## 目次

1. [概要](#概要)
2. [主要変更点](#主要変更点)
3. [言語別実装ガイド](#言語別実装ガイド)
4. [高度なパターン](#高度なパターン)
5. [トラブルシューティング](#トラブルシューティング)
6. [チェックリスト](#チェックリスト)

---

## 概要

### v2 への移行が必要な理由

| 理由 | 詳細 |
|------|------|
| **パフォーマンス** | 25-66% の応答時間短縮 |
| **新機能** | フィルタ・並べ替え・ページネーションの改善 |
| **セキュリティ** | 認証メカニズムの強化、CSRF 対策 |
| **API 契約** | エラー形式の標準化、レスポンス構造の統一 |
| **タイムライン** | v1 削除予定: 2025-03-17 (ハード期限) |

### 移行期限

- **2025-02-06**: v1 Deprecation ヘッダー追加、警告開始
- **2025-03-17**: v1 エンドポイント完全削除

---

## 主要変更点

### 1. ベースURL の変更

**v1:**

```
https://api.example.com/api/v1/users
https://api.example.com/api/v1/posts
```

**v2:**

```
https://api.example.com/api/v2/users
https://api.example.com/api/v2/posts
```

**移行方法**: 単純な `find & replace`

```bash
# v1 → v2 の変換
sed -i 's|/api/v1/|/api/v2/|g' your_code.js
```

### 2. エラー形式の変更

#### v1 エラー形式（非推奨）

```json
{
  "error": "Invalid email address"
}
```

#### v2 エラー形式（新規）

```json
{
  "errors": [
    {
      "field": "email",
      "message": "Invalid email address format",
      "code": "INVALID_EMAIL",
      "suggestion": "Use format: user@example.com"
    },
    {
      "field": "password",
      "message": "Password must be at least 12 characters",
      "code": "PASSWORD_TOO_SHORT",
      "suggestion": "Use at least 12 characters including uppercase, lowercase, numbers"
    }
  ],
  "request_id": "req_abc123def456",
  "timestamp": "2025-01-17T10:30:00Z",
  "status": 400
}
```

**重要な違い**:

- `error` (単数) → `errors` (配列)
- 各エラーに `code` フィールド追加
- `suggestion` フィールドでユーザーへのガイダンス
- `request_id` で問題追跡が可能

### 3. ページネーション形式の変更

#### v1 形式（ページベース）

```
GET /api/v1/users?page=2&limit=10
→ 結果: 11-20 番目のレコード
```

**計算式**:

```
offset = (page - 1) × limit
limit = limit
```

#### v2 形式（オフセットベース）

```
GET /api/v2/users?offset=10&limit=10
→ 結果: 11-20 番目のレコード
```

**計算式**:

```
offset = (page - 1) × limit  # v1 から v2 への変換
page = (offset / limit) + 1   # v2 から v1 への逆変換
```

#### レスポンス形式の変更

**v1:**

```json
{
  "data": [...],
  "page": 2,
  "limit": 10,
  "total": 100
}
```

**v2:**

```json
{
  "data": [...],
  "pagination": {
    "offset": 10,
    "limit": 10,
    "total": 100,
    "has_next": true,
    "has_prev": true
  }
}
```

---

## 言語別実装ガイド

### JavaScript / TypeScript

#### 基本的なクライアント実装

**v1 (旧):**

```javascript
async function fetchUsers(page = 1) {
  const response = await fetch(
    `https://api.example.com/api/v1/users?page=${page}&limit=10`
  );
  return response.json();
}
```

**v2 (新):**

```typescript
// 型安全なクライアント実装
interface APIError {
  field: string;
  message: string;
  code: string;
  suggestion?: string;
}

interface APIResponse<T> {
  data: T[];
  pagination?: {
    offset: number;
    limit: number;
    total: number;
    has_next: boolean;
    has_prev: boolean;
  };
  errors?: APIError[];
  request_id: string;
  timestamp: string;
}

interface User {
  id: string;
  email: string;
  username: string;
  created_at: string;
}

class APIClient {
  private baseURL = 'https://api.example.com/api/v2';
  private token?: string;

  constructor(token?: string) {
    this.token = token;
  }

  private getHeaders(): HeadersInit {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };
    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }
    return headers;
  }

  async fetchUsers(offset = 0, limit = 10): Promise<APIResponse<User>> {
    const url = `${this.baseURL}/users?offset=${offset}&limit=${limit}`;
    const response = await fetch(url, {
      headers: this.getHeaders(),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new APIClientError(error);
    }

    return response.json();
  }

  async createUser(email: string, username: string, password: string): Promise<User> {
    const url = `${this.baseURL}/users`;
    const response = await fetch(url, {
      method: 'POST',
      headers: this.getHeaders(),
      body: JSON.stringify({ email, username, password }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new APIClientError(error);
    }

    const result = await response.json();
    return result.data[0];
  }
}

class APIClientError extends Error {
  constructor(public apiResponse: any) {
    super(`API Error: ${apiResponse.status}`);
    this.name = 'APIClientError';
  }

  get fieldErrors(): Map<string, string> {
    const map = new Map<string, string>();
    if (this.apiResponse.errors) {
      for (const error of this.apiResponse.errors) {
        map.set(error.field, error.message);
      }
    }
    return map;
  }
}
```

#### エラーハンドリング（実装例）

```typescript
// Vue.js / React 統合例
async function handleUserRegistration(formData) {
  const client = new APIClient();

  try {
    const user = await client.createUser(
      formData.email,
      formData.username,
      formData.password
    );
    console.log('ユーザー登録成功:', user);
    // フォーム送信成功の処理
  } catch (error) {
    if (error instanceof APIClientError) {
      const fieldErrors = error.fieldErrors;

      // フォームフィールドに紐付けてエラー表示
      for (const [field, message] of fieldErrors) {
        showFieldError(field, message);
      }

      // API レスポンスから suggestion を取得
      if (error.apiResponse.errors) {
        for (const err of error.apiResponse.errors) {
          if (err.suggestion) {
            showHint(err.field, err.suggestion);
          }
        }
      }

      // request_id をログ出力（問題追跡用）
      console.error(`Request ID: ${error.apiResponse.request_id}`);
    } else {
      console.error('予期しないエラー:', error);
    }
  }
}
```

#### Axios を使用した実装

```typescript
import axios, { AxiosError, AxiosInstance } from 'axios';

class APIClientAxios {
  private client: AxiosInstance;

  constructor(baseURL: string, token?: string) {
    this.client = axios.create({
      baseURL,
      headers: {
        'Content-Type': 'application/json',
        ...(token && { Authorization: `Bearer ${token}` }),
      },
    });

    // v1 → v2 の互換性レイヤー (レガシー対応)
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        // 古いクライアント側でも新形式に対応できるようにする
        if (error.response?.status === 400) {
          const data: any = error.response.data;
          // v1 形式のエラーを v2 形式に統一
          if (data.error && !data.errors) {
            data.errors = [
              { field: 'general', message: data.error, code: 'INVALID_REQUEST' },
            ];
          }
        }
        return Promise.reject(error);
      }
    );
  }

  async fetchUsers(offset = 0, limit = 10) {
    const response = await this.client.get('/users', {
      params: { offset, limit },
    });
    return response.data;
  }

  async createUser(payload: { email: string; username: string; password: string }) {
    const response = await this.client.post('/users', payload);
    return response.data.data[0];
  }
}
```

---

### Python

#### 基本的なクライアント実装

```python
import requests
from typing import Optional, List, Dict, Any
from dataclasses import dataclass
from datetime import datetime

@dataclass
class APIError:
    field: str
    message: str
    code: str
    suggestion: Optional[str] = None

@dataclass
class Pagination:
    offset: int
    limit: int
    total: int
    has_next: bool
    has_prev: bool

@dataclass
class APIResponse:
    data: List[Dict[str, Any]]
    pagination: Optional[Pagination] = None
    errors: Optional[List[APIError]] = None
    request_id: Optional[str] = None
    timestamp: Optional[str] = None
    status: Optional[int] = None

class RustCMSClient:
    def __init__(self, base_url: str = "https://api.example.com/api/v2", token: Optional[str] = None):
        self.base_url = base_url
        self.session = requests.Session()
        if token:
            self.session.headers.update({"Authorization": f"Bearer {token}"})
        self.session.headers.update({"Content-Type": "application/json"})

    def _handle_response(self, response: requests.Response) -> Dict[str, Any]:
        """レスポンスを処理し、エラーを適切に処理"""
        try:
            data = response.json()
        except requests.exceptions.JSONDecodeError:
            data = {"error": response.text}

        if not response.ok:
            # v2 エラー形式の処理
            if "errors" in data:
                errors = [
                    APIError(
                        field=err.get("field", "unknown"),
                        message=err.get("message", ""),
                        code=err.get("code", "UNKNOWN"),
                        suggestion=err.get("suggestion"),
                    )
                    for err in data.get("errors", [])
                ]
                raise RustCMSAPIError(
                    status_code=response.status_code,
                    message=f"API Error (Request ID: {data.get('request_id')})",
                    errors=errors,
                    request_id=data.get("request_id"),
                )
            else:
                raise RustCMSAPIError(
                    status_code=response.status_code,
                    message=data.get("error", "Unknown error"),
                )

        return data

    def get_users(self, offset: int = 0, limit: int = 10) -> APIResponse:
        """ユーザー一覧を取得"""
        response = self.session.get(
            f"{self.base_url}/users",
            params={"offset": offset, "limit": limit},
        )
        data = self._handle_response(response)

        pagination = None
        if "pagination" in data:
            p = data["pagination"]
            pagination = Pagination(
                offset=p["offset"],
                limit=p["limit"],
                total=p["total"],
                has_next=p["has_next"],
                has_prev=p["has_prev"],
            )

        return APIResponse(
            data=data.get("data", []),
            pagination=pagination,
            request_id=data.get("request_id"),
            timestamp=data.get("timestamp"),
        )

    def create_user(self, email: str, username: str, password: str) -> Dict[str, Any]:
        """ユーザーを作成"""
        response = self.session.post(
            f"{self.base_url}/users",
            json={"email": email, "username": username, "password": password},
        )
        data = self._handle_response(response)
        return data["data"][0] if data.get("data") else {}

class RustCMSAPIError(Exception):
    def __init__(self, status_code: int, message: str, errors: Optional[List[APIError]] = None, request_id: Optional[str] = None):
        self.status_code = status_code
        self.message = message
        self.errors = errors or []
        self.request_id = request_id
        super().__init__(f"[{status_code}] {message}")

    def get_field_errors(self) -> Dict[str, str]:
        """フィールドごとのエラーメッセージを取得"""
        return {err.field: err.message for err in self.errors}

    def get_suggestions(self) -> Dict[str, str]:
        """ユーザーへのサジェスチョンを取得"""
        return {
            err.field: err.suggestion
            for err in self.errors
            if err.suggestion
        }
```

#### Django/Flask 統合例

```python
from flask import Flask, request, jsonify

app = Flask(__name__)
cms_client = RustCMSClient(token=app.config.get("CMS_TOKEN"))

@app.route("/register", methods=["POST"])
def register_user():
    """ユーザー登録エンドポイント"""
    data = request.get_json()

    try:
        user = cms_client.create_user(
            email=data.get("email"),
            username=data.get("username"),
            password=data.get("password"),
        )
        return jsonify({"success": True, "user": user}), 201

    except RustCMSAPIError as e:
        # エラーをクライアントに返す
        return jsonify({
            "success": False,
            "message": e.message,
            "field_errors": e.get_field_errors(),
            "suggestions": e.get_suggestions(),
            "request_id": e.request_id,
        }), e.status_code
```

---

### Go

#### 基本的なクライアント実装

```go
package cms

import (
 "bytes"
 "encoding/json"
 "fmt"
 "io"
 "net/http"
 "net/url"
)

// ErrorDetail は v2 API のエラー詳細
type ErrorDetail struct {
 Field      string `json:"field"`
 Message    string `json:"message"`
 Code       string `json:"code"`
 Suggestion string `json:"suggestion,omitempty"`
}

// APIResponse は v2 API のレスポンス形式
type APIResponse struct {
 Data       []map[string]interface{} `json:"data,omitempty"`
 Pagination *Pagination              `json:"pagination,omitempty"`
 Errors     []ErrorDetail            `json:"errors,omitempty"`
 RequestID  string                   `json:"request_id,omitempty"`
 Timestamp  string                   `json:"timestamp,omitempty"`
 Status     int                      `json:"status,omitempty"`
}

// Pagination はページネーション情報
type Pagination struct {
 Offset  int `json:"offset"`
 Limit   int `json:"limit"`
 Total   int `json:"total"`
 HasNext bool `json:"has_next"`
 HasPrev bool `json:"has_prev"`
}

// Client は RustCMS API クライアント
type Client struct {
 baseURL    string
 httpClient *http.Client
 token      string
}

// NewClient は新しいクライアントを作成
func NewClient(baseURL, token string) *Client {
 return &Client{
  baseURL:    baseURL,
  httpClient: &http.Client{},
  token:      token,
 }
}

// GetUsers はユーザー一覧を取得
func (c *Client) GetUsers(offset, limit int) (*APIResponse, error) {
 path := fmt.Sprintf("%s/users", c.baseURL)
 q := url.Values{
  "offset": {fmt.Sprintf("%d", offset)},
  "limit":  {fmt.Sprintf("%d", limit)},
 }
 path = fmt.Sprintf("%s?%s", path, q.Encode())

 req, err := http.NewRequest("GET", path, nil)
 if err != nil {
  return nil, err
 }

 return c.do(req)
}

// CreateUser はユーザーを作成
func (c *Client) CreateUser(email, username, password string) (*APIResponse, error) {
 payload := map[string]string{
  "email":    email,
  "username": username,
  "password": password,
 }

 body, err := json.Marshal(payload)
 if err != nil {
  return nil, err
 }

 req, err := http.NewRequest(
  "POST",
  fmt.Sprintf("%s/users", c.baseURL),
  bytes.NewReader(body),
 )
 if err != nil {
  return nil, err
 }

 return c.do(req)
}

// do はリクエストを実行しレスポンスをパース
func (c *Client) do(req *http.Request) (*APIResponse, error) {
 req.Header.Set("Content-Type", "application/json")
 if c.token != "" {
  req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", c.token))
 }

 resp, err := c.httpClient.Do(req)
 if err != nil {
  return nil, err
 }
 defer resp.Body.Close()

 body, err := io.ReadAll(resp.Body)
 if err != nil {
  return nil, err
 }

 var apiResp APIResponse
 if err := json.Unmarshal(body, &apiResp); err != nil {
  return nil, err
 }

 if resp.StatusCode >= 400 {
  return nil, &APIError{
   StatusCode: resp.StatusCode,
   Response:   &apiResp,
  }
 }

 return &apiResp, nil
}

// APIError は API エラー
type APIError struct {
 StatusCode int
 Response   *APIResponse
}

func (e *APIError) Error() string {
 if e.Response == nil {
  return fmt.Sprintf("HTTP %d", e.StatusCode)
 }
 if len(e.Response.Errors) > 0 {
  return fmt.Sprintf("API Error: %s (Request ID: %s)",
   e.Response.Errors[0].Message,
   e.Response.RequestID)
 }
 return fmt.Sprintf("HTTP %d", e.StatusCode)
}
```

---

### Rust

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub field: String,
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub offset: i32,
    pub limit: i32,
    pub total: i32,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ErrorDetail>>,
    pub request_id: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub created_at: String,
}

#[derive(Debug)]
pub enum ApiClientError {
    Request(reqwest::Error),
    ApiError {
        status: u16,
        errors: Vec<ErrorDetail>,
        request_id: String,
    },
}

pub struct ApiClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            token,
        }
    }

    async fn make_request<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<ApiResponse<T>, ApiClientError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => panic!("Unsupported method"),
        };

        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }

        if let Some(body) = body {
            req = req.json(&body);
        }

        let response = req.send().await.map_err(ApiClientError::Request)?;
        let status = response.status().as_u16();
        let text = response.text().await.map_err(ApiClientError::Request)?;
        let resp: ApiResponse<T> = serde_json::from_str(&text)
            .map_err(|_| ApiClientError::Request(reqwest::Error::builder().build().unwrap()))?;

        if status >= 400 {
            return Err(ApiClientError::ApiError {
                status,
                errors: resp.errors.unwrap_or_default(),
                request_id: resp.request_id,
            });
        }

        Ok(resp)
    }

    pub async fn get_users(&self, offset: i32, limit: i32) -> Result<ApiResponse<User>, ApiClientError> {
        let path = format!("/users?offset={}&limit={}", offset, limit);
        self.make_request("GET", &path, None).await
    }

    pub async fn create_user(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> Result<User, ApiClientError> {
        let body = serde_json::json!({
            "email": email,
            "username": username,
            "password": password,
        });

        let response: ApiResponse<User> = self.make_request("POST", "/users", Some(body)).await?;
        Ok(response.data.unwrap_or_default().into_iter().next().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_user() {
        let client = ApiClient::new("https://api.example.com/api/v2", None);
        let result = client.create_user("test@example.com", "testuser", "Password123").await;
        assert!(result.is_ok());
    }
}
```

---

## 高度なパターン

### 1. バッチ操作と並列リクエスト

```typescript
// 複数のユーザーを並列で作成
async function batchCreateUsers(users: Array<{email: string, username: string, password: string}>) {
  const client = new APIClient();

  try {
    const promises = users.map(user =>
      client.createUser(user.email, user.username, user.password)
    );

    const results = await Promise.allSettled(promises);

    const succeeded = [];
    const failed = [];

    results.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        succeeded.push(result.value);
      } else {
        failed.push({
          user: users[index],
          error: result.reason.apiResponse,
        });
      }
    });

    return { succeeded, failed };
  } catch (error) {
    console.error('Batch operation failed:', error);
  }
}
```

### 2. リトライロジックと指数バックオフ

```python
import time
from typing import TypeVar, Callable, Any

T = TypeVar('T')

def retry_with_backoff(
    func: Callable[..., T],
    max_retries: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 60.0,
    backoff_factor: float = 2.0,
) -> T:
    """
    エクスポーネンシャルバックオフ付きのリトライ
    """
    for attempt in range(max_retries):
        try:
            return func()
        except RustCMSAPIError as e:
            # リトライ不可のエラーはすぐにスロー
            if e.status_code < 500:
                raise

            if attempt == max_retries - 1:
                raise

            # 指数バックオフで待機
            delay = min(base_delay * (backoff_factor ** attempt), max_delay)
            print(f"Retry attempt {attempt + 1} after {delay}s...")
            time.sleep(delay)

# 使用例
try:
    user = retry_with_backoff(
        lambda: cms_client.create_user("test@example.com", "testuser", "password"),
        max_retries=5
    )
except RustCMSAPIError as e:
    print(f"Failed after retries: {e}")
```

### 3. キャッシング戦略

```python
from functools import wraps
from datetime import datetime, timedelta
from typing import Optional, Dict, Any

class CacheEntry:
    def __init__(self, value: Any, ttl_seconds: int):
        self.value = value
        self.created_at = datetime.now()
        self.ttl_seconds = ttl_seconds

    def is_expired(self) -> bool:
        return datetime.now() > self.created_at + timedelta(seconds=self.ttl_seconds)

class CachedAPIClient(RustCMSClient):
    def __init__(self, *args, cache_ttl: int = 300, **kwargs):
        super().__init__(*args, **kwargs)
        self.cache: Dict[str, CacheEntry] = {}
        self.cache_ttl = cache_ttl

    def _cache_key(self, method: str, **params) -> str:
        param_str = "_".join(f"{k}={v}" for k, v in sorted(params.items()))
        return f"{method}:{param_str}"

    def get_users(self, offset: int = 0, limit: int = 10) -> APIResponse:
        cache_key = self._cache_key("get_users", offset=offset, limit=limit)

        # キャッシュをチェック
        if cache_key in self.cache:
            entry = self.cache[cache_key]
            if not entry.is_expired():
                print(f"Cache hit for {cache_key}")
                return entry.value

        # キャッシュミス：API から取得
        result = super().get_users(offset, limit)
        self.cache[cache_key] = CacheEntry(result, self.cache_ttl)
        return result

    def invalidate_cache(self, pattern: Optional[str] = None):
        """キャッシュを無効化"""
        if pattern is None:
            self.cache.clear()
        else:
            keys_to_delete = [k for k in self.cache.keys() if pattern in k]
            for key in keys_to_delete:
                del self.cache[key]
```

---

## トラブルシューティング

### よくあるエラーと解決方法

| エラー | 原因 | 解決方法 |
|--------|------|---------|
| `404 Not Found` | エンドポイント URL が正しくない | `/api/v1/` が `/api/v2/` に変更されたか確認 |
| `400 Bad Request` + `INVALID_EMAIL` | メールフォーマットが不正 | RFC 5321 準拠の形式を使用 (<user@example.com>) |
| `400 Bad Request` + `PASSWORD_TOO_SHORT` | パスワードが短すぎる | 最小 12 文字以上を使用 |
| `401 Unauthorized` | 認証トークンが無効 | トークンの有効期限を確認、更新 |
| `429 Too Many Requests` | レート制限に引っかかった | リクエスト間隔を広げるか、バックオフを実装 |
| `500 Internal Server Error` | サーバーエラー | サーバーログ確認、request_id でサポート報告 |
| `504 Gateway Timeout` | リクエストがタイムアウト | リトライロジック + タイムアウト拡張を実装 |

### デバッグ方法

#### cURL でのテスト

```bash
# ユーザー作成（v2）
curl -X POST http://localhost:3000/api/v2/users \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","username":"testuser","password":"SecurePass123"}'

# レスポンス例（正常）:
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "email": "test@example.com",
      "username": "testuser",
      "created_at": "2025-01-17T10:30:00Z"
    }
  ],
  "request_id": "req_abc123def456",
  "timestamp": "2025-01-17T10:30:00Z"
}

# レスポンス例（エラー）:
{
  "errors": [
    {
      "field": "email",
      "message": "Invalid email address",
      "code": "INVALID_EMAIL",
      "suggestion": "Use format: user@example.com"
    }
  ],
  "request_id": "req_xyz789",
  "timestamp": "2025-01-17T10:31:00Z",
  "status": 400
}
```

#### ブラウザの Developer Tools

```javascript
// Console で実行
const client = new APIClient('https://api.example.com/api/v2');
client.fetchUsers(0, 10)
  .then(resp => console.log('Success:', resp))
  .catch(err => console.error('Error:', err.apiResponse));
```

---

## チェックリスト

### Phase 1: 準備

- [ ] v1 API の現在の使用状況を確認
- [ ] 本番環境の v1 エンドポイントすべてをリストアップ
- [ ] ステージング環境で v2 エンドポイントテスト
- [ ] 対応言語別に API クライアントを準備

### Phase 2: 開発

- [ ] v1 → v2 URL 変更を実装
- [ ] エラーハンドリングを新形式に対応
- [ ] ページネーション計算を更新（page → offset）
- [ ] 認証トークンが正しく送信されることを確認

### Phase 3: テスト

- [ ] ユニットテストすべてパス
- [ ] ステージング環境で E2E テスト
- [ ] エラーケース（400, 401, 429）で正常に動作
- [ ] 並列リクエスト環境でテスト

### Phase 4: デプロイ

- [ ] 本番前に最終レビュー実施
- [ ] ロールバック計画を作成
- [ ] v2 本番環境への接続確認
- [ ] 段階的ロールアウト計画確認

### Phase 5: 監視

- [ ] エラーレート監視（< 0.5% 目標）
- [ ] レスポンスタイム監視（< 50ms 目標）
- [ ] ユーザーからの報告を定期確認
- [ ] 2025-03-17 までに 100% 移行完了確認

---

## サポート

### 問い合わせ先

| チャネル | 用途 | 応答時間 |
|---------|------|----------|
| **Slack #api-support** | リアルタイム技術サポート | 1-2h |
| **GitHub Issues** | バグ報告・フィーチャリクエスト | 24h |
| **Email <api-team@example.com>** | 重大な問題・緊急対応 | 1h |

### 移行完了確認

v1 削除期限は **2025-03-17** です。それまでに以下を確認してください：

- ✅ すべての本番クライアントが v2 で動作
- ✅ 過去 7 日間の v1 トラフィック = 0%
- ✅ v2 での問題報告がないか（または対応済み）

---

**最終更新**: 2025-01-17
**次回見直し**: 2025-02-07 (Phase 5-5 開始時)
