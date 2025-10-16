# Phase 5-4/5-5 品質保証チェックリスト

**目的**: Phase 5-4 (Deprecation) と Phase 5-5 (削除) の実装時に、品質・セキュリティ・パフォーマンスの基準を維持する
**作成日**: 2025-01-17
**対象**: 開発チーム・QA チーム・リリースマネージャー

---

## 🎯 チェックリスト体系

### A. ビルド＆コンパイル品質

#### A-1. Rust コンパイル標準

```bash
# 実行コマンド
cargo build --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

**チェック項目**:

- [ ] `cargo build --all-features` で **0 エラー、0 警告**
- [ ] `cargo clippy` で **0 警告** (Clippy strict mode)
- [ ] `cargo fmt --check` で **0 フォーマット違反**
- [ ] **ビルド時間**: < 3分 (CI/CD 効率)

**失敗時の対応**:

| 症状 | 原因 | 対応 |
|------|------|------|
| ビルド失敗 | コンパイルエラー | error メッセージから修正、PR 前に verify |
| Clippy 警告 | コードスタイル問題 | `cargo fix --allow-dirty` でオート修正試行 |
| フォーマット違反 | スペース・インデント | `cargo fmt` で自動修正 |

---

#### A-2. 依存関係チェック

```bash
# セキュリティスキャン
cargo audit

# CVE チェック（appmod-validate-cve で実装予定）
cargo tree --depth 3
```

**チェック項目**:

- [ ] `cargo audit` で **0 vulnerability**
- [ ] 新規追加依存関係の版が **最新安定版**
- [ ] 依存関係削除時に `Cargo.lock` を確認

---

### B. テスト品質

#### B-1. ユニットテスト (Domain + Application Layers)

```bash
# 実行
cargo test --lib --workspace --no-fail-fast

# カバレッジ測定
cargo tarpaulin --lib --workspace --fail-under 90
```

**チェック項目**:

- [ ] ユニットテスト **成功率 100%**
- [ ] テストカバレッジ **≥ 90%**
- [ ] 実行時間 **< 30秒**

**対象テスト**:

| レイヤー | 対象ファイル | テスト数 | カバレッジ目標 |
|---------|------------|---------|-------------|
| Domain | `src/domain/**_test.rs` | 100+ | 100% |
| Application | `src/application/**_test.rs` | 50+ | 95% |
| Value Objects | `src/domain/value_objects/**` | 50+ | 100% |
| Entities | `src/domain/entities/**` | 30+ | 100% |

---

#### B-2. 統合テスト (Infrastructure Layer)

```bash
# DB/キャッシュテスト
cargo test --test '*' --workspace -- --test-threads=1

# タイムアウト設定: 5分以内
```

**チェック項目**:

- [ ] 統合テスト **成功率 100%**
- [ ] **DB トランザクション** ロールバック確認
- [ ] **Redis キャッシュ** 正常動作確認
- [ ] 実行時間 **< 5分**

---

#### B-3. E2E テスト (HTTP Layer)

```bash
# HTTP エンドポイントテスト
cargo test --test e2e_test -- --nocapture --test-threads=1

# ステージング環境へのテスト
./scripts/run_staging_e2e_tests.sh
```

**チェック項目**:

- [ ] **50+ v1 エンドポイント** すべてテスト成功
- [ ] **50+ v2 エンドポイント** すべてテスト成功
- [ ] Deprecation ヘッダー **100% 付与** (v1 のみ)
- [ ] エラーレスポンス形式 **v2 準拠**
- [ ] 実行時間 **< 10分**

**テスト対象エンドポイント**:

```
Users: 8
  - GET /api/v{1,2}/users
  - POST /api/v{1,2}/users
  - GET /api/v{1,2}/users/{id}
  - PUT /api/v{1,2}/users/{id}
  - DELETE /api/v{1,2}/users/{id}
  - POST /api/v{1,2}/users/{id}/email_change
  - POST /api/v{1,2}/users/{id}/password_change
  - GET /api/v{1,2}/users/search

Posts: 10
... (計 50 エンドポイント)
```

---

### C. セキュリティ品質

#### C-1. 脆弱性スキャン

```bash
# 依存関係脆弱性チェック
cargo audit

# Trivy によるセキュリティスキャン
trivy fs . --severity HIGH,CRITICAL --format json > trivy-report.json
```

**チェック項目**:

- [ ] `cargo audit` で **0 vulnerability**
- [ ] Trivy スキャン で **Critical: 0, High: 0**
- [ ] 新規依存関係に脆弱性なし

---

#### C-2. 認証・認可テスト

```bash
# 認可テスト
cargo test --test auth_integration_test -- --nocapture

# CORS・CSRF 対策確認
curl -H "Origin: http://attacker.com" http://localhost:3000/api/v2/users
```

**チェック項目**:

- [ ] 未認証リクエスト → `401 Unauthorized`
- [ ] 無効トークン → `401 Unauthorized`
- [ ] 権限不足 → `403 Forbidden`
- [ ] CORS ヘッダー正しく設定
- [ ] CSRF トークン必須エンドポイント検証

---

### D. パフォーマンス品質

#### D-1. ベンチマークテスト

```bash
# Criterion ベンチマーク実行
cargo bench --bench baseline

# 前回比較: -5% 以上劣化していないか確認
```

**チェック項目**:

- [ ] エンドポイント平均レスポンスタイム **< 50ms**
- [ ] P99 レスポンスタイム **< 100ms**
- [ ] RPS (Requests Per Second) **> 1000**
- [ ] 前回比パフォーマンス劣化 **< 5%**

**ベンチマーク対象**:

| エンドポイント | P50 (ms) | P99 (ms) | RPS |
|-------------|----------|----------|-----|
| GET /users | 10-15 | 30-40 | 5000+ |
| POST /users | 20-30 | 50-70 | 2000+ |
| GET /posts | 15-20 | 40-50 | 4000+ |

---

#### D-2. ロードテスト

```bash
# Apache Bench による負荷テスト
ab -n 10000 -c 100 http://localhost:3000/api/v2/users

# wrk による複雑な負荷テスト
wrk -t12 -c400 -d30s http://localhost:3000/api/v2/users
```

**チェック項目**:

- [ ] **1000 req/s × 100 concurrent** でエラーなし
- [ ] エラー率 **< 0.1%**
- [ ] p95 レスポンスタイム **< 100ms**
- [ ] メモリリーク検出なし (Valgrind / ASAN)

---

### E. ドキュメント・コード品質

#### E-1. ドキュメント品質

```bash
# Markdown Lint チェック
markdownlint-cli2 '**/*.md'

