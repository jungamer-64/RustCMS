# Phase 5-3: Staging デプロイ実装ガイド

**ステータス**: 実装フェーズ開始準備  
**前提**: Phase 5-1, 5-2 完了 (268/268 tests passing)

---

## 戦略

Phase 5-3 では、**testcontainers の依存関係競合を回避** し、代わりに以下の 3 つのアプローチを採用します：

### アプローチ A: Docker Compose による E2E テスト環境（推奨）

```bash
# docker-compose.staging.yml で PostgreSQL + Redis を起動
docker-compose -f docker-compose.staging.yml up -d

# E2E テスト実行
DATABASE_URL=postgres://user:pass@localhost:5432/cms_test cargo test --test e2e_staging
```

**メリット**:
- 既存の `docker-compose.yml` を活用
- 依存関係競合なし
- CI/CD との統合が容易
- 実環境を忠実に再現

### アプローチ B: シミュレーション E2E テスト（早期検証）

既存の `e2e_api_v2_complete.rs` を拡張し、メモリベースのテストで E2E フローを検証：

```rust
#[test]
fn test_user_registration_full_flow_simulated() {
    // メモリベースで実装を検証
    // Database未接続で実行可能
}
```

**メリット**:
- 即座に実行可能
- CI でコスト削減
- ロジック検証に最適

### アプローチ C: Canary Release 環境変数制御（本番検証）

トラフィック分割ロジックを実装し、段階的なロールアウトを実現：

```bash
export API_V2_TRAFFIC_PERCENTAGE=10
export USE_LEGACY_API_V1=true
cargo run --release
```

**メリット**:
- 実環境での段階的検証
- ロールバック可能
- リスク最小化

---

## 実装タスク

### Task 1: Docker Compose Staging 設定 (1時間)

**ファイル作成**: `docker-compose.staging.yml`

```yaml
version: '3.9'

services:
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: cms_user
      POSTGRES_PASSWORD: cms_password
      POSTGRES_DB: cms_staging
    ports:
      - "5432:5432"
    volumes:
      - postgres_staging_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cms_user"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres_staging_data:
```

**起動スクリプト**: `scripts/start_staging_env.sh`

```bash
#!/bin/bash
set -e

# Docker Compose起動
docker-compose -f docker-compose.staging.yml up -d

# マイグレーション待機
sleep 5

# DB マイグレーション実行
export DATABASE_URL="postgresql://cms_user:cms_password@localhost:5432/cms_staging"
cargo run --bin cms-migrate -- migrate --no-seed

echo "✅ Staging environment ready"
```

**実行**:

```bash
chmod +x scripts/start_staging_env.sh
./scripts/start_staging_env.sh
```

---

### Task 2: Docker Compose 統合 E2E テスト (2-3時間)

**ファイル作成**: `tests/e2e_staging_integration.rs`

```rust
//! E2E tests for staging environment (requires Docker Compose)
//! Run: docker-compose -f docker-compose.staging.yml up -d
//! Then: DATABASE_URL=... cargo test --test e2e_staging_integration

#[cfg(all(feature = "database", feature = "restructure_presentation"))]
mod staging_tests {
    use serde_json::json;
    use uuid::Uuid;

    /// Helper to create HTTP client for staging server
    fn create_staging_client() -> reqwest::Client {
        reqwest::Client::new()
    }

    #[tokio::test]
    async fn test_staging_user_registration_flow() {
        let client = create_staging_client();
        let base_url = "http://localhost:8080"; // Assuming server runs here

        // Register user
        let register_response = client
            .post(format!("{}/api/v2/users", base_url))
            .json(&json!({
                "username": "staging_user",
                "email": "staging@test.local",
                "password": "test_password_123"
            }))
            .send()
            .await
            .expect("Failed to register user");

        assert_eq!(register_response.status(), 201);

        let user: serde_json::Value = register_response
            .json()
            .await
            .expect("Failed to parse response");

        let user_id = user["id"]
            .as_str()
            .expect("Missing user ID in response");

        // Verify user exists
        let get_response = client
            .get(format!("{}/api/v2/users/{}", base_url, user_id))
            .send()
            .await
            .expect("Failed to get user");

        assert_eq!(get_response.status(), 200);
        let retrieved_user: serde_json::Value = get_response
            .json()
            .await
            .expect("Failed to parse response");

        assert_eq!(
            retrieved_user["email"].as_str().unwrap(),
            "staging@test.local"
        );
    }

    #[tokio::test]
    async fn test_staging_post_creation_with_tags() {
        let client = create_staging_client();
        let base_url = "http://localhost:8080";

        // Create post
        let post_response = client
            .post(format!("{}/api/v2/posts", base_url))
            .json(&json!({
                "title": "Staging Test Post",
                "content": "Test content",
                "slug": format!("staging-post-{}", Uuid::new_v4()),
                "tags": ["staging", "test"]
            }))
            .send()
            .await
            .expect("Failed to create post");

        assert_eq!(post_response.status(), 201);

        let post: serde_json::Value = post_response
            .json()
            .await
            .expect("Failed to parse response");

        assert_eq!(post["title"].as_str().unwrap(), "Staging Test Post");
    }
}
```

