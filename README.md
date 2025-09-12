# 🚀 エンタープライズ向け CMS バックエンド

![CI](https://github.com/jungamer-64/RustCMS/actions/workflows/ci.yml/badge.svg)
![Docker Build](https://github.com/jungamer-64/RustCMS/actions/workflows/ci-docker-build.yml/badge.svg)
![Docker Release](https://github.com/jungamer-64/RustCMS/actions/workflows/docker-release.yml/badge.svg)
![Security Audit](https://github.com/jungamer-64/RustCMS/actions/workflows/security.yml/badge.svg)

高性能で本番運用に耐えるコンテンツ管理システム（CMS）API。Rust と Axum を用いて構築され、大規模トラフィックを想定したエンタープライズ向け機能を備えています。

## 🚀 主な特徴

### パフォーマンスとスケーラビリティ

- **高性能アーキテクチャ**: Rust と Axum による最大性能を意識した実装
- **データベース接続プーリング**: PostgreSQL 向けの最適化された接続管理
- **Redis キャッシュ**: レスポンスタイム向上のための多層キャッシュ戦略
- **レートリミット**: 悪用防止のためのインテリジェントなレート制御
- **ロードバランサ対応**: ステートレス設計で水平スケールが容易

### セキュリティ

- **Biscuit トークン (Capability ベース)**: Ed25519 署名付き権限デリゲーション。柔軟なポリシー表現とファクト駆動の権限管理を提供。
- **ロールベースアクセス制御**: 詳細な権限管理
- **入力検証**: リクエストの包括的なバリデーション
- **CORS 設定**: クロスオリジン制御の設定が可能
- **SQL インジェクション防止**: パラメタライズドクエリと型安全性
- **リフレッシュトークンローテーション**: セッションごとに refresh_version をインクリメントし使い捨て化、再利用攻撃を軽減。
- **API キー認証 (X-API-Key)**: サーバ生成の長期利用向けキー。Argon2 での秘密ハッシュ + SHA-256 (base64url) 決定的 lookup hash により O(1) 検索が可能。キー管理エンドポイントはユーザ認証 (Biscuit) 後に作成/一覧/失効操作を提供。

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

### API レスポンス統一 & コントラクトテスト

全エンドポイントは共通構造 `ApiResponse<T>` を返します。

```jsonc
// 成功
{
  "success": true,
  "data": { /* 任意のペイロード */ },
  "message": null,
  "error": null,
  "validation_errors": null
}

// バリデーションエラー
{
  "success": false,
  "data": null,
  "message": null,
  "error": "validation failed",
  "validation_errors": [
    {"field": "title", "message": "must not be empty"}
  ]
}
```

簡易な成功パスはハンドラで `ApiOk(payload)` を返すだけです。従来の `ok/err/ok_message` ヘルパは非推奨 (将来削除)。

#### スナップショット (contract) テスト
`tests/contract_snapshots.rs` で代表的な 4 パターン (成功 / 成功+message / エラー / バリデーションエラー) を `insta` で固定化しています。破壊的変更があると CI で差分が検出されます。

追加で `/health` エンドポイント形状を監視する **統合スナップショット** (`tests/integration_health_snapshot.rs`) を導入しています。これは本物のサービス初期化に依存せず、決定的なダミー `HealthStatus` を生成しタイムスタンプを `<redacted>` にマスクすることでインフラ非依存 & 変動値排除を実現しています。

拡張方針:

- 他エンドポイントを追加する際も「実サービス呼び出しを避けた合成データ or 最小スタブ」を返すテストハーネスを用意し、`insta::assert_json_snapshot!` で `ApiResponse` 形状 (特に `data` 部) を固定化。
- 不安定なフィールド (時刻 / ランダム ID / 並列で順序変動) は事前に書き換え or 削除してください。

実行例:

```powershell
# 既存スナップショットを含む全チェック
cargo insta test

# health のみ (普通の cargo test 経由)
cargo test snapshot_health_endpoint -- --exact
```

新規追加 → 差分確認 → 受け入れのフローは従来と同じです。

更新フロー:
```powershell
# 変更検証
cargo insta test

# 差分を確認 (対話)
cargo insta review

# 全て受け入れる (非推奨: 内容確認後に実行)
cargo insta accept
```

#### 重い鍵生成テストの高速化
`FAST_KEY_TESTS=1` を設定すると高速版圧縮テストのみ実行し、フルバックアップ/圧縮テストをスキップする CI マトリクス構成が可能です。
例: GitHub Actions でのステップ:

```yaml
- name: Fast tests
  run: FAST_KEY_TESTS=1 cargo test --all --no-fail-fast
```
長時間走る完全テストは nightly ジョブに分離する運用を推奨します。

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

## 🔐 認証（実装とドキュメントの同期）

- 標準ヘッダ: `Authorization: Bearer <your_token>`
- 互換許容: `Authorization: Biscuit <token>`（ミドルウェアで同等に検証）

公開/保護はルータ構成で管理します。

- 公開: `/api/v1/auth/register`, `/api/v1/auth/login`, `/api/v1/auth/refresh`, `/api/v1/health/**`, `/api/docs/**`, `/api/v1/metrics`
- 保護: `/api/v1/posts/**`, `/api/v1/users/**`, `/api/v1/api-keys/**`, `/api/v1/auth/logout`, `/api/v1/auth/profile`

保護ルートには認証ミドルウェアが適用され、ハンドラは `Extension<crate::auth::AuthContext>` で検証済みユーザ情報を利用できます。

### レガシー互換 Feature: `legacy-auth-flat`

統一認証レスポンス `AuthSuccessResponse` への移行に伴い、旧 `LoginResponse` スキーマはデフォルトで OpenAPI から除外されています。過去クライアント生成コードや差分検証用途で必要な場合のみ下記で有効化してください。

## Deprecated Admin Token Feature

`legacy-admin-token` feature を有効化すると、旧 admin token ベースの認証ロジック (`check_admin_token`, `get_admin_token`) が利用可能です。
デフォルトでは無効化されており、Biscuit 権限認証への移行を推奨します。

### Runtime Deprecation Warnings

以下の runtime 一度きり警告が出力されます (target:"deprecation"):

- フラット認証トークンフィールド利用 (feature `auth-flat-fields` 有効時)
- 旧 ADMIN_TOKEN 認証利用 (feature `legacy-admin-token` 有効時)

本番運用で警告抑止したい場合は該当 feature を無効化、またはログフィルタで target="deprecation" を除外してください。

### 収集メトリクス (Auth 統一関連)

`monitoring` フィーチャ有効時、以下の統一進捗カウンタが公開 (Prometheus exporter 経由) されます。

| Metric | 意味 | 減少完了条件 |
|--------|------|--------------|
| `auth_flat_fields_legacy_usage_total` | 旧フラットフィールドを含む `AuthSuccessResponse` が構築された回数 | 本番トラフィックで 0 維持 → `auth-flat-fields` 無効化準備完了 |
| `legacy_login_response_conversion_total` | `LoginResponse` (互換) へ変換が行われた回数 | 0 維持 → `legacy-auth-flat` 削除準備完了 |

ダッシュボード例:

```promql
increase(auth_flat_fields_legacy_usage_total[24h])
increase(legacy_login_response_conversion_total[24h])
```

どちらも 0 が安定した期間 (例: 7–14 日) が確保できれば次期メジャー削除 PR を作成してください。

補助スクリプト:

```bash
# 現在の src/ 残存を厳格判定 (ゼロ以外があれば exit 1)
bash scripts/deprecation-strict-check.sh

# 3 連続ゼロ到達トラッキング & 推奨手順表示
bash scripts/deprecation-auto-guidance.sh

# Phase 4 削除 PR ドラフト生成 (ゼロ確認後)
bash scripts/generate_phase4_pr.sh
```

Grafana ダッシュボード例: `monitoring/grafana/auth_unification_dashboard.json` をインポート。

### CI アーティファクト: Deprecated Scan

`ci.yml` の `deprecated-scan` ジョブは `scripts/deprecation-scan.sh --src-only` を実行し以下をアップロードします:

- `deprecation_counts.csv`
- `deprecation_counts.json`

`*.json` はダウンロード後グラフ化や履歴比較に利用できます。`--src-only` によりテスト/ドキュメント由来の許容参照を除外し、実稼働コードの残存状況のみを追跡します。

将来的にゼロ到達後は `--strict` を有効化し回帰を CI 失敗に昇格させることを推奨します。

### Auth Unification Verification (デュアル + 除去プレビュー)

統一認証レスポンスの両構成（フラット互換あり/なし）を検証するヘルパースクリプト:

```bash
./scripts/verify-auth-unification.sh
```

内部で以下を実行します:

- デフォルト (auth-flat-fields 有効) 全テスト
- フラット互換フィールド無効構成 (--no-default-features + 必要最小 features) 全テスト

CI ではさらに `no-flat` マトリクス (auth-flat-fields だけを無効化し他主要機能は維持) を常時実行し、将来 (Phase4) の削除後状態を継続的に検証します。

補助スキャン (任意 CI informational ジョブ例):

```bash
./scripts/deprecation-scan.sh | tee deprecation_scan.txt
```

生成された一覧をアーティファクト化し、実際に残すべき参照 (テスト or docs) 以外が残っていないかを確認してください。

Phase4 直前チェック:

```bash
./scripts/phase4-removal-plan.sh
```

出力が空 (または docs/ / CHANGELOG のみ) になれば最終削除 PR を作成可能です。

```bash
cargo build --features legacy-auth-flat
```

有効化すると以下が有効になります:

- `LoginResponse` 型がコンパイルされ OpenAPI に `LoginResponse` スキーマが追加
- 互換 JSON フラットフィールド (access_token 等) は feature `auth-flat-fields` で提供 (デフォルト有効 / 3.0.0 で削除予定)

将来のメジャーリリースでこの feature とフラットフィールドは削除予定です。新規実装は `response.tokens.*` を参照してください。

### 🧭 将来の Breaking (予告 / 3.0.0 計画)

次期メジャー (3.0.0) では以下を削除予定です:

1. Cargo feature `legacy-auth-flat`
2. フラットフィールド: `access_token`, `refresh_token`, `biscuit_token`, `expires_in`, `session_id`, `token`
3. 互換用 `LoginResponse` スキーマ

移行ガイド (要約):

- すべての直接参照を `response.tokens.*` へ置換 (例: `response.access_token` → `response.tokens.access_token`)
- `token` (access_token エイリアス) を除去
- OpenAPI 生成キャッシュをクリアし SDK 再生成

詳細: `docs/AUTH_MIGRATION_V2.md` の Phase 4 セクションおよび `CHANGELOG.md` の Planned を参照。



## �🛠️ クイックスタート

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

Developer 補助ツール:

- `gen_biscuit_keys` : Biscuit トークン用の鍵ペアを生成するユーティリティ。生成した鍵は標準出力へ base64 で出力されます。ファイルや `.env` へ書き込むオプションもあり、安全に保存することを推奨します。
  - バージョン管理 (`--versioned`) で `biscuit_private_v<N>.b64` / `biscuit_public_v<N>.b64` を連番保存し、`--latest-alias` で常に最新を `biscuit_private.b64` / `biscuit_public.b64` として同期。
  - `manifest.json` に最新バージョン番号と fingerprint (SHA-256) を記録。
  - `--prune <N>` で最新 N バージョンのみ保持、古いものを自動削除。
  - `--list` で存在するバージョン列挙。
  - バックアップ系: `--backup` (上書き前に `.bak.<unix_ts>` へ退避), `--max-backups N`, `--backup-compress` (gzip 圧縮 & 元削除), `--backup-dir DIR`。
  - 出力モード: `--format stdout|files|env|both` (未指定時は従来: 指定ターゲットのみ)。

使い方（簡易）:

```powershell
# stdout に出力のみ
cargo run --bin gen_biscuit_keys

# 鍵ファイルを書き出す（ディレクトリ: keys）
cargo run --bin gen_biscuit_keys -- --format files --out-dir keys

# .env に追記（デフォルト: .env）
cargo run --bin gen_biscuit_keys -- --format env --env-file .env

# 両方に出力して既存を上書き（強制）
cargo run --bin gen_biscuit_keys -- --format both --out-dir keys --env-file .env --force

# 既存をバックアップしてから上書き（バックアップは .bak.<unix_ts>）
cargo run --bin gen_biscuit_keys -- --format files --out-dir keys --backup --force

# バックアップを作成して最新2個だけ保持する
cargo run --bin gen_biscuit_keys -- --format files --out-dir keys --backup --max-backups 2 --force

# バックアップを作成して gzip 圧縮（元ファイルは削除され .gz が残る）
cargo run --bin gen_biscuit_keys -- --format files --out-dir keys --backup --backup-compress --force

# バージョン付きで 3 個だけ保持し manifest 更新、最新 alias も更新
cargo run --bin gen_biscuit_keys -- --format files --out-dir keys --versioned --latest-alias --prune 3 --force

# 既存バージョン列挙
cargo run --bin gen_biscuit_keys -- --out-dir keys --list
```

セキュリティ注意点:

- 生成された秘密鍵は厳重に管理してください。パスワード管理ツールや本番では Doppler / Vault 等のシークレットストアを利用してください。
- リポジトリやコミットに鍵を含めないでください。`.env` をコミットしない（`.gitignore` に含める）か、環境シークレット管理を用いてください。
- バージョン運用時は古い秘密鍵が不要になったタイミングで安全に破棄 (安全な削除/秘匿ストレージ除外) してください。

### API キー認証の概要

| 項目 | 内容 |
|------|------|
| ヘッダ | `X-API-Key: ak_<ランダム>` |
| 保存方式 | Argon2 ハッシュ (秘密) + ルックアップ用 SHA-256(base64url) |
| 失効 | Revoke (DB で削除 / expired_at 設定) |
| パーミッション | 文字列配列 (OpenAPI 拡張で列挙) |
| 所有者検証 | Revoke / List は作成ユーザのみ |
| セキュリティ | 生キーは一度しか返さない (クライアント安全保管必須) |

API キーは長期自動処理 (CI / バッチ / 外部統合) 用。ユーザ認証後に管理エンドポイントで発行し、生キーはそのレスポンス以外では再取得不可。DB では **復号不能** (Argon2) かつ検索高速化のための決定的 `lookup_hash` (衝突極小) を持ちます。

 
#### レガシー行バックフィル

初期段階で `lookup_hash` 空の行が存在する場合、ミドルウェアは検証成功時にその行へハッシュを後書きし徐々に移行します。運用 CLI (backfill) により一括同定・失効も可能です。

`backfill_api_key_lookup` バイナリ:

```powershell
cargo run --features "database,auth" --bin backfill_api_key_lookup
```

オプション:

| フラグ | 説明 |
|--------|------|
| `--expire` | 対象レガシーキーの `expires_at` を即時に設定して失効させる |
| `--dry-run` | 書き込みを行わずスキャン結果のみ表示 (レポートは常に JSON) |
| `--pretty` | 整形 JSON で出力 |

出力例:

```json
{
  "scanned": 2,
  "legacy_missing_lookup": 2,
  "expired_marked": 0,
  "rows": [
    {"id":"...","name":"ci-bot","user_id":"...","created_at":"2025-09-04T08:10:22Z"},
    {"id":"...","name":"legacy-agent","user_id":"...","created_at":"2025-09-03T11:44:55Z"}
  ],
  "expire_mode": false,
  "dry_run": true
}
```

推奨運用フロー:

1. `--dry-run --pretty` で影響範囲を確認
2. 問題なければ `--expire` を付与して失効 (再発行を促す)
3. メトリクス / ログでアクセス失敗が急増しないか監視

注意: raw API キーは保存していないためこのツールで再計算はできません (完全な後付け再構築は不可能)。

#### レート制限 (失敗回数ベース: In-Memory / Redis)

60 秒ウィンドウで同一 lookup hash に紐づく認証失敗 (not_found / hash_mismatch / malformed / expired など) が **10 回** を超えると 429 (Too Many Requests) を返し短期ブルートフォースを緩和します。

バックエンド:

| backend | 指定 | 特徴 |
|---------|------|------|
| In-Memory (デフォルト) | `API_KEY_FAIL_BACKEND=memory` または未設定 | ライト・単一プロセス向け。高速、プロセス間共有なし。|
| Redis | `API_KEY_FAIL_BACKEND=redis` (要 feature `cache`) | 分散 / 水平スケール対応。各失敗は Redis `INCR` + `EXPIRE` (固定ウィンドウ擬似) でカウント。|

Redis バックエンド時の注意:

1. `REDIS_URL` が必須。
2. TTL はウィンドウ秒で `EXPIRE`。閾値超過判定は `INCR` 後の値が `threshold` を超えたかで実施。
3. `tracked_len` メトリクスは 15 秒キャッシュされた SCAN 結果。大量キー環境ではオーバーヘッド低減のため頻度を抑制。
4. キープレフィックスは `API_KEY_FAIL_REDIS_PREFIX` (デフォルト `rk:`) で変更可能。衝突回避のためサービス毎に prefix 設定推奨。


成功時にはそのキーの失敗カウンタを即座に削除し (in-memory: map remove / redis: DEL) 正常利用を阻害しません。

環境変数 (未設定時はデフォルト):

| 変数 | デフォルト | 説明 |
|------|------------|------|
| `API_KEY_FAIL_WINDOW_SECS` | `60` | 固定ウィンドウ秒数 |
| `API_KEY_FAIL_THRESHOLD` | `10` | ウィンドウ内の許容失敗回数 (超過で 429) |
| `API_KEY_FAIL_MAX_TRACKED` | `5000` | (memory) 失敗カウンタを保持する lookup_hash の最大件数 (超過時は古いエントリを削除) |
| `API_KEY_FAIL_DISABLE` | `false` | `true`/`1` でレート制限ロジックを無効化 (計測は一部継続) |
| `API_KEY_FAIL_BACKEND` | `memory` | `memory` / `redis` を選択 (redis 利用には feature `cache` + `REDIS_URL`) |
| `API_KEY_FAIL_REDIS_PREFIX` | `rk:` | Redis backend で使用するキー prefix (衝突回避用) |

容量制御挙動: 追跡件数が最大の 90% を超えた時点でウィンドウ外の古いエントリを opportunistic に掃除し、なお超過する場合は最も古いエントリを 1 件強制削除します。

## ✅ 統一レスポンス仕様 (Unified API Response Layer)

すべての成功レスポンスは `ApiResponse<T>` へ正規化され、ハンドラでは `ApiOk(value)` を返すだけで以下 JSON 形状になります:

```json
{
  "success": true,
  "data": { /* value */ },
  "message": null,
  "error": null,
  "validation_errors": null
}
```

バリデーション / ドメインエラー時は:

```json
{
  "success": false,
  "data": null,
  "message": null,
  "error": "Invalid input",
  "validation_errors": [
    {"field": "title", "message": "must not be empty"}
  ]
}
```

### 利用パターン

| 目的 | ハンドラ戻り値例 | 備考 |
|------|-----------------|------|
| 通常成功 | `ApiOk(entity)` | `entity` は `Serialize` |
| 作成 (201) | `(StatusCode::CREATED, ApiOk(created))` | タプルで任意ステータス |
| ページング | `ApiOk(Paginated<T>)` | 内部 `data` にそのまま格納 |
| メッセージのみ | `ApiOk(json!({"message":"Done"}))` | シンプルテキストラップ不要 |

### 廃止予定 API

| 項目 | 状態 | 代替 |
|------|------|------|
| `IntoApiOk` トレイト | deprecated | `ApiOk(...)` |
| 直接 `ok(value)` ヘルパ | 移行段階 | `ApiOk(value)` |

### 実装メモ (開発者向け)

1. `ApiOk<T>` は `IntoResponse` 実装を持ち `ApiResponse::success(T)` を包む。
2. エラー (`AppError`) は `IntoResponse` 実装内で同じ `ApiResponse` 形状へ統一。
3. OpenAPI では `ApiResponse<serde_json::Value>` + `ApiResponseExample` をスキーマ提供。
4. バリデーション詳細は `validation_errors` 配列 (省略時非表示)。

将来: `IntoApiOk` は段階的に削除予定。コードベース内に存在する場合は PR で置換してください。

### メトリクス (主要抜粋)

| 名前 | ラベル | 説明 |
|------|--------|------|
| `http_requests_total` | method, path | 全リクエスト数 |
| `http_requests_success_total` | status_class | 2xx 成功数 |
| `http_requests_client_error_total` | – | 4xx 数 |
| `http_requests_server_error_total` | – | 5xx 数 |
| `database_queries_total` | model, op | DB クエリ回数 |
| `api_key_auth_attempts_total` | – | API キー認証試行 |
| `api_key_auth_success_total` | – | API キー成功 |
| `api_key_auth_failure_total` | reason | API キー失敗 (missing_header / malformed / not_found / hash_mismatch / expired / rate_limited / invalid_header_encoding) |
| `rate_limit_violations_total` | scope | (他レートリミット用) |

Prometheus 例 (簡易):

```text
api_key_auth_attempts_total 42
api_key_auth_failure_total{reason="not_found"} 7
api_key_auth_success_total 35
```

ダッシュボードでは (success / attempts) 比率や reason 別失敗をウォッチし、異常増加 (例: not_found 連続増) 時にアラート設定することを推奨します。

サンプル Alert ルール: `docs/alerts/prometheus_rules_example.yml` に API キー関連のスパイク検出・成功率低下・期限切れ利用・rate_limited 連発検知の例を用意しています。環境のトラフィック特性に合わせて閾値を調整してください。

関連メトリクス (monitoring feature 有効時):

| メトリクス | 説明 |
|------------|------|
| `api_key_rate_limit_window_seconds` | 現在のウィンドウ秒数 |
| `api_key_rate_limit_threshold` | 閾値 (失敗許容回数) |
| `api_key_rate_limit_max_tracked` | memory backend の最大トラック件数 |
| `api_key_rate_limit_tracked_keys` | 現在追跡中のキー件数 (redis backend は SCAN キャッシュ) |
| `api_key_rate_limit_enabled` | 有効=1 / 無効=0 |
| `api_key_rate_limit_max_tracked` | 最大追跡キー数 |
| `api_key_rate_limit_tracked_keys` | 現在追跡中キー数 (動的) |
| `api_key_rate_limit_enabled` | 1=有効 / 0=無効 (`API_KEY_FAIL_DISABLE` 反映) |


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

保護されたエンドポイントは Authorization ヘッダに Biscuit アクセストークン (Bearer) を必要とします:

```http
Authorization: Bearer <your-biscuit-token>
```

または Biscuit を利用する場合:

```http
Authorization: Biscuit <base64-biscuit-token>
```

OpenAPI では多くの保護エンドポイントで `security` 配列に `[ {"BearerAuth": []}, {"BiscuitAuth": []} ]` が付与されており、クライアントはどちらか一方を提示すれば認証が成立します (OR 条件)。公開エンドポイント (login / register / refresh / search など) は `security` 無しです。

より高度な Biscuit のスコープ/ケイパビリティ定義例は `docs/BISCUIT.md` を参照してください。

### リクエスト例（主要ルート）

#### ログインしてトークンを取得

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "demo_user",
    "password": "<your_password>"
  }'
```

#### 投稿の作成

```bash
curl -X POST http://localhost:3000/api/v1/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_token>" \
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

- **Biscuit 認証**: Capability ベースのトークン認証 (単一方式に統一)
- **入力検証**: カスタムエラーハンドリングを含む包括的な検証
- **SQL インジェクション防止**: パラメタライズドクエリと型安全性
- **CORS 保護**: 設定可能なクロスオリジン制御
- **Refresh Token Rotation**: 盗難リフレッシュトークン無効化のためのバージョン付き JTI 戦略

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
