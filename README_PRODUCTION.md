# Production CMS — 実装と同期したリファレンス

このファイルはリポジトリの実装（Cargo.toml とソース構成）に合わせて更新しています。以下は現行コードベースに即した概要と運用手順です。

## 概要

本プロジェクトは Rust + Axum を用いたスケーラブルな CMS バックエンドです。主要な実装ポイント:

- HTTP 層: axum
- データベース: PostgreSQL + Diesel（feature=`database` にて有効）
- 検索: Tantivy（feature=`search` にて有効）
- 認証: biscuit-auth / JWT + Argon2（feature=`auth` にて有効）
- TLS: rustls（OpenSSL 非依存）

注: Elasticsearch 等の外部検索は現行実装に含まれていません（Tantivy をオンプロセスで使用する形）。

## 主要バイナリ

Cargo マニフェストに定義されている主要バイナリ:

- `cms-server` — デフォルトの HTTP サーバ（`default-run`）
- `cms-migrate` — マイグレーション実行用
- `cms-admin` — 管理用 CLI（簡易）

開発支援・デバッグ用バイナリ（`dev-tools` feature が必要）:

- `admin_server`, `run_docs`, `dev-tools`, `cms-simple`, `cms-lightweight`, など（Cargo.toml の [[bin]] を参照）

## feature とビルド設定

主な feature:

- `auth` — Argon2 / biscuit-auth / jsonwebtoken を含む認証機能
- `database` — Diesel / deadpool-diesel 等の DB レイヤ
- `search` — Tantivy による全文検索
- `cache` — Redis-based キャッシュ（optional）
- `monitoring` — metrics / prometheus（オプショナル）
- `dev-tools` — 開発用ユーティリティ群（いくつかの追加バイナリが有効化される）

デフォルトで有効化される feature は `default`（Cargo.toml で定義）。実行時は必要な feature を明示してビルド/実行してください。

## 前提（推奨）

- Rust toolchain (Rust 1.70+ を想定; Cargo.toml の features により最新の安定版を推奨)
- PostgreSQL 13/14 以上（production 環境に合わせて）
- Docker はオプション（ローカル検証向け）

## 環境変数（例）

主要な環境変数例（`.env` に設定）:

- `DATABASE_URL` — PostgreSQL 接続文字列
- `JWT_SECRET` / `SESSION_SECRET` — 認証シークレット
- `CMS_ENVIRONMENT` / `CONFIG_FILE` — 実行時設定

（詳細は `config/` フォルダの TOML を参照）

## ビルドと実行（Windows PowerShell 例）

以下は簡潔な例。環境や必要な feature に応じて適宜調整してください。

```powershell
# フル（デフォルト feature）を使ってサーバを起動 (デフォルトバイナリ: cms-server)
cargo run --bin cms-server

# デフォルトを無効化して DB と Auth のみで起動する例
cargo run --no-default-features --features database,auth --bin cms-server

# マイグレーション実行（database feature を有効にすることを推奨）
cargo run --no-default-features --features database --bin cms-migrate

# dev-tools バイナリを実行する例（dev-tools feature が必要）
cargo run --bin admin_server --features "dev-tools"
```

注意: 一部の開発用バイナリは `required-features = ["dev-tools"]` により有効化されるため、`--features dev-tools` を付与する必要があります。

## Docker（簡易）

ローカル検証用に Docker イメージを作る例:

```powershell
# イメージをビルド
docker build -t production-cms .

# コンテナを起動 (環境変数は --env-file で渡す)
docker run -d --name production-cms -p 3000:3000 --env-file .env production-cms
```

また、`docker-compose.yml` を用意しているので複合サービス構成も可能です（Compose ファイルを確認してください）。

## API（実装に合わせた主要エンドポイント）

本実装では API は `/api/v1` プレフィックス配下に定義されます。代表的なエンドポイント:

- 認証 (feature=`auth` が有効な場合)
  - POST /api/v1/auth/register
  - POST /api/v1/auth/login
  - POST /api/v1/auth/logout
  - GET  /api/v1/auth/profile

- ポスト / ユーザ操作 (feature=`database`)
  - GET  /api/v1/posts/
  - POST /api/v1/posts/
  - GET  /api/v1/posts/:id
  - PUT  /api/v1/posts/:id
  - DELETE /api/v1/posts/:id

- 検索 (feature=`search`)
  - GET /api/v1/search
  - POST /api/v1/search/reindex

- ヘルスチェック
  - GET /api/v1/health
  - GET /api/v1/health/liveness
  - GET /api/v1/health/readiness

※ 実際のルート一覧は `src/routes` 内の定義を参照してください。

## モニタリング / ロギング

- ロギングは `tracing`/`tracing-subscriber` を用いており、構造化ログ（JSON）出力が可能です。
- Prometheus 等のメトリクスはオプション（`monitoring` feature）で、Cargo.toml に metrics 関連依存が定義されていますが、エンドポイント `/metrics` はプロジェクト設定や feature によって有効化が必要です。

## テスト

```powershell
# 単体テスト
cargo test

# 統合テスト（テストファイル名に依存）
cargo test --test integration_tests
```

## 本番デプロイのヒント

- release ビルド: `cargo build --release`
- バイナリは `target/release/` に作成されます。必要な環境変数／シークレットはデプロイ環境側で安全に管理してください（Doppler 等の外部シークレットマネージャも想定）。
- システム起動スクリプトや systemd ユニットで `cms-server` バイナリを起動する形を推奨します。

## 実装差分・注意点

- Cargo.toml の `default` feature により、ローカルで `cargo run` を行うと複数の機能が有効化されます。プロダクションでは必要な feature / 設定だけを有効化してください。
- 一部の機能（メール送信, Redis キャッシュ, Prometheus エクスポーター等）は optional / feature-gated です。Cargo.toml を確認して必要な feature を明示的に有効化してください。

## コントリビューション

- PR 前に issue を立てる
- フォーマット: `cargo fmt`
- リンター: `cargo clippy`
- テスト: `cargo test`

## ライセンス

MIT

## サポート

- Issues: リポジトリの Issues
- Documentation: `/api/docs`（実装されている場合）