**実行コマンド**:

```bash
# Staging環境起動
docker-compose -f docker-compose.staging.yml up -d

# テスト実行
DATABASE_URL=postgresql://cms_user:cms_password@localhost:5432/cms_staging \
  cargo test --test e2e_staging_integration \
  --features "database,restructure_presentation"

# クリーンアップ
docker-compose -f docker-compose.staging.yml down
```

---

### Task 3: Canary Release トラフィック制御 (1-2時間)

**ファイル作成・修正**: `src/routes/mod.rs`

```rust
pub mod canary {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// Get the current traffic split percentage for API v2 (0-100)
    pub fn get_api_v2_traffic_percentage() -> u32 {
        std::env::var("API_V2_TRAFFIC_PERCENTAGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    /// Determine if a request should be routed to API v2 based on Canary percentage
    pub fn should_route_to_api_v2(request_id: &str) -> bool {
        let percentage = get_api_v2_traffic_percentage();

        if percentage >= 100 {
            return true; // All traffic to v2
        }

        if percentage == 0 {
            return false; // No traffic to v2
        }

        // Hash-based distribution for consistent routing per user/session
        let mut hasher = DefaultHasher::new();
        request_id.hash(&mut hasher);
        let hash_value = hasher.finish();

        (hash_value % 100) < (percentage as u64)
    }
}
```

**ミドルウェア実装**: `src/middleware/mod.rs`

```rust
pub async fn canary_router<S>(
    req: Request<Body>,
    next: Next,
) -> Response {
    use crate::routes::canary::should_route_to_api_v2;

    // リクエスト ID から v2 への振り分け判定
    if let Some(request_id) = req.headers().get("x-request-id") {
        if let Ok(id_str) = request_id.to_str() {
            if should_route_to_api_v2(id_str) {
                // Route to API v2
                return next.run(req).await;
            }
        }
    }

    // Default: Route to API v1
    next.run(req).await
}
```

**Canary タイムライン**:

```bash
# Week 1: 10% traffic to v2
export API_V2_TRAFFIC_PERCENTAGE=10

# Week 2: 50% traffic to v2
export API_V2_TRAFFIC_PERCENTAGE=50

# Week 3: 90% traffic to v2
export API_V2_TRAFFIC_PERCENTAGE=90

# Week 4: 100% traffic to v2
export API_V2_TRAFFIC_PERCENTAGE=100
```

---

### Task 4: Performance Benchmark スクリプト (1-2時間)

**ファイル作成**: `benches/staging_perf.rs`

