# API ドキュメント

このドキュメントは、プロジェクト内で標準化された API レスポンス形状とページネーション、及び消費側の移行方法をまとめたものです。

## 標準レスポンス `ApiResponse<T>`
共通 API レスポンス型は `src/utils/api_types.rs` に定義されています。全ての HTTP ハンドラは基本的に次の形で JSON を返します。

- 成功時:
  {
    "success": true,
    "data": <T>,
    "message": <optional string>
  }

- 失敗時:
  {
    "success": false,
    "error": <string>
  }

Rust 型定義（抜粋）:

```rust
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self { /* ... */ }
    pub fn success_with_message(data: T, message: String) -> Self { /* ... */ }
    pub fn error(error: String) -> ApiResponse<()> { /* ... */ }
}
```

## ページネーション
ページネーション情報は `Pagination` と `PaginatedResponse<T>` を使って返します。

成功ページ付きレスポンス例:

{
  "success": true,
  "data": {
    "data": [ ... items ... ],
    "pagination": { "page": 1, "per_page": 10, "total": 123, "total_pages": 13 }
  }
}

Rust 型（抜粋）:

```rust
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}
```

## ハンドラの記述例
- 単一リソースの成功:

```rust
Ok(Json(ApiResponse::success(item)))
```

- 作成時（メッセージ付き）:

```rust
Ok((StatusCode::CREATED, Json(ApiResponse::success_with_message(created, "Created".to_string()))))
```

- エラー時（汎用）:

```rust
Err(AppError::NotFound("..."))
// または
Ok((StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Not found".to_string()))))
```

## 既存コードの移行ポイント
以前は一部バイナリや例 (`simple-cms.rs`, `src/bin/cms-simple.rs`, `src/utils/response.rs` 等) が独自の `ApiResponse` を定義していました。移行手順:

1. 共通型を使う: `use crate::utils::api_types::ApiResponse`（バイナリでは `use cms_backend::utils::api_types::ApiResponse`）
2. 成功は `ApiResponse::success(...)` か `ApiResponse::success_with_message(...)` を使う
3. 直接 `ApiResponse { success: ..., data: ..., pagination: ... }` の構築をやめ、`PaginatedResponse` を作る
4. 消費側は `ApiResponse<T>` を受け取るので、直アクセスする場合は `response.data` を参照する

例: `let count = api_response.data.as_ref().map(|v| v.len()).unwrap_or(0);`

## テストと CI
- 既に多数のテストが `tests/` に存在します。変更後は `cargo test` で回帰チェックを実施してください。
- 一部 CLI テストは環境に依存するためエラーを許容する形に変更されています（`missing field \`environment\`` 等）。CI 環境では設定を与えるか、その振る舞いを理解してお使いください。

## 追記: 今後の改善案
- OpenAPI ドキュメントの `components/schemas` に `ApiResponse` と `PaginatedResponse` を明示的に出力してクライアントコード生成を容易にする
- `AppError` を `ApiResponse` のエラーペイロードにマップして一貫したエラーフォーマットを提供する

## クライアント使用例

### TypeScript (fetch) — 単純な API 呼び出し

次の例は、`/api/posts` から `ApiResponse<PaginatedResponse<Post>>` を取得してデータを取り出す方法です。

```ts
type ApiResponse<T> = {
  success: boolean;
  data?: T;
  message?: string;
  error?: string;
}

type Pagination = { page: number; per_page: number; total: number; total_pages: number };
type PaginatedResponse<T> = { data: T[]; pagination: Pagination };

async function fetchPosts() {
  const res = await fetch('/api/posts?page=1&limit=10');
  const json: ApiResponse<PaginatedResponse<Post>> = await res.json();
  if (!json.success) throw new Error(json.error || 'Unknown error');
  const posts = json.data?.data ?? [];
  const pagination = json.data?.pagination;
  return { posts, pagination };
}
```

### Rust (reqwest) — サーバー間呼び出しの例

```rust
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct ApiResponse<T> { success: bool, data: Option<T>, message: Option<String>, error: Option<String> }

#[derive(Deserialize)]
struct Pagination { page: u32, per_page: u32, total: u64, total_pages: u32 }

#[derive(Deserialize)]
struct PaginatedResponse<T> { data: Vec<T>, pagination: Pagination }

async fn get_posts(client: &Client) -> anyhow::Result<(Vec<Post>, Pagination)> {
    let resp = client.get("http://localhost:3000/api/posts?page=1&limit=10").send().await?;
    let body: ApiResponse<PaginatedResponse<Post>> = resp.json().await?;
    if !body.success { anyhow::bail!(body.error.unwrap_or_else(|| "api error".to_string())); }
    let data = body.data.expect("expected data");
    Ok((data.data, data.pagination))
}
```

## 移行チェックリスト（開発者向け）
ハンドラとバイナリを共通 `ApiResponse` に揃えるための短いチェックリスト:

- [ ] 依存箇所の検出
  - ワークスペース内で `ApiResponse {` や `success_response`、`.0.len(` などを検索し、古い直書き箇所を特定します。
- [ ] ハンドラ側の修正
  - 成功は `ApiResponse::success(...)`、作成は `ApiResponse::success_with_message(...)` を使う。
  - ページネーションは `PaginatedResponse<T>` をデータ部として返す。
- [ ] 消費側（バイナリ／テスト）の修正
  - `ApiResponse<T>` を受け取るようにし、必要に応じて `response.data` を参照して中身を扱う。
  - 例: `let count = api_response.data.as_ref().map(|p| p.data.len()).unwrap_or(0);`
- [ ] 型推論が失敗する箇所の注釈
  - `ApiResponse::error(...)` は型推論が効かない場合 `ApiResponse::<()>::error(...)` のように明示的に注釈する。
- [ ] テストと CI の調整
  - 環境依存の CLI テストは、設定が無い環境でも失敗扱いにならないよう許容も検討するか、テスト側でモックを行う。

## 参考コマンド（検索用）
プロジェクトルートで古いパターンを探す簡単なコマンド例（PowerShell）:

```powershell
Select-String -Path "**\*.rs" -Pattern "ApiResponse \{" -SimpleMatch
Select-String -Path "**\*.rs" -Pattern "\.0\.len\(" -SimpleMatch
```

---
更新: 2025-08-20

## 一覧APIのフィルタ/ソート仕様（追記）

以下の一覧APIでは、指定されたフィルタ条件に一致する総件数（total）が返却されます。総件数はフィルタを無視せず、ページネーションの精度を保証します。

- GET /api/posts
  - クエリ: page, limit, status, author, tag, sort
  - sort の値: "created_at", "updated_at", "published_at", "title"（先頭に '-' で降順: 例 "-created_at"）
  - 注意: published_at の NULL は安定した順序になるよう内部で補助キーを併用

- GET /api/users
  - クエリ: page, limit, role, active, sort
  - sort の値: "created_at", "updated_at", "username"（同上で '-' で降順）

返却形式は従来どおり `ApiResponse<PaginatedResponse<T>>`。`pagination.total` は各 API で適用されたフィルタ後の件数です。

更新: 2025-09-07
