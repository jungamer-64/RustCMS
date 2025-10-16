# Phase 5-3 拡張: HTTP E2E テスト実装ガイド

**ステータス**: 🔄 実装中 (2025-01-17)
**目的**: Staging 環境での実際の HTTP 通信テストを実装し、API の動作検証を自動化

## 📋 概要

前のセッション (Phase 5-3) で以下を完成させました:

- ✅ Canary traffic split 制御ロジック (環境変数ベース)
- ✅ Docker Compose Staging 環境 (PostgreSQL + Redis)
- ✅ Staging E2E 統合テスト (7個、モック)

本セッションの拡張内容:

- 🔄 **HTTP E2E テストスイート** (`tests/e2e_http_staging.rs`)
  - 16 個の HTTP ベースのテスト
  - `reqwest` クライアント使用
  - 実際の Staging 環境サービス連携

## 🎯 HTTP E2E テストの目的

| テスト種類 | 検証項目 | 重要性 |
|----------|--------|------|
| **GET エンドポイント** | ステータスコード、レスポンス形式 | 🔴 高 |
| **POST エンドポイント** | 入力検証、レスポンス型、作成確認 | 🔴 高 |
| **エラー処理** | 4xx/5xx ステータス、エラーメッセージ | 🟡 中 |
| **ヘッダー** | Content-Type、Deprecation (v1) | 🟡 中 |
| **パフォーマンス** | レスポンスタイム測定 | 🟡 中 |
| **並行処理** | 複数同時リクエスト | 🟡 中 |
| **Canary ルーティング** | トラフィック分割検証 | 🔴 高 |

## 📦 実装内容

### ファイル: `tests/e2e_http_staging.rs` (450+ 行)

#### セクション 1: GET エンドポイント (3 テスト)

```rust
#[tokio::test]
async fn test_http_get_health_endpoint()       // ✅ サーバー稼働確認
async fn test_http_get_tags_empty_list()       // ✅ 空リスト取得
async fn test_http_get_user_not_found()        // ✅ 404 エラー検証
```

**検証項目**:

- ステータスコード (200 OK, 404 Not Found)
- JSON レスポンス形式
- 空配列/エラーレスポンス構造

#### セクション 2: POST エンドポイント (3 テスト)

```rust
#[tokio::test]
async fn test_http_post_user_registration()    // ✅ ユーザー作成
async fn test_http_post_user_invalid_email()   // ✅ 入力検証 (400)
async fn test_http_post_create_post()          // ✅ 投稿作成
```

**検証項目**:

- 201 CREATED ステータス
- リソース ID の返却
- バリデーションエラー (400)

#### セクション 3: ヘッダー (2 テスト)

```rust
#[tokio::test]
async fn test_http_response_content_type()     // ✅ Content-Type
async fn test_http_deprecation_headers()       // ✅ API v1 非推奨ヘッダー
```

**検証項目**:

- `Content-Type: application/json`
- `Deprecation` ヘッダー (API v1)

#### セクション 4: エラーハンドリング (2 テスト)

```rust
#[tokio::test]
async fn test_http_method_not_allowed()        // ✅ 405 エラー
async fn test_http_request_timeout()           // ✅ タイムアウト処理
```

**検証項目**:

- 不正な HTTP メソッド処理
- リクエストタイムアウト

#### セクション 5: パフォーマンス (2 テスト)

```rust
#[tokio::test]
async fn test_http_concurrent_requests()       // ✅ 並行処理
async fn test_http_response_time_measurement() // ✅ レスポンスタイム
```

**検証項目**:

- 5 個の同時リクエスト成功
- エンドポイントごとのレスポンスタイム

#### セクション 6: Canary リリース (2 テスト)

```rust
#[tokio::test]
async fn test_http_canary_v2_routing()         // ✅ API v2 アクセス
async fn test_http_api_v1_backward_compat()    // ✅ API v1 互換性
```

**検証項目**:

- `/api/v2` エンドポイント動作
- `/api/v1` 後方互換性

#### セクション 7: ワークフロー (2 テスト)

```rust
#[tokio::test]
async fn test_http_workflow_user_and_tag_creation()  // ✅ 複合ワークフロー
async fn test_http_response_schema_validation()      // ✅ JSON スキーマ
```

**検証項目**:

- 複数エンドポイント連携
- JSON 形式の一貫性

## 🚀 実行方法

### 前提条件

1. **Staging 環境の起動**

   ```bash
   docker-compose -f docker-compose.staging.yml up -d
   ```

2. **環境変数設定**

   ```bash
   export DATABASE_URL="postgres://postgres:password@localhost:5432/cms_staging"
   export REDIS_URL="redis://localhost:6379"
   export API_V2_TRAFFIC_PERCENTAGE=100  # 100% v2 traffic
   ```

3. **マイグレーション実行**

   ```bash
   cargo run --bin cms-migrate -- migrate --no-seed
   ```

4. **アプリケーション起動**

   ```bash
   cargo run --bin cms-server --features "database,restructure_presentation"
   ```

### テスト実行コマンド

#### 全テスト実行

