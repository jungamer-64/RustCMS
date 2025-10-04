# GitHub Actions Workflows

このディレクトリには、RustCMSプロジェクトのCI/CDパイプラインを構成するワークフローが含まれています。

## 📋 ワークフロー一覧

### Core Workflows（コアワークフロー）

#### `ci.yml` - メインCI

**トリガー:** push(main), PR, 毎日3:23 UTC

メインの継続的インテグレーションパイプライン。以下を含みます：

- **Lint & Format**: `cargo fmt`, `cargo clippy`
- **Build & Test**: 複数のRustバージョン（1.89.0, stable）と機能セット（default, minimal, no-flat）でビルド・テスト
- **Security**: `cargo-deny`, `cargo-audit`, gitleaks
- **Coverage**: tarpaulin + Codacy統合
- **Integration Tests**: PostgreSQL統合テスト
- **Deprecation Scan**: 非推奨機能のスキャンと自動ガイダンス

**依存サービス:**

- PostgreSQL 16
- Redis 7

**必要なシークレット:**

- `DATABASE_URL`
- `BISCUIT_PRIVATE_KEY_B64` / `BISCUIT_PUBLIC_KEY_B64`
- `CODACY_PROJECT_TOKEN` (オプション)

---

#### `docker.yml` - Docker統合ワークフロー

**トリガー:** push(main, tags), PR, CI完了後

PRビルドと本番リリースを統合したワークフロー。

**PRモード（`build-pr`）:**

- 3つのバリアント（prod, admin, minimal）をビルド
- Trivyセキュリティスキャン
- イメージをアーティファクトとしてエクスポート
- プッシュなし（検証のみ）

**リリースモード（`build-release`）:**

- multi-arch（amd64, arm64）ビルド
- GHCR（GitHub Container Registry）へプッシュ
- Cosignによるキーレス署名
- SBOM/Provenanceメタデータ生成
- 本番環境向けTrivyスキャン

**生成されるイメージタグ:**

- `latest` - mainブランチ最新
- `prod-latest` - 本番最新
- `v{version}` - セマンティックバージョン
- `sha-{commit}` - コミットSHA

---

#### `benchmarks.yml` - パフォーマンスベンチマーク

**トリガー:** push/PR（src/, benches/変更時）, 毎日2:00 UTC, 手動

継続的なパフォーマンス測定とリグレッション検出。

**ベンチマークカテゴリ:**

- 認証（auth_benchmark）
- キャッシュ（cache_benchmark）
- 検索（search_benchmark）
- データベース（database_benchmark）

**機能:**

- ベースラインとの比較（PR）
- パフォーマンスレポート自動生成
- リグレッション警告
- 夜間レポート（365日保持）

---

### Automation Workflows（自動化ワークフロー）

#### `gemini-dispatcher.yml` - AIアシスタントディスパッチャー

**トリガー:** PR, issue, コメント

Gemini AI機能の中央ディスパッチャー。ユーザーリクエストを解析し、適切なサブワークフローにルーティングします。

**サポートされるコマンド:**

- `@gemini-cli /review` - コードレビュー
- `@gemini-cli /triage` - Issue分類
- `@gemini-cli [その他]` - 汎用コマンド

**権限チェック:**
OWNER, MEMBER, COLLABORATORのみが`@gemini-cli`を呼び出し可能。

---

#### `gemini-review.yml` - AIコードレビュー

**トリガー:** workflow_call（gemini-dispatcherから）

PR差分を分析し、以下の観点でレビューコメントを投稿：

1. Correctness（正確性）
2. Security（セキュリティ）
3. Efficiency（効率性）
4. Maintainability（保守性）
5. Testing（テスト）
6. Performance（パフォーマンス）

**重大度レベル:**

- 🔴 Critical - 本番障害リスク
- 🟠 High - 重大な問題
- 🟡 Medium - ベストプラクティス違反
- 🟢 Low - スタイル、マイナー

**使用ツール:**

- `mcp__github__*` - GitHub MCP Server
- Gemini API / Vertex AI

---

#### `gemini-triage.yml` - Issue自動分類

**トリガー:** workflow_call（gemini-dispatcherから）

新規issueを分析し、適切なラベルを自動的に付与。

**処理フロー:**

1. リポジトリの利用可能なラベルを取得
2. issueのタイトル・本文を分析
3. セマンティックマッチングでラベル選択
4. ラベルを自動適用

---

#### `gemini-invoke.yml` - 汎用AIアシスタント

