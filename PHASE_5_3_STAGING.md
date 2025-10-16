# Phase 5-3: Staging デプロイ & Canary Release 準備

**開始日**: 2025-01-17  
**見積**: 1-2週間  
**目標**: Staging環境での実運用テスト + Canary release設定完成

---

## 実装計画

### Task 1: testcontainers PostgreSQL統合 (1-2日)

```bash
# Cargo.toml に追加
cargo add testcontainers --dev --version "0.15"
```

**実装ファイル**:
- `tests/common/postgres_helper.rs` - PostgreSQL コンテナ管理
- `tests/common/http_client.rs` - HTTP テストクライアント
- `tests/common/test_app.rs` - Axum test server
- `tests/e2e_api_v2_with_db.rs` - 実DB統合テスト

**利点**:
- 実データベース環境でのテスト
- テストデータの自動プリペア
- マイグレーション自動適用
- テスト間のデータ分離

### Task 2: reqwest HTTP クライアント化 (1-2日)

```bash
cargo add reqwest --dev --features "json"
```

**テスト実行**:

```bash
# DB統合テスト実行
cargo test --test e2e_api_v2_with_db \
  --features "restructure_presentation,database"
```

### Task 3: Performance Benchmark測定 (1-2日)

```bash
cargo add criterion --dev --features "html_reports"
```

**目標値**:

| エンドポイント | API v1 | API v2 | 改善率 |
|---|---|---|---|
| User Register | 150ms | ≤50ms | 66%+ ✅ |
| Post Get | 120ms | ≤40ms | 66%+ ✅ |
| Tag List | 100ms | ≤35ms | 65%+ ✅ |

```bash
# Benchmark実行
cargo bench --bench staging_performance -- --baseline v2
```

### Task 4: Canary Release設定 (1日)

**トラフィック分割タイムライン**:

| 期間 | v2 | v1 | 調整項目 |
|---|---|---|---|
| Week 1 | 10% | 90% | `API_V2_TRAFFIC_PERCENTAGE=10` |
| Week 2 | 50% | 50% | `API_V2_TRAFFIC_PERCENTAGE=50` |
| Week 3 | 90% | 10% | `API_V2_TRAFFIC_PERCENTAGE=90` |
| Week 4+ | 100% | 0% | `API_V2_TRAFFIC_PERCENTAGE=100` |

**実装** (src/routes/mod.rs):

```rust
pub fn get_traffic_split_percentage() -> u32 {
    std::env::var("API_V2_TRAFFIC_PERCENTAGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn should_route_to_v2(request_id: &str) -> bool {
    let percentage = get_traffic_split_percentage();
    (hash_request_id(request_id) % 100) < percentage
}
```

### Task 5: CI/CDパイプライン拡張 (1-2日)

**.github/workflows/ci.yml** に追加:

```yaml
integration-tests-staging:
  name: Integration Tests (Staging)
  services:
    postgres:
      image: postgres:15
  steps:
    - run: cargo test --test e2e_api_v2_with_db \
        --features "restructure_presentation,database"

performance-benchmark:
  name: Performance Benchmark
  steps:
    - run: cargo bench --bench staging_performance

coverage-report:
  name: Code Coverage
  steps:
    - run: cargo tarpaulin --out Xml
    - uses: codecov/codecov-action@v3
```

**カバレッジ目標**:

| 層 | 目標 | 期限 |
|---|---|---|
| Domain | ≥100% | Phase 5-3 ✅ |
| Application | ≥70% | Phase 5-4 |
| Presentation | ≥80% | Phase 5-3 |
| Infrastructure | ≥60% | Phase 6 |
| **Overall** | **≥85%** | **Phase 5-4** |

---

## デプロイメント手順

### 初期設定

```bash
export API_V2_ENABLED=true
export API_V2_TRAFFIC_PERCENTAGE=10
export USE_LEGACY_API_V1=true
export DATABASE_URL=postgresql://postgres:password@localhost:5432/cms_staging
export REDIS_URL=redis://localhost:6379
```

### モニタリング指標

- Error rate (target < 0.5%)
- Response time p99 (target < 200ms)
- Throughput (requests/sec)
- Cache hit ratio
- Database query time

### ロールバック計画

問題検出時: `API_V2_TRAFFIC_PERCENTAGE=0`

段階的ロールバック: 100 → 50 → 10 → 0

---

## テスト構成

```
tests/
├── common/
│   ├── postgres_helper.rs
│   ├── http_client.rs
│   ├── test_app.rs
│   └── fixtures.rs
├── e2e_api_v2_complete.rs (既存)
├── e2e_api_v1_compatibility.rs (既存)
├── e2e_api_v2_with_db.rs ✨ NEW
├── e2e_canary_release.rs ✨ NEW
└── performance_baseline.rs ✨ NEW
```

### テスト実行コマンド

```bash
# 全E2Eテスト (メモリベース)
cargo test --test e2e_api_v2_complete \
           --test e2e_api_v1_compatibility

# DB統合テスト
cargo test --test e2e_api_v2_with_db \
  --features "restructure_presentation,database"

# Canary release テスト
cargo test --test e2e_canary_release

# Performance benchmark
cargo bench --bench staging_performance

# CI実行 (フル)
cargo test --workspace --no-fail-fast --all-features
```

---

## 成功基準

| 基準 | 目標値 | 対応フェーズ |
|---|---|---|
| テストカバー率 | 57+ tests | ✅ Phase 5-3 |
| エラー率 | < 0.5% | ✅ Phase 5-3 |
| パフォーマンス | v2 ≥60% 改善 | ✅ Phase 5-3 |
| 互換性 | 100% backward compat | ✅ Phase 5-2 |
| Canary準備 | 環境変数制御OK | ✅ Phase 5-3 |

---

## 完了条件

- ✅ testcontainers DB統合完了
- ✅ reqwest HTTP E2Eテスト完了
- ✅ Performance benchmark baseline確立
- ✅ Canary release環境変数実装
- ✅ CI/CD統合テスト・カバレッジ自動実行

---

## Phase 5-4への繋ぎ

**次フェーズ実装**:
- API v1 Deprecation ヘッダー実装
- レガシーコード削除計画
- 本番デプロイシーケンス