```bash
cargo test --test e2e_http_staging \
  --no-default-features --features "database,restructure_presentation" \
  -- --ignored --nocapture
```

#### 特定テスト実行

```bash
cargo test --test e2e_http_staging test_http_get_health_endpoint \
  --no-default-features --features "database,restructure_presentation" \
  -- --ignored --nocapture
```

#### タイムアウト設定変更

```bash
RUST_TEST_TIME_UNIT=10s cargo test --test e2e_http_staging \
  --no-default-features --features "database,restructure_presentation" \
  -- --ignored --nocapture
```

## 📊 テスト統計

| 指標 | 数値 |
|-----|------|
| **テスト数** | 16個 |
| **GET テスト** | 3個 |
| **POST テスト** | 3個 |
| **ヘッダーテスト** | 2個 |
| **エラー処理テスト** | 2個 |
| **パフォーマンステスト** | 2個 |
| **Canary テスト** | 2個 |
| **ワークフローテスト** | 2個 |
| **実装行数** | 450+行 |
| **feature gate** | `database`, `restructure_presentation` |

## ⚙️ 設定値

### HTTP クライアント設定

```rust
const BASE_URL: &str = "http://localhost:3000";
const STAGING_TIMEOUT: Duration = Duration::from_secs(30);
```

### サーバー起動待機

```rust
pub async fn wait_for_server(&self, max_retries: u32) {
    // 最大 max_retries × 500ms = max_retries * 500ms 待機
    // デフォルト: 10 retries × 500ms = 5 秒
}
```

## 🔧 トラブルシューティング

### エラー: "Connection refused"

**原因**: Staging 環境が起動していない

**対応**:

```bash
docker-compose -f docker-compose.staging.yml up -d
docker ps  # 確認
```

### エラー: "Failed to parse JSON response"

**原因**: レスポンス形式が期待と異なる

**対応**: `--nocapture` フラグでレスポンス内容を確認

```bash
cargo test test_http_get_tags_empty_list -- --ignored --nocapture
```

### テスト失敗: "timeout"

**原因**: サーバーレスポンスが遅い

**対応**: タイムアウト値を増やす

```bash
RUST_TEST_TIME_STEP=100s cargo test ...
```

### テスト失敗: "Server not available"

**原因**: アプリケーションが起動していない

**対応**:

```bash
cargo run --bin cms-server --features "database,restructure_presentation"
```

## 📝 テスト設計パターン

### パターン 1: サーバー準備確認

```rust
let setup = HttpTestSetup::new();
setup.wait_for_server(10).await;  // サーバー起動待ち
```

### パターン 2: ステータスコード検証

```rust
assert_eq!(response.status(), StatusCode::OK);
```

### パターン 3: JSON レスポンス検証

```rust
let body: Value = response.json().await?;
assert!(body.is_array());  // または is_object()
```

### パターン 4: エラーレスポンス検証

```rust
let body: Value = response.json().await?;
assert!(body.get("error").is_some());
```

### パターン 5: 複数エンドポイント検証

```rust
let endpoints = vec!["/api/v2/tags", "/api/v2/categories"];
for endpoint in endpoints {
    let response = setup.client.get(&format!("{}{}", base_url, endpoint)).send().await;
    // 検証
}
```

## 🔄 次ステップ

### Phase 5-3 残タスク

1. **Performance Benchmark** (priority: 高)
   - `criterion` を使った性能測定
   - v1 vs v2 比較分析
   - 目標: v2 > 66% 改善

2. **CI/CD パイプライン統合** (priority: 高)
   - GitHub Actions へ HTTP E2E テスト追加
   - 自動カバレッジレポート
   - Codacy セキュリティ分析

3. **Canary タイムライン検証** (priority: 中)
   - 段階的なトラフィック分割テスト
   - ロールバック手順検証

### Phase 5-4 予定

- API v1 非推奨マーク追加
- エンドポイント削除タイムライン設定
- クライアント移行ガイド作成

## 📖 参考リンク

- [reqwest ドキュメント](https://docs.rs/reqwest/latest/reqwest/)
- [tokio::test マクロ](https://tokio.rs/tokio/tutorial/testing)
- [HTTP ステータスコード](https://en.wikipedia.org/wiki/List_of_HTTP_status_codes)
- [RFC 7231 HTTP Semantics](https://tools.ietf.org/html/rfc7231)

## ✅ チェックリスト

テスト実装時の確認項目:

- [ ] 全 16 テストがコンパイルできる
- [ ] Staging 環境で少なくとも 1 テスト成功
- [ ] タイムアウト設定が適切 (30秒)
- [ ] `#[ignore]` フラグで手動実行に設定
- [ ] 代表的な成功/失敗ケースを網羅
- [ ] HTTP メソッドの種類を検証 (GET, POST, PATCH など)
- [ ] ステータスコード範囲を検証 (2xx, 4xx, 5xx)
- [ ] エラーメッセージ形式を確認
- [ ] JSON スキーマが一貫性
- [ ] Codacy 分析で品質問題なし

---

**作成日**: 2025年1月17日
**バージョン**: 1.0
**ステータス**: Phase 5-3 実装中
**次回レビュー**: Performance Benchmark 実装後