**トリガー:** workflow_call（gemini-dispatcherから）

汎用的なAI支援タスクを実行。

**機能:**

- 計画作成 → 承認待ち → 実行 → レポート
- ブランチ作成、ファイル編集、PR作成
- セキュリティ重視（ユーザー入力を信頼しない）

**承認フロー:**

1. タスクを分析し実行計画を投稿
2. メンテナーが`/approve`でコメント
3. 承認後に計画を実行
4. 完了レポートを投稿

---

#### `gemini-scheduled.yml` - 定期Issue Triage

**トリガー:** 毎時0分, 手動

ラベルのないissueや`status/needs-triage`ラベルのissueを定期的にスキャンし、一括でトリアージ。

**処理フロー:**

1. 未トリアージissueを最大100件取得
2. Geminiで一括分析
3. 各issueに適切なラベルを付与

---

### Security Workflows（セキュリティワークフロー）

> **注意:** `security-scan.yml`は削除されました。セキュリティスキャンは`ci.yml`と`docker.yml`に統合されています。

**統合されたセキュリティチェック:**

1. **依存関係チェック**（ci.yml）
   - `cargo-deny`: ライセンス、アドバイザリ
   - `cargo-audit`: 既知の脆弱性

2. **シークレットスキャン**（ci.yml）
   - `gitleaks`: コミット履歴内のシークレット検出

3. **コンテナスキャン**（docker.yml）
   - `Trivy`: HIGH/CRITICAL脆弱性検出
   - SARIF形式でGitHub Security Alertsに統合

---

## 🔧 ワークフローの設定

### 必須シークレット

| シークレット名 | 説明 | 必須 |
|---------------|------|------|
| `DATABASE_URL` | PostgreSQL接続文字列 | ✅ |
| `POSTGRES_PASSWORD` | PostgreSQLパスワード | ✅ |
| `BISCUIT_PRIVATE_KEY_B64` | Biscuit認証秘密鍵（base64） | ✅ |
| `BISCUIT_PUBLIC_KEY_B64` | Biscuit認証公開鍵（base64） | ✅ |
| `GITHUB_TOKEN` | 自動生成（明示的設定不要） | 自動 |

### オプションシークレット

| シークレット名 | 説明 | 用途 |
|---------------|------|------|
| `CODACY_PROJECT_TOKEN` | Codacyカバレッジトークン | カバレッジレポート |
| `GEMINI_API_KEY` | Gemini APIキー | AI機能 |
| `GOOGLE_API_KEY` | Google APIキー | AI機能（代替） |
| `APP_PRIVATE_KEY` | GitHub App秘密鍵 | Gemini認証 |

### 必須Variables

| 変数名 | 説明 | 例 |
|--------|------|-----|
| `APP_ID` | GitHub App ID | `123456` |
| `GCP_WIF_PROVIDER` | Workload Identity Provider | `projects/.../providers/...` |
| `GOOGLE_CLOUD_PROJECT` | GCPプロジェクトID | `my-project` |
| `GOOGLE_CLOUD_LOCATION` | GCPリージョン | `us-central1` |
| `SERVICE_ACCOUNT_EMAIL` | サービスアカウント | `sa@project.iam.gserviceaccount.com` |

### オプションVariables

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `DEBUG` | デバッグモード有効化 | `false` |
| `GEMINI_MODEL` | 使用するGeminiモデル | `gemini-pro` |
| `GOOGLE_GENAI_USE_VERTEXAI` | Vertex AI使用 | `false` |
| `GOOGLE_GENAI_USE_GCA` | Code Assist使用 | `false` |

---

## 🚀 ローカルでのテスト

### CIワークフローをローカルで実行

```bash
# act（GitHub Actions local runner）をインストール
brew install act  # macOS
# または
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# ワークフローを実行
act -j lint  # lintジョブのみ
act -j test  # testジョブのみ
act push     # pushイベントで全ジョブ

# シークレットを渡す
act -s DATABASE_URL="postgres://..." -s BISCUIT_PRIVATE_KEY_B64="..."
```

### Dockerビルドをローカルでテスト

```bash
# PRビルドのシミュレーション
docker build \
  --build-arg FEATURES=production \
  --build-arg BINARY=cms-server \
  --build-arg BUILD_VARIANT=prod \
  -t rustcms:local-test .

# Trivyスキャン
docker run --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy:latest image \
  --severity HIGH,CRITICAL \
  rustcms:local-test
```