# Markdown lint issues: 0
```

**チェック項目**:

- [ ] `PHASE_5_4_*.md` ファイル **Markdown Lint 警告: 0**
- [ ] コード例 **すべて実行可能**
- [ ] リンク **すべて有効**
- [ ] 日本語・英語 **自然な表現**

---

#### E-2. コードドキュメント

**チェック項目**:

- [ ] 新規 public API に `///` doc comments
- [ ] 複雑な関数に **Example セクション**
- [ ] `#[deprecated]` 属性で非推奨 API 標識
- [ ] `SAFETY` コメント で unsafe ブロック説明

---

### F. ステージング環境検証

#### F-1. ステージング環境デプロイ

```bash
# ステージング環境へのデプロイ
docker build -t cms:staging -f Dockerfile .
docker push registry.example.com/cms:staging

# ヘルスチェック
curl http://staging.example.com/api/v2/health
```

**チェック項目**:

- [ ] ステージング環境 **デプロイ成功**
- [ ] ヘルスチェック `status: "healthy"`
- [ ] DB マイグレーション **成功**
- [ ] サービス **起動確認** (3分以内)

---

#### F-2. ステージング環境 E2E テスト

**チェック項目**:

- [ ] v1 エンドポイント **すべて動作**
- [ ] v1 → v2 Deprecation ヘッダー確認
- [ ] v2 エンドポイント **すべて動作**
- [ ] エラー形式 **v2 準拠**
- [ ] パフォーマンス **目標値内** (< 50ms)

---

### G. リリース前最終確認

#### G-1. Git コミット＆PR 確認

**チェック項目**:

- [ ] すべての変更を `git commit` で確認
- [ ] コミットメッセージ **conventional commits 準拠**

  ```
  ✨ feat: Add Deprecation headers to v1 endpoints
  🐛 fix: Correct pagination offset calculation
  📝 docs: Update migration guide with examples
  ```

- [ ] PR テンプレート **完全入力**
- [ ] コードレビュー **承認数 ≥ 2**

---

#### G-2. CHANGELOG 更新

**チェック項目**:

