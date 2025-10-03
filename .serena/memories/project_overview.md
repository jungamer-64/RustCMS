# RustCMS プロジェクト概要

## プロジェクトの目的

RustCMS は、Rust と Axum を使用した高性能で本番運用に耐えるコンテンツ管理システム（CMS）APIです。大規模トラフィックを想定した機能を備え、スケーラビリティとセキュリティに重点を置いています。

## 技術スタック

### 言語とフレームワーク
- **言語**: Rust 2024 Edition (1.75以降)
- **Webフレームワーク**: Axum 0.8 + Tower/Tower-HTTP
- **非同期ランタイム**: Tokio 1.47 (full features)

### データベースとORM
- **データベース**: PostgreSQL 14以降
- **ORM**: Diesel 2.1 (with migrations support)
- **接続プール**: deadpool-diesel 0.6

### 検索とキャッシュ
- **全文検索**: Tantivy 0.25 (Pure Rust)
- **キャッシュ**: Redis 6以降

### 認証とセキュリティ
- **トークン認証**: Biscuit-auth 6.0 (Ed25519署名、Capability-based)
- **APIキー認証**: X-API-Key (Argon2ハッシュ + SHA-256 lookup hash)
- **パスワードハッシュ**: Argon2 0.5
- **WebAuthn**: webauthn-rs (FIDO2対応)

### 監視と可観測性
- **メトリクス**: Prometheus (axum-prometheus)
- **ログ**: tracing + tracing-subscriber
- **ヘルスチェック**: カスタムヘルスエンドポイント

### ドキュメント
- **API仕様**: OpenAPI 3.0 (utoipa)
- **UI**: Swagger UI / ReDoc

## 主な機能

1. **認証システム**
   - Biscuitトークン（Ed25519署名、柔軟な権限デリゲーション）
   - リフレッシュトークンローテーション（再利用攻撃防止）
   - APIキー認証（長期利用向け）
   - WebAuthn対応（パスワードレス認証）

2. **パフォーマンス最適化**
   - データベース接続プーリング
   - Redisによる多層キャッシュ
   - インテリジェントなレート制限

3. **セキュリティ**
   - ロールベースアクセス制御（RBAC）
   - 包括的な入力検証
   - SQL インジェクション防止（型安全性）
   - CORS設定

4. **開発者体験**
   - 自動生成されるOpenAPIドキュメント
   - 統一されたAPIレスポンス構造（ApiResponse<T>）
   - スナップショットテスト（insta）
   - 包括的なベンチマークスイート

## バージョンとライセンス

- **バージョン**: 3.0.0
- **ライセンス**: MIT
- **リポジトリ**: https://github.com/jungamer-64/Rust-CMS
