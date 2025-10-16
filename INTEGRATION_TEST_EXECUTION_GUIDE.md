# RustCMS 統合テスト実行ガイド

**目的**: Phase 5-4 (Deprecation) と Phase 5-5 (v1 削除) の実装時に、すべてのテストを一元管理し実行する  
**対象者**: 開発チーム・QA チーム・CI/CD 管理者  
**作成日**: 2025-01-17

---

## 🎯 テスト体系概要

```
Domain Layer Tests (ユニットテスト)
  ├── Value Objects (100% coverage required)
  ├── Entities (100% coverage required)
  └── Domain Services (95%+ coverage)

Application Layer Tests (ユニット + 統合)
  ├── Use Cases (95%+ coverage)
  ├── DTOs (90%+ coverage)
  └── Application Services (90%+ coverage)

Infrastructure Layer Tests (統合テスト)
  ├── Database (Diesel + PostgreSQL)
  ├── Cache (Redis)
  └── Search (Tantivy)

Presentation Layer Tests (E2E)
  ├── HTTP Handlers (v1 + v2)
  ├── Middleware (Deprecation, Auth, CORS)
  └── Error Handling (各種 HTTP status)

Performance Tests (ベンチマーク)
  ├── Endpoint latency (P50, P99)
  ├── Throughput (RPS)
  └── Load testing (1000 req/s)
```

---

## 🚀 クイックスタート

### ローカル開発環境での実行

```bash
# 全テスト実行（ユニット + 統合）
cargo test --workspace --no-fail-fast

# Domain layer のみ実行
cargo test --lib domain

# 特定の機能のみテスト
cargo test --lib users
cargo test --lib posts
cargo test --lib deprecation

# テストを停止しないで全て実行
cargo test --workspace --no-fail-fast -- --nocapture
```

### 必須環境設定

```bash
# Docker コンテナで DB/Redis 起動
docker-compose up -d postgres redis

# 環境変数設定
export DATABASE_URL=postgres://postgres:password@localhost:5432/cms_test
export REDIS_URL=redis://localhost:6379

# マイグレーション実行
cargo run --bin cms-migrate -- migrate --no-seed
```

---

## 📋 テスト実行リファレンス

### A. Domain Layer Tests (Domain 駆動設計)

#### 実行コマンド

```bash
cargo test --lib domain --all-features

# または個別に
cargo test --lib domain::value_objects
cargo test --lib domain::entities
cargo test --lib domain::services
```

#### 対象ファイル

| テストモジュール | ファイルパス | テスト数 | カバレッジ目標 |
|-------------|----------|---------|-------------|
| Value Objects | `src/domain/value_objects/**_test.rs` | 50+ | 100% |
| Entities | `src/domain/entities/**_test.rs` | 30+ | 100% |
| Domain Services | `src/domain/services/**_test.rs` | 20+ | 95% |
| Domain Events | `src/domain/events/**_test.rs` | 10+ | 90% |

#### テスト詳細

```bash
# Value Objects: UserId, Email, Username, Title等の検証
cargo test --lib value_objects::email
# 出力例:
# test value_objects::email::test_valid_email ... ok
# test value_objects::email::test_invalid_email ... ok
# test value_objects::email::test_email_normalization ... ok

# Entities: User, Post, Comment, Tag等のビジネスルール
cargo test --lib entities::user
# 出力例:
# test entities::user::test_create_user ... ok
# test entities::user::test_user_activation ... ok
# test entities::user::test_user_deactivation ... ok

# Domain Events: イベント発行と消費
cargo test --lib events::user_registered
# 出力例:
# test events::test_user_registered_event ... ok
# test events::test_event_aggregation ... ok
```

#### 実行時間

- **単体実行**: 5-10 秒
- **全テスト**: 20-30 秒

---

### B. Application Layer Tests (Use Cases & DTOs)

#### 実行コマンド

```bash
cargo test --lib application --all-features

# または個別に
cargo test --lib application::use_cases
cargo test --lib application::dto
```

#### 対象ファイル

| テストモジュール | ファイルパス | テスト数 | カバレッジ目標 |
|-------------|----------|---------|-------------|
| Use Cases | `src/application/use_cases/**_test.rs` | 40+ | 95% |
| DTOs | `src/application/dto/**_test.rs` | 20+ | 90% |
| Services | `src/application/services/**_test.rs` | 15+ | 90% |

#### テスト詳細

**Use Case テスト例**:

```bash
cargo test --lib use_cases::register_user

# テスト項目
# - 正常系: 有効なメール・パスワードで登録成功
# - エラー系: 重複メール時のエラー
# - エラー系: 無効なパスワードでのエラー
# - イベント検証: UserRegistered イベント発行確認
```

**DTO テスト例**:

