# RustCMS 推奨コマンド集

## セットアップとビルド

```bash
# 依存関係のインストール
cargo build

# リリースビルド
cargo build --release

# 全機能を有効化してビルド
cargo build --all-features
```

## データベース管理

```bash
# データベース作成
createdb cms_dev

# マイグレーション実行
cargo run --bin cms-migrate

# マイグレーション（本番用）
cargo run --release --bin cms-migrate
```

## 認証キー生成

```bash
# Biscuitキー生成
cargo run --bin gen_biscuit_keys

# APIキー関連のバックフィル
cargo run --features "database auth" --bin backfill_api_key_lookup
```

## サーバー起動

```bash
# 開発サーバー起動
cargo run --bin cms-server

# リリースモードで起動
cargo run --release --bin cms-server

# 管理CLI
cargo run --bin cms-admin
```

## テスト

```bash
# 全テスト実行
cargo test

# 全テスト実行（出力表示）
cargo test -- --nocapture

# 全テスト実行（並列なし）
cargo test --all --no-fail-fast

# 特定のテスト実行
cargo test test_name

# スナップショットテスト
cargo insta test

# スナップショット確認（対話モード）
cargo insta review

# 統合テスト
cargo test --test '*'
```

## コード品質

```bash
# Clippy（リンター）実行
cargo clippy

# Clippy（全警告チェック）
cargo clippy --all-targets --all-features -- -D warnings

# 自動修正可能な警告を修正
cargo clippy --fix --allow-dirty --allow-staged

# フォーマット確認
cargo fmt --check

# フォーマット適用
cargo fmt

# 厳格なClippyチェック
bash scripts/clippy-strict.sh
```

## セキュリティ

```bash
# セキュリティ監査
cargo audit

# 依存関係チェック（deny.toml）
cargo deny check

# 非推奨機能スキャン
bash scripts/deprecation-scan.sh

# 非推奨機能スキャン（ソースのみ）
bash scripts/deprecation-scan.sh --src-only

# 非推奨機能の厳格チェック
bash scripts/deprecation-strict-check.sh
```

## ベンチマーク

```bash
# 全ベンチマーク実行
cargo bench

# 特定のベンチマーク実行
cargo bench benchmark_name

# ベンチマーク結果分析
cargo build --release --bin benchmark-analyzer
./target/release/benchmark-analyzer results.json

# ベースラインとの比較
./target/release/benchmark-analyzer current.json baseline.json
```

## ドキュメント生成

```bash
# ドキュメント生成
cargo doc

# ドキュメント生成（全機能）
cargo doc --all-features

# ドキュメント生成して開く
cargo doc --all-features --open
```

## Docker

```bash
# Dockerイメージビルド
docker build -t rustcms .

# セキュリティ強化版ビルド
docker build -f Dockerfile.security -t rustcms:secure .

# Docker Compose起動
docker-compose up

# Docker Compose（バックグラウンド）
docker-compose up -d
```

## CI/CDとGit

```bash
# ブランチ確認
git branch

# 変更をステージング
git add .

# コミット
git commit -m "message"

# プッシュ
git push origin branch-name

# upstreamから最新を取得
git fetch upstream
git merge upstream/main
```

## Linux システムコマンド（よく使用）

```bash
# ディレクトリ内容表示
ls -la

# ファイル検索
find . -name "*.rs"

# テキスト検索
grep -r "pattern" src/

# プロセス確認
ps aux | grep cms

# ポート確認
netstat -tulpn | grep 8080
lsof -i :8080
```

## 開発フロー推奨

1. ブランチ作成: `git checkout -b feature/new-feature`
2. 変更実装
3. フォーマット: `cargo fmt`
4. リンター: `cargo clippy --fix --allow-dirty`
5. テスト: `cargo test`
6. スナップショット確認: `cargo insta test && cargo insta review`
7. コミット: `git commit -m "feat: description"`
8. プッシュ: `git push origin feature/new-feature`
9. プルリクエスト作成