```rust
use std::time::Instant;

fn benchmark_api_endpoints() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let results = rt.block_on(async {
        let client = reqwest::Client::new();
        let base_url = "http://localhost:8080";

        // Benchmark user registration
        let start = Instant::now();
        for i in 0..100 {
            let _ = client
                .post(format!("{}/api/v2/users", base_url))
                .json(&serde_json::json!({
                    "username": format!("bench_user_{}", i),
                    "email": format!("bench{}@test.local", i),
                    "password": "bench_pass"
                }))
                .send()
                .await;
        }
        let v2_registration = start.elapsed();

        // Benchmark v1
        let start = Instant::now();
        for i in 0..100 {
            let _ = client
                .post(format!("{}/api/v1/users", base_url))
                .json(&serde_json::json!({
                    "username": format!("bench_user_v1_{}", i),
                    "email": format!("benchv1{}@test.local", i),
                    "password": "bench_pass"
                }))
                .send()
                .await;
        }
        let v1_registration = start.elapsed();

        (v2_registration, v1_registration)
    });

    println!("User Registration (100 requests):");
    println!("  API v2: {:?}", results.0);
    println!("  API v1: {:?}", results.1);
    println!(
        "  Improvement: {:.1}%",
        ((results.1.as_millis() - results.0.as_millis()) as f64
            / results.1.as_millis() as f64
            * 100.0)
    );
}

fn main() {
    benchmark_api_endpoints();
}
```

**実行**:

```bash
# Staging環境起動
docker-compose -f docker-compose.staging.yml up -d
cargo run --bin cms-server --release &

# Benchmark実行
cargo run --release --example staging_perf

# クリーンアップ
pkill cms-server
docker-compose down
```

---

### Task 5: CI/CD パイプライン拡張 (1-2時間)

**ファイル更新**: `.github/workflows/ci.yml`

```yaml
staging-integration-tests:
  name: Staging Integration Tests
  runs-on: ubuntu-latest
  needs: build

  services:
    postgres:
      image: postgres:15-alpine
      env:
        POSTGRES_USER: cms_user
        POSTGRES_PASSWORD: cms_password
        POSTGRES_DB: cms_test
      options: >-
        --health-cmd pg_isready
        --health-interval 10s
        --health-timeout 5s
        --health-retries 5
      ports:
        - 5432:5432

    redis:
      image: redis:7-alpine
      options: >-
        --health-cmd "redis-cli ping"
        --health-interval 10s
        --health-timeout 5s
        --health-retries 5
      ports:
        - 6379:6379

  steps:
    - uses: actions/checkout@v4

    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Run integration tests
      env:
        DATABASE_URL: postgresql://cms_user:cms_password@localhost:5432/cms_test
        REDIS_URL: redis://localhost:6379
      run: |
        cargo test --test e2e_staging_integration \
          --features "database,restructure_presentation" \
          --no-fail-fast

    - name: Run performance benchmarks
      run: |
        cargo run --release --example staging_perf
```

---

## 実装スケジュール

| 日 | タスク | 予定工数 | 状態 |
|---|---|---|---|
| Day 1 | Docker Compose 設定 | 1h | ⏳ |
| Day 1 | Staging E2E テスト | 2-3h | ⏳ |
| Day 2 | Canary トラフィック制御 | 1-2h | ⏳ |
| Day 2 | Performance Benchmark | 1-2h | ⏳ |
| Day 3 | CI/CD パイプライン | 1-2h | ⏳ |
| **TOTAL** | **全タスク完了** | **7-10h** | **1-2 days** |

---

## 検証チェックリスト

- [ ] Docker Compose で PostgreSQL + Redis 起動確認
- [ ] E2E 統合テスト実行 (5+ テスト passing)
- [ ] Canary traffic split ロジック検証
- [ ] Performance 目標値確認 (v2 ≥60% 改善)
- [ ] CI/CD パイプライン統合確認
- [ ] Rollback シーケンス検証
- [ ] モニタリング ログ確認

---

## Next Phase (Phase 5-4)

Phase 5-3 完了後、以下を実装:

- API v1 Deprecation ヘッダー
- レガシーコード削除計画
- 本番環境デプロイ準備

**目標**: Week 4 に API v2 100% traffic → v1 廃止予定

---

## リソース

- 📘 PHASE_5_PLAN.md - 大局
- 📘 PHASE_5_TEST_SUMMARY.md - テスト統計
- 📘 docker-compose.yml - 既存設定参照
- 🔗 [Docker Compose Reference](https://docs.docker.com/compose/)
