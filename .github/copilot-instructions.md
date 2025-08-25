## 概要 — これを読めば即戦力になります
このリポジトリは Rust 製の CMS バックエンドです。主要ポイントは：
- `axum` を使った HTTP レイヤ、`diesel`（Postgres）で DB、`tantivy`（任意）で全文検索。
- 単一の `AppState`（`src/app.rs`）がサービス（DB/Auth/Cache/Search）とメトリクスを一元管理します。
- 機能は Cargo の feature フラグ（`database` / `auth` / `cache` / `search` / `dev-tools` 等）で有効化されます。

## まず見るべきファイル（優先順位）
- `src/app.rs` — 中央の状態・メトリクス・AppState ビルダー。サービス初期化と健康チェック、AppState 経由のラッパーが重要。
- `src/utils/init.rs` — エントリポイント用の初期化ヘルパ（ログ/設定/AppState の作成）。バイナリはここを使います。
- `src/auth/mod.rs` — 認証ロジック（JWT / Biscuit / セッション管理）。現在 authenticate 内で DB を直接使う箇所が残るため、AppState の DB ラッパーを使う方針に注意。
- `src/database/mod.rs` — Database 構築、CRUD の async ラッパー。DB タイミングは AppState 側で記録する設計になっています。
- `src/handlers/*.rs` と `src/middleware/*.rs` — ハンドラはできるだけ `AppState` の wrapper (`state.auth_*`, `state.db_*`, `state.search_*`) を使う。
- `Cargo.toml` — autobins 無効化、feature 定義、`dev-tools` に dev 用バイナリがまとめられています。
- `README.md` と `migrations/`、`config/` フォルダ — 実行・環境・マイグレーション情報。

## コーディング規約・パターン（このプロジェクト固有）
- AppState が単一の計測・初期化ポイント。個別ハンドラで DB や計測を行わず、AppState の wrapper を通す。例：
  - `state.auth_authenticate(request).await?` → ユーザー認証（AuthService を直接呼ばない）
  - `state.db_create_user(...)` / `state.db_get_user_by_id(...)` などの `db_*` API を優先。
- Cargo feature ガードが多数あるため、編集/追加時は feature を考慮する（テスト時は `--features` を指定する）。
- データベース実装は sync な Diesel 呼び出しを async wrapper で包むスタイル。DB トランザクションや接続取得は `Database` 型経由。

## ビルド・実行・デバッグ（Windows PowerShell 環境向け具体例）
- 開発ビルド（Windows ではビルド並列数が問題になるので単一ジョブ推奨）:
```powershell
cargo build -j1
```
- サーバ起動（環境変数か Doppler を利用）:
```powershell
# .env を使う開発
cargo run --bin cms-server

# Doppler 経由 (推奨: production-like)
.\start-with-doppler.bat
# あるいは
doppler run -- cargo run --bin cms-server
```
- dev-only バイナリ群を動かす（feature を有効化）:
```powershell
cargo run --bin dev-tools --features dev-tools
```
- テスト実行:
```powershell
cargo test -j1
```

## 典型的な変更パターンと注意点
- 新しい DB 操作を追加するなら `src/database/mod.rs` に async メソッドを追加し、合わせて `src/app.rs` に `db_*` の wrapper を追加して計測を一箇所で行う。
- 認証ロジック（`src/auth/mod.rs`）を触る場合、可能なら直接 DB コネクションを呼び出さず、AppState の `db_*` wrapper を使うようにリファクタする（既にこの方針へ移行中）。
- ロギング/トレーシングは `tracing` と `opentelemetry` を使う。初期化は `src/utils/init.rs` や `telemetry` モジュールを参照。

## 依存・統合ポイント（外部サービス）
- PostgreSQL（Diesel） — `DATABASE_URL` を環境で指定。マイグレーションは `migrations/` フォルダ。
- Redis（任意・deadpool-redis） — キャッシュ機能は feature-gated。
- Tantivy（全文検索、feature=`search`） — 検索インデックス化は `src/search` 系。
- Doppler — 本番向けのシークレット管理。リポジトリに `doppler.*` サンプルあり。
- OpenTelemetry / Jaeger — tracing を使った分散トレーシング設定が可能。

## ルールショートリファレンス（AI エージェント向け）
1. まず `AppState` を探す。サービス追加/削除は AppState を通すこと。  
2. ハンドラは `State<AppState>` を受け、直接 `Database` を取得しないこと（既に存在する wrapper を先に探す）。  
3. Cargo feature による条件付きコンパイルを壊さない。新しいコードは対応する feature ガードを付ける。  
4. Windows 開発では `-j1` を使ったビルドを推奨（OOM / ファイルロック回避）。

## 最後に（フィードバック）
このファイルをベースに追加して欲しい情報（例えば特定のデバッグログ出力例や CI コマンド）はありますか？不明点があれば指示をください。更新します。
