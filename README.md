# 🚀 エンタープライズ向け CMS バックエンド

高性能で本番運用に耐えるコンテンツ管理システム（CMS）API。Rust と Axum を用いて構築され、大規模トラフィックを想定したエンタープライズ向け機能を備えています。

## 🚀 主な特徴

### パフォーマンスとスケーラビリティ

- **高性能アーキテクチャ**: Rust と Axum による最大性能を意識した実装
- **データベース接続プーリング**: PostgreSQL 向けの最適化された接続管理
- **Redis キャッシュ**: レスポンスタイム向上のための多層キャッシュ戦略
- **レートリミット**: 悪用防止のためのインテリジェントなレート制御
- **ロードバランサ対応**: ステートレス設計で水平スケールが容易

### セキュリティ

- **JWT 認証**: トークンベースの安全な認証方式
- **ロールベースアクセス制御**: 詳細な権限管理
- **入力検証**: リクエストの包括的なバリデーション
- **CORS 設定**: クロスオリジン制御の設定が可能
- **SQL インジェクション防止**: パラメタライズドクエリと型安全性

### モニタリングと可観測性

- **Prometheus メトリクス**: 包括的なメトリクス収集
- **構造化ログ**: tracing による詳細ログ出力
- **ヘルスチェック**: エンドポイントおよびサービスのヘルス監視
- **パフォーマンストラッキング**: リクエスト時間などの分析

### 開発者体験

- **OpenAPI ドキュメント**: 自動生成される Swagger UI
- **型安全性**: Rust の型システムによる実行時エラーの低減
- **モダンな非同期処理**: Tokio ベースの async ランタイム
- **Docker サポート**: 本番対応のコンテナ化

## 📊 アーキテクチャ

```text
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   ロードバランサー │    │     Redis       │    │   PostgreSQL    │
│     (Nginx)     │◄──►│     キャッシュ     │    │    データベース    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                 │                       ▲                       ▲
                 ▼                       │                       │
┌─────────────────┐              │                       │
│   CMS バックエンド  │              │                       │
│   (Rust/Axum)   │──────────────┼───────────────────────┘
│                 │              │
│  ┌─────────────┐│              │
│  │レートリミッター││──────────────┘
│  └─────────────┘│
│  ┌─────────────┐│
│  │   認証       ││
│  └─────────────┘│
│  ┌─────────────┐│
│  │  メトリクス   ││
│  └─────────────┘│
└─────────────────┘
```

## 🛠️ クイックスタート

### 前提条件

- Rust 1.75 以上
- PostgreSQL 13 以上
- Redis 6 以上
- Docker（任意）

#### ローカル開発

1. リポジトリをクローンします

```bash
git clone <repository-url>
cd Rust-CMS  # 任意のフォルダ名
```

1. 環境の設定

設定は `config/default.toml` を基点に環境変数で上書きできます。最低限の例:

```bash
cp .env.example .env   # （存在する場合）
# set DATABASE_URL=postgres://user:pass@localhost:5432/cms_db
```

1. （任意）外部サービスを起動

```bash
docker compose up -d postgres redis  # Redis / search を使わないなら省略可
```

1. マイグレーション（Diesel を使用する場合、feature `database` 有効時）

```bash
cargo run --bin cms-migrate  # 実装されている簡易マイグレーションバイナリ
```

1. サーバ起動

```bash
cargo run --bin cms-server
```

外部サービスを最小にしたビルド例:

```bash
cargo build --no-default-features --features "dev-tools,auth,database"
cargo run --no-default-features --features "dev-tools,auth,database" --bin cms-server
```

補助バイナリ:

- `cms-admin` : 管理・運用用 CLI（ユーザ作成など）
- `cms-migrate`: DB マイグレーション実行

デフォルト起動バイナリは `cms-server`（`Cargo.toml` の `default-run` 設定に依存）。

### Docker デプロイ

```bash
# すべてのサービスをビルドして起動
docker-compose up -d

# ログ確認
docker-compose logs -f cms-backend

# バックエンドをスケール
docker-compose up -d --scale cms-backend=3
```

## 📚 API ドキュメント / ルート一覧

ベースパス: `http://localhost:3000/api/v1`

