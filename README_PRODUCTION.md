# Production CMS - 本番環境対応のコンテンツ管理システム

## 概要

本プロジェクトは、大規模アクセスに対応する実用的なコンテンツ管理システム（CMS）です。以下の最新技術を使用して構築されています：

- **Rust** + **Axum** - 高性能なWebサーバー
- **PostgreSQL** + **Diesel** - 堅牢なデータベースアクセス
- **Elasticsearch** - 高速な全文検索
- **biscuit-auth** - トークンベース認証
- **WebAuthn** - パスワードレス認証

## 特徴

### 🚀 パフォーマンス
- 非同期処理による高いスループット
- コネクションプーリングによる効率的なDB接続
- Elasticsearchによる高速検索

### 🔒 セキュリティ
- WebAuthnによるFIDO2対応のパスワードレス認証
- biscuit-authによる細かな権限制御
- bcryptによる安全なパスワードハッシュ化
- レート制限とCSRF保護

### 📈 スケーラビリティ
- PostgreSQLによる堅牢なデータストレージ
- Elasticsearchによるスケーラブルな検索
- Docker対応による簡単なデプロイメント

## 前提条件

- **Rust** 1.75以上
- **PostgreSQL** 14以上
- **Elasticsearch** 8.5以上
- **Docker** (オプション)

## セットアップ

### 1. リポジトリのクローン

```bash
git clone <repository-url>
cd rust-backend
```

### 2. 環境変数の設定

`.env.example`を`.env`にコピーして設定を編集：

```bash
cp .env.example .env
```

重要な設定項目：
```env
DATABASE_URL=postgresql://username:password@localhost:5432/production_cms
ELASTICSEARCH_URL=http://localhost:9200
JWT_SECRET=your-super-secret-jwt-key-must-be-at-least-32-characters
SESSION_SECRET=your-super-secret-session-key-must-be-at-least-32-characters
WEBAUTHN_RP_ID=localhost
WEBAUTHN_ORIGIN=http://localhost:3000
```

### 3. データベースのセットアップ

#### PostgreSQLの起動
```bash
# Dockerを使用する場合
docker run --name postgres-cms \
  -e POSTGRES_DB=production_cms \
  -e POSTGRES_USER=cms_user \
  -e POSTGRES_PASSWORD=cms_password \
  -p 5432:5432 \
  -d postgres:15
```

#### マイグレーションの実行
```bash
# Windows
migrate.bat

# Linux/Mac
chmod +x migrate.sh
./migrate.sh
```

### 4. Elasticsearchのセットアップ

```bash
# Dockerを使用する場合
docker run --name elasticsearch-cms \
  -e "discovery.type=single-node" \
  -e "xpack.security.enabled=false" \
  -p 9200:9200 \
  -d elasticsearch:8.11.0
```

### 5. 依存関係のインストール

```bash
cargo build --release
```

### 6. アプリケーションの起動

```bash
cargo run --release
```

または

```bash
./target/release/cms-backend
```

## Docker Compose でのセットアップ

完全な環境をDocker Composeで起動：

```bash
docker-compose up -d
```

これにより以下が起動されます：
- PostgreSQL (ポート5432)
- Elasticsearch (ポート9200)
- CMS Application (ポート3000)

## API エンドポイント

### 認証
- `POST /auth/register` - ユーザー登録
- `POST /auth/login` - ログイン
- `POST /auth/logout` - ログアウト
- `GET /auth/profile` - プロフィール取得
- `PUT /auth/profile` - プロフィール更新

### WebAuthn
- `POST /auth/webauthn/register/start` - WebAuthn登録開始
- `POST /auth/webauthn/register/finish` - WebAuthn登録完了
- `POST /auth/webauthn/login/start` - WebAuthnログイン開始
- `POST /auth/webauthn/login/finish` - WebAuthnログイン完了

### ポスト管理
- `GET /posts` - 公開ポスト一覧
- `GET /posts/:id` - ポスト詳細
- `GET /posts/slug/:slug` - スラグによるポスト取得
- `POST /posts` - ポスト作成 (認証必要)
- `PUT /posts/:id` - ポスト更新 (認証必要)
- `DELETE /posts/:id` - ポスト削除 (認証必要)

### 検索
- `GET /search` - ポスト検索
- `GET /search/suggest` - 検索候補
- `GET /search/analytics` - 検索分析 (編集者権限必要)

### ヘルスチェック
- `GET /health` - 基本ヘルスチェック
- `GET /health/detailed` - 詳細ヘルスチェック (管理者権限必要)

### ドキュメント
- `GET /docs` - API ドキュメント (Swagger UI)
- `GET /docs/openapi.json` - OpenAPI 仕様

## ユーザーロール

1. **User** - 基本ユーザー
2. **Author** - 記事作成・編集
3. **Editor** - 記事公開・編集管理
4. **Admin** - システム全体の管理

## テスト

```bash
# 単体テスト
cargo test

# 統合テスト
cargo test --test integration_tests

# カバレッジレポート
cargo tarpaulin --out Html
```

## 本番環境デプロイ

### Docker使用

```bash
# イメージをビルド
docker build -t production-cms .

# コンテナを起動
docker run -d \
  --name production-cms \
  -p 3000:3000 \
  --env-file .env \
  production-cms
```

### 直接デプロイ

```bash
# リリースビルド
cargo build --release

# バイナリを本番サーバーにコピー
scp target/release/cms-backend user@server:/opt/cms/

# Systemdサービスとして起動
sudo systemctl start cms-backend
sudo systemctl enable cms-backend
```

## モニタリング

### ヘルスチェックエンドポイント
- `GET /health` - アプリケーションの基本状態
- `GET /health/detailed` - データベース・Elasticsearch・サービス状態

### ログ
アプリケーションは構造化ログ（JSON形式）を出力し、以下の情報を記録：
- リクエスト/レスポンス
- エラー詳細
- パフォーマンスメトリクス
- セキュリティイベント

### メトリクス
Prometheusメトリクスエンドポイント（将来実装予定）：
- `GET /metrics` - アプリケーションメトリクス

## セキュリティ考慮事項

1. **環境変数の保護** - `.env`ファイルを本番環境では適切に保護
2. **HTTPS使用** - 本番環境では必ずHTTPS通信を使用
3. **定期的な更新** - 依存関係とセキュリティパッチの定期更新
4. **アクセス制御** - データベースとElasticsearchへの適切なアクセス制限

## トラブルシューティング

### よくある問題

1. **データベース接続エラー**
   ```
   ERROR: Connection refused
   ```
   - PostgreSQLサービスが起動しているか確認
   - DATABASE_URLが正しいか確認

2. **Elasticsearch接続エラー**
   ```
   ERROR: Elasticsearch unreachable
   ```
   - Elasticsearchサービスが起動しているか確認
   - ELASTICSEARCH_URLが正しいか確認

3. **WebAuthn登録失敗**
   ```
   ERROR: WebAuthn registration failed
   ```
   - WEBAUTHN_ORIGINがブラウザのURLと一致するか確認
   - HTTPSを使用しているか確認（localhost以外）

## コントリビューション

1. プルリクエストを作成する前にissueを作成
2. コードフォーマット: `cargo fmt`
3. リンター: `cargo clippy`
4. テスト: `cargo test`

## ライセンス

MIT License

## サポート

- Issues: GitHub Issues
- Email: support@example.com
- Documentation: `/docs` エンドポイント
