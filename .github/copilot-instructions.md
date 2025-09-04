## Rust CMS AI ガイド (最短ブートストラップ)
Axum + Diesel(Postgres) + (可選) Redis / Tantivy。全サービスは単一 `AppState` (`src/app.rs`) に集約し、計測と依存管理を中央化。機能は feature flag (`auth`, `database`, `cache`, `search`, `dev-tools`, `monitoring`, `email`) で ON/OFF。

### 重要ファイル / 境界
| 区分 | 位置 | 目的 / パターン |
|------|------|----------------|
| App State | `src/app.rs` | サービス初期化・ヘルスチェック・計測。`timed_op!` マクロで DB / auth / search 呼び出し時間を集計。追加処理はここに wrapper を生やす。|
| 認証 | `src/auth/mod.rs` | Biscuit トークン + セッション (in-memory)。DB へは極力 `state.db_*` 経由。鍵生成は `gen_biscuit_keys` バイナリ。|
| DB | `src/database/` | Diesel 同期 API を async ラッパ化。接続取得は必ず `AppState.database.get_connection()` 経由。|
| ルーティング/処理 | `src/handlers/`, `src/middleware/`, `src/routes/` | ハンドラ内で直接 `Database` / `AuthService` を触らず `state.xxx_*` wrapper。|
| 検索 | `src/search/` | Tantivy index 操作。呼び出しは `state.search_*` wrapper 経由で計測。|
| キャッシュ | `src/cache/` | Redis + In-memory。`state.cache_get_or_set` / `cache_invalidate_prefix` を利用。|
| 設定 | `config/*.toml` + `src/config/` | `Config::from_env()` でロード。|
| OpenAPI | `src/openapi.rs` + `utoipa` | Feature 連動したスキーマ生成。|
| Migrations | `migrations/` + `cms-migrate` | Diesel SQL / バイナリ。|
| API Key Backfill | `src/bin/backfill_api_key_lookup.rs` | 旧行の `lookup_hash` 移行 / 失効 CLI。|

### コア原則 (守ると壊れない)
1. 直接サービス呼び出し禁止: まず既存 `AppState` wrapper (`auth_*`, `db_*`, `search_*`) を探し再利用。無ければ同名ポリシーで追加。 
2. 計測一元化: 新しい外部 I/O は `AppState` にメソッドを追加し中で `timed_op!` か既存記録ヘルパ (`record_db_query` 等) を使う。ハンドラ側で `Instant` を測らない。
3. Feature gate を必ず考慮: 依存型/メソッドは `#[cfg(feature="...")]` で守る。Wrapper 追加時も同条件を付与。
4. API キー: 生キーは一度しか返さない。`ApiKey` 作成/取得/失効は `db_*_api_key` wrapper 参照し同じパターン（`db_time_api` + 手動 Diesel）。
5. 認証: Biscuit の期限/セッション管理は `AuthService` 内。外部からは認証結果ユーザのみ利用。ロール→権限はサービス内部で解決。

### 代表的な変更手順 (例: 新しい User 集計クエリ追加)
1. `src/database/mod.rs` に同期 Diesel ロジック (接続取得→クエリ) を追加。
2. `src/app.rs` に `db_***` wrapper を追加し `timed_op!(self, "db", self.database.xxx(...))` で包む。
3. ハンドラで `state.db_new_metric(...).await?` を呼ぶ。直 DB 呼び出し禁止。
4. 必要なら OpenAPI 型を `models/` や `api_types` に追加。

### Biscuit 鍵運用ショートノート
`cargo run --bin gen_biscuit_keys -- --format files --out-dir keys --backup --max-backups 3 --force` でローテ + バックアップ。`.env` 連携は `--format env`。秘密鍵は commit しない。

### ビルド / 実行 (PowerShell)
```powershell
cargo run --bin cms-server            # 標準 (default features)
cargo run --no-default-features --features "auth,database" --bin cms-server  # 絞った構成
cargo test                            # 全テスト
cargo run --bin cms-migrate           # マイグレーション
cargo run --bin backfill_api_key_lookup --features "database,auth" -- --dry-run
```

### テスト指針 (抜粋)
統合テストは `tests/`。`rstest` / `serial_test` を併用。Biscuit / API キー生成はユーティリティ関数で重複回避。新しいラッパー追加時は: 正常系 + エラー (NotFound / Authorization) を最低限。Feature 依存は `#[cfg(feature = ...)]` でガード。

### メトリクス/ヘルス
`state.health_check()` が全サービス (database/cache/search/auth) を並列的に評価し `up|down|degraded`。新規サービス追加時は `HealthStatus` / 個別 check メソッド / wrapper で統一。Prometheus 追加は計測箇所を AppState に集中。

### キャッシュパターン
計算コストのある取得は `state.cache_get_or_set("posts:page:1", ttl, || async { ... })`。更新系は関連 prefix (`posts:*`) を `cache_invalidate_prefix` で失効させる。Redis 未使用時も API は同じ。

### よくある落とし穴
| 問題 | 回避 |
|------|------|
| ハンドラで Diesel を直に呼ぶ | `AppState` wrapper 経由に統一 (計測欠落防止) |
| Feature ガード漏れで CI 失敗 | `#[cfg(feature="...")]` を DB/Auth/Search 関連型と wrapper に付与 |
| テストで鍵未設定 | 事前に `gen_biscuit_keys` 実行 or 環境変数 `BISCUIT_PRIVATE_KEY_B64`/`BISCUIT_PUBLIC_KEY_B64` を注入 |
| API キー検索が遅い | `lookup_hash` 追加済み。レガシー行は backfill CLI / ランタイム後書きで解消 |

### 追加情報リクエスト歓迎
不足: 例) CI ワークフロー概要 / 典型トレースログ / 監視メトリクス一覧 など要望があれば指示してください。ここに追補します。

（以上 迅速参照用。汎用アドバイスは省略し、このリポ特有の決まりのみ記載。）