```bash
cargo test --lib dto::user_response

# テスト項目
# - Serialization: Rust 型 → JSON
# - Deserialization: JSON → Rust 型
# - Validation: 不正な型データの拒否
# - Backward compatibility: v1 形式との互換性
```

#### 実行時間

- **単体実行**: 10-15 秒
- **全テスト**: 30-45 秒

---

### C. Infrastructure Layer Tests (統合テスト)

#### 前提条件

```bash
# Docker で DB/Cache 起動
docker-compose up -d postgres redis

# 環境変数設定
export DATABASE_URL=postgres://postgres:password@localhost:5432/cms_test
export REDIS_URL=redis://localhost:6379
```

#### 実行コマンド

```bash
# 全統合テスト実行
cargo test --test '*' -- --test-threads=1

# または個別に
cargo test --test database_integration_test -- --test-threads=1
cargo test --test cache_integration_test -- --test-threads=1
cargo test --test search_integration_test -- --test-threads=1
```

#### テスト対象

| テスト | コマンド | 検証項目 |
|-------|---------|---------|
| **Database** | `cargo test --test database_*` | トランザクション、マイグレーション、連結 |
| **Cache** | `cargo test --test cache_*` | キャッシュ書き込み、無効化、TTL |
| **Search** | `cargo test --test search_*` | インデックス作成、クエリ実行、結果ランキング |
| **Auth** | `cargo test --test auth_*` | Biscuit token 検証、WebAuthn |

#### テスト詳細

```bash
# Database トランザクション: ロールバック確認
cargo test --test database_integration_test test_transaction_rollback -- --nocapture
# 出力:
# - User 作成
# - ロールバック実行
# - User が存在しないことを確認

# Cache TTL 検証: キャッシュ有効期限
cargo test --test cache_integration_test test_cache_ttl -- --nocapture
# 出力:
# - キャッシュ書き込み
# - TTL 秒後に確認
# - キャッシュが削除されていることを確認

# Search インデックス: 全文検索動作
cargo test --test search_integration_test test_full_text_search -- --nocapture
# 出力:
# - 複数 Post インデックス化
# - キーワード検索実行
# - 関連性ランキング確認
```

#### 実行時間

- **単体実行**: 15-30 秒
- **全テスト**: 2-5 分

---

### D. Presentation Layer Tests (E2E)

#### 前提条件

```bash
# ローカルサーバー起動
cargo run --bin cms-server --all-features &

# またはステージング環境
export API_BASE_URL=https://staging.example.com
```

#### 実行コマンド

```bash
# 全 HTTP E2E テスト
cargo test --test e2e_test -- --test-threads=1 --nocapture

# または個別エンドポイント
cargo test --test e2e_test test_users_get -- --nocapture
cargo test --test e2e_test test_users_create -- --nocapture
cargo test --test e2e_test test_deprecation_headers -- --nocapture
```

#### テスト対象エンドポイント (50+)

**v1 エンドポイント (Deprecation ヘッダー確認)**:

```
Users (8)
- GET /api/v1/users
- POST /api/v1/users
- GET /api/v1/users/{id}
- PUT /api/v1/users/{id}
- DELETE /api/v1/users/{id}
- POST /api/v1/users/{id}/email_change
- POST /api/v1/users/{id}/password_change
- GET /api/v1/users/search

Posts (10)
- GET /api/v1/posts
- POST /api/v1/posts
- ... (その他の CRUD + publish/draft)

... (Comments, Tags, Categories, etc.)
```

**v2 エンドポイント (正常動作確認)**:

```
Users (8)
- GET /api/v2/users → 新ページネーション (offset/limit)
- POST /api/v2/users → 新エラー形式
- ... (同等のテスト)

... (その他のリソース)
```

#### テスト詳細

```bash
# Deprecation ヘッダー確認テスト
cargo test --test e2e_test test_v1_deprecation_headers -- --nocapture

# テスト内容:
# 1. GET /api/v1/users でリクエスト
# 2. レスポンスヘッダー確認:
#    - Deprecation: true
#    - Sunset: Sun, 17 Mar 2025 00:00:00 GMT
#    - Link: </api/v2/users>; rel="successor-version"
#    - Warning: 299 - "Deprecation: ..."
# 3. v2 エンドポイントでは Deprecation ヘッダーなし確認

# エラー形式新旧比較テスト
cargo test --test e2e_test test_error_format_v1_vs_v2 -- --nocapture

# テスト内容:
# 1. v1 エラー: { error: "..." }
# 2. v2 エラー: { errors: [...], request_id: "...", ... }
# 3. クライアントが両形式に対応できることを確認
```

#### 実行時間

- **単体実行**: 5-10 秒
- **全テスト (50+ エンドポイント)**: 10-15 分

---

### E. Performance Tests (ベンチマーク)

#### 実行コマンド

```bash
# Criterion ベンチマーク実行
cargo bench --bench baseline

# または特定のベンチマーク
cargo bench --bench baseline -- --test-threads=1
```