- API 情報: `GET /api/v1` または `GET /api/v1/info`
- ヘルスチェック: `GET /api/v1/health`（`/liveness`, `/readiness` のサブパスあり）
- 認証（feature=auth）: `POST /api/v1/auth/register`, `POST /api/v1/auth/login`, `POST /api/v1/auth/logout`, `GET /api/v1/auth/profile`, `POST /api/v1/auth/refresh`
- 投稿（feature=database）: `/api/v1/posts` 以下で CRUD
- ユーザ（feature=database）: `/api/v1/users` 以下で CRUD
- 管理 API（feature=database）: `/api/v1/admin/posts` (一覧/作成), `/api/v1/admin/posts/:id` (削除)
- 検索（feature=search）: `/api/v1/search`, `/suggest`, `/stats`, `/reindex`, `/health`
- OpenAPI UI: `GET /api/docs`
- OpenAPI JSON: `GET /api/docs/openapi.json`

ルート直下の `/health` は簡易的な別実装が残る場合があります。標準的には `/api/v1/health` を利用してください。

### 認証

保護されたエンドポイントは Authorization ヘッダに JWT トークンを必要とします:

```http
Authorization: Bearer <your-jwt-token>
```

### リクエスト例（主要ルート）

#### ログインしてトークンを取得

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "demo_user",
    "password": "password"
  }'
```

#### 投稿の作成

```bash
curl -X POST http://localhost:3000/api/v1/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "title": "最初の投稿",
    "content": "これは最初の投稿の本文です。",
    "published": true
  }'
```

#### 投稿一覧の取得（ページネーション付き）

```bash
curl "http://localhost:3000/api/v1/posts?page=1&limit=10"
```

#### 検索（feature=search）

```bash
curl "http://localhost:3000/api/v1/search?q=rust"
```

（その他: `drill --benchmark benchmark.yml` などのベンチマークコマンド参照）

## 詳細: 設定・パフォーマンス・監視・テスト等

本番運用向けの詳細（環境変数一覧、パフォーマンスチューニング、監視・ヘルスチェック、ロードテスト、セキュリティ設定など）は `README_PRODUCTION.md` にまとめてあります。トップ README には開発・ビルドの最小手順と参照先のみを掲載しています。

必要であれば `README_PRODUCTION.md` の特定セクション（例: デプロイ手順、監視設定）をトップ README に抜粋します。どのセクションを抜粋するか指示してください。

## 🤝 コントリビュート

1. リポジトリを Fork する
1. 機能用ブランチを作成する
1. 新機能にはテストを追加する
1. すべてのテストが通ることを確認する
1. プルリクエストを作成する

## 📝 ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。詳細は `LICENSE` ファイルを参照してください。

## 🆘 サポート

- **ドキュメント**: API ドキュメントは `GET /api/docs` で確認できます
- **Issues**: バグは GitHub Issues へ報告してください
- **パフォーマンス**: インサイトには `/metrics` エンドポイントを使用してください
- **監視**: 本番監視には Prometheus の導入を推奨します

---

## 🎯 最近の改善点

この CMS バックエンドは大規模リファクタにより、以下の改善を実施しています。

### ✅ パフォーマンス改善

- **データベース接続プーリング**: PostgreSQL 向けに SQLx 等で実装
- **Redis キャッシュ**: 自動無効化を含む多層キャッシュ戦略
- **レートリミッター**: エンドポイント毎のインテリジェントな制御
- **非同期処理**: Tokio ベースの完全な async/await 実装

### ✅ セキュリティ改善

- **JWT 認証**: 安全なトークンベース認証
- **入力検証**: カスタムエラーハンドリングを含む包括的な検証
- **SQL インジェクション防止**: パラメタライズドクエリと型安全性
- **CORS 保護**: 設定可能なクロスオリジン制御

### ✅ スケーラビリティ機能

- **水平スケーリング**: ロードバランサとの互換性を意識したステートレス設計
- **接続管理**: DB/キャッシュ接続の最適化
- **メモリ効率**: 効率的なデータ構造とメモリ管理
- **リソース最適化**: 高スループット向けのチューニング

### ✅ 開発者体験

- **OpenAPI ドキュメント**: 自動生成される Swagger UI
- **型安全**: Rust による実行時エラー低減
- **エラーハンドリング**: カスタムエラー型による網羅的な処理
- **テスト**: 信頼性向上のためのユニット・統合テスト

### ✅ モニタリング & 可観測性

- **Prometheus メトリクス**: 包括的メトリクス収集
- **ヘルスチェック**: すべてのサービスに対する詳細ヘルス監視
- **構造化ログ**: tracing サポートによる詳細ログ
- **パフォーマンストラッキング**: リクエスト時間などの分析

このリファクタ済み CMS バックエンドはエンタープライズ向けデプロイを想定しており、大規模トラフィックに対応可能です。
