# モジュール機能一覧（RustCMS）

本書は `src/` 配下の主要モジュールの責務と代表的な型/関数を概説します。詳細な使い方や契約は各Rustdocを参照してください。

## エントリーポイント

- `main.rs`: サーバ起動。設定読込→依存初期化→ルータ構築→HTTP待受。
- `lib.rs`: クレートの公開モジュールの集約。

## 横断関心事

- `telemetry.rs`: ログ/トレース/メトリクス初期化。
- `error.rs`: アプリ共通エラーとHTTPマッピング。
- `openapi.rs`: utoipa によるOpenAPIスキーマ生成。`legacy-auth-flat` でスキーマ差分。

## アプリ構成

- `app.rs`: AppState/metrics/health、各サービス(DB/Auth/Cache/Search)を束ねる中枢。
- `routes/`: ルータ作成（`create_router`）。
- `middleware/`: セキュリティ/監査などの中間処理。

## ハンドラ（REST）

- `handlers/health.rs`: 健康診断。
- `handlers/metrics.rs`: Prometheus等のメトリクス出力。
- `handlers/auth.rs`: 認証/認可、トークン更新。
- `handlers/posts.rs`: 投稿CRUD/公開/タグ検索。
- `handlers/users.rs`: ユーザーCRUD/ロール変更。
- `handlers/search.rs`: 検索/サジェスト/統計/再索引/ヘルス。
- `handlers/api_keys.rs`: APIキー管理（作成/一覧/失効）。

## 認証・認可

- `auth/`: 認証サービス（Biscuit/WebAuthn）、権限検証。

## キャッシュ

- `cache/`: Redis + メモリの多層キャッシュ。

## データベース

- `database/`: 接続プール、スキーマ、低レベル操作。
- `repositories/`: リポジトリ層（高レベルの永続化API）。
- `models/`: ドメイン/DTO。

## 検索

- `search/`: Tantivy全文検索、インデックス管理。

## ユーティリティ

- `utils/`: ApiResponse、Pagination、共通型、各種ヘルパ。

## レート制限

- `limiter/`: 固定ウィンドウ等のレート制限実装。

## 監視

- `monitoring/`: ダッシュボード連携など（存在時）。

## Featureフラグ（Cargo.toml 抜粋）

- `auth`: 認証機能（argon2/biscuit）。
- `auth-flat-fields`: 旧フラットトークン互換フィールド（3.0.0で削除予定）。
- `cache`: Redisキャッシュ。
- `database`: Diesel/PostgreSQL。
- `email`: メール送信。
- `legacy-auth-flat`: 歴史的 `LoginResponse` をOpenAPIに含める。
- `monitoring`: メトリクス/Prometheus。
- `search`: Tantivy全文検索。