#### ベンチマーク対象

| エンドポイント | P50 (ms) 目標 | P99 (ms) 目標 | RPS 目標 |
|-------------|------------|------------|---------|
| GET /users | 10-15 | 30-40 | 5000+ |
| POST /users | 20-30 | 50-70 | 2000+ |
| GET /posts | 15-20 | 40-50 | 4000+ |
| POST /posts | 25-35 | 60-80 | 1500+ |

#### テスト詳細

```bash
# レスポンスタイムベンチマーク
cargo bench --bench phase5_3_performance

# 出力例:
# GET /api/v2/users            time:   [12.5 ms 13.2 ms 14.1 ms]
# POST /api/v2/users           time:   [25.3 ms 26.8 ms 28.5 ms]
# GET /api/v2/posts/{id}       time:   [10.2 ms 11.0 ms 12.1 ms]
```

#### 実行時間

- **ローカル PC**: 2-5 分
- **CI/CD パイプライン**: 10-15 分

---

## 🔄 CI/CD パイプライン統合

### GitHub Actions ワークフロー

```yaml
name: RustCMS Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: password
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build
        run: cargo build --all-features
      
      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
      
      - name: Format Check
        run: cargo fmt -- --check
      
      - name: Domain Layer Tests
        run: cargo test --lib domain --all-features
      
      - name: Application Layer Tests
        run: cargo test --lib application --all-features
      
      - name: Infrastructure Tests
        run: cargo test --test '*' -- --test-threads=1
        env:
          DATABASE_URL: postgres://postgres:password@localhost:5432/postgres
          REDIS_URL: redis://localhost:6379
      
      - name: E2E Tests
        run: cargo test --test e2e_test -- --test-threads=1 --nocapture
      
      - name: Benchmarks
        run: cargo bench --bench baseline
      
      - name: Coverage
        run: cargo tarpaulin --fail-under 90 --workspace
```

### ステージング環境での実行

```bash
# ステージングへデプロイ
docker build -t cms:staging -f Dockerfile .
docker push registry.example.com/cms:staging

# ステージング E2E テスト実行
export API_BASE_URL=https://staging.example.com
cargo test --test e2e_staging_test -- --test-threads=1 --nocapture
```

---

## ✅ テスト実行チェックリスト

### PR マージ前

- [ ] Domain layer テスト: ✅ 全パス
- [ ] Application layer テスト: ✅ 全パス
- [ ] Infrastructure テスト: ✅ 全パス
- [ ] Clippy 警告: ✅ 0 個
- [ ] フォーマット: ✅ OK
- [ ] カバレッジ: ✅ ≥ 90%

### 本番デプロイ前

- [ ] ステージング E2E: ✅ 全パス
- [ ] ベンチマーク: ✅ 目標値内
- [ ] セキュリティスキャン: ✅ 脆弱性なし
- [ ] ロードテスト: ✅ エラー < 0.1%

---

## 🚨 トラブルシューティング

### DB テスト失敗時

```bash
# DB を再起動
docker-compose restart postgres

# データベース初期化
cargo run --bin cms-migrate -- migrate --no-seed

# テスト再実行
cargo test --test database_integration_test -- --test-threads=1
```

### キャッシュテスト失敗時

```bash
# Redis を再起動
docker-compose restart redis

# Redis をフラッシュ
redis-cli FLUSHALL

# テスト再実行
cargo test --test cache_integration_test -- --test-threads=1
```

### タイムアウトエラー

```bash
# テストタイムアウトを延長
cargo test -- --test-threads=1 --timeout 300

# または個別テストを実行
cargo test test_specific_case -- --nocapture
```

---

## 📊 テスト実行結果の解釈

### 成功例

```
test result: ok. 285 passed; 0 failed; 0 ignored; 50 measured

Coverage: 92.3% (≥ 90% クリア) ✅
Performance: avg 12.5ms (< 50ms クリア) ✅
```

### 失敗例 and 対応

| エラー | 原因 | 対応 |
|-------|------|------|
| `thread 'main' panicked at 'database connection failed'` | DB 接続エラー | `docker-compose ps` で確認、再起動 |
| `test timed out after 60s` | テストが遅すぎる | タイムアウト拡張、またはボトルネック調査 |
| `assertion failed: expected 200, got 500` | サーバーエラー | サーバーログ確認、デバッグ |

---

## 📚 参考資料

- `QUALITY_ASSURANCE_CHECKLIST.md` — 品質チェックリスト
- `TESTING_STRATEGY.md` — テスト戦略詳細
- `PHASE_5_4_IMPLEMENTATION_GUIDE.md` — Phase 5-4 実装ガイド
- `.github/workflows/ci.yml` — CI/CD ワークフロー

---

**最終更新**: 2025-01-17  
**次回更新**: 2025-02-07 (Phase 5-5 開始)  
**所有者**: QA Team