### ベンチマークをローカルで実行

```bash
# すべてのベンチマーク
cargo bench

# 特定のベンチマーク
cargo bench --bench auth_benchmark
cargo bench --bench cache_benchmark

# Criterion HTMLレポート生成
cargo criterion
open target/criterion/report/index.html
```

---

## 📊 ワークフローの依存関係

```
ci.yml (Main CI)
  ├─→ lint ────→ test ────→ integration-tests
  │                  │
  │                  ├─→ deprecated-scan
  │                  ├─→ cargo-deny
  │                  ├─→ secrets-scan
  │                  ├─→ audit
  │                  └─→ coverage
  │
  └─→ triggers → docker.yml (on success)

docker.yml
  ├─→ build-pr (if PR)
  └─→ build-release (if push/workflow_run)

gemini-dispatcher.yml
  ├─→ gemini-review.yml
  ├─→ gemini-triage.yml
  └─→ gemini-invoke.yml

gemini-scheduled.yml (independent, hourly)
  └─→ gemini-triage.yml (logic)

benchmarks.yml (independent)
  ├─→ benchmark (daily/on-demand)
  └─→ benchmark-comparison (if PR)
```

---

## 🔄 ワークフローの更新履歴

### v2.0.0 (統合リファクタリング)

- ✅ `security-scan.yml`を削除（`ci.yml`に統合）
- ✅ `ci-docker-build.yml` + `docker-release.yml` → `docker.yml`に統合
- ✅ `gemini-dispatch.yml` → `gemini-dispatcher.yml`にリネーム
- ✅ すべてのGitHub Actionsをcommit SHAでpin
- ✅ 冗長なトリガーとpaths-ignoreを最適化
- ✅ コンカレンシー制御を改善

### v1.x (レガシー)

- 個別のセキュリティワークフロー
- 分散したDockerワークフロー
- 一貫性のない命名規則

---

## 🛠️ トラブルシューティング

### よくある問題

#### 1. テストが失敗する（DATABASE_URL）

**症状:** `connection refused` エラー

**解決策:**

```bash
# シークレットが設定されているか確認
gh secret list

# ローカルでPostgreSQLを起動
docker run -d \
  -e POSTGRES_PASSWORD=test \
  -e POSTGRES_DB=cms_test \
  -p 5432:5432 \
  postgres:16-alpine

export DATABASE_URL="postgres://postgres:test@localhost:5432/cms_test"
cargo test
```

#### 2. Biscuitキー生成エラー

**症状:** `BISCUIT_PRIVATE_KEY_B64` not found

**解決策:**

```bash
# キーを生成
cargo run --bin gen_biscuit_keys

# 出力をシークレットとして設定
gh secret set BISCUIT_PRIVATE_KEY_B64 -b"$(echo 'output_from_above' | base64)"
gh secret set BISCUIT_PUBLIC_KEY_B64 -b"$(echo 'output_from_above' | base64)"
```

#### 3. Dockerビルドが遅い

**症状:** ビルドが60分以上かかる

**解決策:**

- BuildKitキャッシュが機能しているか確認
- `cache-from`/`cache-to`の設定を確認
- 不要な`COPY`コマンドを削減

#### 4. Geminiワークフローが動作しない

**症状:** `@gemini-cli`コマンドに反応しない

**確認項目:**

- [ ] `APP_ID`と`APP_PRIVATE_KEY`が設定されているか
- [ ] ユーザーがOWNER/MEMBER/COLLABORATORか
- [ ] GitHub Appに適切な権限があるか
- [ ] `GEMINI_API_KEY`または`GOOGLE_API_KEY`が設定されているか

---

## 📚 参考リソース

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Docker Build Push Action](https://github.com/docker/build-push-action)
- [Trivy Security Scanner](https://github.com/aquasecurity/trivy)
- [Cosign Keyless Signing](https://docs.sigstore.dev/cosign/overview/)
- [Gemini API Documentation](https://ai.google.dev/docs)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
- [cargo-audit](https://github.com/rustsec/rustsec)

---

## 📞 サポート

ワークフローに関する問題や提案がある場合：

1. 既存のissueを検索: [Issues](../../issues)
2. 新しいissueを作成（ラベル: `ci`, `workflows`）
3. PRを作成（小規模な修正）

**緊急の場合:** `@gemini-cli /invoke` でAIアシスタントに相談できます。

---

**最終更新:** 2025-10-04
**メンテナー:** @jungamer-64