- [ ] `CHANGELOG.md` 最新版更新
- [ ] `## Unreleased` セクション作成 or 削除
- [ ] 次のバージョン番号 **正確** (Semantic Versioning)

```markdown
## [3.0.0] - 2025-03-17

### Added
- Deprecation headers (RFC 8594) on v1 endpoints
- Comprehensive migration guide (5 languages)
- Extended domain model tests (Rating, Favorite, etc.)

### Changed
- API v2 is now the default recommended version

### Removed
- API v1 endpoints (use v2 endpoints instead)
```

---

### H. CI/CD パイプライン確認

#### H-1. GitHub Actions 全テスト実行

```yaml
# 確認項目
- name: Build
  run: cargo build --all-features

- name: Clippy
  run: cargo clippy --all-targets --all-features -- -D warnings

- name: Tests
  run: cargo test --workspace --no-fail-fast

- name: Coverage
  run: cargo tarpaulin --fail-under 90
```

**チェック項目**:

- [ ] CI Pipeline **すべてのジョブ成功 (Green)**
- [ ] ビルド時間 **< 10分**
- [ ] テスト実行時間 **< 15分**
- [ ] 全体実行時間 **< 45分** (タイムアウト前)

---

#### H-2. マージ前チェック

**チェック項目**:

- [ ] **CI Status**: すべて Green ✅
- [ ] **Code Review**: 最低 2 名の Approve
- [ ] **Conflicts**: マージコンフリクトなし
- [ ] **Branch Protection Rules**: 通過

---

## 📋 チェックリスト実装手順

### Week 1: Deprecation ヘッダー実装時

```markdown
### Day 1-2: コード実装
- [ ] A-1: ビルド成功
- [ ] A-2: 依存関係チェック
- [ ] B-1: ユニットテスト 100% パス

### Day 3: 統合テスト
- [ ] B-2: 統合テスト 100% パス
- [ ] B-3: E2E テスト v1/v2 両方確認
- [ ] D-1: ベンチマーク確認 (< 5% 劣化)

### Day 4-5: レビュー＆マージ
- [ ] C-1: セキュリティスキャン
- [ ] E-1: ドキュメント品質
- [ ] H-2: CI/CD 全テスト緑
```

### Week 3-4: v1 削除準備時

```markdown
### ステージング検証
- [ ] F-1: ステージング デプロイ成功
- [ ] F-2: ステージング E2E テスト 100% パス
- [ ] G-2: CHANGELOG 更新

### 本番適用前
- [ ] H-1: CI/CD Pipeline 全成功
- [ ] C-1: 脆弱性スキャン クリア
- [ ] D-2: ロードテスト 合格
```

---

## 🚨 品質ゲート（品質が満たない場合のアクション）

| 項目 | 失敗時のアクション | 判断者 |
|------|----------------|-------|
| ビルド失敗 | マージ不可（自動ブロック） | CI/CD |
| テスト失敗 | マージ不可（自動ブロック） | CI/CD |
| カバレッジ < 90% | マージ不可（手動確認） | QA Lead |
| セキュリティ脆弱性 | マージ不可（手動確認） | Security Team |
| パフォーマンス > 5% 劣化 | マージ保留（調査） | Tech Lead |
| コードレビュー < 2 approve | マージ不可 | Maintainer |

---

## ✅ 本番リリースチェックリスト

```markdown
## Pre-Release Checklist

### コード品質
- [ ] すべてのテスト成功 (100%)
- [ ] Clippy 警告ゼロ
- [ ] カバレッジ ≥ 90%
- [ ] セキュリティスキャン クリア

### ドキュメント
- [ ] CHANGELOG 最新
- [ ] README 更新
- [ ] Migration Guide 完成
- [ ] API ドキュメント最新

### ステージング検証
- [ ] E2E テスト 100% パス
- [ ] パフォーマンステスト 合格
- [ ] ロードテスト エラーなし

### リスク管理
- [ ] ロールバック計画 作成済み
- [ ] Incident Response 準備完了
- [ ] On-Call エンジニア 配置

### 最終承認
- [ ] Tech Lead: 承認 ___
- [ ] QA Lead: 承認 ___
- [ ] Product Manager: 承認 ___

## Release Date: 2025-03-17
```

---

**最終更新**: 2025-01-17
**次回レビュー**: 2025-02-07
**所有者**: QA Team / Tech Lead
