# 用語集（日本語/英語）

- 認証 / Authentication: ユーザーの本人確認。Biscuit トークン/セッション/パスワード検証を含む。
- 認可 / Authorization: 権限チェック（ロール/ポリシー）。
- トークン / Token: アクセス権限を表す資格情報（Biscuit/JWT 等）。
- セッション / Session: ログイン状態のサーバー側表現。
- CSRF: クロスサイトリクエストフォージェリ対策。
- レート制限 / Rate limiting: 一定期間内のリクエスト数を制限すること。
- フィーチャーフラグ / Cargo feature: コンパイル時に機能の有効/無効を切り替える仕組み。
- 互換フィールド / Compatibility fields: `auth-flat-fields` で一時的に提供する旧形式のフラットトークン。
- レガシー認証スキーマ / Legacy auth schema: `legacy-auth-flat` で OpenAPI に含める歴史的 `LoginResponse` スキーマ。
- テレメトリ / Telemetry: ログ/トレース/メトリクスの総称。
- 検索インデックス / Search index: Tantivy の全文検索インデックス。
- キャッシュ / Cache: Redis やメモリによる応答高速化層。
- 移行 / Migration: DBスキーマ変更の適用。
